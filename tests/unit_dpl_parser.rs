use labelize::elements::barcode_128::BarcodeMode;
use labelize::elements::barcode_qr::QrErrorCorrectionLevel;
use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::font_face::FontFace;
use labelize::elements::label_element::LabelElement;
use labelize::{DplParser, Language};

fn parse(dpl: &[u8]) -> Vec<LabelInfo> {
    DplParser::new().parse(dpl).expect("DPL parse failed")
}

fn parse_one(dpl: &[u8]) -> LabelInfo {
    let labels = parse(dpl);
    assert!(!labels.is_empty(), "expected one label, got none");
    labels.into_iter().next().unwrap()
}

use labelize::LabelInfo;

fn rec(rot: u8, b: &str, c: u8, d: u8, eee: u32, row: u32, col: u32, data: &str) -> String {
    format!(
        "{}{}{}{}{:03}{:04}{:04}{}",
        rot as char, b, c as char, d as char, eee, row, col, data
    )
}

fn dpl(records: &[&str]) -> Vec<u8> {
    let mut out = vec![0x02];
    out.extend_from_slice(records.join("\r").as_bytes());
    out.push(b'\r');
    out
}

fn text_at(label: &LabelInfo, idx: usize) -> &labelize::elements::text_field::TextField {
    match &label.elements[idx] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn bitmap_fonts_map_cells_multipliers_and_rotations() {
    let label = parse_one(&dpl(&[
        "L",
        "D11",
        &rec(b'1', "0", b'1', b'1', 0, 100, 10, "F0"),
        &rec(b'1', "4", b'2', b'3', 0, 200, 10, "F4"),
        &rec(b'2', "2", b'1', b'1', 0, 300, 10, "ROT2"),
        &rec(b'3', "2", b'1', b'1', 0, 400, 10, "ROT3"),
        &rec(b'4', "2", b'1', b'1', 0, 500, 10, "ROT4"),
        "Q0001",
        "E",
    ]));

    let f0 = text_at(&label, 0);
    // Font 0 cell: 5x7 dots at 1x → 5x7 at the engine's 8 dpmm.
    assert_eq!(f0.font.width, 5.0);
    assert_eq!(f0.font.height, 7.0);
    assert_eq!(f0.font.face, FontFace::BitmapMono);
    assert_eq!(f0.position.from_bottom, true);

    let f4 = text_at(&label, 1);
    assert_eq!(f4.font.width, 36.0); // 18 × 2
    assert_eq!(f4.font.height, 108.0); // 36 × 3

    assert_eq!(
        text_at(&label, 2).font.orientation,
        FieldOrientation::Rotated90
    );
    assert_eq!(
        text_at(&label, 3).font.orientation,
        FieldOrientation::Rotated180
    );
    assert_eq!(
        text_at(&label, 4).font.orientation,
        FieldOrientation::Rotated270
    );
}

#[test]
fn positions_convert_hundredths_of_an_inch_from_bottom() {
    // 100 units = 1 inch = 203.2 dots at 8 dpmm (mm→dots rounds).
    let label = parse_one(&dpl(&[
        "L",
        &rec(b'1', "0", b'1', b'1', 0, 100, 50, "X"),
        "E",
    ]));
    let t = text_at(&label, 0);
    assert_eq!(t.position.x, 102); // 0.5 in = 12.7 mm × 8 = 101.6 → 102
    assert_eq!(t.position.y, 203); // 1.0 in = 25.4 mm × 8 = 203.2 → 203
    assert!(t.position.from_bottom);
    assert!(!t.position.calculate_from_bottom);
}

#[test]
fn metric_mode_uses_tenths_of_a_millimetre() {
    let label = parse_one(&dpl(&[
        "L",
        "m",
        &rec(b'1', "0", b'1', b'1', 0, 100, 100, "MM"),
        "E",
    ]));
    let t = text_at(&label, 0);
    // 100 units = 10.0 mm → 80 dots at 8 dpmm.
    assert_eq!(t.position.x, 80);
    assert_eq!(t.position.y, 80);
}

#[test]
fn column_and_row_offsets_apply() {
    let label = parse_one(&dpl(&[
        "L",
        "C0050",
        "R0037",
        &rec(b'1', "0", b'1', b'1', 0, 100, 100, "OFF"),
        "E",
    ]));
    let t = text_at(&label, 0);
    // col 100+50=150 units, row 100+37=137 units.
    assert_eq!(
        t.position.x,
        (150.0_f64 * 25.4 / 100.0 * 8.0).round() as i32
    );
    assert_eq!(
        t.position.y,
        (137.0_f64 * 25.4 / 100.0 * 8.0).round() as i32
    );
}

#[test]
fn smooth_font9_point_sizes() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(b'1', "9", b'1', b'1', 6, 100, 10, "SMALL"),
        &rec(b'1', "9", b'1', b'1', 1, 300, 10, "SIX"),
        &rec(b'1', "9", b'1', b'1', 2, 500, 10, "EIGHT"),
        "E",
    ]));
    // eee=006 is the 0nn form of A18 (18 pt); 001 = A06; 002 = A08.
    let heights: Vec<f64> = (0..3).map(|i| text_at(&label, i).font.height).collect();
    let dots = |pts: f64| pts / 72.307 * 25.4 * 8.0;
    assert_eq!(heights[0], dots(18.0));
    assert_eq!(heights[1], dots(6.0));
    assert_eq!(heights[2], dots(8.0));
    assert_eq!(text_at(&label, 0).font.face, FontFace::CondensedBold);
}

