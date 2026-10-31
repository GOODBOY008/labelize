use labelize::elements::barcode_128::BarcodeMode;
use labelize::elements::barcode_qr::QrErrorCorrectionLevel;
use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::font_face::FontFace;
use labelize::elements::label_element::LabelElement;
use labelize::{CpclParser, Language};

fn parse(cpcl: &[u8]) -> Vec<LabelInfo> {
    CpclParser::new().parse(cpcl).expect("CPCL parse failed")
}

fn parse_one(cpcl: &[u8]) -> LabelInfo {
    let labels = parse(cpcl);
    assert!(!labels.is_empty(), "expected one label, got none");
    labels.into_iter().next().unwrap()
}

fn text_at(label: &LabelInfo, idx: usize) -> &labelize::elements::text_field::TextField {
    match &label.elements[idx] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {other:?}"),
    }
}

use labelize::LabelInfo;

#[test]
fn header_drives_label_height_and_page_width() {
    let label = parse_one(b"! 0 203 203 203 1\r\nPAGE-WIDTH 576\r\nTEXT 4 0 10 10 X\r\nPRINT\r\n");
    // Manual arithmetic: 8 dots per mm at 203 dpi.
    assert_eq!(label.label_height_mm, Some(203.0 / 8.0));
    assert_eq!(label.label_width_mm, Some(576.0 / 8.0));
}

#[test]
fn text_maps_font_cells_orientations_and_offset() {
    let label = parse_one(
        b"! 16 200 200 200 1\r\n\
          TEXT 0 0 10 20 base\r\n\
          VTEXT 4 1 30 40 vert\r\n\
          T90 2 1 50 60 r90\r\n\
          T180 7 0 70 80 r180\r\n\
          T270 5 3 90 100 r270\r\n\
          PRINT\r\n",
    );

    // Header offset (16 dots) shifts every x by the same amount.
    assert_eq!(text_at(&label, 0).position.x, 26);
    assert_eq!(text_at(&label, 0).position.y, 20);
    assert_eq!(text_at(&label, 0).font.width, 5.0);
    assert_eq!(text_at(&label, 0).font.height, 9.0);

    let vert = text_at(&label, 1);
    // 90° counter-clockwise = the IR's 270°-clockwise variant.
    assert_eq!(vert.font.orientation, FieldOrientation::Rotated270);
    assert_eq!(vert.font.width, 94.0);
    assert_eq!(vert.font.height, 94.0);

    assert_eq!(
        text_at(&label, 2).font.orientation,
        FieldOrientation::Rotated270
    );
    assert_eq!(
        text_at(&label, 3).font.orientation,
        FieldOrientation::Rotated180
    );
    assert_eq!(
        text_at(&label, 4).font.orientation,
        FieldOrientation::Rotated90
    );
}

#[test]
fn setmag_scales_resident_font_cells_and_resets() {
    let label = parse_one(
        b"! 0 200 200 200 1\r\n\
          SETMAG 2 3\r\n\
          TEXT 0 0 0 0 big\r\n\
          SETMAG 0 0\r\n\
          TEXT 0 0 0 50 reset\r\n\
          PRINT\r\n",
    );
    assert_eq!(text_at(&label, 0).font.width, 10.0);
    assert_eq!(text_at(&label, 0).font.height, 27.0);
    assert_eq!(text_at(&label, 1).font.width, 5.0);
    assert_eq!(text_at(&label, 1).font.height, 9.0);
}

#[test]
fn scale_text_uses_points_and_rotate_state() {
    let label = parse_one(
        b"! 0 200 200 300 1\r\n\
          SCALE-TEXT PLL_LAT.CSF 10 10 10 20 abc\r\n\
          ROTATE 90\r\n\
          ST PLL_LAT.CSF 10 10 10 120 rot\r\n\
          PRINT\r\n",
    );
    let first = text_at(&label, 0);
    assert_eq!(first.font.face, FontFace::CondensedBold);
    // 10 pt = 25.4/72 mm ≈ 28.44 dots at 8 dpmm.
    assert_eq!(
        first.font.height,
        (10.0 * 25.4 / 72.0 * 8.0 + 0.5) as i32 as f64
    );
    assert_eq!(first.font.orientation, FieldOrientation::Normal);
    assert_eq!(
        text_at(&label, 1).font.orientation,
        FieldOrientation::Rotated90
    );
}

