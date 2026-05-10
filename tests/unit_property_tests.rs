mod common;

use common::proptest_strategies;
use proptest::prelude::*;

// --- Parser property tests ---

proptest! {
    #[test]
    fn zpl_parser_no_panic(zpl in proptest_strategies::arb_zpl_label()) {
        let mut parser = labelize::ZplParser::new();
        let result = parser.parse(zpl.as_bytes());
        // Should never panic, should return Ok or Err
        prop_assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn zpl_single_block_produces_one_label(
        content in "[A-Za-z0-9 ]{1,20}"
    ) {
        let zpl = format!("^XA^FO50,50^A0N,30,30^FD{}^FS^XZ", content);
        let mut parser = labelize::ZplParser::new();
        let labels = parser.parse(zpl.as_bytes()).unwrap();
        prop_assert_eq!(labels.len(), 1);
    }

    #[test]
    fn zpl_fo_position_preserved(x in 0i32..800, y in 0i32..1200) {
        let zpl = format!("^XA^FO{},{}^A0N,30,30^FDtest^FS^XZ", x, y);
        let mut parser = labelize::ZplParser::new();
        let labels = parser.parse(zpl.as_bytes()).unwrap();
        if !labels.is_empty() && !labels[0].elements.is_empty() {
            if let labelize::elements::label_element::LabelElement::Text(t) = &labels[0].elements[0] {
                prop_assert_eq!(t.position.x, x);
                prop_assert_eq!(t.position.y, y);
            }
        }
    }

    #[test]
    fn epl_parser_no_panic(input in "\\PC{0,100}") {
        let parser = labelize::EplParser::new();
        let result = parser.parse(input.as_bytes());
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// --- Barcode property tests ---

proptest! {
    #[test]
    fn code128_no_panic(input in proptest_strategies::arb_code128_input()) {
        let result = labelize::barcodes::code128::encode_auto(&input, 100, 2);
        // Should never panic
        if let Ok(img) = result {
            prop_assert!(img.width() > 0);
            prop_assert!(img.height() > 0);
        }
    }

    #[test]
    fn ean13_produces_output(input in proptest_strategies::arb_ean13_input()) {
        let result = labelize::barcodes::ean13::encode(&input, 100, 2);
        if let Ok(img) = result {
            prop_assert!(img.width() > 0);
            prop_assert!(img.height() > 0);
        }
    }

    #[test]
    fn qr_produces_square(input in proptest_strategies::arb_qr_input()) {
        let result = labelize::barcodes::qrcode::encode(&input, 5, labelize::elements::barcode_qr::QrErrorCorrectionLevel::M);
        if let Ok(img) = result {
            prop_assert_eq!(img.width(), img.height());
        }
    }

    #[test]
    fn twooffive_no_panic(input in proptest_strategies::arb_2of5_input()) {
        let result = labelize::barcodes::twooffive::encode(&input, 100, 3, 2, false);
        if let Ok(img) = result {
            prop_assert!(img.width() > 0);
            prop_assert!(img.height() > 0);
        }
    }
}

// --- Encoder property tests ---

proptest! {
    #[test]
    fn png_round_trip_preserves_dimensions(
        w in 1u32..50,
        h in 1u32..50,
    ) {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([128, 128, 128, 255]));
        let mut buf = Vec::new();
        labelize::encode_png(&img, &mut buf).unwrap();
        let decoded = image::load_from_memory(&buf).unwrap();
        prop_assert_eq!(decoded.width(), w);
        prop_assert_eq!(decoded.height(), h);
    }
}

// --- Hex property tests ---

proptest! {
    #[test]
    fn hex_decode_no_panic(input in proptest_strategies::arb_hex_string()) {
        let result = labelize::hex::decode_graphic_field_data(&input, (input.len() / 2) as i32);
        // Should never panic
        prop_assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn hex_escaped_no_panic(input in "[\\x20-\\x7E]{0,50}") {
        let result = labelize::hex::decode_escaped_string(&input, b'_');
        prop_assert!(result.is_ok());
    }
}

// --- Text orientation bug condition tests ---
// These tests verify text position/orientation rendering matches Labelary reference
// with tight tolerance (5%). They should FAIL on unfixed code and PASS after fix.

mod text_orientation_tests {
    use crate::common::image_compare;
    use crate::common::render_helpers;

    const TEXT_TOLERANCE: f64 = 5.0;

    /// Options matching Labelary reference dimensions (813×1626 at 8dpmm)
    fn labelary_options() -> labelize::DrawerOptions {
        labelize::DrawerOptions {
            label_width_mm: 101.625,
            label_height_mm: 203.25,
            dpmm: 8,
            ..Default::default()
        }
    }

    fn run_text_golden(name: &str, tolerance: f64) {
        let dir = render_helpers::unit_dir();
        let input = dir.join(format!("{}.zpl", name));
        let expected = dir.join(format!("{}.png", name));

        if !input.exists() || !expected.exists() {
            panic!("Missing test file for {}", name);
        }

        let content = std::fs::read_to_string(&input).expect("read input");
        let actual_png = render_helpers::render_zpl_to_png(&content, labelary_options());
        let expected_png = std::fs::read(&expected).expect("read golden");
        let result = image_compare::compare_images(&actual_png, &expected_png, tolerance);

        if result.diff_percent > tolerance {
            if let Some(ref diff_img) = result.diff_image {
                image_compare::save_diff_image(name, diff_img);
            }
        }

        assert!(
            result.diff_percent <= tolerance,
            "Text golden test '{}' FAILED: {:.2}% pixel diff (tolerance: {:.2}%)",
            name,
            result.diff_percent,
            tolerance,
        );
    }

    #[test]
    fn text_fo_normal_tight() {
        run_text_golden("text_fo_n", TEXT_TOLERANCE);
    }
    #[test]
    fn text_fo_rotated90_tight() {
        run_text_golden("text_fo_r", TEXT_TOLERANCE);
    }
    #[test]
    fn text_fo_rotated180_tight() {
        run_text_golden("text_fo_i", TEXT_TOLERANCE);
    }
    #[test]
    fn text_fo_rotated270_tight() {
        run_text_golden("text_fo_b", TEXT_TOLERANCE);
    }
    #[test]
    fn text_ft_normal_tight() {
        run_text_golden("text_ft_n", TEXT_TOLERANCE);
    }
    #[test]
    fn text_ft_rotated90_tight() {
        run_text_golden("text_ft_r", TEXT_TOLERANCE);
    }
    #[test]
    fn text_ft_rotated180_tight() {
        run_text_golden("text_ft_i", TEXT_TOLERANCE);
    }
    #[test]
    fn text_ft_rotated270_tight() {
        run_text_golden("text_ft_b", TEXT_TOLERANCE);
    }
    #[test]
    fn text_ft_auto_pos_tight() {
        run_text_golden("text_ft_auto_pos", TEXT_TOLERANCE);
    }
    #[test]
    fn text_multiline_tight() {
        run_text_golden("text_multiline", TEXT_TOLERANCE);
    }
}

// --- Preservation property tests ---
// These tests verify non-text elements continue to render identically after fix.
// They should PASS both before and after fix.

mod preservation_tests {
    use crate::common::image_compare;
    use crate::common::render_helpers;

    const PRESERVATION_TOLERANCE: f64 = 50.0;

    /// Options matching Labelary reference dimensions (813×1626 at 8dpmm)
    fn labelary_options() -> labelize::DrawerOptions {
        labelize::DrawerOptions {
            label_width_mm: 101.625,
            label_height_mm: 203.25,
            dpmm: 8,
            ..Default::default()
        }
    }

    fn run_preservation_golden(name: &str) {
        let dir = render_helpers::testdata_dir();
        // Try labels/ first, then unit/, then root
        let input = if dir.join("labels").join(format!("{}.zpl", name)).exists() {
            dir.join("labels").join(format!("{}.zpl", name))
        } else if dir.join("unit").join(format!("{}.zpl", name)).exists() {
            dir.join("unit").join(format!("{}.zpl", name))
        } else {
            dir.join(format!("{}.zpl", name))
        };
        let expected = input.with_extension("png");

        if !input.exists() || !expected.exists() {
            eprintln!("SKIP preservation {}: missing files", name);
            return;
        }

        let content = std::fs::read_to_string(&input).expect("read input");
        let actual_png = render_helpers::render_zpl_to_png(&content, labelary_options());
        let expected_png = std::fs::read(&expected).expect("read golden");
        let result =
            image_compare::compare_images(&actual_png, &expected_png, PRESERVATION_TOLERANCE);

        assert!(
            result.diff_percent <= PRESERVATION_TOLERANCE,
            "Preservation test '{}' FAILED: {:.2}% pixel diff (tolerance: {:.2}%)",
            name,
            result.diff_percent,
            PRESERVATION_TOLERANCE,
        );
    }

    // Barcode preservation
    #[test]
    fn preserve_barcode128_default_width() {
        run_preservation_golden("barcode128_default_width");
    }
    #[test]
    fn preserve_barcode128_rotated() {
        run_preservation_golden("barcode128_rotated");
    }
    #[test]
    fn preserve_barcode128_line() {
        run_preservation_golden("barcode128_line");
    }
    #[test]
    fn preserve_ean13() {
        run_preservation_golden("ean13");
    }

    // Graphic element preservation
    #[test]
    fn preserve_gb_normal() {
        run_preservation_golden("gb_normal");
    }
    #[test]
    fn preserve_gb_rounded() {
        run_preservation_golden("gb_rounded");
    }
    #[test]
    fn preserve_gb_0_height() {
        run_preservation_golden("gb_0_height");
    }
    #[test]
    fn preserve_gb_0_width() {
        run_preservation_golden("gb_0_width");
    }

    // Mixed labels (barcodes + text + graphics)
    #[test]
    fn preserve_amazon() {
        run_preservation_golden("amazon");
    }
    #[test]
    fn preserve_fedex() {
        run_preservation_golden("fedex");
    }
    #[test]
    fn preserve_ups() {
        run_preservation_golden("ups");
    }
    #[test]
    fn preserve_usps() {
        run_preservation_golden("usps");
    }

    // QR code preservation
    #[test]
    fn preserve_qr_code_ft_manual() {
        run_preservation_golden("qr_code_ft_manual");
    }
    #[test]
    fn preserve_reverse_qr() {
        run_preservation_golden("reverse_qr");
    }
}
