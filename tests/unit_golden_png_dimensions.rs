mod common;

use common::render_helpers;

/// Labelary reference dimensions: 101.625 mm × 203.25 mm at 8 dpmm (labels/ and root).
const LABELARY_WIDTH: u32 = 813;
const LABELARY_HEIGHT: u32 = 1626;

/// Unit test dimensions: 50 mm × 80 mm at 8 dpmm (unit/ golden files).
const UNIT_WIDTH: u32 = 400;
const UNIT_HEIGHT: u32 = 640;

/// Verify that every golden PNG has the expected dimensions for its directory.
/// - `testdata/` and `testdata/labels/` → 813 × 1626 px (Labelary reference)
/// - `testdata/unit/` → 400 × 640 px (smaller for easier visual comparison)
#[test]
fn all_golden_pngs_have_standard_dimensions() {
    let dir = render_helpers::testdata_dir();

    let scan_dirs: Vec<(std::path::PathBuf, u32, u32)> = vec![
        (dir.clone(), LABELARY_WIDTH, LABELARY_HEIGHT),
        (dir.join("labels"), LABELARY_WIDTH, LABELARY_HEIGHT),
        (dir.join("unit"), UNIT_WIDTH, UNIT_HEIGHT),
    ];

    let mut checked = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for (scan_dir, expected_w, expected_h) in &scan_dirs {
        for entry in std::fs::read_dir(scan_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unknown>")
                .to_string();

            let bytes =
                std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", name, e));
            let img = image::load_from_memory(&bytes)
                .unwrap_or_else(|e| panic!("cannot decode {}: {}", name, e));

            let (w, h) = (img.width(), img.height());
            if w != *expected_w || h != *expected_h {
                failures.push(format!(
                    "  {} — {}×{} (expected {}×{})",
                    name, w, h, expected_w, expected_h
                ));
            }
            checked += 1;
        }
    }

    assert!(
        checked > 0,
        "no PNG files found in {:?} — check the testdata directory",
        dir
    );

    assert!(
        failures.is_empty(),
        "{} golden PNG(s) have non-standard dimensions:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