#[test]
fn units_commands_switch_measurements() {
    let dots = parse_one(b"! 0 200 200 100 1\r\nTEXT 4 0 64 16 mm\r\nPRINT\r\n");
    let mm = parse_one(b"! 0 200 200 12.7 1\r\nIN-MILLIMETERS\r\nTEXT 4 0 8 2 mm\r\nPRINT\r\n");
    // 64 dots at 8 dots/mm = 8 mm — identical position both ways.
    assert_eq!(text_at(&dots, 0).position.x, 64);
    assert_eq!(text_at(&mm, 0).position.x, 64);
    // A UNITS command right after the header also applies to its fields:
    // 12.7 stays mm → 12.7 mm height ≈ 102 dots at 8 dpmm.
    assert_eq!(mm.label_height_mm, Some(12.7));
    assert_eq!(dots.label_height_mm, Some(12.5));
}

#[test]
fn justification_uses_end_point_blocks() {
    let label = parse_one(
        b"! 0 200 200 200 1\r\n\
          PAGE-WIDTH 400\r\n\
          CENTER 300\r\n\
          TEXT 4 0 50 40 C\r\n\
          RIGHT\r\n\
          TEXT 4 0 50 80 R\r\n\
          LEFT\r\n\
          TEXT 4 0 50 120 L\r\n\
          PRINT\r\n",
    );
    let c = text_at(&label, 0);
    let block = c.block.as_ref().expect("centered field has a block");
    assert_eq!(block.max_width, 300 - 50);
    assert_eq!(
        block.alignment,
        labelize::elements::text_alignment::TextAlignment::Center
    );
    let r = text_at(&label, 1);
    let block = r.block.as_ref().expect("right field has a block");
    // RIGHT without [end] falls back to PAGE-WIDTH.
    assert_eq!(block.max_width, 400 - 50);
    assert_eq!(
        block.alignment,
        labelize::elements::text_alignment::TextAlignment::Right
    );
    assert!(text_at(&label, 2).block.is_none());
}

#[test]
fn linear_barcodes_map_types_ratio_and_hri() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          BARCODE 128 2 1 60 10 20 8412345678\r\n\
          BARCODE UCCEAN128 2 1 60 10 100 001234567890123456\r\n\
          BARCODE 39C 2 26 50 10 180 ACME-991\r\n\
          BARCODE EAN13 2 1 50 10 300 4006381333931\r\n\
          BARCODE UPCA 2 1 50 300 300 40123456784\r\n\
          BARCODE EAN8 2 1 40 300 20 4012345\r\n\
          BARCODE I2OF5C 2 3 40 300 90 1234\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::Barcode128(bc) => {
            assert_eq!(bc.barcode.mode, BarcodeMode::Automatic);
            assert_eq!(bc.width, 2);
            assert_eq!(bc.barcode.height, 60);
            assert!(!bc.barcode.line, "HRI off by default");
        }
        other => panic!("expected Barcode128, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::Barcode128(bc) => assert_eq!(bc.barcode.mode, BarcodeMode::Ucc),
        other => panic!("expected Barcode128, got {other:?}"),
    }
    match &label.elements[2] {
        LabelElement::Barcode39(bc) => {
            assert!(bc.barcode.check_digit);
            assert_eq!(bc.width_ratio, 2.6);
        }
        other => panic!("expected Barcode39, got {other:?}"),
    }
    assert!(matches!(&label.elements[3], LabelElement::BarcodeEan13(_)));
    assert!(matches!(&label.elements[4], LabelElement::BarcodeUca(_)));
    assert!(matches!(&label.elements[5], LabelElement::BarcodeEan8(_)));
    match &label.elements[6] {
        LabelElement::Barcode2of5(bc) => assert!(bc.barcode.check_digit),
        other => panic!("expected Barcode2of5, got {other:?}"),
    }

    // BARCODE-TEXT flips the human-readable line for subsequent codes.
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          BARCODE-TEXT 7 0 5\r\n\
          BARCODE 128 2 1 60 10 20 8412345678\r\n\
          BT OFF\r\n\
          BARCODE 128 2 1 60 10 120 8412345678\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::Barcode128(bc) => assert!(bc.barcode.line),
        other => panic!("expected Barcode128, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::Barcode128(bc) => assert!(!bc.barcode.line),
        other => panic!("expected Barcode128, got {other:?}"),
    }
}

