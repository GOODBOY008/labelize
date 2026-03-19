use crate::common::image_compare;
use crate::common::labelary_client;
use crate::common::render_helpers;

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

    let labelary_png = match labelary_client::labelary_render(zpl, opts.dpmm as u8, width_in, height_in) {
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
    compare_against_labelary(
        "^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ",
        "text_label",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_barcode128() {
    compare_against_labelary(
        "^XA^FO50,50^BCN,100,Y,N,N^FD123456789^FS^XZ",
        "barcode128",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_graphic_box() {
    compare_against_labelary(
        "^XA^FO50,50^GB200,100,3^FS^XZ",
        "graphic_box",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_qr_code() {
    compare_against_labelary(
        "^XA^FO50,50^BQN,2,5^FDQA,Hello^FS^XZ",
        "qr_code",
    );
}

#[test]
#[ignore = "requires network access to Labelary API"]
fn labelary_mixed_label() {
    compare_against_labelary(
        "^XA^FO50,50^A0N,30,30^FDShipping Label^FS^FO50,100^BCN,80,Y,N,N^FD12345^FS^FO50,250^GB300,100,3^FS^XZ",
        "mixed_label",
    );
}
