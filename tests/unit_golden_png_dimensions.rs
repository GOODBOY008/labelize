mod common;

use common::render_helpers;

/// Standard golden-file dimensions: 101.625 mm × 203.25 mm at 8 dpmm.
const EXPECTED_WIDTH: u32 = 813;
const EXPECTED_HEIGHT: u32 = 1626;

/// Verify that every golden PNG in the testdata directories has the standard
/// Labelary reference dimensions (813 × 1626 px).  Non-standard dimensions would
/// cause the diff comparisons in e2e_golden to count size-mismatch pixels as
/// differences, making the reported diff percentages unreliable.
#[test]
fn all_golden_pngs_have_standard_dimensions() {
    let dir = render_helpers::testdata_dir();

    let scan_dirs = [dir.clone(), dir.join("labels"), dir.join("unit")];

    let mut checked = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for entry in scan_dirs
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
    {
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

        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", name, e));
        let img = image::load_from_memory(&bytes)
            .unwrap_or_else(|e| panic!("cannot decode {}: {}", name, e));

        let (w, h) = (img.width(), img.height());
        if w != EXPECTED_WIDTH || h != EXPECTED_HEIGHT {
            failures.push(format!(
                "  {} — {}×{} (expected {}×{})",
                name, w, h, EXPECTED_WIDTH, EXPECTED_HEIGHT
            ));
        }
        checked += 1;
    }

    assert!(
        checked > 0,
        "no PNG files found in {:?} — check the testdata directory",
        dir
    );

    assert!(
        failures.is_empty(),
        "{} golden PNG(s) have non-standard dimensions (expected {}×{}):\n{}",
        failures.len(),
        EXPECTED_WIDTH,
        EXPECTED_HEIGHT,
        failures.join("\n")
    );
}