#[test]
fn scalable_font_record_parses_point_sizes() {
    let label = parse_one(&dpl(&[
        "L",
        &format!("1911S0100010000P015P015SCALABLE"),
        "E",
    ]));
    let t = text_at(&label, 0);
    let dots = 15.0 / 72.307 * 25.4 * 8.0;
    assert_eq!(t.font.height, dots);
    assert_eq!(t.font.width, dots);
}

#[test]
fn linear_barcodes_map_types_hri_and_defaults() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(b'1', "E", b'2', b'1', 40, 30, 26, "1234567890"), // 128 + HRI
        &rec(b'1', "e", b'2', b'1', 40, 30, 260, "1234567890"), // 128, no HRI
        &rec(b'1', "A", b'0', b'0', 0, 200, 26, "ACME-991"),   // 39 defaults
        &rec(b'1', "j", b'0', b'0', 0, 300, 26, "12345"),      // I2of5 + mod10
        &rec(b'1', "F", b'2', b'1', 80, 400, 26, "4006381333931"), // EAN-13
        &rec(b'1', "B", b'2', b'1', 80, 500, 26, "40123456784"), // UPC-A
        &rec(b'1', "G", b'2', b'1', 40, 600, 26, "4012345"),   // EAN-8
        &rec(b'1', "Q", b'2', b'1', 140, 700, 26, "001234567890123456"), // UCC128
        "E",
    ]));
    match &label.elements[0] {
        LabelElement::Barcode128(bc) => {
            assert!(bc.barcode.line, "uppercase E prints the HRI line");
            assert_eq!(bc.barcode.height, 81); // 0.40 in = 81 dots
        }
        other => panic!("expected Barcode128, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::Barcode128(bc) => assert!(!bc.barcode.line),
        other => panic!("expected Barcode128, got {other:?}"),
    }
    match &label.elements[2] {
        LabelElement::Barcode39(bc) => {
            assert!(!bc.barcode.check_digit);
            assert_eq!(bc.width_ratio, 3.0, "default 6:2 ratio");
        }
        other => panic!("expected Barcode39, got {other:?}"),
    }
    match &label.elements[3] {
        LabelElement::Barcode2of5(bc) => assert!(bc.barcode.check_digit),
        other => panic!("expected Barcode2of5, got {other:?}"),
    }
    assert!(matches!(&label.elements[4], LabelElement::BarcodeEan13(_)));
    assert!(matches!(&label.elements[5], LabelElement::BarcodeUca(_)));
    assert!(matches!(&label.elements[6], LabelElement::BarcodeEan8(_)));
    match &label.elements[7] {
        LabelElement::Barcode128(bc) => assert_eq!(bc.barcode.mode, BarcodeMode::Ucc),
        other => panic!("expected Barcode128, got {other:?}"),
    }
}

