use image::RgbaImage;
use labelize::{DrawerOptions, Renderer, ZplParser};
use rxing::common::DecoderRXingResult;

fn render(data: &str) -> Result<RgbaImage, String> {
    let source = format!("^XA^FO20,20^BQN,2,2^FD{data}^FS^XZ");
    let label = ZplParser::new().parse(source.as_bytes())?.remove(0);
    let mut png = Vec::new();
    Renderer::new().draw_label_as_png(
        &label,
        &mut png,
        DrawerOptions {
            label_width_mm: 50.0,
            label_height_mm: 50.0,
            dpmm: 8,
            ..Default::default()
        },
    )?;
    Ok(image::load_from_memory(&png).unwrap().to_rgba8())
}

fn decode_modules(image: &RgbaImage, magnification: u32) -> DecoderRXingResult {
    // Find the QR's black bounds; finder patterns reach every edge of the matrix.
    let (mut left, mut top, mut right, mut bottom) = (image.width(), image.height(), 0, 0);
    for (x, y, p) in image.enumerate_pixels() {
        if p[3] != 0 && p[0] < 128 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    let width = (right - left + 1) / magnification;
    assert_eq!(right - left, bottom - top);
    let matrix: Vec<Vec<bool>> = (0..width)
        .map(|y| {
            (0..width)
                .map(|x| {
                    let p = image.get_pixel(left + x * magnification, top + y * magnification);
                    p[3] != 0 && p[0] < 128
                })
                .collect()
        })
        .collect();
    rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&matrix).unwrap()
}

#[test]
fn manual_byte_digits_reach_the_symbol_as_a_byte_segment() {
    let decoded = decode_modules(&render("QM,B002012345678901234567890").unwrap(), 2);
    assert_eq!(
        decoded.getRawBytes()[0] >> 4,
        0b0100,
        "must be a byte segment, not optimized numeric data"
    );
    assert_eq!(
        decoded.getByteSegments(),
        &vec![b"12345678901234567890".to_vec()]
    );
    assert_eq!(decoded.getText(), "12345678901234567890");
}

#[test]
fn manual_alphanumeric_digits_do_not_become_a_numeric_segment() {
    let decoded = decode_modules(&render("QM,A12345678901234567890").unwrap(), 2);
    assert_eq!(decoded.getRawBytes()[0] >> 4, 0b0010);
    assert_eq!(decoded.getText(), "12345678901234567890");
}

#[test]
fn invalid_manual_numeric_and_alphanumeric_data_fail_rendering() {
    for data in [
        "QM,N12A",
        "QM,N-12",
        "QM,N１２",
        "QM,Aabc",
        "QM,AHELLO@",
        "QM,K漢字",
        "QM,XHELLO",
    ] {
        assert!(render(data).is_err(), "must reject {data}");
    }
}

#[test]
fn explicit_modes_match_independent_labelary_references() {
    for (name, field, expected_mode) in [
        ("qr_manual_byte", "QM,B0009lowercase", 0b0100),
        ("qr_manual_numeric", "QM,N12345678901234567890", 0b0001),
        ("qr_manual_alphanumeric", "QM,AAC-42", 0b0010),
    ] {
        let reference = image::open(format!("testdata/unit/{name}.png"))
            .unwrap()
            .to_rgba8();
        let expected = decode_modules(&reference, 4);
        let actual = decode_modules(&render(field).unwrap(), 2);
        assert_eq!(
            expected.getRawBytes()[0] >> 4,
            expected_mode,
            "Labelary {name}"
        );
        assert_eq!(
            actual.getRawBytes()[0] >> 4,
            expected_mode,
            "Labelize {name}"
        );
        assert_eq!(actual.getText(), expected.getText());
        assert_eq!(actual.getByteSegments(), expected.getByteSegments());
    }
}

#[test]
fn numeric_and_byte_modes_preserve_payload_at_every_error_correction_level() {
    for level in ["L", "M", "Q", "H"] {
        let numeric = decode_modules(&render(&format!("{level}M,N000123456789")).unwrap(), 2);
        assert_eq!(numeric.getRawBytes()[0] >> 4, 0b0001);
        assert_eq!(numeric.getText(), "000123456789");
        let bytes = decode_modules(&render(&format!("{level}M,B0006ä😀")).unwrap(), 2);
        assert_eq!(bytes.getRawBytes()[0] >> 4, 0b0100);
        assert_eq!(bytes.getByteSegments(), &vec!["ä😀".as_bytes().to_vec()]);
    }
    let alpha = decode_modules(
        &render("QM,A0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:").unwrap(),
        2,
    );
    assert_eq!(alpha.getRawBytes()[0] >> 4, 0b0010);
    assert_eq!(
        alpha.getText(),
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"
    );
}

#[test]
fn manual_modes_grow_past_the_version_nine_count_boundary() {
    use labelize::barcodes::qrcode::encode_with_mode;
    use labelize::elements::barcode_qr::{QrCharacterMode as Mode, QrErrorCorrectionLevel as Ec};
    for (mode, content, indicator) in [
        (Mode::Binary, "A".repeat(300), 0b0100),
        (Mode::Alphanumeric, "A".repeat(600), 0b0010),
        (Mode::Numeric, "1".repeat(1100), 0b0001),
    ] {
        let img = encode_with_mode(&content, 1, Ec::L, mode).unwrap();
        assert!(img.width() >= 17 + 4 * 10 + 8, "must select version >= 10");
        let result = decode_modules(&img, 1);
        assert_eq!(result.getRawBytes()[0] >> 4, indicator);
        assert_eq!(result.getText(), content);
        if mode == Mode::Binary {
            assert_eq!(result.getByteSegments(), &vec![content.into_bytes()]);
        }
    }
}

