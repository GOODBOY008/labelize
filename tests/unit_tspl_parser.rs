use labelize::elements::barcode_qr::QrErrorCorrectionLevel;
use labelize::elements::field_alignment::FieldAlignment;
use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::font_face::FontFace;
use labelize::elements::label_element::LabelElement;
use labelize::{Language, TsplParser};

fn parse(tspl: &[u8]) -> Vec<LabelInfo> {
    TsplParser::new().parse(tspl).expect("TSPL parse failed")
}

/// Like `parse`, but asserts a label was produced (commands that leave no
/// elements emit no LabelInfo, matching the ZPL/EPL empty-label convention).
fn parse_one(tspl: &[u8]) -> LabelInfo {
    let labels = parse(tspl);
    assert!(!labels.is_empty(), "expected one label, got none");
    labels.into_iter().next().unwrap()
}
use labelize::LabelInfo;

#[test]
fn size_carries_physical_dims_on_label_info() {
    let labels = parse(
        br#"SIZE 100 mm,50 mm
CLS
TEXT 10,20,"3",0,1,1,"Hello"
PRINT 1
"#,
    );

    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].print_width, 0);
    assert_eq!(labels[0].label_width_mm, Some(100.0));
    assert_eq!(labels[0].label_height_mm, Some(50.0));
}

#[test]
fn size_supports_inches_and_dots() {
    let labels = parse(
        br#"SIZE 2,1
CLS
TEXT 0,0,"1",0,1,1,"A"
PRINT 1
SIZE 812 dot,406 dot
CLS
TEXT 0,0,"1",0,1,1,"B"
PRINT 1
"#,
    );

    assert_eq!(labels.len(), 2);
    assert!((labels[0].label_width_mm.unwrap() - 50.8).abs() < 0.001);
    assert!((labels[0].label_height_mm.unwrap() - 25.4).abs() < 0.001);
    assert!((labels[1].label_width_mm.unwrap() - 101.5).abs() < 0.001);
    assert!((labels[1].label_height_mm.unwrap() - 50.75).abs() < 0.001);
}

#[test]
fn text_maps_font_rotation_alignment_and_reference_offsets() {
    let labels = parse(
        br#"SIZE 4,2
REFERENCE 10,20
SHIFT 3,4
CLS
TEXT 100,50,"3",90,2,3,2,"Aligned"
PRINT 1
"#,
    );

    let text = match &labels[0].elements[0] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(text.text, "Aligned");
    assert_eq!(text.position.x, 113);
    assert_eq!(text.position.y, 74);
    assert_eq!(text.font.orientation, FieldOrientation::Rotated90);
    assert_eq!(text.font.width, 32.0);
    assert_eq!(text.font.height, 72.0);
    assert_eq!(text.alignment, FieldAlignment::Center);
}

#[test]
fn text_resolves_tspl_resident_font_cells_parser_side() {
    let labels = parse(
        br#"SIZE 4,2
CLS
TEXT 0,0,"1",0,1,1,"F1"
TEXT 0,20,"5",0,2,3,"F5"
TEXT 0,80,"8",0,1,2,"F8"
PRINT 1
"#,
    );

    let text = |idx| match &labels[0].elements[idx] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };

    // Font cells and faces are resolved at parse time; the renderer never
    // sees a TSPL font name.
    assert_eq!(text(0).font.name, "1");
    assert_eq!(text(0).font.face, FontFace::BitmapMono);
    assert_eq!(text(0).font.width, 8.0);
    assert_eq!(text(0).font.height, 12.0);

    assert_eq!(text(1).font.name, "5");
    assert_eq!(text(1).font.width, 64.0);
    assert_eq!(text(1).font.height, 144.0);

    assert_eq!(text(2).font.name, "8");
    assert_eq!(text(2).font.width, 14.0);
    assert_eq!(text(2).font.height, 50.0);
}