#[test]
fn unsupported_barcode_types_error_explicitly() {
    for bad in ["I", "K", "O", "T", "v"] {
        let src = dpl(&["L", &rec(b'1', bad, b'2', b'1', 40, 10, 10, "X"), "E"]);
        let err = DplParser::new()
            .parse(&src)
            .err()
            .unwrap_or_else(|| panic!("expected an error for bar code {bad}"));
        assert!(err.contains("unsupported DPL bar code"), "{bad} → {err}");
    }
}

#[test]
fn graphics_lines_and_boxes_map_to_filled_boxes() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(b'1', "X", b'1', b'1', 0, 60, 0, "L600001"),
        &rec(b'1', "X", b'1', b'1', 0, 60, 600, "B600100005005"),
        "E",
    ]));
    match &label.elements[0] {
        LabelElement::GraphicBox(b) => {
            // 6.00 in wide, 0.01 in tall (in dots at 8 dpmm).
            assert_eq!(b.width, (600.0_f64 * 25.4 / 100.0 * 8.0).round() as i32);
            assert_eq!(b.height, (1.0_f64 * 25.4 / 100.0 * 8.0).round() as i32);
            assert!(b.position.from_bottom);
        }
        other => panic!("expected GraphicBox, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::GraphicBox(b) => {
            assert_eq!(b.width, (600.0_f64 * 25.4 / 100.0 * 8.0).round() as i32);
            assert_eq!(b.height, (100.0_f64 * 25.4 / 100.0 * 8.0).round() as i32);
        }
        other => panic!("expected GraphicBox, got {other:?}"),
    }
}

#[test]
fn unsupported_graphics_error_explicitly() {
    for data in ["P 001 0001 0010 0001 0040 0025", "C 001 0010 0010 0040"] {
        let src = dpl(&["L", &rec(b'1', "X", b'1', b'1', 0, 10, 10, data), "E"]);
        let err = DplParser::new()
            .parse(&src)
            .err()
            .unwrap_or_else(|| panic!("expected an error for {data:?}"));
        assert!(err.contains("not supported"), "{data} → {err}");
    }
}

#[test]
fn qr_auto_and_manual_forms_resolve() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(
            b'1',
            "W1d",
            b'4',
            b'4',
            0,
            100,
            10,
            "This is the data portion",
        ),
        "",
        &rec(b'1', "W1D", b'4', b'4', 0, 300, 10, "H0M,N0123456789012345"),
        &rec(
            b'1',
            "W1D",
            b'4',
            b'4',
            0,
            500,
            10,
            "2,HM,AHTTPS://EXAMPLE.COM",
        ),
        "E",
    ]));
    match &label.elements[0] {
        LabelElement::BarcodeQr(qr) => {
            assert_eq!(qr.data, "This is the data portion");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::M);
            // c=4 → 4 × 0.01 in = 1.016 mm → 8 dots at 8 dpmm.
            assert_eq!(qr.barcode.magnification, 8);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::BarcodeQr(qr) => {
            assert_eq!(qr.data, "0123456789012345");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::H);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
    match &label.elements[2] {
        LabelElement::BarcodeQr(qr) => {
            // Model prefix `2,` accepted and skipped.
            assert_eq!(qr.data, "HTTPS://EXAMPLE.COM");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::H);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
}

#[test]
fn pdf417_options_map_and_datamatrix_autosizes() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(b'1', "z", b'0', b'0', 0, 150, 200, "F1000000PDF417 DATA"),
        &rec(b'1', "W1c", b'3', b'3', 0, 150, 20, "0000000000DM DATA"),
        "E",
    ]));
    match &label.elements[0] {
        LabelElement::BarcodePdf417(bc) => {
            assert_eq!(bc.barcode.module_width, 2, "0 multiplier → 2-dot default");
            assert_eq!(bc.barcode.row_height, 6, "0 multiplier → 6-dot default");
            assert!(!bc.barcode.truncate);
            assert_eq!(bc.barcode.security, 1);
            assert_eq!(bc.data, "PDF417 DATA");
        }
        other => panic!("expected BarcodePdf417, got {other:?}"),
    }
    match &label.elements[1] {
        LabelElement::BarcodeDatamatrix(bc) => {
            assert_eq!(bc.barcode.height, 3, "module multiplier");
            assert_eq!(bc.barcode.rows, 0, "000 = auto size");
            assert_eq!(bc.data, "DM DATA");
        }
        other => panic!("expected BarcodeDatamatrix, got {other:?}"),
    }
}

