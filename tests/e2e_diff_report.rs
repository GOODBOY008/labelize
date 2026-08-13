/// Auto-discovers all ZPL/EPL test files under testdata/, renders them, compares
/// against reference PNGs, and produces two detailed diff-percentage reports:
/// - `testdata/diffs/diff_report_labels.txt` — carrier/real-world labels (813×1626)
/// - `testdata/diffs/diff_report_unit.txt` — unit/synthetic tests (813×1626)
///
/// Missing reference PNGs are auto-generated: ZPL files are fetched from Labelary
/// (813×1626 via default_options dimensions), EPL files fall back to our renderer.
///
/// Run with:
///   cargo test --test e2e_diff_report diff_report -- --nocapture
mod common;

use common::image_compare;
use common::labelary_client;
use common::render_helpers;
use std::fmt::Write as FmtWrite;
use std::io::Write;

/// Entry for a single test case in the report.
struct ReportEntry {
    name: String,
    ext: String,
    diff_percent: f64,
    actual_dims: (u32, u32),
    expected_dims: (u32, u32),
    status: &'static str,
}

/// Canvas size produced by `default_options()` (813×1626 px).
const CANVAS_W: u32 = 813;
const CANVAS_H: u32 = 1626;

/// Generate a reference PNG for `path` if it does not already exist.
///
/// ZPL: tries Labelary first at `default_options()` dimensions. If Labelary returns a
/// PNG at a different size (e.g. 812×1624 due to server-side rounding) it is padded to
/// 813×1626 with white so the reference always matches our renderer canvas and
/// size-mismatch noise is eliminated from diff results.
/// EPL: always uses the renderer (Labelary is ZPL-only).
fn ensure_ref_png(path: &std::path::Path, content: &str, ext: &str) {
    if path.exists() {
        return;
    }
    let opts = render_helpers::default_options_grayscale();
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let png = if ext != "epl" {
        let w = opts.label_width_mm / 25.4;
        let h = opts.label_height_mm / 25.4;
        labelary_client::labelary_render(content, opts.dpmm as u8, w, h)
            .map(|fetched| {
                let normalized = labelary_client::pad_png_to_size(&fetched, CANVAS_W, CANVAS_H);
                eprintln!(
                    "[bootstrap] {}: fetched from Labelary, normalized to {}×{}",
                    name, CANVAS_W, CANVAS_H
                );
                normalized
            })
            .unwrap_or_else(|| {
                eprintln!(
                    "[bootstrap] {}: Labelary unavailable — renderer baseline",
                    name
                );
                render_helpers::render_zpl_to_png(content, opts)
            })
    } else {
        eprintln!("[bootstrap] {}: EPL — renderer baseline", name);
        render_helpers::render_epl_to_png(content, opts)
    };
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    std::fs::write(path, &png).expect("write auto-generated ref PNG");
}

/// Scan a set of directories for ZPL/EPL files, render, and compare against reference PNGs.
/// Always uses `default_options()` (813×1626) for both rendering and Labelary fetches so
/// that the render canvas and the reference image are the same size.
fn scan_dirs(dirs: &[std::path::PathBuf]) -> Vec<ReportEntry> {
    let mut entries: Vec<ReportEntry> = Vec::new();

    let mut label_files: Vec<_> = dirs
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .filter(|e| {
            let ext = e
                .path()
                .extension()
                .map(|x| x.to_string_lossy().to_string());
            matches!(ext.as_deref(), Some("zpl") | Some("epl"))
        })
        .map(|e| e.path())
        .collect();
    label_files.sort();

    for path in &label_files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let ext = path.extension().unwrap().to_string_lossy().to_string();
        let ref_png = path.parent().unwrap().join(format!("{}.png", name));

        let content = std::fs::read_to_string(path).unwrap_or_default();

        // Auto-generate the reference PNG if missing (Labelary → renderer fallback).
        ensure_ref_png(&ref_png, &content, &ext);

        if !ref_png.exists() {
            // Should only happen if both Labelary and renderer failed unexpectedly.
            entries.push(ReportEntry {
                name: name.clone(),
                ext: ext.clone(),
                diff_percent: -1.0,
                actual_dims: (0, 0),
                expected_dims: (0, 0),
                status: "SKIP(no ref)",
            });
            continue;
        }

        let opts = render_helpers::default_options_grayscale();

        let actual_png = match ext.as_str() {
            "epl" => std::panic::catch_unwind(|| {
                render_helpers::render_epl_to_png(&content, opts.clone())
            }),
            _ => std::panic::catch_unwind(|| {
                render_helpers::render_zpl_to_png(&content, opts.clone())
            }),
        };

        let actual_png = match actual_png {
            Ok(png) => png,
            Err(_) => {
                entries.push(ReportEntry {
                    name: name.clone(),
                    ext: ext.clone(),
                    diff_percent: -1.0,
                    actual_dims: (0, 0),
                    expected_dims: (0, 0),
                    status: "ERR(render)",
                });
                continue;
            }
        };

        let expected_png = std::fs::read(&ref_png).expect("read reference");
        let result = image_compare::compare_images(&actual_png, &expected_png, 0.0);

        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(&name, diff_img);
        }

        if let (Some(ref expected_img), Some(ref actual_img)) =
            (&result.expected_image, &result.actual_image)
        {
            image_compare::save_comparison_image(&name, expected_img, actual_img);
        }

        let status = if result.diff_percent == 0.0 {
            "PERFECT"
        } else if result.diff_percent < 1.0 {
            "GOOD(<1%)"
        } else if result.diff_percent < 5.0 {
            "MINOR(<5%)"
        } else if result.diff_percent < 15.0 {
            "MODERATE(<15%)"
        } else {
            "HIGH(>=15%)"
        };

        entries.push(ReportEntry {
            name: name.clone(),
            ext: ext.clone(),
            diff_percent: result.diff_percent,
            actual_dims: result.actual_dims,
            expected_dims: result.expected_dims,
            status,
        });
    }

    entries
}

