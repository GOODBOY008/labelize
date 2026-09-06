use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::label_element::LabelElement;
use labelize::EplParser;

fn parse(epl: &str) -> Vec<labelize::LabelInfo> {
    let parser = EplParser::new();
    parser.parse(epl.as_bytes()).expect("EPL parse failed")
}

// ─── Single label ───

#[test]
fn parse_single_label() {
    let labels = parse("N\nA10,20,0,1,1,1,N,\"Hello\"\nP1\n");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].elements.len(), 1);
}

// ─── Text command ───

#[test]
fn parse_text() {
    let labels = parse("N\nA50,100,0,2,1,1,N,\"Hello World\"\nP1\n");
    let tf = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(tf.text, "Hello World");
    assert_eq!(tf.position.x, 50);
    assert_eq!(tf.position.y, 100);
    assert_eq!(tf.font.orientation, FieldOrientation::Normal);
    // Font 2 base: 10x16, mult 1x1 → Width == Height
    assert_eq!(tf.font.width, 16.0);
    assert_eq!(tf.font.height, 16.0);
}

#[test]
fn parse_text_rotated() {
    let labels = parse("N\nA50,100,1,1,1,1,N,\"Rotated\"\nP1\n");
    let tf = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(tf.font.orientation, FieldOrientation::Rotated90);
}

#[test]
fn parse_text_reverse() {
    let labels = parse("N\nA50,100,0,1,1,1,R,\"Reverse\"\nP1\n");
    let tf = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    assert!(tf.reverse_print.value, "expected reverse print to be true");
}

#[test]
fn parse_text_multiplier() {
    let labels = parse("N\nA10,20,0,3,2,3,N,\"Big\"\nP1\n");
    let tf = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    // Font 3 base: 12w x 20h, mult 2x3 → hMult≠vMult so width scaled by ratio
    assert_eq!(tf.font.width, 24.0);
    assert_eq!(tf.font.height, 60.0);
}

#[test]
fn parse_empty_text_skipped() {
    let labels = parse("N\nA10,20,0,1,1,1,N,\"\"\nP1\n");
    // Empty text should produce no elements, hence no label
    assert_eq!(labels.len(), 0);
}

// ─── Barcode command ───

#[test]
fn parse_barcode() {
    let labels = parse("N\nB50,100,0,1,3,6,200,B,\"12345\"\nP1\n");
    assert_eq!(labels[0].elements.len(), 1);
    let bc = match &labels[0].elements[0] {
        LabelElement::Barcode128(b) => b,
        other => panic!("expected Barcode128, got {:?}", other),
    };
    assert_eq!(bc.data, "12345");
    assert_eq!(bc.position.x, 50);
    assert_eq!(bc.position.y, 100);
    assert_eq!(bc.barcode.height, 200);
    assert!(bc.barcode.line, "expected human-readable line to be true");
}

#[test]
fn parse_barcode_code39() {
    // EPL2 Table 1: "3" = Code 39 std. or extended
    let labels = parse("N\nB50,100,0,3,2,5,100,N,\"ABC123\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::Barcode39(b) => b,
        other => panic!("expected Barcode39, got {:?}", other),
    };
    assert_eq!(bc.data, "ABC123");
    assert_eq!(bc.width, 2);
    assert_eq!(bc.width_ratio, 2.5);
}

#[test]
fn parse_barcode_code39_check_digit() {
    // EPL2 Table 1: "3C" = Code 39 with check digit
    let labels = parse("N\nB50,100,0,3C,2,5,100,N,\"ABC123\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::Barcode39(b) => b,
        other => panic!("expected Barcode39, got {:?}", other),
    };
    assert!(bc.barcode.check_digit, "expected check digit to be true");
}

#[test]
fn parse_barcode_interleaved_2of5() {
    // EPL2 Table 1: "2" = Interleaved 2 of 5
    let labels = parse("N\nB50,100,0,2,2,5,100,B,\"1234567890\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::Barcode2of5(b) => assert!(
            !b.barcode.check_digit,
            "plain 2of5 must not add a check digit"
        ),
        other => panic!("expected Barcode2of5, got {:?}", other),
    }
}

