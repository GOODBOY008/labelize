use labelize::EplParser;
use labelize::elements::label_element::LabelElement;

fn parse(epl: &str) -> Vec<labelize::LabelInfo> {
    let parser = EplParser::new();
    parser.parse(epl.as_bytes()).expect("EPL parse failed")
}

// --- Text command ---

#[test]
fn text_command_produces_text_element() {
    let labels = parse("A50,50,0,1,1,1,N,\"Hello EPL\"\n");
    assert!(!labels.is_empty());
    let has_text = labels[0]
        .elements
        .iter()
        .any(|e| matches!(e, LabelElement::Text(_)));
    assert!(has_text, "expected Text element from EPL A command");
}

// --- Barcode command ---

#[test]
fn barcode_command_produces_barcode() {
    let labels = parse("B50,50,0,1,2,2,100,N,\"12345\"\n");
    assert!(!labels.is_empty());
    let has_barcode = labels[0].elements.iter().any(|e| {
        matches!(
            e,
            LabelElement::Barcode128(_)
                | LabelElement::Barcode39(_)
                | LabelElement::Barcode2of5(_)
                | LabelElement::BarcodeEan13(_)
        )
    });
    assert!(has_barcode, "expected barcode element from EPL B command");
}

// --- Line draw command ---

#[test]
fn lo_command_produces_graphic_box() {
    let labels = parse("LO10,20,200,100\n");
    assert!(!labels.is_empty());
    let has_box = labels[0]
        .elements
        .iter()
        .any(|e| matches!(e, LabelElement::GraphicBox(_)));
    assert!(has_box, "expected GraphicBox element from EPL LO command");
}

// --- Multiple commands ---

#[test]
fn multiple_epl_commands() {
    let labels = parse("A50,50,0,1,1,1,N,\"Hello\"\nA50,100,0,1,1,1,N,\"World\"\n");
    assert!(!labels.is_empty());
    let text_count = labels[0]
        .elements
        .iter()
        .filter(|e| matches!(e, LabelElement::Text(_)))
        .count();
    assert!(text_count >= 2, "expected at least 2 text elements");
}

// --- Parser robustness ---

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
    // Should either return Ok with empty labels or Ok with best-effort parse
    assert!(result.is_ok());
}
