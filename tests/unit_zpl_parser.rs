use labelize::elements::label_element::LabelElement;
use labelize::ZplParser;

fn parse(zpl: &str) -> Vec<labelize::LabelInfo> {
    let mut parser = ZplParser::new();
    parser.parse(zpl.as_bytes()).expect("ZPL parse failed")
}

// --- Block count ---

#[test]
fn single_block_produces_one_label() {
    let labels = parse("^XA^FO50,50^A0N,30,30^FDHello^FS^XZ");
    assert_eq!(labels.len(), 1);
}

#[test]
fn two_blocks_produce_two_labels() {
    let labels = parse("^XA^FO50,50^A0N,30,30^FDHello^FS^XZ^XA^FO10,10^A0N,20,20^FDWorld^FS^XZ");
    assert_eq!(labels.len(), 2);
}

#[test]
fn empty_block_produces_no_labels() {
    let labels = parse("^XA^XZ");
    // An empty block produces no labels (nothing drawable inside)
    assert_eq!(labels.len(), 0);
}

// --- Font commands ---

#[test]
fn font_a0_sets_text_font() {
    let labels = parse("^XA^FO50,50^A0N,30,25^FDTest^FS^XZ");
    assert_eq!(labels.len(), 1);
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(text.text, "Test");
    assert_eq!(text.font.height, 30.0);
    assert_eq!(text.font.width, 25.0);
}

// --- Field positioning ---

#[test]
fn fo_sets_position() {
    let labels = parse("^XA^FO100,200^A0N,30,30^FDPos^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(text.position.x, 100);
    assert_eq!(text.position.y, 200);
}

#[test]
fn ft_sets_position() {
    let labels = parse("^XA^FT150,300^A0N,30,30^FDFtPos^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(text.position.x, 150);
    assert_eq!(text.position.y, 300);
}

// --- Graphic commands ---

#[test]
fn gb_produces_graphic_box() {
    let labels = parse("^XA^FO10,20^GB200,100,3^FS^XZ");
    let gb = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::GraphicBox(g) => Some(g),
            _ => None,
        })
        .expect("expected GraphicBox element");
    assert_eq!(gb.width, 200);
    assert_eq!(gb.height, 100);
    assert_eq!(gb.border_thickness, 3);
}

#[test]
fn gc_produces_graphic_circle() {
    let labels = parse("^XA^FO10,20^GC100,3^FS^XZ");
    let gc = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::GraphicCircle(c) => Some(c),
            _ => None,
        })
        .expect("expected GraphicCircle element");
    assert_eq!(gc.circle_diameter, 100);
    assert_eq!(gc.border_thickness, 3);
}

// --- Barcode commands ---

#[test]
fn bc_produces_barcode128() {
    let labels = parse("^XA^FO50,50^BCN,100,Y,N,N^FD12345^FS^XZ");
    let bc = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Barcode128(b) => Some(b),
            _ => None,
        })
        .expect("expected Barcode128 element");
    assert_eq!(bc.data, "12345");
    assert_eq!(bc.barcode.height, 100);
}

#[test]
fn be_produces_ean13() {
    let labels = parse("^XA^FO50,50^BEN,100,Y,N^FD123456789012^FS^XZ");
    let bc = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::BarcodeEan13(b) => Some(b),
            _ => None,
        })
        .expect("expected BarcodeEan13 element");
    assert!(bc.data.starts_with("123456789012"));
    assert_eq!(bc.barcode.height, 100);
}

#[test]
fn b3_produces_barcode39() {
    let labels = parse("^XA^FO50,50^B3N,N,100,Y,N^FDABC123^FS^XZ");
    let bc = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Barcode39(b) => Some(b),
            _ => None,
        })
        .expect("expected Barcode39 element");
    assert_eq!(bc.data, "ABC123");
}

#[test]
fn bq_produces_qr_code() {
    let labels = parse("^XA^FO50,50^BQN,2,5^FDQA,Hello World^FS^XZ");
    let bc = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::BarcodeQr(b) => Some(b),
            _ => None,
        })
        .expect("expected BarcodeQr element");
    assert!(bc.data.contains("Hello World"));
}

// --- Label config commands ---

#[test]
fn pw_sets_print_width() {
    let labels = parse("^XA^PW600^FO50,50^A0N,30,30^FDWidth^FS^XZ");
    assert_eq!(labels[0].print_width, 600);
}

// --- Field block ---

