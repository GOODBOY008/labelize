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

#[test]
fn pdf417_raw_and_quoted_payloads_keep_delimiters_and_following_fields() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"E2,C2,W2,H6,L5,,ABCD", b",ABCD"),
        (b"\" ,ABCD \"", b" ,ABCD "),
        (b"\",L12,part\"", b",L12,part"),
        (
            b"W2,\"one,\\\"two,L5,three\\\"\r\nfour\"",
            b"one,\"two,L5,three\"\r\nfour",
        ),
        (b"E30R4C4L5\r\n,ABCD", b",ABCD"),
        (b"E2,\r\nW2,C2L3,,,,", b",,,"),
        (b"L8,\r\n\x00,tail", b"\r\n\x00,tail"),
        (b"L8,\nPRINT 1", b"\nPRINT 1"),
        (b"L11,X\r\nCLS\r\nend", b"X\r\nCLS\r\nend"),
        (b"L5,\"\xc3\xa9ABC\"", b"\xc3\xa9ABC"),
        (b"L4,\"A\r\nB\"", b"A\r\nB"),
        (b"L2,\"A\n\" \t", b"A\n"),
        (b"L5,\"ABC\"", b"\"ABC\""),
    ];
    for (last, payload) in cases {
        let command = [b"PDF417 0,0,300,100,0,".as_slice(), last].concat();
        for eol in [b"\r\n".as_slice(), b"\n", b"\r"] {
            let lines = [
                command.as_slice(),
                b"BAR 7,9,2,2",
                b"PRINT 1,1",
                b"CLS",
                b"",
            ];
            let source = lines.join(eol);
            let (_, formatted) = labelize::format_source(&source, Some(Language::Tspl));
            assert_eq!(
                formatted,
                lines.join(b"\n".as_slice()),
                "internal byte preservation"
            );
            for dpmm in [8, 12] {
                for job in [&source, &formatted] {
                    let labels = labelize::parse_labels(Language::Tspl, job, dpmm).unwrap();
                    assert_eq!(labels.len(), 1);
                    let [LabelElement::BarcodePdf417(code), LabelElement::GraphicBox(bar)] =
                        labels[0].elements.as_slice()
                    else {
                        panic!("unexpected fields for {command:?}")
                    };
                    assert_eq!(code.data.as_bytes(), *payload);
                    assert_eq!((bar.position.x, bar.position.y), (7, 9));
                }
            }
        }
    }
}

#[test]
fn pdf417_invalid_utf8_is_reported_without_replacing_or_damaging_bytes() {
    for last in [b"L3,,\xff\0".as_slice(), b"\",\xff\0\""] {
        let job = [b"PDF417 0,0,300,100,0,".as_slice(), last, b"\nPRINT 1,1\n"].concat();
        let (_, formatted) = labelize::format_source(&job, Some(Language::Tspl));
        assert_eq!(job, formatted);
        for dpmm in [8, 12] {
            let error = labelize::parse_labels(Language::Tspl, &job, dpmm).unwrap_err();
            assert!(error.to_string().contains("non-UTF-8"), "{error}");
        }
    }
}

#[test]
fn pdf417_truncated_payload_is_reported_and_formats_verbatim() {
    let job = b"PDF417 0,0,300,100,0,L40,,abc\r\nPRINT 1\r\n\0\xff  \t\r";
    assert!(labelize::parse_labels(Language::Tspl, job, 8)
        .unwrap_err()
        .to_string()
        .contains("truncated"));
    let (_, formatted) = labelize::format_source(job, Some(Language::Tspl));
    assert_eq!(formatted, job);
}

#[test]
fn counted_mpdf417_reports_unsupported_command() {
    let job = b"MPDF417 0,0,300,100,0,L5,,ABCD\r\nPRINT 1,1\r\n";
    let (_, formatted) = labelize::format_source(job, Some(Language::Tspl));
    for source in [job.as_slice(), &formatted] {
        let error = labelize::parse_labels(Language::Tspl, source, 8).unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported TSPL command MPDF417"));
    }
}