#[test]
fn text_supports_tspl_bitmap_file_fonts() {
    let labels = parse(
        br#"SIZE 4,2
CLS
TEXT 0,0,"TSS24.BF2",0,1,1,"F24"
PRINT 1
"#,
    );

    match &labels[0].elements[0] {
        LabelElement::Text(text) => {
            assert_eq!(text.font.name, "TSS24.BF2");
            assert_eq!(text.font.face, FontFace::BitmapSquare);
            assert_eq!(text.font.width, 24.0);
            assert_eq!(text.font.height, 24.0);
        }
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn unknown_font_falls_back_to_font_1() {
    let labels = parse(
        br#"SIZE 4,2
CLS
TEXT 0,0,"9",0,1,1,"X"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::Text(text) => {
            assert_eq!(text.font.face, FontFace::BitmapMono);
            assert_eq!(text.font.width, 8.0);
            assert_eq!(text.font.height, 12.0);
        }
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn barcode_maps_types_and_human_readable() {
    let labels = parse(
        br#"SIZE 4,2
CLS
BARCODE 10,10,"128",80,1,0,2,3,"A123"
BARCODE 10,120,"EAN128",80,0,0,2,3,"00123456789012345678"
BARCODE 10,230,"39C",80,0,0,2,4,"ABC-123"
BARCODE 10,340,"EAN13",80,0,0,2,3,"4006381333931"
PRINT 1
"#,
    );

    match &labels[0].elements[0] {
        LabelElement::Barcode128(bc) => {
            assert_eq!(
                bc.barcode.mode,
                labelize::elements::barcode_128::BarcodeMode::Automatic
            );
            assert!(bc.barcode.line);
            assert_eq!(bc.width, 2);
            assert_eq!(bc.data, "A123");
        }
        other => panic!("expected Barcode128, got {:?}", other),
    }
    match &labels[0].elements[1] {
        LabelElement::Barcode128(bc) => {
            assert_eq!(
                bc.barcode.mode,
                labelize::elements::barcode_128::BarcodeMode::Ucc
            );
            assert!(!bc.barcode.line);
        }
        other => panic!("expected Barcode128, got {:?}", other),
    }
    match &labels[0].elements[2] {
        LabelElement::Barcode39(bc) => {
            assert!(bc.barcode.check_digit, "39C enables the check digit");
            assert!((bc.width_ratio - 2.0).abs() < 1e-9);
        }
        other => panic!("expected Barcode39, got {:?}", other),
    }
    match &labels[0].elements[3] {
        LabelElement::BarcodeEan13(_) => {}
        other => panic!("expected BarcodeEan13, got {:?}", other),
    }
}

#[test]
fn unsupported_barcode_type_errors_explicitly() {
    let err = TsplParser::new()
        .parse(
            br#"SIZE 4,2
CLS
BARCODE 10,10,"93",80,1,0,2,3,"X"
PRINT 1
"#,
        )
        .unwrap_err();
    assert!(err.contains("unsupported TSPL barcode type"), "{err}");
}

#[test]
fn qr_carries_raw_payload_and_explicit_ecc() {
    let labels = parse(
        br#"SIZE 4,2
CLS
QRCODE 10,10,H,4,A,0,M2,"HTTPS://EXAMPLE.COM/P?V=1"
PRINT 1
"#,
    );

    match &labels[0].elements[0] {
        LabelElement::BarcodeQr(qr) => {
            // Raw payload — no ZPL ^FD-style format prefix is synthesized.
            assert_eq!(qr.data, "HTTPS://EXAMPLE.COM/P?V=1");
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::H);
            assert_eq!(qr.barcode.magnification, 4);
            assert_eq!(qr.barcode.orientation, FieldOrientation::Normal);
        }
        other => panic!("expected BarcodeQr, got {:?}", other),
    }
}

#[test]
fn direction_zero_inverts_the_label() {
    let normal = parse_one(b"SIZE 4,2\nCLS\nTEXT 10,10,\"1\",0,1,1,\"X\"\nPRINT 1\n");
    assert!(!normal.inverted);

    let inverted =
        parse_one(b"SIZE 4,2\nDIRECTION 0\nCLS\nTEXT 10,10,\"1\",0,1,1,\"X\"\nPRINT 1\n");
    assert!(inverted.inverted);
}

#[test]
fn cls_clears_pending_elements() {
    let labels = parse(
        br#"SIZE 4,2
CLS
TEXT 10,10,"1",0,1,1,"Dropped"
CLS
TEXT 10,10,"1",0,1,1,"Kept"
PRINT 1
"#,
    );
    assert_eq!(labels.len(), 1);
    match &labels[0].elements[0] {
        LabelElement::Text(text) => assert_eq!(text.text, "Kept"),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn bitmap_reads_binary_data_by_byte_cursor() {
    let mut data = b"SIZE 4,2\nCLS\nBITMAP 10,10,4,4,0,".to_vec();
    let payload = [
        0xF0, 0x0F, 0x80, 0xFF, 0, b'\n', b'\r', b' ', b'\t', b',', b'P', b'R', b'I', b'N', b'T',
        b'1',
    ];
    data.extend_from_slice(&payload);
    data.extend_from_slice(b"\nPRINT 1\nCLS\nBAR 0,0,1,1\nPRINT 1\n");

    let labels = parse(&data);
    assert_eq!(labels.len(), 2, "commands after the binary payload survive");
    match &labels[0].elements[0] {
        LabelElement::GraphicField(gf) => {
            assert_eq!(gf.row_bytes, 4);
            assert_eq!(gf.data_bytes, 16);
            assert_eq!(gf.data, payload);
        }
        other => panic!("expected GraphicField, got {:?}", other),
    }
}

#[test]
fn bitmap_exact_hex_payloads_decode_without_consuming_following_commands() {
    for payload in ["FF00aB7f", "FF 00\taB 7f \t"] {
        for ending in ["\n", "\r\n", "\r", ""] {
            let tail = if ending.is_empty() {
                ""
            } else {
                "PRINT 1\nCLS\nBAR 0,0,1,1\nPRINT 1\n"
            };
            let src = format!("BITMAP 0,0,2,2,0,{payload}{ending}{tail}");
            let labels = parse(src.as_bytes());
            assert_eq!(labels.len(), if ending.is_empty() { 1 } else { 2 });
            let LabelElement::GraphicField(gf) = &labels[0].elements[0] else {
                panic!("expected bitmap")
            };
            assert_eq!(gf.data, [0xff, 0, 0xab, 0x7f]);
            assert!(gf.zero_bits_black);
        }
    }
}

#[test]
fn bitmap_ambiguous_or_malformed_hex_stays_binary() {
    // All-hex binary, whitespace bytes, embedded line endings, short/odd/
    // overlong hex, and a full hex prefix followed by non-hex are all raw.
    for payload in [
        "ABCD",
        "A B\t",
        "AB\nC",
        "AB\rC",
        "ABC",
        "ABCDEF",
        "ABCDEF0123",
        "ABCDEF01Z",
    ] {
        let src = format!("BITMAP 0,0,4,1,0,{payload}\nPRINT 1\n");
        let labels = parse(src.as_bytes());
        let LabelElement::GraphicField(gf) = &labels[0].elements[0] else {
            panic!("expected bitmap")
        };
        let expected = format!("{payload}\n");
        assert_eq!(gf.data, &expected.as_bytes()[..4], "{payload:?}");
    }
    // The original issue has 32 ASCII digits but declares 32 bytes. It is
    // indistinguishable from a complete binary payload: do not halve it.
    let payload = b"FF00FF00FF00FF00FF00FF00FF00FF00";
    let mut src = b"BITMAP 0,0,4,8,0,".to_vec();
    src.extend_from_slice(payload);
    src.extend_from_slice(b"\nPRINT 1\n");
    let labels = parse(&src);
    let LabelElement::GraphicField(gf) = &labels[0].elements[0] else {
        panic!("expected bitmap")
    };
    assert_eq!(gf.data, payload);
}

#[test]
fn bitmap_modes_map_and_carry_tsc_polarity() {
    let mut data = b"SIZE 4,2\nCLS\n".to_vec();
    let payload = [0xF0u8; 16];
    for (y, mode) in [(40i32, 0i32), (140, 1), (240, 2)] {
        data.extend_from_slice(format!("BITMAP 10,{y},4,4,{mode},").as_bytes());
        data.extend_from_slice(&payload);
        data.push(b'\n');
    }
    data.extend_from_slice(b"PRINT 1\n");

    let labels = parse(&data);
    let modes = |i: usize| match &labels[0].elements[i] {
        LabelElement::GraphicField(gf) => (gf.mode, gf.zero_bits_black),
        other => panic!("expected GraphicField, got {other:?}"),
    };
    use labelize::elements::graphic_field::GraphicFieldMode;
    assert_eq!(modes(0).0, GraphicFieldMode::Overwrite);
    assert_eq!(modes(1).0, GraphicFieldMode::Or);
    assert_eq!(modes(2).0, GraphicFieldMode::Xor);
    // TSC BITMAP prints cleared bits (inverse of ^GF/GW) — resolved in the
    // element so the renderer stays language-blind.
    for i in 0..3 {
        assert!(modes(i).1, "BITMAP must carry zero-bits-black polarity");
    }
}

#[test]
fn device_setup_commands_are_noops() {
    let labels = parse(
        br#"GAP 0,0
DENSITY 8
SPEED 4
CODEPAGE 1252
OFFSET 0
SIZE 4,2
CLS
TEXT 10,10,"1",0,1,1,"X"
PRINT 1
"#,
    );
    assert_eq!(labels.len(), 1);
}

#[test]
fn language_detection_and_front_door() {
    assert_eq!(
        labelize::detect_language(b"SIZE 4,2\nCLS\nTEXT 10,10,\"1\",0,1,1,\"Hi\"\nPRINT 1\n"),
        Language::Tspl
    );
    let labels = labelize::parse_labels(
        Language::Tspl,
        b"SIZE 76 mm,50 mm\nCLS\nTEXT 10,10,\"1\",0,1,1,\"Hi\"\nPRINT 1\n",
        8,
    )
    .unwrap();
    assert_eq!(labels[0].label_width_mm, Some(76.0));
}

#[test]
fn pdf417_options_map_to_the_element() {
    let labels = parse(
        br#"SIZE 4,2
CLS
PDF417 10,10,200,80,0,0,E3,R4,C4,H10,"DATA"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::BarcodePdf417(bc) => {
            assert_eq!(bc.barcode.security, 3);
            assert_eq!(bc.barcode.rows, 4);
            assert_eq!(bc.barcode.columns, 4);
            assert_eq!(bc.barcode.row_height, 10);
            assert_eq!(bc.data, "DATA");
        }
        other => panic!("expected BarcodePdf417, got {:?}", other),
    }
}

#[test]
fn pdf417_compound_options_split_into_letter_runs() {
    // TSC's documented sample packs options into one token and marks the
    // raw expression with a trailing L<len>: `PDF417 …,E30R4C4L12`.
    let labels = parse(
        br#"SIZE 4,2
CLS
PDF417 10,10,200,80,0,0,2,E30R4C4L12
HELLO PDF417
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::BarcodePdf417(bc) => {
            // E's domain is 0-8 (single digit): the TSC doc sample
            //  reads as E3 + stray 0, not ECC 30.
            assert_eq!(bc.barcode.security, 3);
            assert_eq!(bc.barcode.rows, 4);
            assert_eq!(bc.barcode.columns, 4);
            assert_eq!(bc.data, "HELLO PDF417");
        }
        other => panic!("expected BarcodePdf417, got {other:?}"),
    }
}

#[test]
fn pdf417_trailing_l_suffix_without_other_options() {
    // A bare `L<len>` token still works (its own form).
    let labels = parse(
        br#"SIZE 4,2
CLS
PDF417 10,10,200,80,0,0,L5
12345
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::BarcodePdf417(bc) => {
            assert_eq!(bc.barcode.rows, 0);
            assert_eq!(bc.data, "12345");
        }
        other => panic!("expected BarcodePdf417, got {other:?}"),
    }
}

#[test]
fn qr_ecc_defaults_to_m_when_omitted() {
    // TSC QRCODE default error-correction level is M.
    let labels = parse(
        br#"SIZE 4,2
CLS
QRCODE 10,10,,4,A,0,M2,"X"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::BarcodeQr(qr) => {
            assert_eq!(qr.error_correction, QrErrorCorrectionLevel::M);
        }
        other => panic!("expected BarcodeQr, got {other:?}"),
    }
}

#[test]
fn barcode_itf14_maps_to_2of5_with_check_digit() {
    let labels = parse(
        br#"SIZE 4,2
CLS
BARCODE 10,10,"ITF14",80,1,0,2,3,"12345678901231"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::Barcode2of5(bc) => {
            assert!(
                bc.barcode.check_digit,
                "ITF14 always carries the check digit"
            );
            assert_eq!(bc.data, "12345678901231");
        }
        other => panic!("expected Barcode2of5, got {other:?}"),
    }
}

#[test]
fn dmatrix_maps_options_and_raw_content() {
    // Option shape from a real-world TSC capture (FeederCable):
    // DMATRIX 187,415,187,415,x8,r180,14,14,"P203S730"
    let labels = parse(
        br#"SIZE 4,2
CLS
DMATRIX 187,415,187,415,x8,r180,14,14,"P203S730"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::BarcodeDatamatrix(bc) => {
            assert_eq!(bc.barcode.quality, 200);
            // x8 is the module size in dots (TSPL manual), not the area
            // height (which is 415 here).
            assert_eq!(bc.barcode.height, 8);
            assert_eq!(bc.barcode.rows, 14);
            assert_eq!(bc.barcode.columns, 14);
            assert_eq!(
                bc.barcode.orientation,
                labelize::elements::field_orientation::FieldOrientation::Rotated180
            );
            assert_eq!(bc.data, "P203S730");
        }
        other => panic!("expected BarcodeDatamatrix, got {other:?}"),
    }
}

#[test]
fn dmatrix_module_size_default_and_option_map() {
    // The manual's option order is c#,x#,r#,a#,row,col — a1 is rectangular,
    // c126 activates ~ escapes, row comes before col, and the symbol area
    // width/height never feed the module size.
    let labels = parse(
        br#"SIZE 4,2
CLS
DMATRIX 560,540,180,180,x5,"FLOWSTOCK|SCANNER-DIAG|V1"
DMATRIX 100,110,400,400,"DMATRIX EXAMPLE 1"
DMATRIX 100,50,100,100,c126,x6,r90,a1,16,48,"DMATRIX EXAMPLE 4"
PRINT 1
"#,
    );
    let element = match &labels[0].elements[0] {
        LabelElement::BarcodeDatamatrix(bc) => bc,
        other => panic!("expected BarcodeDatamatrix, got {other:?}"),
    };
    // Real-world TE210 capture shape: x5 → 5-dot modules; 180 (the symbol
    // area height) must not leak into the module size (it used to render
    // the symbol as a giant black slab).
    assert_eq!(element.barcode.height, 5);
    assert_eq!(element.data, "FLOWSTOCK|SCANNER-DIAG|V1");

    match &labels[0].elements[1] {
        LabelElement::BarcodeDatamatrix(bc) => {
            // No x#: default module size 4, calibrated from the manual's
            // result figure (an 18×18 symbol of ~72 dots).
            assert_eq!(bc.barcode.height, 4);
            assert_eq!(bc.barcode.escape, 0);
            assert_eq!(
                bc.barcode.ratio,
                Some(labelize::elements::barcode_datamatrix::DatamatrixRatio::Square)
            );
        }
        other => panic!("expected BarcodeDatamatrix, got {other:?}"),
    }

    match &labels[0].elements[2] {
        LabelElement::BarcodeDatamatrix(bc) => {
            assert_eq!(bc.barcode.height, 6);
            assert_eq!(bc.barcode.escape, 126);
            assert_eq!(bc.barcode.rows, 16);
            assert_eq!(bc.barcode.columns, 48);
            assert_eq!(
                bc.barcode.orientation,
                labelize::elements::field_orientation::FieldOrientation::Rotated90
            );
            assert_eq!(
                bc.barcode.ratio,
                Some(labelize::elements::barcode_datamatrix::DatamatrixRatio::Rectangular)
            );
        }
        other => panic!("expected BarcodeDatamatrix, got {other:?}"),
    }
}

#[test]
fn comment_lines_and_template_placeholders_pass_through() {
    // Real captures carry `;` comment lines and ${placeholder} template
    // text; both must parse without error.
    let labels = parse(
        br#"; a comment line
SIZE 40,57
CLS
TEXT 17,11,"3",20,1,1,"${drawing}"
PRINT 1
"#,
    );
    assert_eq!(labels.len(), 1);
}

#[test]
fn reverse_maps_to_print_time_region_inversion() {
    let labels = parse(
        br#"SIZE 4,2
CLS
REVERSE 330,300,300,76
TEXT 350,326,"2",0,1,1,"6  REVERSE"
PRINT 1
"#,
    );
    match &labels[0].elements[0] {
        LabelElement::InvertRegion(r) => {
            assert_eq!(
                (r.position.x, r.position.y, r.width, r.height),
                (330, 300, 300, 76)
            );
        }
        other => panic!("expected InvertRegion, got {other:?}"),
    }
}

#[test]
fn empty_text_field_is_skipped() {
    let labels = parse(
        br#"SIZE 4,2
CLS
TEXT 10,10,"1",0,1,1,""
TEXT 10,40,"1",0,1,1,"Kept"
PRINT 1
"#,
    );
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].elements.len(), 1);
}
