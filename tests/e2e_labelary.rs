mod common;

use common::image_compare;
use common::labelary_client;
use common::render_helpers;

/// Tolerance for Labelary comparison tests.
const LABELARY_TOLERANCE: f64 = 15.0;

/// Convert label dimensions from mm to inches for Labelary API.
fn mm_to_inches(mm: f64) -> f64 {
    mm / 25.4
}

fn compare_against_labelary(zpl: &str, name: &str) {
    let opts = render_helpers::default_options();
    let width_in = mm_to_inches(opts.label_width_mm);
    let height_in = mm_to_inches(opts.label_height_mm);

    let labelary_png =
        match labelary_client::labelary_render(zpl, opts.dpmm as u8, width_in, height_in) {
            Some(png) => png,
            None => {
                eprintln!("SKIP {}: Labelary API unreachable", name);
                return;
            }
        };

    let actual_png = render_helpers::render_zpl_to_png(zpl, opts);
    let result = image_compare::compare_images(&actual_png, &labelary_png, LABELARY_TOLERANCE);

    eprintln!(
        "Labelary comparison '{}': {:.2}% pixel diff, dims match: {}",
        name, result.diff_percent, result.dimensions_match
    );

    if result.diff_percent > LABELARY_TOLERANCE {
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(&format!("labelary_{}", name), diff_img);
        }
    }

    assert!(
        result.diff_percent <= LABELARY_TOLERANCE,
        "Labelary comparison '{}' FAILED: {:.2}% pixel diff (tolerance: {:.2}%)",
        name,
        result.diff_percent,
        LABELARY_TOLERANCE,
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_text_label() {
    compare_against_labelary("^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ", "text_label");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_barcode128() {
    compare_against_labelary("^XA^FO50,50^BCN,100,Y,N,N^FD123456789^FS^XZ", "barcode128");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_graphic_box() {
    compare_against_labelary("^XA^FO50,50^GB200,100,3^FS^XZ", "graphic_box");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_qr_code() {
    compare_against_labelary("^XA^FO50,50^BQN,2,5^FDQA,Hello^FS^XZ", "qr_code");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_mixed_label() {
    compare_against_labelary(
        "^XA^FO50,50^A0N,30,30^FDShipping Label^FS^FO50,100^BCN,80,Y,N,N^FD12345^FS^FO50,250^GB300,100,3^FS^XZ",
        "mixed_label",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_gd_thin_right() {
    compare_against_labelary("^XA^FO50,50^GD200,200,5,B,R^FS^XZ", "gd_thin_right");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_gd_thin_left() {
    compare_against_labelary("^XA^FO50,50^GD200,200,5,B,L^FS^XZ", "gd_thin_left");
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_gd_thick_fill() {
    compare_against_labelary(
        "^XA^FO50,50^GD200,300,200,B,R^FS^FO300,50^GD200,300,200,B,L^FS^XZ",
        "gd_thick_fill",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_gd_default_params() {
    compare_against_labelary(
        "^XA^FO50,50^GD300,400,10^FS^FO400,50^GD300,400,10,B,L^FS^XZ",
        "gd_default_params",
    );
}

/// Download Labelary reference PNGs (813×1626) for every ZPL file in testdata/unit/.
/// Run with: cargo test --test e2e_labelary update_unit_golden_pngs -- --ignored --nocapture
#[test]
#[ignore = "requires network access; updates unit golden PNGs in place"]
fn update_unit_golden_pngs() {
    let opts = render_helpers::default_options();
    let width_in = mm_to_inches(opts.label_width_mm);
    let height_in = mm_to_inches(opts.label_height_mm);
    let unit_dir = render_helpers::testdata_dir().join("unit");

    let mut paths: Vec<_> = std::fs::read_dir(&unit_dir)
        .expect("read unit dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("zpl"))
        .collect();
    paths.sort();

    let mut updated = 0usize;
    let mut failed = 0usize;

    for zpl_path in &paths {
        let name = zpl_path.file_stem().unwrap().to_string_lossy().to_string();
        let zpl = std::fs::read_to_string(zpl_path).expect("read zpl");
        eprint!("  {}: ", name);

        match labelary_client::labelary_render(&zpl, opts.dpmm as u8, width_in, height_in) {
            Some(png) => {
                let out = unit_dir.join(format!("{}.png", name));
                std::fs::write(&out, &png).expect("write png");
                eprintln!("OK ({} bytes)", png.len());
                updated += 1;
            }
            None => {
                eprintln!("FAILED (API unreachable or error)");
                failed += 1;
            }
        }
    }

    eprintln!("\nDone — updated: {}, failed: {}", updated, failed);
    assert_eq!(
        failed, 0,
        "{} unit golden PNGs could not be fetched from Labelary",
        failed
    );
}