#[test]
fn quoted_text_keeps_legacy_escapes_and_multiline_content() {
    for (encoded, expected) in [
        (r#"x\"y\",z\\q"#, "x\"y\",z\\\\q"),
        ("x\\\"\r\ntwo\r\nthree", "x\"\r\ntwo\r\nthree"),
        ("A\\\"\r\nPRINT 1\r\nB", "A\"\r\nPRINT 1\r\nB"),
        ("Use \\\"\r\nSIZE matters", "Use \"\r\nSIZE matters"),
        ("Use \\\"\r\nPDF417 matters", "Use \"\r\nPDF417 matters"),
        ("A\\\"\r\nB,", "A\"\r\nB,"),
        ("x\\[\"]\r\nPRINT 1\r\nthree", "x\"\r\nPRINT 1\r\nthree"),
        (r#"x\\\"#, r"x\\\"),
    ] {
        let fields = [
            format!(r#"TEXT 10,10,"3",0,1,1,"{encoded}""#),
            r#"  tExt 10,65,"3",0,1,1,"NEXT""#.into(),
            "PRINT 1".into(),
        ];
        let source = fields.join("\r\n");
        let (_, formatted) = labelize::format_source(source.as_bytes(), Some(Language::Tspl));
        for job in [source.as_bytes(), &formatted] {
            let label = parse_one(job);
            let [LabelElement::Text(first), LabelElement::Text(next)] = label.elements.as_slice()
            else {
                panic!("missing text or following command: {source:?}");
            };
            assert_eq!(first.text, expected);
            assert_eq!(next.text, "NEXT");
            assert_eq!(next.position.y, 65);
        }
    }
}

#[test]
fn trailing_backslash_preserves_opaque_records_and_their_bytes() {
    let payload = b"\x80\"\r\n";
    let bitmap = [b"BITMAP 2,2,4,1,0,".as_slice(), payload].concat();
    for comment in [
        None,
        Some(b"REM product location".as_slice()),
        Some(b"   rEm opaque comment with an unmatched \""),
        Some(b"SET \"a device directive with an unmatched opening quote"),
        Some(b"VENDOR \"an opaque command with an unmatched opening quote"),
    ] {
        let mut lines = vec![br#"TEXT 0,0,"3",0,1,1,"A\""#.as_slice()];
        lines.extend(comment);
        lines.extend([
            bitmap.as_slice(),
            br#"TEXT 0,65,"3",0,1,1,"NEXT""#,
            b"PRINT 1",
            b"",
        ]);
        let source = lines.join(b"\r\n".as_slice());
        let (_, formatted) = labelize::format_source(&source, Some(Language::Tspl));
        let expected: Vec<_> = lines.iter().map(|line| line.trim_ascii_start()).collect();
        assert_eq!(formatted, expected.join(b"\n".as_slice()));
        for job in [&source, &formatted] {
            let label = parse_one(job);
            let [LabelElement::Text(first), LabelElement::GraphicField(graphic), LabelElement::Text(next)] =
                label.elements.as_slice()
            else {
                panic!("expected both fields and the bitmap")
            };
            assert_eq!(first.text, r"A\");
            assert_eq!(graphic.data, payload);
            assert_eq!(next.text, "NEXT");
        }
    }
}

#[test]
fn trailing_backslash_before_a_qr_field_that_opens_at_end_of_line() {
    let lines = [
        br#"TEXT 0,0,"3",0,1,1,"A\""#.as_slice(),
        b"QRCODE 10,10,M,4,A,0,\"\r\nDATA\"",
        b"PRINT 1",
        b"",
    ];
    let source = lines.join(b"\r\n".as_slice());
    let (_, formatted) = labelize::format_source(&source, Some(Language::Tspl));
    assert_eq!(formatted, lines.join(b"\n".as_slice()));
    for job in [&source, &formatted] {
        let label = parse_one(job);
        let [LabelElement::Text(text), LabelElement::BarcodeQr(qr)] = label.elements.as_slice()
        else {
            panic!("expected text and separate QR field")
        };
        assert_eq!(text.text, r"A\");
        assert_eq!(qr.data, "\r\nDATA");
    }
}