#[test]
fn unsupported_barcode_types_error_explicitly() {
    let err = CpclParser::new()
        .parse(b"! 0 200 200 200 1\r\nBARCODE MSI 2 1 40 10 10 1234\r\nPRINT\r\n")
        .unwrap_err();
    assert!(err.contains("unsupported CPCL barcode type MSI"), "{err}");

    let err = CpclParser::new()
        .parse(b"! 0 200 200 200 1\r\nBARCODE EAN132 2 1 40 10 10 123\r\nPRINT\r\n")
        .unwrap_err();
    assert!(err.contains("EAN132"), "{err}");
}

#[test]
fn qr_payload_conventions_resolve_parser_side() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          B QR 20 20 M 2 U 6\r\n\
          MA,QR code ABC123\r\n\
          ENDQR\r\n\
          B QR 300 20 U 4\r\n\
          H0M,N0123456789012345\r\n\
          ENDQR\r\n\
          B QR 20 200 U 3\r\n\
          LM,AQRcode,N0123456789012345,AQRCODE,B0006qrcode\r\n\
          ENDQR\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::BarcodeQr(qr) => {
            assert_eq!(qr.data, "QR code ABC123");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::M);
            assert_eq!(qr.barcode.magnification, 6);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::BarcodeQr(qr) => {
            // Manual mode: N-segment digits only; mask digit stripped.
            assert_eq!(qr.data, "0123456789012345");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::H);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
    match &label.elements[2] {
        LabelElement::BarcodeQr(qr) => {
            // Multi-segment manual payload: contents concatenate; the
            // binary segment's 4-digit count is stripped.
            assert_eq!(qr.data, "QRcode0123456789012345QRCODEqrcode");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::L);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
}

#[test]
fn pdf417_options_map_to_element() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          B PDF-417 10 20 XD 3 YD 12 C 4 S 2\r\n\
          PDF Data\r\n\
          ABCDE12345\r\n\
          ENDPDF\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::BarcodePdf417(bc) => {
            assert_eq!(bc.barcode.module_width, 3);
            assert_eq!(bc.barcode.row_height, 12);
            assert_eq!(bc.barcode.columns, 4);
            assert_eq!(bc.barcode.security, 2);
            assert_eq!(bc.data, "PDF Data\nABCDE12345");
        }
        other => panic!("expected BarcodePdf417, got {other:?}"),
    }
}

#[test]
fn maxicode_tags_build_the_structured_payload() {
    let label = parse_one(
        b"! 0 200 200 600 1\r\n\
          B MAXICODE 20 20\r\n\
          CC 840\r\n\
          MSG LOW PRIORITY MESSAGE.\r\n\
          SC 001\r\n\
          POST 02886\r\n\
          ENDMAXICODE\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::Maxicode(m) => {
            assert_eq!(m.code.mode, 2);
            // class + country + 9-digit postal, then the message header.
            assert!(m.data.starts_with("001840028860000"), "{}", m.data);
            assert!(
                m.data.ends_with("[)>\x1e\x1dLOW PRIORITY MESSAGE."),
                "{}",
                m.data
            );
            // The shared encoder accepts what we built.
            assert!(labelize::barcodes::maxicode::encode(&m.data, m.code.mode).is_ok());
        }
        other => panic!("expected Maxicode, got {other:?}"),
    }
}

