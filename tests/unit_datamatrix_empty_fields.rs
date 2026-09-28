use labelize::barcodes::{datamatrix, datamatrix_legacy};
use labelize::elements::label_element::LabelElement;
use labelize::{DrawerOptions, LabelInfo, Renderer, ZplParser};
use std::io::Cursor;

fn parse(source: &str) -> LabelInfo {
    ZplParser::new().parse(source.as_bytes()).unwrap().remove(0)
}

fn render(label: &LabelInfo) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(
        label,
        &mut output,
        DrawerOptions {
            label_width_mm: 50.0,
            label_height_mm: 25.0,
            dpmm: 8,
            ..Default::default()
        },
    )?;
    Ok(output.into_inner())
}

#[test]
fn empty_fields_leave_other_label_content_unchanged_for_all_qualities() {
    let tail = "^FO180,20^BXN,2,200^FDAB12^FS^FO4,4^GB3,3,3^FS^XZ";
    let expected = render(&parse(&format!("^XA{tail}"))).unwrap();
    for parameters in [
        "N,2", "N,2,", "N,2,0", "N,2,50", "N,2,80", "N,2,100", "N,2,140", "N,2,200",
    ] {
        for field in ["^FD", "^FV"] {
            let source = format!("^XA^FO20,20^BX{parameters}{field}^FS{tail}");
            assert_eq!(
                render(&parse(&source)).unwrap(),
                expected,
                "{parameters} {field}"
            );
        }
    }
}

#[test]
fn legacy_empty_check_uses_preserved_bytes_instead_of_display_text() {
    let empty = render(&parse("^XA^FO4,4^GB3,3,3^FS^XZ")).unwrap();
    for quality in [0, 50, 80, 100, 140] {
        let mut label = parse(&format!(
            "^XA^FO20,20^BXN,2,{quality}^FDAB^FS^FO4,4^GB3,3,3^FS^XZ"
        ));
        let expected = render(&label).unwrap();
        if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
            bc.data.clear(); // Preserved AB must still render.
        }
        assert_eq!(render(&label).unwrap(), expected);
        if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
            bc.data = "stale display text".into();
            bc.data_bytes = Some(Vec::new());
        }
        assert_eq!(render(&label).unwrap(), empty);
        if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
            bc.data.clear();
            bc.data_bytes = None;
        }
        assert_eq!(render(&label).unwrap(), empty);
    }
}

#[test]
fn raw_ecc200_empty_symbol_and_legacy_validation_remain_separate() {
    for size in [0, 12] {
        let raw = datamatrix::encode("", 2, size, size).unwrap();
        assert_eq!(raw.width(), if size == 0 { 20 } else { 24 });
        assert_eq!(
            datamatrix::encode_zpl(b"", 2, size, size, b'_').unwrap(),
            raw
        );
    }
    assert!(datamatrix_legacy::encode(b"", 6, None).is_err());
    assert!(render(&parse("^XA^BXN,2,42^FD^FS^XZ"))
        .unwrap_err()
        .contains("Invalid DataMatrix quality 42"));
}