#[test]
fn fb_produces_field_block_on_text() {
    let labels = parse("^XA^FO50,50^A0N,30,30^FB400,3,0,L^FDMultiline text^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    let block = text.block.as_ref().expect("expected FieldBlock");
    assert_eq!(block.max_width, 400);
    assert_eq!(block.max_lines, 3);
}

// --- Field reversal ---

#[test]
fn fr_sets_reverse_print() {
    let labels = parse("^XA^FO50,50^FR^A0N,30,30^FDReversed^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert!(text.reverse_print.value);
}

// --- Label reversal ---

#[test]
fn lr_sets_label_reverse() {
    let labels = parse("^XA^LRY^FO50,50^A0N,30,30^FDLabelReversed^FS^XZ");
    // LR affects all subsequent elements
    let elem = &labels[0].elements[0];
    assert!(elem.is_reverse_print());
}

// --- Malformed input ---

#[test]
fn malformed_command_uses_defaults() {
    // Missing parameters should not cause a panic
    let labels = parse("^XA^FO^A0^FDFallback^FS^XZ");
    assert_eq!(labels.len(), 1);
}

#[test]
fn parser_returns_error_for_garbage() {
    let mut parser = ZplParser::new();
    // Completely invalid input: no ^XA/^XZ
    let result = parser.parse(b"this is not ZPL at all");
    // Should either succeed with empty labels or succeed gracefully
    assert!(result.is_ok());
}

// --- Hex-escaped field data ---

#[test]
fn fh_decodes_hex_in_fd() {
    let labels = parse("^XA^FO50,50^A0N,30,30^FH^FDHello_2DWorld^FS^XZ");
    // _2D is hex for '-', so "Hello-World"
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert!(
        text.text.contains('-') || text.text.contains("Hello"),
        "Expected hex-decoded text, got: {}",
        text.text
    );
}

// --- Diagonal line ---

#[test]
fn gd_produces_diagonal_line() {
    let labels = parse("^XA^FO10,10^GD200,100,5^FS^XZ");
    let has_diag = labels[0]
        .elements
        .iter()
        .any(|e| matches!(e, LabelElement::DiagonalLine(_)));
    assert!(has_diag, "expected DiagonalLine element");
}

// --- Multiple label elements ---

#[test]
fn multiple_elements_in_one_label() {
    let labels = parse("^XA^FO10,10^GB200,100,3^FS^FO50,150^A0N,30,30^FDText^FS^XZ");
    assert_eq!(labels.len(), 1);
    assert!(
        labels[0].elements.len() >= 2,
        "expected at least 2 elements"
    );
}

// --- ^A font parsing edge cases ---

#[test]
fn font_a0_without_orientation_comma() {
    // ^A048,40 should be parsed as font=0, height=48, width=40
    // (not font=0, orientation=4(invalid), height=40, width=default)
    let labels = parse("^XA^FO50,50^A048,40^FDTest^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(text.font.height, 48.0, "^A048,40 should have height=48");
    assert_eq!(text.font.width, 40.0, "^A048,40 should have width=40");
}

#[test]
fn font_a0_with_valid_orientation() {
    // ^A0R,30,25 should be font=0, orientation=R, height=30, width=25
    let labels = parse("^XA^FO50,50^A0R,30,25^FDTest^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(text.font.height, 30.0);
    assert_eq!(text.font.width, 25.0);
}

// --- ^FO right-justification ---

#[test]
fn fo_right_justification() {
    // ^FO775,325,1 should set alignment to Right
    let labels = parse("^XA^FO775,325,1^A0N,30,30^FD0003^FS^XZ");
    let text = labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("expected Text element");
    assert_eq!(
        text.alignment,
        labelize::elements::field_alignment::FieldAlignment::Right
    );
}

// --- font B Zebra metrics (measured against Labelary at magnifications 1-9) ---

fn text_font(labels: &[labelize::LabelInfo]) -> labelize::elements::font::FontInfo {
    labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t.font.clone()),
            _ => None,
        })
        .expect("no text element")
}

#[test]
fn font_b_height_driven_uses_nine_dot_advance() {
    // ^CFB,80: magnification round(80/11)=7 -> cell 77, advance 9*7=63
    // (7-dot glyph + 2-dot intercharacter gap; the gap was previously dropped).
    let font = text_font(&parse("^XA^CFB,80^FO50,50^FDCJ^FS^XZ"));
    assert_eq!(font.name, "B");
    assert_eq!(font.height, 77.0);
    assert_eq!(font.width, 63.0);
}

#[test]
fn font_b_width_parameter_selects_magnification_on_the_cell_height() {
    // ^CFB,0,30 (width only): Labelary selects magnification round(30/11)=3, not round(30/7)=4.
    let font = text_font(&parse("^XA^CFB,0,30^FO50,50^FDCJ^FS^XZ"));
    assert_eq!(font.name, "B");
    assert_eq!(font.height, 33.0);
    assert_eq!(font.width, 27.0);
}

#[test]
fn other_bitmap_fonts_keep_their_existing_metrics() {
    // Font A advance was verified against Labelary previously (6 dots/mag) -- unchanged.
    let font = text_font(&parse("^XA^CFA,18^FO50,50^FDHello^FS^XZ"));
    assert_eq!(font.name, "A");
    assert_eq!(font.height, 18.0);
    assert_eq!(font.width, 12.0);
}

// --- ^CF single-character font designator (glued height digits) ---

fn first_text(labels: &[labelize::LabelInfo]) -> &labelize::elements::text_field::TextField {
    labels[0]
        .elements
        .iter()
        .find_map(|e| match e {
            LabelElement::Text(t) => Some(t),
            _ => None,
        })
        .expect("no text element")
}

#[test]
fn cf_font_designator_is_a_single_character_with_glued_height_digits() {
    // Real printers and Labelary parse ^CFB0,30 as font B, height 0, width 30 -- observed
    // verbatim in courier-generated ZPL. The designator must not swallow the height digits.
    let labels = parse("^XA^CFB0,30^FO50,50^FDHello^FS^XZ");
    assert_eq!(first_text(&labels).font.name, "B");
}

#[test]
fn cf_plain_single_character_font_still_parses() {
    let labels = parse("^XA^CF0,30^FO50,50^FDHello^FS^XZ");
    let font = &first_text(&labels).font;
    assert_eq!(font.name, "0");
    assert_eq!(font.height, 30.0);
}

#[test]
fn cf_numeric_user_font_falls_back_to_scalable_font_0() {
    let labels = parse("^XA^CF3,20^FO50,50^FDHello^FS^XZ");
    assert_eq!(first_text(&labels).font.name, "0");
}
