/// Auto-discovers all ZPL/EPL test files under testdata/, renders them, compares
/// against reference PNGs, and produces a detailed diff-percentage report.
///
/// Run with:
///   cargo test --test e2e_diff_report diff_report -- --nocapture
///
/// The report is also written to `testdata/diffs/diff_report.txt`.
mod common;

use common::image_compare;
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

/// Discover and render all testdata label files, compare against reference PNGs.
fn generate_diff_report() -> Vec<ReportEntry> {
    let dir = render_helpers::testdata_dir();
    let mut entries: Vec<ReportEntry> = Vec::new();

    let scan_dirs = [
        dir.clone(),
        dir.join("labels"),
        dir.join("unit"),
    ];
    let mut label_files: Vec<_> = scan_dirs
        .iter()
        .flat_map(|d| {
            std::fs::read_dir(d).into_iter().flatten().flatten()
        })
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

        if !ref_png.exists() {
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

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let opts = render_helpers::default_options();

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

        // Save diff image when there are differences
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(&name, diff_img);
        }

        // Save side-by-side comparison image (left=Labelary, right=Labelize)
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

fn format_report(entries: &[ReportEntry]) -> String {
    let mut report = String::new();
    writeln!(
        report,
        "╔══════════════════════════════════════════════════════════════════════════════╗"
    )
    .unwrap();
    writeln!(
        report,
        "║                    ZPL/EPL Rendering Diff Report                            ║"
    )
    .unwrap();
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

#[test]
fn diff_report_all() {
    let entries = generate_diff_report();
    let report = format_report(&entries);

    // Print to stdout
    println!("\n{}", report);

    // Save to file
    let report_path = render_helpers::testdata_dir()
        .join("diffs")
        .join("diff_report.txt");
    std::fs::create_dir_all(report_path.parent().unwrap()).ok();
    let mut f = std::fs::File::create(&report_path).expect("create report file");
    f.write_all(report.as_bytes()).expect("write report");

    println!("Report saved to: {}", report_path.display());

    // Collect entries with HIGH diff for assertion message
    let high_diffs: Vec<&ReportEntry> = entries
        .iter()
        .filter(|e| e.status == "HIGH(>=15%)")
        .collect();
    if !high_diffs.is_empty() {
        let mut msg = String::from("The following test cases have HIGH diff (>=15%):\n");
        for e in &high_diffs {
            writeln!(msg, "  - {}.{}: {:.2}%", e.name, e.ext, e.diff_percent).unwrap();
        }
        eprintln!("{}", msg);
        // Don't fail the test - this is a report. Individual golden tests enforce tolerance.
    }
}