#[test]
fn parse_barcode_2of5_check_digit() {
    // EPL2 Table 1: "2C" = mod-10 check digit, "2D" = human readable check digit
    for code in ["2C", "2D"] {
        let labels = parse(&format!("N\nB50,100,0,{},2,5,100,B,\"1234\"\nP1\n", code));
        match &labels[0].elements[0] {
            LabelElement::Barcode2of5(b) => {
                assert!(b.barcode.check_digit, "{} must add a check digit", code)
            }
            other => panic!("expected Barcode2of5 for {}, got {:?}", code, other),
        }
    }
}

#[test]
fn parse_barcode_code128_modes() {
    // EPL2 Table 1: "0" = Code 128 UCC, "1" = auto, "1E" = UCC/EAN 128
    let cases = vec![
        ("0", labelize::elements::barcode_128::BarcodeMode::Ucc),
        ("1", labelize::elements::barcode_128::BarcodeMode::Automatic),
        ("1E", labelize::elements::barcode_128::BarcodeMode::Ean),
        // Subset pins have no BarcodeMode variant; auto re-encodes the data.
        (
            "1A",
            labelize::elements::barcode_128::BarcodeMode::Automatic,
        ),
        (
            "1B",
            labelize::elements::barcode_128::BarcodeMode::Automatic,
        ),
        (
            "1C",
            labelize::elements::barcode_128::BarcodeMode::Automatic,
        ),
    ];
    for (code, want) in cases {
        let labels = parse(&format!("N\nB50,100,0,{},2,5,100,N,\"12345\"\nP1\n", code));
        let bc = match &labels[0].elements[0] {
            LabelElement::Barcode128(b) => b,
            other => panic!("expected Barcode128 for {}, got {:?}", code, other),
        };
        assert_eq!(bc.barcode.mode, want, "type {}", code);
    }
}

#[test]
fn parse_barcode_ean13() {
    // EPL2 Table 1: "E30" = EAN-13
    let labels = parse("N\nB50,100,0,E30,2,5,100,B,\"4006381333931\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeEan13(b) => {
            assert_eq!(b.data, "4006381333931");
            assert_eq!(b.width, 2);
            assert!(b.barcode.line);
        }
        other => panic!("expected BarcodeEan13, got {:?}", other),
    }
}

#[test]
fn parse_barcode_ean8() {
    // EPL2 Table 1: "E80" = EAN-8
    let labels = parse("N\nB50,100,0,E80,2,5,100,B,\"1234567\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeEan8(b) => {
            assert_eq!(b.data, "1234567");
            assert_eq!(b.width, 2);
        }
        other => panic!("expected BarcodeEan8, got {:?}", other),
    }
}

#[test]
fn parse_barcode_upca() {
    // EPL2 Table 1: "UA0" = UPC-A
    let labels = parse("N\nB50,100,0,UA0,2,5,100,B,\"01234567890\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeUca(b) => {
            assert_eq!(b.data, "01234567890");
            assert_eq!(b.width, 2);
        }
        other => panic!("expected BarcodeUca, got {:?}", other),
    }
}

#[test]
fn parse_barcode_upce() {
    // EPL2 Table 1: "UE0" = UPC-E
    let labels = parse("N\nB50,100,0,UE0,2,5,100,B,\"01234565\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeUcpe(b) => {
            assert_eq!(b.data, "01234565");
            assert!(b.barcode.check_digit, "UPC-E check digit prints by default");
        }
        other => panic!("expected BarcodeUcpe, got {:?}", other),
    }
}