#[test]
fn invalid_or_oversized_manual_api_data_returns_errors_without_panicking() {
    use labelize::barcodes::qrcode::encode_with_mode;
    use labelize::elements::barcode_qr::{QrCharacterMode as Mode, QrErrorCorrectionLevel as Ec};
    for (mode, content) in [
        (Mode::Numeric, "-1".to_string()),
        (Mode::Numeric, "１２".to_string()),
        (Mode::Alphanumeric, "lowercase".to_string()),
        (Mode::Alphanumeric, "@".to_string()),
        (Mode::Kanji, "漢字".to_string()),
        (Mode::Numeric, "1".repeat(8000)),
        (Mode::Alphanumeric, "A".repeat(5000)),
        (Mode::Binary, "A".repeat(4000)),
        (Mode::Binary, String::new()),
    ] {
        let result = std::panic::catch_unwind(|| encode_with_mode(&content, 1, Ec::L, mode));
        assert!(result.is_ok(), "encoder panicked for {mode:?}");
        assert!(
            result.unwrap().is_err(),
            "encoder accepted invalid/oversized {mode:?}"
        );
    }
}

#[test]
fn automatic_api_preserves_codewords_and_scaling_when_mask_selection_changes() {
    use labelize::barcodes::qrcode::encode;
    use labelize::elements::barcode_qr::QrErrorCorrectionLevel as Ec;
    use qrcode::{
        types::{Color, EcLevel},
        QrCode,
    };
    for (ec, legacy_ec) in [
        (Ec::L, EcLevel::L),
        (Ec::M, EcLevel::M),
        (Ec::Q, EcLevel::Q),
        (Ec::H, EcLevel::H),
    ] {
        for data in [
            "12345678901234567890",
            "ABC-123",
            "ABC12345678901234567890abc",
            "ä😀",
        ] {
            let legacy = QrCode::with_error_correction_level(data.as_bytes(), legacy_ec).unwrap();
            let matrix: Vec<Vec<bool>> = (0..legacy.width())
                .map(|y| {
                    (0..legacy.width())
                        .map(|x| legacy[(x, y)] == Color::Dark)
                        .collect()
                })
                .collect();
            let expected =
                rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&matrix).unwrap();
            let unscaled = encode(data, 1, ec).unwrap();
            for mag in [1, 2, 4] {
                let actual = encode(data, mag, ec).unwrap();
                let decoded = decode_modules(&actual, mag as u32);
                assert_eq!(decoded.getRawBytes(), expected.getRawBytes());
                assert_eq!(decoded.getByteSegments(), expected.getByteSegments());
                assert_eq!(decoded.getECLevel(), expected.getECLevel());
                assert_eq!(actual.width(), (legacy.width() as u32 + 8) * mag as u32);
                for (x, y, pixel) in actual.enumerate_pixels() {
                    let (mx, my) = (x / mag as u32, y / mag as u32);
                    assert_eq!(
                        pixel,
                        unscaled.get_pixel(mx, my),
                        "{ec:?} {data} scale {mag} at {x},{y}"
                    );
                    if mx < 4
                        || my < 4
                        || mx >= legacy.width() as u32 + 4
                        || my >= legacy.width() as u32 + 4
                    {
                        assert_eq!(pixel[3], 0, "quiet zone");
                    }
                }
            }
        }
    }
}
#[test]
fn labelary_digit_probes_document_optimization_instead_of_defining_manual_modes() {
    for (name, field, mode) in [
        ("qr_manual_byte", "QM,B002012345678901234567890", 0b0100),
        ("qr_manual_alphanumeric", "QM,A12345678901234567890", 0b0010),
    ] {
        let reference = image::open(format!("testdata/qr-mode-probes/{name}.png"))
            .unwrap()
            .to_rgba8();
        let labelary = decode_modules(&reference, 4);
        let actual = decode_modules(&render(field).unwrap(), 2);
        // These saved Labelary responses optimize both requests to Numeric.
        // Zebra's explicit mode and our toolkit require the selected segment.
        assert_eq!(labelary.getRawBytes()[0] >> 4, 0b0001);
        assert_eq!(actual.getRawBytes()[0] >> 4, mode);
        assert_eq!(actual.getText(), labelary.getText());
    }
}

#[test]
fn epl_synthesized_byte_mode_preserves_the_original_data() {
    let source = "N\nb20,20,Q,s2,eH,\"12345|67890\"\nP1\n";
    let label = labelize::EplParser::new()
        .parse(source.as_bytes())
        .unwrap()
        .remove(0);
    let mut png = Vec::new();
    Renderer::new()
        .draw_label_as_png(
            &label,
            &mut png,
            DrawerOptions {
                label_width_mm: 50.0,
                label_height_mm: 50.0,
                dpmm: 8,
                ..Default::default()
            },
        )
        .unwrap();
    let decoded = decode_modules(&image::load_from_memory(&png).unwrap().to_rgba8(), 2);
    assert_eq!(decoded.getRawBytes()[0] >> 4, 0b0100);
    assert_eq!(decoded.getByteSegments(), &vec![b"12345|67890".to_vec()]);
}
