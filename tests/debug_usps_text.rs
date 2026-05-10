mod common;
use labelize::elements::LabelElement;

#[test]
fn debug_usps_text_elements() {
    let zpl = std::fs::read_to_string("testdata/labels/usps.zpl").unwrap();
    let mut parser = labelize::ZplParser::new();
    let labels = parser.parse(zpl.as_bytes()).unwrap();
    println!("\n=== USPS FULL LABEL TEXT ELEMENTS ===");
    for label in &labels {
        for elem in &label.elements {
            if let LabelElement::Text(t) = elem {
                let scale_x = t.font.get_scale_x();
                let font_size = t.font.get_size();
                println!(
                    "  text={:?} h={:.0} w={:.0} pos=({},{}) scale_x={:.4} px_x={:.2}",
                    t.text,
                    t.font.height,
                    t.font.width,
                    t.position.x,
                    t.position.y,
                    scale_x,
                    font_size * scale_x
                );
            }
        }
    }

    let zpl2 = std::fs::read_to_string("testdata/unit/snippets/usps_test_merchant.zpl").unwrap();
    let mut parser2 = labelize::ZplParser::new();
    let labels2 = parser2.parse(zpl2.as_bytes()).unwrap();
    println!("\n=== SNIPPET TEXT ELEMENTS ===");
    for label in &labels2 {
        for elem in &label.elements {
            if let LabelElement::Text(t) = elem {
                let scale_x = t.font.get_scale_x();
                let font_size = t.font.get_size();
                println!(
                    "  text={:?} h={:.0} w={:.0} pos=({},{}) scale_x={:.4} px_x={:.2}",
                    t.text,
                    t.font.height,
                    t.font.width,
                    t.position.x,
                    t.position.y,
                    scale_x,
                    font_size * scale_x
                );
            }
        }
    }
}