#[test]
fn parse_barcode_unsupported_type_errors() {
    // Symbologies from EPL2 Table 1 with no labelize encoder must fail
    // loudly instead of silently rendering another symbology.
    let unsupported = [
        ("9", "Code 93"),
        ("K", "Codabar"),
        ("E82", "EAN-8 2-digit add-on"),
        ("E85", "EAN-8 5-digit add-on"),
        ("E32", "EAN-13 2-digit add-on"),
        ("E35", "EAN-13 5-digit add-on"),
        ("UA2", "UPC-A 2-digit add-on"),
        ("UA5", "UPC-A 5-digit add-on"),
        ("UE2", "UPC-E 2-digit add-on"),
        ("UE5", "UPC-E 5-digit add-on"),
        ("2G", "German Post Code"),
        ("2U", "UPC Interleaved 2 of 5"),
        ("P", "Postnet"),
        ("PL", "Planet"),
        ("J", "Japanese Postnet"),
        ("L", "Plessey (MSI-1)"),
        ("M", "MSI-3"),
        ("1D", "Deutsche Post check digit"),
    ];
    let parser = EplParser::new();
    for (code, name) in unsupported {
        let epl = format!("N\nB50,100,0,{},2,5,100,N,\"12345\"\nP1\n", code);
        let err = parser
            .parse(epl.as_bytes())
            .expect_err(&format!("type {} must be rejected", code));
        assert!(
            err.contains(name),
            "error for {} should mention {}: {}",
            code,
            name,
            err
        );
    }
}

#[test]
fn parse_barcode_invalid_type_errors() {
    // Values that are not EPL2 Table 1 codes must be rejected.
    let parser = EplParser::new();
    for code in ["B", "G", "H", "7", "code128", ""] {
        let epl = format!("N\nB50,100,0,{},2,5,100,N,\"12345\"\nP1\n", code);
        assert!(
            parser.parse(epl.as_bytes()).is_err(),
            "invalid type {:?} must be rejected",
            code
        );
    }
}

// ─── Line draw command ───

#[test]
fn parse_line() {
    let labels = parse("N\nLO10,20,300,5\nP1\n");
    let gb = match &labels[0].elements[0] {
        LabelElement::GraphicBox(b) => b,
        other => panic!("expected GraphicBox, got {:?}", other),
    };
    assert_eq!(gb.position.x, 10);
    assert_eq!(gb.position.y, 20);
    assert_eq!(gb.width, 300);
    assert_eq!(gb.height, 5);
    assert_eq!(gb.border_thickness, 5);
}

// ─── Reference point ───

#[test]
fn parse_reference_point() {
    let labels = parse("N\nR40,10\nA50,100,0,1,1,1,N,\"Offset\"\nP1\n");
    let tf = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    // Position should include reference offset: (50+40, 100+10) = (90, 110)
    assert_eq!(tf.position.x, 90);
    assert_eq!(tf.position.y, 110);
}

// ─── Multiple labels ───

#[test]
fn parse_multiple_labels() {
    let labels = parse("N\nA10,20,0,1,1,1,N,\"Label1\"\nP1\nN\nA30,40,0,1,1,1,N,\"Label2\"\nP1\n");
    assert_eq!(labels.len(), 2);
    let tf1 = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(tf1.text, "Label1");
    let tf2 = match &labels[1].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(tf2.text, "Label2");
}

// ─── Without P command ───

#[test]
fn parse_without_p_command() {
    let labels = parse("N\nA10,20,0,1,1,1,N,\"NoPrint\"\n");
    assert_eq!(labels.len(), 1, "expected 1 label (auto-emitted)");
}

// ─── Ignored commands ───

#[test]
fn parse_ignored_commands() {
    let labels = parse("N\nQ822,24\nS4\nD15\nZB\nA10,20,0,1,1,1,N,\"Test\"\nP1\n");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].elements.len(), 1);
}

// ─── Mixed elements ───

#[test]
fn parse_mixed_elements() {
    let epl = "N\nA10,20,0,1,1,1,N,\"Header\"\nB50,100,0,1,3,6,200,N,\"12345\"\nLO0,300,400,2\nA10,320,0,2,1,1,N,\"Footer\"\nP1\n";
    let labels = parse(epl);
    assert_eq!(labels[0].elements.len(), 4);
    assert!(matches!(&labels[0].elements[0], LabelElement::Text(_)));
    assert!(matches!(
        &labels[0].elements[1],
        LabelElement::Barcode128(_)
    ));
    assert!(matches!(
        &labels[0].elements[2],
        LabelElement::GraphicBox(_)
    ));
    assert!(matches!(&labels[0].elements[3], LabelElement::Text(_)));
}

