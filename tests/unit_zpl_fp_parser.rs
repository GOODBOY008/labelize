//! Field-scoped print direction and character-spacing semantics.
use labelize::elements::label_element::LabelElement;
use labelize::elements::text_field::TextField;
use labelize::elements::text_flow::{TextDirection, TextFlow};
use labelize::{LabelInfo, ZplParser};

fn parse(source: &str) -> Vec<LabelInfo> {
    ZplParser::new().parse(source.as_bytes()).unwrap()
}

fn text_fields(labels: &[LabelInfo]) -> Vec<&TextField> {
    labels
        .iter()
        .flat_map(|label| &label.elements)
        .filter_map(|element| match element {
            LabelElement::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn flow(parameter: &str) -> TextFlow {
    let labels = parse(&format!("^XA^FO100,60^A0N,40,32{parameter}^FDABCDE^FS^XZ"));
    text_fields(&labels)[0].flow
}

#[test]
fn field_parameter_defaults_preserve_existing_text_layout() {
    for parameter in ["", "^FP", "^FPH", "^FPH,0", "^FP,0"] {
        assert_eq!(flow(parameter), TextFlow::default(), "{parameter}");
    }
}

#[test]
fn horizontal_gap_is_a_signed_number_of_dots() {
    for gap in [-10, -5, 0, 5, 15, 30] {
        assert_eq!(
            flow(&format!("^FPH,{gap}")),
            TextFlow {
                direction: TextDirection::Horizontal,
                character_spacing: gap,
                wrap_spacing: gap as f64 * 10.0 / 32.0,
                ..Default::default()
            }
        );
    }
}

#[test]
fn reverse_direction_retains_the_original_logical_text() {
    let labels = parse("^XA^FO260,60^A0N,40,32^FPR,10^FDABCDE^FS^XZ");
    let field = text_fields(&labels)[0];
    assert_eq!(field.text, "ABCDE");
    assert_eq!(field.flow.direction, TextDirection::Reverse);
    assert_eq!(field.flow.character_spacing, 10);
}

#[test]
fn vertical_gap_is_retained_for_field_block_wrapping() {
    for gap in [-5, 0, 5, 10, 30] {
        assert_eq!(
            flow(&format!("^FPV,{gap}")),
            TextFlow {
                direction: TextDirection::Vertical,
                character_spacing: gap,
                origin_width: 24.0,
                wrap_spacing: gap as f64 * 10.0 / 32.0,
                ..Default::default()
            }
        );
    }
}

#[test]
fn field_separator_clears_direction_and_gap() {
    let labels = parse(
        "^XA^FO260,60^A0N,40,32^FPR,10^FDABCDE^FS\
         ^FO260,120^A0N,40,32^FDABCDE^FS^XZ",
    );
    let fields = text_fields(&labels);
    assert_eq!(fields[0].flow.direction, TextDirection::Reverse);
    assert_eq!(fields[1].flow, TextFlow::default());
}

#[test]
fn each_parameter_command_replaces_omitted_values_with_defaults() {
    assert_eq!(flow("^FPR,10^FP"), TextFlow::default());
    assert_eq!(
        flow("^FPH,15^FPR"),
        TextFlow {
            direction: TextDirection::Reverse,
            character_spacing: 0,
            origin_width: 24.0,
            ..Default::default()
        }
    );
    assert_eq!(
        flow("^FPR,10^FP,5"),
        TextFlow {
            direction: TextDirection::Horizontal,
            character_spacing: 5,
            wrap_spacing: 50.0 / 32.0,
            ..Default::default()
        }
    );
}

#[test]
fn direction_letters_are_case_insensitive() {
    for (lower, upper) in [
        ("^fph,5", "^FPH,5"),
        ("^fpv,30", "^FPV,30"),
        ("^fpr,10", "^FPR,10"),
    ] {
        assert_eq!(flow(lower), flow(upper));
    }
}

#[test]
fn invalid_values_fall_back_and_out_of_range_gaps_are_clamped() {
    assert_eq!(flow("^FPX,garbage"), TextFlow::default());
    assert_eq!(flow("^FPH,-99999").character_spacing, -10);
    let labels = parse("^XA^A0N,40,1000^FPH,99999^FDAB^FS^XZ");
    assert_eq!(text_fields(&labels)[0].flow.character_spacing, 9999);
}

#[test]
fn positive_gap_above_ten_font_widths_is_dropped() {
    assert_eq!(flow("^FPH,320").character_spacing, 320);
    assert_eq!(flow("^FPH,321").character_spacing, 0);
    assert_eq!(flow("^FPH,99999").character_spacing, 0);
}

#[test]
fn excessive_gap_is_resolved_after_the_final_font_command() {
    let labels = parse(
        "^XA^FPH,320^A0N,40,32^FDAB^FS\
         ^FPH,320^A0N,40,31^FDAB^FS\
         ^FPH,320^CF0,40,31^FDAB^FS^XZ",
    );
    let fields = text_fields(&labels);
    assert_eq!(fields[0].flow.character_spacing, 320);
    assert_eq!(fields[1].flow.character_spacing, 0);
    assert_eq!(fields[2].flow.character_spacing, 0);
}

#[test]
fn character_spacing_stays_in_dots_with_millimeter_units() {
    let labels = parse("^XA^MUM^FO10,10^A0N,5,4^FPH,5^FDABCDE^FS^XZ");
    let field = text_fields(&labels)[0];
    assert_eq!(field.font.width, 32.0);
    assert_eq!(field.font.height, 40.0);
    assert_eq!(field.flow.character_spacing, 5);
}

#[test]
fn stored_numbered_text_fields_retain_direction_and_gap() {
    let labels = parse(
        "^XA^DFR:FLOW.ZPL^FO260,60^A0N,40,32^FPR,10^FN1^FS^XZ\
         ^XA^XFR:FLOW.ZPL^FN1^FDABCDE^FS^XZ",
    );
    let fields = text_fields(&labels);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].text, "ABCDE");
    assert_eq!(fields[0].flow.direction, TextDirection::Reverse);
    assert_eq!(fields[0].flow.character_spacing, 10);
}

#[test]
fn field_parameter_on_a_barcode_does_not_leak_to_later_text() {
    let labels = parse(
        "^XA^FO20,20^FPR,10^BCN,50,N,N,N^FD12345^FS\
         ^FO20,100^A0N,40,32^FDABCDE^FS^XZ",
    );
    assert_eq!(text_fields(&labels)[0].flow, TextFlow::default());
}

#[test]
fn validation_and_formatting_preserve_field_flow() {
    let source = b"^XA^FO260,60^A0N,40,32^FPR,10^FDABCDE^FS^XZ";
    let report = labelize::validate(source, 8, None);
    assert!(report.ok);
    assert_eq!(report.language, labelize::Language::Zpl);
    assert_eq!(report.labels, 1);
    let (language, formatted) = labelize::format_source(source, None);
    assert_eq!(language, labelize::Language::Zpl);
    let labels = ZplParser::new().parse(&formatted).unwrap();
    assert_eq!(
        text_fields(&labels)[0].flow,
        TextFlow {
            direction: TextDirection::Reverse,
            character_spacing: 10,
            origin_width: 24.0,
            wrap_spacing: 100.0 / 32.0,
        }
    );
    assert_eq!(text_fields(&labels)[0].text, "ABCDE");
}

#[test]
fn stored_fields_resolve_excessive_gap_with_their_definition_font() {
    let labels = parse(
        "^XA^DFR:LIMIT.ZPL^FPH,320^A0N,40,31^FN1^FS^XZ\
         ^XA^CF0,40,1000^XFR:LIMIT.ZPL^FN1^FDAB^FS^XZ",
    );
    let field = text_fields(&labels)[0];
    assert_eq!(field.font.width, 31.0);
    assert_eq!(field.flow.character_spacing, 0);
}

#[test]
fn bitmap_gap_limit_uses_the_width_derived_original_font_cell() {
    for (font, height, width, limit) in [("A", 72, 20, 360), ("B", 44, 30, 440), ("C", 36, 20, 360)]
    {
        for gap in [limit, limit + 1] {
            let labels = parse(&format!(
                "^XA^A{font}N,{height},{width}^FPH,{gap}^FDAB^FS^XZ"
            ));
            assert_eq!(
                text_fields(&labels)[0].flow.character_spacing,
                if gap == limit { gap } else { 0 },
                "font {font}, gap {gap}"
            );
        }
    }
}

#[test]
fn bitmap_gap_limit_derives_an_omitted_width_from_height() {
    for (gap, expected) in [(720, 720), (721, 0)] {
        let labels = parse(&format!("^XA^AAN,72^FPH,{gap}^FDAB^FS^XZ"));
        assert_eq!(text_fields(&labels)[0].flow.character_spacing, expected);
    }
}

#[test]
fn resident_font_p_uses_its_independently_measured_gap_limit() {
    for (font_command, limit) in [("^APN,40,20", 162), ("^APN,40,36", 324), ("^APN,40", 162)] {
        for gap in [limit, limit + 1] {
            let labels = parse(&format!("^XA{font_command}^FPH,{gap}^FDAB^FS^XZ"));
            assert_eq!(
                text_fields(&labels)[0].flow.character_spacing,
                if gap == limit { gap } else { 0 },
                "{font_command}, gap {gap}"
            );
        }
    }
}
