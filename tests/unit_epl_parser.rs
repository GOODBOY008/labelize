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
    let labels = parse("N\nB50,100,0,0,2,5,100,N,\"ABC123\"\nP1\n");
    let bc = match &labels[0].elements[0] {
        LabelElement::Barcode39(b) => b,
        other => panic!("expected Barcode39, got {:?}", other),
    };
    assert_eq!(bc.data, "ABC123");
    assert_eq!(bc.width, 2);
    assert_eq!(bc.width_ratio, 2.5);
}

#[test]
fn parse_barcode_interleaved_2of5() {
    let labels = parse("N\nB50,100,0,G,2,5,100,B,\"1234567890\"\nP1\n");
    match &labels[0].elements[0] {
        LabelElement::Barcode2of5(_) => {}
        other => panic!("expected Barcode2of5, got {:?}", other),
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
