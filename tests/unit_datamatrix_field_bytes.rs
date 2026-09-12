use labelize::barcodes::datamatrix_legacy;
use labelize::elements::label_element::LabelElement;
use labelize::{DrawerOptions, LabelInfo, Renderer, ZplParser};
use std::io::Cursor;

fn barcode(
    label: &LabelInfo,
) -> &labelize::elements::barcode_datamatrix::BarcodeDatamatrixWithData {
    match &label.elements[0] {
        LabelElement::BarcodeDatamatrix(bc) => bc,
        other => panic!("expected DataMatrix, got {other:?}"),
    }
}

fn assert_rendered_bytes(label: &LabelInfo, expected: &[u8], quality: u16) {
    assert_eq!(barcode(label).data_bytes.as_deref(), Some(expected));
    let mut output = Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(label, &mut output, DrawerOptions::default())
        .unwrap();
    let actual = image::load_from_memory(output.get_ref())
        .unwrap()
        .to_rgba8();
    let matrix = datamatrix_legacy::encode_with_ecc(expected, 6, quality, None).unwrap();
    for y in 0..matrix.height() * 2 {
        for x in 0..matrix.width() * 2 {
            assert_eq!(
                actual.get_pixel(20 + x as u32, 20 + y as u32)[0] == 0,
                matrix.get(x / 2, y / 2)
            );
        }
    }
}

#[test]
fn manually_replaced_legacy_data_requires_explicit_non_ascii_bytes() {
    let mut label = ZplParser::new()
        .parse(b"^XA^FO20,20^BXN,2,0^FDOLD^FS^XZ")
        .unwrap()
        .remove(0);
    if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
        bc.data = "NEW".into();
        bc.data_bytes = None;
    }
    let render = |label: &LabelInfo| {
        let mut output = Cursor::new(Vec::new());
        Renderer::new().draw_label_as_png(label, &mut output, DrawerOptions::default())?;
        Ok::<_, String>(output.into_inner())
    };
    assert_eq!(
        render(&label).unwrap(),
        rendered(b"^XA^FO20,20^BXN,2,0^FDNEW^FS^XZ").unwrap()
    );
    if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
        bc.data = "\u{e4}".into();
    }
    assert!(render(&label).unwrap_err().contains("data_bytes"));
    if let LabelElement::BarcodeDatamatrix(bc) = &mut label.elements[0] {
        bc.data_bytes = Some(vec![0xe4]);
    }
    assert_eq!(
        render(&label).unwrap(),
        rendered(b"^XA^FO20,20^BXN,2,0^FH_^FD_E4^FS^XZ").unwrap()
    );
}

#[test]
fn all_fh_byte_values_survive_without_utf8_reinterpretation_or_resplitting() {
    let expected: Vec<u8> = (0..=255).collect();
    let field: String = expected.iter().map(|b| format!("_{b:02X}")).collect();
    let source = format!("^XA^FO20,20^BXN,2,0^FH_^FD{field}^FS^XZ");
    let labels = ZplParser::new().parse(source.as_bytes()).unwrap();
    assert_rendered_bytes(&labels[0], &expected, 0);
}

#[test]
fn identical_display_strings_do_not_collapse_different_field_bytes() {
    let source = b"^XA^FO20,20^BXN,2,0^FH_^FD_E4^FS^XZ^XA^FO20,20^BXN,2,0^FH_^FD_C3_A4^FS^XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_eq!(barcode(&labels[0]).data, barcode(&labels[1]).data);
    assert_rendered_bytes(&labels[0], &[0xe4], 0);
    assert_rendered_bytes(&labels[1], &[0xc3, 0xa4], 0);
}

#[test]
fn raw_invalid_utf8_is_preserved_for_every_legacy_quality() {
    for quality in [0, 50, 80, 100, 140] {
        let mut source = format!("^XA^FO20,20^BXN,2,{quality}^FD").into_bytes();
        source.extend_from_slice(&[0x80, 0xff, 0xc3, 0xa4]);
        source.extend_from_slice(b"^FS^XZ");
        let labels = ZplParser::new().parse(&source).unwrap();
        assert_rendered_bytes(&labels[0], &[0x80, 0xff, 0xc3, 0xa4], quality);
    }
}

#[test]
fn field_bytes_survive_stored_format_recall() {
    let source = b"^XA^DFR:LEG.ZPL^FO20,20^BXN,2,0^FN1^FS^XZ\
                   ^XA^XFR:LEG.ZPL^FN1^FH_^FD_80_FF^FS^XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_eq!(labels.len(), 1);
    assert_rendered_bytes(&labels[0], &[0x80, 0xff], 0);
}

#[test]
fn custom_command_prefix_and_fh_indicator_preserve_field_bytes() {
    let source = b"^XA^CC!!FO20,20!BXN,2,0!FH#!FD#80#5E#7E!FS!XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_rendered_bytes(&labels[0], &[0x80, b'^', b'~'], 0);
}