#[test]
fn maxicode_fixed_field_form_builds_structured_payload() {
    let label = parse_one(&dpl(&[
        "L",
        &rec(
            b'1',
            "u",
            b'0',
            b'0',
            0,
            10,
            10,
            "327895555840611THIS PACKAGE IS GOING TO DATAMAXCORP",
        ),
        "E",
    ]));
    match &label.elements[0] {
        LabelElement::Maxicode(m) => {
            assert_eq!(m.code.mode, 2);
            // class + country + zip9, then the structured message header.
            assert!(m.data.starts_with("611840327895555"), "{}", m.data);
            assert!(
                m.data
                    .ends_with("[)>\x1e\x1dTHIS PACKAGE IS GOING TO DATAMAXCORP"),
                "{}",
                m.data
            );
            assert!(labelize::barcodes::maxicode::encode(&m.data, m.code.mode).is_ok());
        }
        other => panic!("expected Maxicode, got {other:?}"),
    }

    // The [)>RS… structured form is rejected explicitly.
    let src = dpl(&["L", &rec(b'1', "u", b'0', b'0', 0, 10, 10, "[)> data"), "E"]);
    let err = DplParser::new().parse(&src).unwrap_err();
    assert!(err.contains("structured"), "{err}");
}

#[test]
fn terminator_command_reassigns_record_end() {
    // T~ makes ~ the field terminator; the data stops at the first ~ and
    // the remainder ("B") becomes a plain (ignored) line.
    let src = b"\x02L\rT~\r191100000000100A~B\rE\r".to_vec();
    let label = parse_one(&src);
    match &label.elements[0] {
        LabelElement::Text(t) => assert_eq!(t.text, "A"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn sessions_e_prints_x_renders_and_stx_l_opens() {
    let labels = parse(&dpl(&[
        "L",
        &rec(b'1', "0", b'1', b'1', 0, 10, 10, "one"),
        "E",
    ]));
    assert_eq!(labels.len(), 1);

    // X terminates without printing; the image is the same, so both flush.
    let labels = parse(&dpl(&[
        "L",
        &rec(b'1', "0", b'1', b'1', 0, 10, 10, "two"),
        "X",
    ]));
    assert_eq!(labels.len(), 1);
}

#[test]
fn system_level_commands_between_formats_are_ignored() {
    let mut src = b"\x02D10\r\x02S1\r\x02L\r".to_vec();
    src.extend_from_slice(rec(b'1', "0", b'1', b'1', 0, 10, 10, "kept").as_bytes());
    src.extend_from_slice(b"\rQ0001\rE\r");
    let label = parse_one(&src);
    assert_eq!(label.elements.len(), 1);
    assert_eq!(text_at(&label, 0).text, "kept");
}

#[test]
fn language_detection_and_front_door() {
    let src = dpl(&["L", &rec(b'1', "0", b'1', b'1', 0, 10, 10, "X"), "E"]);
    assert_eq!(labelize::detect_language(&src), Language::Dpl);
    let labels = labelize::parse_labels(Language::Dpl, &src, 8).unwrap();
    assert_eq!(labels.len(), 1);
}

#[test]
fn formatter_round_trips_fixtures() {
    let src = std::fs::read("testdata/unit/dpl_shipping.dpl").unwrap();
    let before = DplParser::with_dpmm(8).parse(&src).unwrap();
    let (_, formatted) = labelize::format_source(&src, Some(Language::Dpl));
    let after = DplParser::with_dpmm(8).parse(&formatted).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before[0].elements.len(), after[0].elements.len());
}