// ─── DPD UK EPL label ───

#[test]
fn parse_dpd_uk() {
    let file = std::fs::read("testdata/labels/dpduk.epl").expect("failed to read dpduk.epl");
    let parser = EplParser::new();
    let labels = parser.parse(&file).expect("EPL parse failed");

    assert!(!labels.is_empty(), "no labels parsed from dpduk.epl");
    let label = &labels[0];
    assert!(
        !label.elements.is_empty(),
        "no elements in the parsed label"
    );

    let mut texts = 0;
    let mut barcodes = 0;
    let mut lines = 0;
    for el in &label.elements {
        match el {
            LabelElement::Text(_) => texts += 1,
            LabelElement::Barcode128(_) => barcodes += 1,
            LabelElement::GraphicBox(_) => lines += 1,
            _ => {}
        }
    }

    assert!(texts > 0, "expected at least one text element");
    assert!(barcodes > 0, "expected at least one barcode element");
    assert!(lines > 0, "expected at least one line element");
}

#[test]
fn draw_dpd_uk() {
    use labelize::{DrawerOptions, Renderer};
    use std::io::Cursor;

    let file = std::fs::read("testdata/labels/dpduk.epl").expect("failed to read dpduk.epl");
    let parser = EplParser::new();
    let labels = parser.parse(&file).expect("EPL parse failed");
    assert!(!labels.is_empty(), "no labels parsed");

    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(&labels[0], &mut buf, DrawerOptions::default())
        .expect("render failed");
    assert!(buf.into_inner().len() > 0, "empty PNG output");
}

// ─── N resets reference point ───

#[test]
fn parse_n_resets_reference_point() {
    let labels =
        parse("N\nR40,10\nA10,20,0,1,1,1,N,\"First\"\nP1\nN\nA10,20,0,1,1,1,N,\"Second\"\nP1\n");
    assert_eq!(labels.len(), 2);

    let tf1 = match &labels[0].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    // First label has R40,10 offset
    assert_eq!(tf1.position.x, 50);
    assert_eq!(tf1.position.y, 30);

    let tf2 = match &labels[1].elements[0] {
        LabelElement::Text(t) => t,
        other => panic!("expected Text, got {:?}", other),
    };
    // Second label: N resets reference point to (0,0)
    assert_eq!(tf2.position.x, 10);
    assert_eq!(tf2.position.y, 20);
}

// ─── Font sizes ───

#[test]
fn parse_font_sizes() {
    let tests = vec![
        (1, 12.0, 12.0), // 8x12, equal mult → Width == Height
        (2, 16.0, 16.0), // 10x16
        (3, 20.0, 20.0), // 12x20
        (4, 24.0, 24.0), // 14x24
        (5, 48.0, 48.0), // 32x48
        (9, 12.0, 12.0), // Unknown font defaults to font 1 (8x12)
    ];

    for (font_num, expected_width, expected_height) in tests {
        let epl = format!("N\nA10,20,0,{},1,1,N,\"test\"\nP1\n", font_num);
        let labels = parse(&epl);
        let tf = match &labels[0].elements[0] {
            LabelElement::Text(t) => t,
            other => panic!("expected Text, got {:?}", other),
        };
        assert_eq!(
            tf.font.width, expected_width,
            "font {}: width = {}, want {}",
            font_num, tf.font.width, expected_width
        );
        assert_eq!(
            tf.font.height, expected_height,
            "font {}: height = {}, want {}",
            font_num, tf.font.height, expected_height
        );
    }
}

// ─── Rotations ───

