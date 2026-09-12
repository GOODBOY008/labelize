/// Auto-discovers all ZPL/EPL test files under testdata/, renders them, compares
/// against reference PNGs, and produces two detailed diff-percentage reports:
/// - `testdata/diffs/diff_report_labels.txt` — carrier/real-world labels (813×1626)
/// - `testdata/diffs/diff_report_unit.txt` — unit/synthetic tests (813×1626)
///
/// Inputs and reference PNGs must exist. Validation never creates references or uses the network.
///
/// Run with:
///   cargo test --test e2e_diff_report diff_report -- --nocapture
mod common;
#[path = "common/fixture_test_dir.rs"]
mod fixture_test_dir;

use common::image_compare;
#[path = "common/golden_fixture.rs"]
mod golden_fixture;
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

/// Scan a set of directories for ZPL/EPL files, render, and compare against reference PNGs.
/// Renders with `default_options()` (813×1626); reads committed references unchanged.
fn scan_dirs(dirs: &[std::path::PathBuf]) -> Result<Vec<ReportEntry>, String> {
    let mut entries: Vec<ReportEntry> = Vec::new();

    let label_files = golden_fixture::discover_inputs(dirs)?;

    for path in &label_files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let ext = path.extension().unwrap().to_string_lossy().to_string();
        let fixture = golden_fixture::load_fixture(path)?;
        let content = fixture.content;

        let opts = render_helpers::default_options();

        let actual_png = match ext.as_str() {
            "epl" => std::panic::catch_unwind(|| {
                render_helpers::render_epl_to_png(&content, opts.clone())
            }),
            _ => std::panic::catch_unwind(|| {
                render_helpers::render_zpl_to_png(&content, opts.clone())
            }),
        };

        let actual_png =
            actual_png.map_err(|_| format!("cannot render fixture {}", path.display()))?;
        let expected_png = fixture.reference;
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

    Ok(entries)
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
    let entries = scan_dirs(&dirs).unwrap_or_else(|e| panic!("{e}"));
    let report = format_report("Labels Diff Report (813×1626)", &entries);

    println!("\n{}", report);
    save_report("diff_report_labels.txt", &report);
    check_high_diffs(&entries);
}

#[test]
fn diff_report_unit() {
    let dir = render_helpers::testdata_dir();
    let dirs = vec![dir.join("unit")];
    let entries = scan_dirs(&dirs).unwrap_or_else(|e| panic!("{e}"));
    let report = format_report("Unit Diff Report (813×1626)", &entries);

    println!("\n{}", report);
    save_report("diff_report_unit.txt", &report);
    check_high_diffs(&entries);
}

#[test]
fn diff_report_contract_rejects_missing_and_corrupt_references() {
    for extension in ["zpl", "epl"] {
        let dir = fixture_test_dir::TestDir::new();
        let input = dir.0.join(format!("case.{extension}"));
        std::fs::write(&input, "input").unwrap();
        let reference = input.with_extension("png");
        let error = scan_dirs(std::slice::from_ref(&dir.0))
            .err()
            .expect("missing reference must fail");
        assert!(error.contains(&reference.display().to_string()), "{error}");
        assert!(!reference.exists());
        std::fs::write(&reference, "broken PNG").unwrap();
        let error = scan_dirs(std::slice::from_ref(&dir.0))
            .err()
            .expect("corrupt reference must fail");
        assert!(error.contains("invalid golden reference"), "{error}");
        assert_eq!(std::fs::read(&reference).unwrap(), b"broken PNG");
    }
}

#[test]
fn diff_report_contract_render_errors_fail_instead_of_passing_with_an_error_row() {
    let dir = fixture_test_dir::TestDir::new();
    let input = dir.0.join("empty.zpl");
    std::fs::write(&input, "^XA^XZ").unwrap();
    std::fs::write(input.with_extension("png"), fixture_test_dir::png()).unwrap();
    let error = scan_dirs(std::slice::from_ref(&dir.0))
        .err()
        .expect("no rendered label must fail");
    assert!(error.contains(&input.display().to_string()), "{error}");
}
