mod common;

use common::render_helpers;

/// Labelary reference dimensions: 101.625 mm × 203.25 mm at 8 dpmm (labels/ and root).
const LABELARY_WIDTH: u32 = 813;
const LABELARY_HEIGHT: u32 = 1626;

/// Unit test reference dimensions.
/// Labelary-bootstrapped goldens are 812 × 1624 px (101.5mm × 203.0mm at 8 dpmm).
/// Renderer-baseline goldens (produced via LABELIZE_UPDATE_GOLDEN) are 813 × 1626 px
/// (default_options canvas — same as labels/).
const UNIT_WIDTH_LABELARY: u32 = 812;
const UNIT_HEIGHT_LABELARY: u32 = 1624;
const UNIT_WIDTH_RENDERER: u32 = 813;
const UNIT_HEIGHT_RENDERER: u32 = 1626;

/// Compute the canvas a `.tspl` fixture's own `SIZE` command implies, at
/// 8 dpmm, rounding up like the renderer's mm→px conversion. TSPL goldens
/// are SIZE-driven rather than the fixed Labelary canvas.
fn tspl_fixture_dims(path: &std::path::Path) -> Option<(u32, u32)> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if !line.to_ascii_uppercase().starts_with("SIZE") {
            continue;
        }
        let labels = render_helpers::parse_tspl(content.as_bytes());
        let label = labels.first()?;
        let dpmm = 8.0_f64;
        let w = label.label_width_mm? * dpmm;
        let h = label.label_height_mm? * dpmm;
        // Renderer conversion: canvas = ceil(mm × dpmm).
        return Some((w.ceil() as u32, h.ceil() as u32));
    }
    None
}

/// Compute the canvas a `.cpcl` fixture implies from its header height and
/// PAGE-WIDTH, at 8 dpmm — CPCL goldens are header-driven like TSPL's
/// SIZE-driven ones.
fn cpcl_fixture_dims(path: &std::path::Path) -> Option<(u32, u32)> {
    let content = std::fs::read_to_string(path).ok()?;
    let labels = render_helpers::parse_cpcl(content.as_bytes());
    let label = labels.first()?;
    let dpmm = 8.0_f64;
    let w = label.label_width_mm? * dpmm;
    let h = label.label_height_mm? * dpmm;
    Some((w.ceil() as u32, h.ceil() as u32))
}

/// Verify that every golden PNG has the expected dimensions for its directory.
/// - `testdata/` and `testdata/labels/` → 813 × 1626 px (Labelary reference at 101.625mm × 203.25mm)
/// - `testdata/unit/` → 812 × 1624 px (Labelary) OR 813 × 1626 px (renderer baseline)
/// - `.tspl` fixtures are exempt: their canvas comes from the fixture's own
///   SIZE command and is verified against it instead.
#[test]
fn all_golden_pngs_have_standard_dimensions() {
    let dir = render_helpers::testdata_dir();

    let mut checked = 0u32;
    let mut failures: Vec<String> = Vec::new();

    // testdata/ and testdata/labels/ — must be 813×1626 (Labelary at 101.625mm × 203.25mm)
    for scan_dir in &[dir.clone(), dir.join("labels")] {
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
            if w != LABELARY_WIDTH || h != LABELARY_HEIGHT {
                failures.push(format!(
                    "  {} — {}×{} (expected {}×{})",
                    name, w, h, LABELARY_WIDTH, LABELARY_HEIGHT
                ));
            }
            checked += 1;
        }
    }

    // testdata/unit/ — accept either 812×1624 (Labelary bootstrap) or 813×1626 (renderer baseline)
    for entry in std::fs::read_dir(dir.join("unit"))
        .into_iter()
        .flatten()
        .flatten()
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
        let tspl_source = path.with_extension("tspl");
        let cpcl_source = path.with_extension("cpcl");
        let dpl_source = path.with_extension("dpl");
        let ok = if tspl_source.exists() {
            match tspl_fixture_dims(&tspl_source) {
                Some((ew, eh)) => {
                    if (ew, eh) != (w, h) {
                        failures.push(format!(
                            "  {} — {}×{} (TSPL SIZE implies {}×{})",
                            name, w, h, ew, eh
                        ));
                    }
                    // Already validated; skip the standard-dims check.
                    checked += 1;
                    continue;
                }
                None => false,
            }
        } else if dpl_source.exists() {
            // DPL declares no in-band label size; real-corpus goldens
            // derive their canvas from the external reference (dpmm 12),
            // pinned by the golden test itself rather than a fixed size.
            checked += 1;
            continue;
        } else if cpcl_source.exists() {
            match cpcl_fixture_dims(&cpcl_source) {
                Some((ew, eh)) => {
                    if (ew, eh) != (w, h) {
                        failures.push(format!(
                            "  {} — {}×{} (CPCL header implies {}×{})",
                            name, w, h, ew, eh
                        ));
                    }
                    checked += 1;
                    continue;
                }
                None => false,
            }
        } else {
            (w == UNIT_WIDTH_LABELARY && h == UNIT_HEIGHT_LABELARY)
                || (w == UNIT_WIDTH_RENDERER && h == UNIT_HEIGHT_RENDERER)
        };
        if !ok {
            failures.push(format!(
                "  {} — {}×{} (expected {}×{} or {}×{})",
                name,
                w,
                h,
                UNIT_WIDTH_LABELARY,
                UNIT_HEIGHT_LABELARY,
                UNIT_WIDTH_RENDERER,
                UNIT_HEIGHT_RENDERER,
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
        "{} golden PNG(s) have non-standard dimensions:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