#[test]
fn parse_rotations() {
    let tests = vec![
        (0, FieldOrientation::Normal),
        (1, FieldOrientation::Rotated90),
        (2, FieldOrientation::Rotated180),
        (3, FieldOrientation::Rotated270),
        (7, FieldOrientation::Normal), // Invalid defaults to normal
    ];

    for (rotation, expected) in tests {
        let epl = format!("N\nA10,20,{},1,1,1,N,\"test\"\nP1\n", rotation);
        let labels = parse(&epl);
        let tf = match &labels[0].elements[0] {
            LabelElement::Text(t) => t,
            other => panic!("expected Text, got {:?}", other),
        };
        assert_eq!(
            tf.font.orientation, expected,
            "rotation {}: got {:?}, want {:?}",
            rotation, tf.font.orientation, expected
        );
    }
}

// ─── Parser robustness ───

#[test]
fn empty_input_does_not_panic() {
    let parser = EplParser::new();
    let result = parser.parse(b"");
    assert!(result.is_ok());
}

#[test]
fn garbage_input_does_not_panic() {
    let parser = EplParser::new();
    let result = parser.parse(b"not EPL at all!");
    assert!(result.is_ok());
}

// ─── P1: LW (white line) ───

#[test]
fn parse_line_white() {
    let labels = parse("N\nLW10,20,300,5\nP1\n");
    let gb = match &labels[0].elements[0] {
        LabelElement::GraphicBox(b) => b,
        other => panic!("expected GraphicBox, got {:?}", other),
    };
    assert_eq!(gb.position.x, 10);
    assert_eq!(gb.position.y, 20);
    assert_eq!(gb.width, 300);
    assert_eq!(gb.height, 5);
    assert!(matches!(
        gb.line_color,
        labelize::elements::line_color::LineColor::White
    ));
}

// ─── P1: LS (diagonal line) ───

#[test]
fn parse_diagonal_top_to_bottom() {
    // LS,x1,y1,thickness,x2,y2 (per sharpzebra / manual example LS10,10,20,200,200)
    let labels = parse("N\nLS10,10,20,200,200\nP1\n");
    let dl = match &labels[0].elements[0] {
        LabelElement::DiagonalLine(dl) => dl,
        other => panic!("expected DiagonalLine, got {:?}", other),
    };
    assert_eq!(dl.position.x, 10);
    assert_eq!(dl.position.y, 10);
    assert_eq!(dl.width, 190);
    assert_eq!(dl.height, 190);
    assert_eq!(dl.border_thickness, 20);
    assert!(dl.top_to_bottom, "same-sign dx/dy draws a \\ diagonal");
}

#[test]
fn parse_diagonal_bottom_to_top() {
    let labels = parse("N\nLS200,10,5,10,200\nP1\n");
    let dl = match &labels[0].elements[0] {
        LabelElement::DiagonalLine(dl) => dl,
        other => panic!("expected DiagonalLine, got {:?}", other),
    };
    assert_eq!(dl.position.x, 10);
    assert_eq!(dl.position.y, 10);
    assert_eq!(dl.width, 190);
    assert_eq!(dl.height, 190);
    assert!(!dl.top_to_bottom, "opposite-sign dx/dy draws a / diagonal");
}

#[test]
fn parse_diagonal_requires_five_params() {
    let parser = EplParser::new();
    assert!(parser.parse(b"N\nLS10,10,20,200\nP1\n").is_err());
}

// ─── P1: X (box draw) ───

#[test]
fn parse_box_draw() {
    // X,x1,y1,thickness,x2,y2 (manual example X50,200,5,400,204)
    let labels = parse("N\nX50,200,5,400,300\nP1\n");
    let gb = match &labels[0].elements[0] {
        LabelElement::GraphicBox(b) => b,
        other => panic!("expected GraphicBox, got {:?}", other),
    };
    assert_eq!(gb.position.x, 50);
    assert_eq!(gb.position.y, 200);
    assert_eq!(gb.width, 350);
    assert_eq!(gb.height, 100);
    assert_eq!(gb.border_thickness, 5);
}