#[test]
fn aztec_options_map_to_element() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          B AZTEC 50 100 XD 7 EC 47\r\n\
          123456789012\r\n\
          ENDAZTEC\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::BarcodeAztec(bc) => {
            assert_eq!(bc.barcode.magnification, 7);
            assert_eq!(bc.barcode.size, 47);
            assert_eq!(bc.data, "123456789012");
        }
        other => panic!("expected BarcodeAztec, got {other:?}"),
    }
}

#[test]
fn graphics_map_box_line_and_inverse() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          BOX 10 10 200 100 3\r\n\
          LINE 10 150 200 150 2\r\n\
          LINE 250 10 250 200 4\r\n\
          INVERSE-LINE 0 250 300 250 30\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::GraphicBox(b) => {
            assert_eq!((b.position.x, b.position.y), (10, 10));
            assert_eq!((b.width, b.height), (190, 90));
            assert_eq!(b.border_thickness, 3);
        }
        other => panic!("expected GraphicBox, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::GraphicBox(b) => assert_eq!((b.width, b.height), (190, 2)),
        other => panic!("horizontal LINE, got {other:?}"),
    }
    match &label.elements[2] {
        LabelElement::GraphicBox(b) => assert_eq!((b.width, b.height), (4, 190)),
        other => panic!("vertical LINE, got {other:?}"),
    }
    match &label.elements[3] {
        LabelElement::InvertRegion(r) => {
            assert_eq!((r.position.x, r.position.y), (0, 250));
            assert_eq!((r.width, r.height), (300, 30));
        }
        other => panic!("expected InvertRegion, got {other:?}"),
    }
}

