use labelize::elements::label_element::LabelElement;
use labelize::{DrawerOptions, EplParser, LabelInfo, Renderer, ZplParser};
use std::io::Cursor;

fn parse(parameters: &str) -> LabelInfo {
    let source = format!("^XA^FO20,20^BX{parameters}^FDAB12^FS^XZ");
    ZplParser::new().parse(source.as_bytes()).unwrap().remove(0)
}

fn quality(label: &LabelInfo) -> i32 {
    match &label.elements[0] {
        LabelElement::BarcodeDatamatrix(bc) => bc.barcode.quality,
        other => panic!("expected DataMatrix, got {other:?}"),
    }
}

fn render(label: &LabelInfo) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(label, &mut output, DrawerOptions::default())?;
    Ok(output.into_inner())
}

#[test]
fn omitted_and_empty_quality_remain_zero_and_are_not_rendered_as_ecc200() {
    for parameters in ["N,4", "N,4,", "N,4,,0,0,6"] {
        let label = parse(parameters);
        assert_eq!(quality(&label), 0);
        assert_eq!(render(&label).unwrap(), render(&parse("N,4,0")).unwrap());
        assert_ne!(render(&label).unwrap(), render(&parse("N,4,200")).unwrap());
    }
}

#[test]
fn convolutional_legacy_qualities_are_preserved_and_rendered_without_ecc200_fallback() {
    for value in ["50", "050", "80", "080", "100", "140"] {
        let label = parse(&format!("N,4,{value}"));
        let expected = value.parse::<i32>().unwrap();
        assert_eq!(quality(&label), expected);
        let image = render(&label).unwrap();
        assert_ne!(image, render(&parse("N,4,200")).unwrap());
        assert_ne!(image, render(&parse("N,4,0")).unwrap());
        assert_eq!(image, render(&parse(&format!("N,4,{expected}"))).unwrap());
    }
}

#[test]
fn invalid_numeric_quality_is_not_rendered_as_ecc200() {
    for value in [-1, 42, 201] {
        let error = render(&parse(&format!("N,4,{value}")))
            .err()
            .expect("invalid quality must fail");
        assert!(
            error.contains(&format!("Invalid DataMatrix quality {value}")),
            "{error}"
        );
    }
}

#[test]
fn quality_defaults_do_not_inherit_ecc200_from_previous_fields_or_labels() {
    let source = b"^XA^FO20,20^BXN,4,200^FDAB12^FS^FO80,20^BXN,4^FDAB12^FS^XZ\
                   ^XA^FO20,20^BXN,4,^FDAB12^FS^XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(quality(&labels[0]), 200);
    match &labels[0].elements[1] {
        LabelElement::BarcodeDatamatrix(bc) => assert_eq!(bc.barcode.quality, 0),
        other => panic!("expected DataMatrix, got {other:?}"),
    }
    for label in &labels {
        assert!(render(label).is_ok());
    }
}

#[test]
fn epl_explicitly_uses_ecc200_and_matches_zpl_rendering() {
    let epl = EplParser::new()
        .parse(b"N\nb20,20,D,h4,\"AB12\"\nP1\n")
        .unwrap();
    let zpl = parse("N,4,200");
    assert_eq!(quality(&epl[0]), 200);
    let png = render(&zpl).expect("explicit ECC 200 must still render");
    let image = image::load_from_memory(&png).unwrap().to_rgba8();
    assert!(image.pixels().any(|pixel| pixel[0] == 0 && pixel[3] != 0));
    assert_eq!(render(&epl[0]).unwrap(), png);
}