#[test]
fn parse_box_draw_reversed_corners() {
    let labels = parse("N\nX400,300,8,50,200\nP1\n");
    let gb = match &labels[0].elements[0] {
        LabelElement::GraphicBox(b) => b,
        other => panic!("expected GraphicBox, got {:?}", other),
    };
    assert_eq!(gb.position.x, 50);
    assert_eq!(gb.position.y, 200);
    assert_eq!(gb.width, 350);
    assert_eq!(gb.height, 100);
    assert_eq!(gb.border_thickness, 8);
}

#[test]
fn parse_box_requires_five_params() {
    let parser = EplParser::new();
    assert!(parser.parse(b"N\nX50,200,5,400\nP1\n").is_err());
}

// ─── P1: GW (direct graphic write, raw binary data) ───

#[test]
fn parse_graphic_write_binary_data() {
    // GW,x,y,width_bytes,lines,<raw binary>. The data must be consumed from
    // the byte stream: it contains newline bytes and is followed by more
    // commands on the same "line".
    let mut epl = b"N\nGW10,20,2,3,".to_vec();
    epl.extend_from_slice(&[0xF0, 0x0A, 0xFF, 0x0D, 0xAA, 0x55]);
    epl.extend_from_slice(b"P1\n");

    let parser = EplParser::new();
    let labels = parser.parse(&epl).expect("parse failed");
    assert_eq!(labels.len(), 1);
    let gf = match &labels[0].elements[0] {
        LabelElement::GraphicField(gf) => gf,
        other => panic!("expected GraphicField, got {:?}", other),
    };
    assert_eq!(gf.position.x, 10);
    assert_eq!(gf.position.y, 20);
    assert_eq!(gf.row_bytes, 2);
    assert_eq!(gf.total_bytes, 6);
    assert_eq!(gf.data, vec![0xF0, 0x0A, 0xFF, 0x0D, 0xAA, 0x55]);
}

#[test]
fn parse_graphic_write_truncated_data_errors() {
    let mut epl = b"N\nGW10,20,2,3,".to_vec();
    epl.extend_from_slice(&[0xF0, 0x0A]); // 2 of 6 required bytes
    let parser = EplParser::new();
    assert!(parser.parse(&epl).is_err(), "truncated GW data must fail");
}

#[test]
fn parse_graphic_write_requires_four_params() {
    let parser = EplParser::new();
    assert!(parser.parse(b"N\nGW10,20,2\nP1\n").is_err());
}

// ─── P1: b (2D bar codes) ───

#[test]
fn parse_2d_aztec() {
    let labels = parse("N\nb10,20,A,d4,e208,\"DATA\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::BarcodeAztec(b) => b,
        other => panic!("expected BarcodeAztec, got {:?}", other),
    };
    assert_eq!(bc.position.x, 10);
    assert_eq!(bc.position.y, 20);
    assert_eq!(bc.barcode.magnification, 4);
    assert_eq!(bc.barcode.size, 208);
    assert_eq!(bc.data, "DATA");
}

#[test]
fn parse_2d_aztec_defaults() {
    // d defaults to 3, e omitted -> size 0 (encoder default)
    let labels = parse("N\nb10,20,A,\"DATA\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeAztec(b) => {
            assert_eq!(b.barcode.magnification, 3);
            assert_eq!(b.barcode.size, 0);
        }
        other => panic!("expected BarcodeAztec, got {:?}", other),
    }
}

#[test]
fn parse_2d_datamatrix() {
    let labels = parse("N\nb10,20,D,c10,r8,h6,\"DATA\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::BarcodeDatamatrix(b) => b,
        other => panic!("expected BarcodeDatamatrix, got {:?}", other),
    };
    assert_eq!(bc.barcode.columns, 10);
    assert_eq!(bc.barcode.rows, 8);
    assert_eq!(bc.barcode.height, 6, "h is the module size in dots");
}

#[test]
fn parse_2d_maxicode_explicit_mode() {
    let labels = parse("N\nb10,20,M,m4,\"DATA\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::Maxicode(m) => assert_eq!(m.code.mode, 4),
        other => panic!("expected Maxicode, got {:?}", other),
    }
}