#[test]
fn field_bytes_and_fh_indicator_reset_between_labels_and_parser_calls() {
    let mut parser = ZplParser::new();
    let first = parser
        .parse(b"^XA^FO20,20^BXN,2,0^FH_^FD_FF^FS^XZ")
        .unwrap();
    assert_rendered_bytes(&first[0], &[0xff], 0);
    let second = parser.parse(b"^XA^FO20,20^BXN,2,0^FD_FF^FS^XZ").unwrap();
    assert_rendered_bytes(&second[0], b"_FF", 0);
}

#[test]
fn malformed_hex_is_literal_and_fv_keeps_bytes() {
    assert_eq!(
        labelize::hex::decode_escaped_bytes(b"_GG_4_80", b'_'),
        b"_GG_4\x80"
    );
    let labels = ZplParser::new()
        .parse(b"^XA^FO20,20^BXN,2,0^FH_^FV_80_FF^FS^XZ")
        .unwrap();
    assert_rendered_bytes(&labels[0], &[0x80, 0xff], 0);
}

fn rendered(source: &[u8]) -> Result<Vec<u8>, String> {
    let labels = ZplParser::new().parse(source)?;
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(&labels[0], &mut output, DrawerOptions::default())?;
    Ok(output.into_inner())
}

#[test]
fn printer_observed_legacy_fields_preserve_literal_bytes_in_ci13_and_ci27() {
    // L01-L04: measured ECC000/F6 on ZD421 V93.21.17Z, not PDF417 rules.
    for ci in [13, 27] {
        for (field, expected) in [
            (r"^FDA\&B", &b"A\\&B"[..]),
            ("^FH#^FDA#0D#0AB", &b"A\r\nB"[..]),
            ("^FH#^FDA#5C#26B", &b"A\\&B"[..]),
            (r"^FDA\\B", &b"A\\\\B"[..]),
            ("^FH#^FDA#5CB", &b"A\\B"[..]),
            ("^FDA||B", &b"A||B"[..]),
        ] {
            let source = format!("^XA^CI{ci}^FO20,20^BXN,2,0,0,0,6{field}^FS^XZ");
            let labels = ZplParser::new().parse(source.as_bytes()).unwrap();
            assert_rendered_bytes(&labels[0], expected, 0);
        }
    }
}

#[test]
fn legacy_literal_policy_is_consistent_across_qualities() {
    // Cross-quality implementation policy; hardware evidence is ECC000 only.
    for quality in [0, 50, 80, 100, 140] {
        let source = format!(r"^XA^FO20,20^BXN,2,{quality}^FD_1A\&\\||\q\^FS^XZ");
        let labels = ZplParser::new().parse(source.as_bytes()).unwrap();
        assert_rendered_bytes(&labels[0], br"_1A\&\\||\q\", quality);
    }
}

#[test]
fn ecc200_does_not_use_legacy_field_substitutions() {
    let escaped = rendered(b"^XA^FO20,20^BXN,2,200^FDAB\\&12^FS^XZ").unwrap();
    let crlf = rendered(b"^XA^FO20,20^BXN,2,200^FH_^FDAB_0D_0A12^FS^XZ").unwrap();
    assert_ne!(escaped, crlf);
}

#[test]
fn switching_legacy_and_ecc200_keeps_escape_paths_isolated() {
    let sources = [
        "^XA^FO20,20^BXN,2,200,0,0,6,#^FD#1A\\&B^FS^XZ",
        "^XA^FO20,20^BXN,2,0,0,0,6,#^FD#1A\\&B^FS^XZ",
        "^XA^FO20,20^BXN,2,200^FD_1A\\&B^FS^XZ",
        "^XA^FO20,20^BXN,2,50^FD_1A\\&B^FS^XZ",
    ];
    let labels = ZplParser::new().parse(sources.join("").as_bytes()).unwrap();
    assert_eq!(labels.len(), sources.len());
    for (label, source) in labels.iter().zip(sources) {
        let mut output = Cursor::new(Vec::new());
        Renderer::new()
            .draw_label_as_png(label, &mut output, DrawerOptions::default())
            .unwrap();
        assert_eq!(output.into_inner(), rendered(source.as_bytes()).unwrap());
    }
    // Legacy ignores g and preserves FNC-looking text, and literal backslashes.
    for (index, expected, quality) in [(1, &b"#1A\\&B"[..], 0), (3, &b"_1A\\&B"[..], 50)] {
        let mut prepared = labels[index].clone();
        if let LabelElement::BarcodeDatamatrix(bc) = &mut prepared.elements[0] {
            bc.data_bytes = Some(expected.to_vec());
        }
        assert_rendered_bytes(&prepared, expected, quality);
        assert_eq!(rendered(sources[index].as_bytes()).unwrap(), {
            let mut output = Cursor::new(Vec::new());
            Renderer::new()
                .draw_label_as_png(&prepared, &mut output, DrawerOptions::default())
                .unwrap();
            output.into_inner()
        });
    }
}