#[test]
fn eg_reads_counted_hex_across_lines() {
    let label = parse_one(
        b"! 0 200 200 200 1\r\n\
          EG 2 16 90 45 F0F0F0F0F0F0F0F00F0F0F0F0F0F0F0F\r\n\
          F0F0F0F0F0F0F0F00F0F0F0F0F0F0F0F\r\n\
          TEXT 4 0 10 100 after\r\n\
          PRINT\r\n",
    );
    match &label.elements[0] {
        LabelElement::GraphicField(g) => {
            assert_eq!(g.row_bytes, 2);
            assert_eq!(g.data_bytes, 32);
            assert_eq!(g.data.len(), 32);
            assert_eq!(g.data[0], 0xF0);
            assert!(!g.zero_bits_black, "set bits print black");
        }
        other => panic!("expected GraphicField, got {other:?}"),
    }
    // The consumed bytes stop before the next command.
    match &label.elements[1] {
        LabelElement::Text(t) => assert_eq!(t.text, "after"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn ml_emits_one_text_per_data_line_at_pitch() {
    let label = parse_one(
        b"! 0 200 200 400 1\r\n\
          ML 40\r\n\
          TEXT 5 0 20 50\r\n\
          first\r\n\
          second\r\n\
          third\r\n\
          ENDML\r\n\
          PRINT\r\n",
    );
    assert_eq!(label.elements.len(), 3);
    assert_eq!(text_at(&label, 0).text, "first");
    assert_eq!(text_at(&label, 0).position.y, 50);
    assert_eq!(text_at(&label, 1).position.y, 90);
    assert_eq!(text_at(&label, 2).position.y, 130);
}

#[test]
fn sessions_flush_on_print_and_abort_discards() {
    let labels = parse(
        b"! 0 200 200 100 1\r\nTEXT 4 0 0 0 one\r\nPRINT\r\n\
          ! 0 200 200 100 1\r\nTEXT 4 0 0 0 two\r\nEND\r\n\
          ! 0 200 200 100 1\r\nTEXT 4 0 0 0 dropped\r\nABORT\r\n",
    );
    assert_eq!(labels.len(), 2);
    // END executes (renders) without printing; ABORT discards entirely.
    let texts: Vec<&str> = labels
        .iter()
        .map(|l| match &l.elements[0] {
            LabelElement::Text(t) => t.text.as_str(),
            other => panic!("expected Text, got {other:?}"),
        })
        .collect();
    assert_eq!(texts, vec!["one", "two"]);
}

#[test]
fn comments_and_device_commands_are_ignored() {
    let label = parse_one(
        b"! 0 200 200 200 1\r\n\
          ; a comment line\r\n\
          CONTRAST 0\r\n\
          TONE 100\r\n\
          JOURNAL\r\n\
          PACE\r\n\
          WAIT 80\r\n\
          BEEP 16\r\n\
          CUT\r\n\
          PREFEED 40\r\n\
          COUNT 1\r\n\
          TEXT 4 0 10 10 kept\r\n\
          FORM\r\n\
          PRINT\r\n",
    );
    assert_eq!(label.elements.len(), 1);
    assert_eq!(text_at(&label, 0).text, "kept");
}

#[test]
fn unsupported_constructs_error_explicitly() {
    for bad in [
        "CONCAT 75 75",
        "FG 3 0 0 7 0 5 0 4 0",
        "PCX 0 30",
        "SCALE-TO-FIT PLL_LAT.CSF 40 10 0 10 SALE",
        "PATTERN 103",
        "VEG 2 16 90 45",
        "DEFINE-FORMAT SHELF.FMT",
    ] {
        let src = format!("! 0 200 200 200 1\r\n{bad}\r\nPRINT\r\n");
        let err = CpclParser::new()
            .parse(src.as_bytes())
            .err()
            .unwrap_or_else(|| panic!("expected an error for: {bad}"));
        assert!(err.contains("CPCL"), "{bad} → {err}");
    }
}

#[test]
fn language_detection_and_front_door() {
    assert_eq!(
        labelize::detect_language(b"! 0 200 200 210 1\r\nTEXT 4 0 30 40 Hi\r\nPRINT\r\n"),
        Language::Cpcl
    );
    let labels = labelize::parse_labels(
        Language::Cpcl,
        b"! 0 203 203 203 1\r\nPAGE-WIDTH 576\r\nTEXT 4 0 30 40 Hi\r\nPRINT\r\n",
        8,
    )
    .unwrap();
    assert_eq!(labels[0].label_width_mm, Some(72.0));
}

#[test]
fn formatter_round_trips_fixtures() {
    let src = std::fs::read_to_string("testdata/unit/cpcl_2d_graphics.cpcl").unwrap();
    let before = CpclParser::with_dpmm(8).parse(src.as_bytes()).unwrap();
    let (_, formatted) = labelize::format_source(src.as_bytes(), Some(Language::Cpcl));
    let after = CpclParser::with_dpmm(8).parse(&formatted).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before[0].elements.len(), after[0].elements.len());
}

#[test]
fn counted_graphics_preserve_every_byte_and_following_commands() {
    let payload: Vec<u8> = (0..=255).collect();
    for command in ["CG", "COMPRESSED-GRAPHICS"] {
        let mut src = format!("! 0 203 203 400 1\r\n{command} 16 16 0 0 ").into_bytes();
        src.extend_from_slice(&payload);
        src.extend_from_slice(b"\r\nTEXT 0 0 0 20 after  \r\nPRINT\r\n");
        let (_, formatted) = labelize::format_source(&src, Some(Language::Cpcl));
        for input in [&src, &formatted] {
            let label = parse_one(input);
            assert_eq!(label.elements.len(), 2);
            match &label.elements[0] {
                LabelElement::GraphicField(g) => assert_eq!(g.data, payload),
                other => panic!("expected graphic, got {other:?}"),
            }
            assert_eq!(text_at(&label, 1).text, "after  ");
        }
    }
}

#[test]
fn graphic_sizes_are_checked_before_allocating() {
    for command in [
        "CG -1 2 0 0 x",
        "EG 1 0 0 0 FF",
        "CG 999999999999 999999999999 0 0 x",
        "EG 1 2 0 0 FF",
        "CG NaN 1 0 0 x",
    ] {
        let src = format!("! 0 203 203 100 1\n{command}\n");
        assert!(
            CpclParser::new().parse(src.as_bytes()).is_err(),
            "{command}"
        );
        assert_eq!(
            labelize::format_source(src.as_bytes(), Some(Language::Cpcl)).1,
            src.as_bytes()
        );
    }
}

#[test]
fn physical_units_do_not_resize_resident_fonts() {
    let dots = parse_one(b"! 0 203 203 100 1\nTEXT 0 0 8 16 X\nPRINT\n");
    let mm = parse_one(b"! 0 203 203 12.5 1\nIN-MILLIMETERS\nTEXT 0 0 1 2 X\nPRINT\n");
    assert_eq!(text_at(&dots, 0).font.width, text_at(&mm, 0).font.width);
    assert_eq!(text_at(&dots, 0).font.height, text_at(&mm, 0).font.height);
}

#[test]
fn multiline_accepts_inline_text_and_preserves_blank_rows_and_spaces() {
    let src = b"! 0 203 203 100 1\nML 10 TEXT 0 0 3 5\n first  \n\nthird\nENDML\nPRINT\n";
    let (_, formatted) = labelize::format_source(src, Some(Language::Cpcl));
    for input in [src.as_slice(), formatted.as_slice()] {
        let label = parse_one(input);
        assert_eq!(label.elements.len(), 2);
        assert_eq!(text_at(&label, 0).text, " first  ");
        assert_eq!(text_at(&label, 0).position.y, 5);
        assert_eq!(text_at(&label, 1).text, "third");
        assert_eq!(text_at(&label, 1).position.y, 25);
    }
}

#[test]
fn multiline_accepts_scalable_text() {
    let label = parse_one(
        b"! 0 203 203 100 1\nML 10 ST PLL_LAT.CSF 10 12 3 5\nfirst\nsecond\nENDML\nPRINT\n",
    );
    assert_eq!(
        text_at(&label, 0).font.height,
        (12.0_f64 * 25.4 / 72.0 * 8.0).round()
    );
    assert_eq!(text_at(&label, 1).position.y, 15);
}

#[test]
fn unterminated_data_blocks_are_errors() {
    for block in [
        "ML 10 TEXT 0 0 0 0\nhello",
        "B QR 0 0\nMA,hello",
        "B PDF-417 0 0\nhello",
        "B AZTEC 0 0\nhello",
        "B MAXICODE 0 0\nPOST 12345",
    ] {
        let src = format!("! 0 203 203 100 1\n{block}\n");
        assert!(CpclParser::new().parse(src.as_bytes()).is_err(), "{block}");
    }
}

#[test]
fn qr_binary_counts_preserve_commas_unicode_and_trailing_spaces() {
    use labelize::parsers::cpcl::qr_payload::resolve_cpcl_qr_payload as resolve;
    for (source, expected) in [
        ("M,hello  ", "hello  "),
        ("MA,,hello", ",hello"),
        ("HM,B0005a,b c,N12", "a,b c12"),
        ("MM,B0002é,AEND", "éEND"),
    ] {
        assert_eq!(resolve(source).unwrap().0, expected);
    }
    for bad in [
        "MM,B0008short",
        "MM,B0001é",
        "MM,Babcddata",
        "MM,B0001ab",
        "D010203,MA,x",
    ] {
        assert!(resolve(bad).is_err(), "{bad}");
    }
}

#[test]
fn graphics_height_honors_units_and_formatter_keeps_payload_spaces() {
    let src =
        b"! 0 203 203 20 1\nIN-MILLIMETERS\nCG 1 0.5 0 0 \xff\n\r \nTEXT 0 0 0 1 after\nPRINT\n";
    let (_, formatted) = labelize::format_source(src, Some(Language::Cpcl));
    for input in [src.as_slice(), formatted.as_slice()] {
        let label = parse_one(input);
        match &label.elements[0] {
            LabelElement::GraphicField(g) => assert_eq!(g.data, b"\xff\n\r "),
            other => panic!("expected graphic, got {other:?}"),
        }
        assert_eq!(text_at(&label, 1).text, "after");
    }
}