#[test]
fn parse_2d_maxicode_auto_modes() {
    let parser = EplParser::new();
    // All-numeric postal code -> Mode 2
    let numeric = format!("N\nb10,20,M,\"00184093065[)>\u{1e}01\u{1d}96Zc\"\nP1\n");
    match &parser.parse(numeric.as_bytes()).expect("ok")[0].elements[0] {
        LabelElement::Maxicode(m) => assert_eq!(m.code.mode, 2),
        other => panic!("expected Maxicode, got {:?}", other),
    }
    // Alphanumeric postal code -> Mode 3
    let alpha = format!("N\nb10,20,M,\"001840K1B2C[)>\u{1e}01\u{1d}96Zc\"\nP1\n");
    match &parser.parse(alpha.as_bytes()).expect("ok")[0].elements[0] {
        LabelElement::Maxicode(m) => assert_eq!(m.code.mode, 3),
        other => panic!("expected Maxicode, got {:?}", other),
    }
    // No AIM header -> standard symbol, Mode 4
    let plain = parse("N\nb10,20,M,\"DATA\"\nP1\n");
    match &plain[0].elements[0] {
        LabelElement::Maxicode(m) => assert_eq!(m.code.mode, 4),
        other => panic!("expected Maxicode, got {:?}", other),
    }
}

#[test]
fn parse_2d_pdf417() {
    let labels = parse("N\nb10,20,P,s5,x4,y20,r10,l6,t1,o1,\"DATA\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::BarcodePdf417(b) => b,
        other => panic!("expected BarcodePdf417, got {:?}", other),
    };
    assert_eq!(bc.barcode.security, 5);
    assert_eq!(bc.barcode.module_width, 4);
    assert_eq!(bc.barcode.row_height, 20, "y is the per-row bar height");
    assert_eq!(bc.barcode.rows, 10);
    assert_eq!(bc.barcode.columns, 6);
    assert!(bc.barcode.truncate);
    assert_eq!(bc.barcode.orientation, FieldOrientation::Rotated90);
}

#[test]
fn parse_2d_pdf417_row_height_defaults_to_module_times_four() {
    let labels = parse("N\nb10,20,P,x4,\"DATA\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodePdf417(b) => {
            assert_eq!(b.barcode.module_width, 4);
            assert_eq!(b.barcode.row_height, 16, "y defaults to 4 x module width");
        }
        other => panic!("expected BarcodePdf417, got {:?}", other),
    }
}

#[test]
fn parse_2d_qr() {
    let labels = parse("N\nb10,20,Q,s4,eH,\"HELLO\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::BarcodeQr(b) => b,
        other => panic!("expected BarcodeQr, got {:?}", other),
    };
    assert_eq!(bc.barcode.magnification, 4);
    // Data is synthesized into the ZPL-style payload: <ecc>M,B<len><content>.
    assert_eq!(bc.data, "HM,B0005HELLO");
    // And it must round-trip through the element's own parser.
    let (content, level, mode) = bc.get_input_data().expect("qr data parses");
    assert_eq!(content, "HELLO");
    assert_eq!(
        level,
        labelize::elements::barcode_qr::QrErrorCorrectionLevel::H
    );
    assert_eq!(
        mode,
        labelize::elements::barcode_qr::QrCharacterMode::Binary
    );
}

#[test]
fn parse_2d_qr_params_without_commas() {
    // Commas between optional params are optional per the manual.
    let labels = parse("N\nb10,20,Q,m2s3eMiA,\"X\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::BarcodeQr(b) => {
            assert_eq!(b.barcode.magnification, 3);
            assert_eq!(b.data, "MM,B0001X");
        }
        other => panic!("expected BarcodeQr, got {:?}", other),
    }
}

#[test]
fn parse_2d_unsupported_type_errors() {
    let parser = EplParser::new();
    for code in ["Z", "p", "P417", "aztec", ""] {
        let epl = format!("N\nb10,20,{},\"DATA\"\nP1\n", code);
        assert!(
            parser.parse(epl.as_bytes()).is_err(),
            "2D type {:?} must be rejected",
            code
        );
    }
}