fn format_report(title: &str, entries: &[ReportEntry]) -> String {
    let mut report = String::new();
    writeln!(
        report,
        "╔══════════════════════════════════════════════════════════════════════════════╗"
    )
    .unwrap();
    writeln!(report, "║ {:^76} ║", title).unwrap();
    writeln!(
        report,
        "╠══════════════════════════════════════════════════════════════════════════════╣"
    )
    .unwrap();
    writeln!(
        report,
        "║ {:30} │ {:4} │ {:>8} │ {:>13} │ {:>13} │ {:12} ║",
        "Name", "Ext", "Diff%", "Actual(WxH)", "Expected(WxH)", "Status"
    )
    .unwrap();
    writeln!(
        report,
        "╠══════════════════════════════════════════════════════════════════════════════╣"
    )
    .unwrap();

    let mut perfect = 0usize;
    let mut good = 0usize;
    let mut minor = 0usize;
    let mut moderate = 0usize;
    let mut high = 0usize;
    let mut skipped = 0usize;
    let mut errored = 0usize;

    for e in entries {
        let dims_actual = if e.actual_dims == (0, 0) {
            "N/A".to_string()
        } else {
            format!("{}x{}", e.actual_dims.0, e.actual_dims.1)
        };
        let dims_expected = if e.expected_dims == (0, 0) {
            "N/A".to_string()
        } else {
            format!("{}x{}", e.expected_dims.0, e.expected_dims.1)
        };
        let diff_str = if e.diff_percent < 0.0 {
            "N/A".to_string()
        } else {
            format!("{:.2}%", e.diff_percent)
        };

        writeln!(
            report,
            "║ {:30} │ {:4} │ {:>8} │ {:>13} │ {:>13} │ {:12} ║",
            e.name, e.ext, diff_str, dims_actual, dims_expected, e.status
        )
        .unwrap();

        match e.status {
            "PERFECT" => perfect += 1,
            "GOOD(<1%)" => good += 1,
            "MINOR(<5%)" => minor += 1,
            "MODERATE(<15%)" => moderate += 1,
            "HIGH(>=15%)" => high += 1,
            "SKIP(no ref)" => skipped += 1,
            _ => errored += 1,
        }
    }

    let total = entries.len();
    writeln!(
        report,
        "╠══════════════════════════════════════════════════════════════════════════════╣"
    )
    .unwrap();
    writeln!(report, "║ Summary: {} total │ {} perfect │ {} good │ {} minor │ {} moderate │ {} high │ {} skip │ {} err",
        total, perfect, good, minor, moderate, high, skipped, errored).unwrap();
    writeln!(
        report,
        "╚══════════════════════════════════════════════════════════════════════════════╝"
    )
    .unwrap();

    report
}

fn save_report(filename: &str, report: &str) {
    let report_path = render_helpers::testdata_dir().join("diffs").join(filename);
    std::fs::create_dir_all(report_path.parent().unwrap()).ok();
    let mut f = std::fs::File::create(&report_path).expect("create report file");
    f.write_all(report.as_bytes()).expect("write report");
    println!("Report saved to: {}", report_path.display());
}

fn check_high_diffs(entries: &[ReportEntry]) {
    let high_diffs: Vec<&ReportEntry> = entries
        .iter()
        .filter(|e| e.status == "HIGH(>=15%)")
        .collect();
    if !high_diffs.is_empty() {
        let mut msg = String::from("HIGH diff entries (>=15%):\n");
        for e in &high_diffs {
            writeln!(msg, "  - {}.{}: {:.2}%", e.name, e.ext, e.diff_percent).unwrap();
        }
        eprintln!("{}", msg);
    }
}

#[test]
fn diff_report_labels() {
    let dir = render_helpers::testdata_dir();
    let dirs = vec![dir.clone(), dir.join("labels")];
    let entries = scan_dirs(&dirs);
    let report = format_report("Labels Diff Report (813×1626)", &entries);

    println!("\n{}", report);
    save_report("diff_report_labels.txt", &report);
    check_high_diffs(&entries);
}

#[test]
fn diff_report_unit() {
    let dir = render_helpers::testdata_dir();
    let dirs = vec![dir.join("unit")];
    let entries = scan_dirs(&dirs);
    let report = format_report("Unit Diff Report (813×1626)", &entries);

    println!("\n{}", report);
    save_report("diff_report_unit.txt", &report);
    check_high_diffs(&entries);
}
