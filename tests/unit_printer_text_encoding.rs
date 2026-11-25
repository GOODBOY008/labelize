//! Byte fixtures are literal printer input, independent of the decoder's tables.
use labelize::{elements::label_element::LabelElement, format_source, Language};

fn text(element: &LabelElement) -> &str {
    let LabelElement::Text(field) = element else {
        panic!("expected TEXT, got {element:?}")
    };
    &field.text
}

#[test]
fn tspl_codepage_936_preserves_dbcs_trails_and_following_commands() {
    // 81 5C = 乗. Its second byte looks like a backslash before either
    // a bracketed quote escape or the closing delimiter.
    let lines: &[&[u8]] = &[
        b"SIZE 40 mm,30 mm",
        b"CLS",
        b"codepage 936",
        b"TEXT 10,10,\"TSS24.BF2\",0,1,1,\"\xb1\xa3\xce\xc2\xb1\xad\"",
        b"TEXT 10,20,\"3\",0,1,1,\"\x81\x5c\\[\"]\r\n\x81\x5c\"",
        b"PRINT 1,1",
        b"CLS",
        b"TEXT 10,10,\"3\",0,1,1,\"\x81\x40\"",
        b"CODEPAGE UTF-8",
        "TEXT 10,20,\"3\",0,1,1,\"温杯\"".as_bytes(),
        b"PRINT 1,1",
        b"",
    ];
    let job = lines.join(b"\r\n".as_slice());
    let (_, formatted) = format_source(&job, Some(Language::Tspl));
    assert_eq!(formatted, lines.join(b"\n".as_slice()));
    for bytes in [&job, &formatted] {
        let labels = Language::Tspl.parse(bytes, 8).unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(text(&labels[0].elements[0]), "保温杯");
        assert_eq!(text(&labels[0].elements[1]), "乗\"\r\n乗");
        assert_eq!(text(&labels[1].elements[0]), "丂");
        assert_eq!(text(&labels[1].elements[1]), "温杯");
    }
}

#[test]
fn tspl_codepage_applies_only_to_text() {
    let lines: &[&[u8]] = &[
        b"CODEPAGE 936",
        b"BITMAP 0,0,4,1,0,\xff\r\n\x00",
        b"AZTEC 0,0,0,4,0,0,0,1,0,4,\xff\r\n\x00",
        b"PDF417 0,0,300,100,0,L3,\xe6\xb8\xa9",
        b"TEXT 0,0,\"3\",0,1,1,\"\xce\xc2\"",
        b"PRINT 1,1",
        b"",
    ];
    let job = lines.join(b"\r\n".as_slice());
    let (_, formatted) = format_source(&job, Some(Language::Tspl));
    assert_eq!(formatted, lines.join(b"\n".as_slice()));
    for bytes in [&job, &formatted] {
        let labels = Language::Tspl.parse(bytes, 8).unwrap();
        let [LabelElement::GraphicField(bitmap), LabelElement::BarcodeSymbol(aztec), LabelElement::BarcodePdf417(pdf), field] =
            labels[0].elements.as_slice()
        else {
            panic!("unexpected elements")
        };
        assert_eq!(bitmap.data, b"\xff\r\n\x00");
        assert_eq!(aztec.data, b"\xff\r\n\x00");
        assert_eq!(pdf.data, "温");
        assert_eq!(text(field), "温");
    }
}

#[test]
fn cpcl_codepage_applies_to_text_and_multiline_rows_within_the_session() {
    let lines: &[&[u8]] = &[
        b"! 0 200 200 240 1",
        b"country gb18030",
        b"TEXT GBUNSG24.CPF 0 10 10 \xb1\xa3\xce\xc2\xb1\xad",
        b"ML 24 VSCALE-TEXT FONT.CPF 12 12 10 34",
        b"\x95\x32\x82\x36",
        b"  \xce\xc2 ",
        b"ENDML",
        b"ENCODING UTF-8",
        "T180 4 0 10 100 温杯".as_bytes(),
        b"ENCODING GB18030",
        b"ST FONT.CPF 10 10 10 135 \x95\x32\x82\x36",
        b"PRINT",
        b"! 0 200 200 240 1",
        "T 4 0 10 10 温杯".as_bytes(),
        b"PRINT",
        b"",
    ];
    let job = lines.join(b"\r\n".as_slice());
    let (_, formatted) = format_source(&job, Some(Language::Cpcl));
    assert_eq!(formatted, lines.join(b"\n".as_slice()));
    for bytes in [&job, &formatted] {
        let labels = Language::Cpcl.parse(bytes, 8).unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(
            labels[0].elements.iter().map(text).collect::<Vec<_>>(),
            ["保温杯", "𠀀", "  温 ", "温杯", "𠀀"]
        );
        assert_eq!(text(&labels[1].elements[0]), "温杯");
    }
}

#[test]
fn cpcl_codepage_preserves_graphic_and_datamatrix_bytes() {
    let lines: &[&[u8]] = &[
        b"! 0 200 200 240 1",
        b"ENCODING GB18030",
        b"CG 4 1 0 0 \xff\r\n\x00",
        b"BARCODE DATAMATRIX 10 10 H 3 S 200",
        b"\xff\x00",
        b"ENDDATAMATRIX",
        b"T90 0 0 0 0 \x95\x32\x82\x36",
        b"PRINT",
        b"",
    ];
    let job = lines.join(b"\r\n".as_slice());
    let (_, formatted) = format_source(&job, Some(Language::Cpcl));
    assert_eq!(formatted, lines.join(b"\n".as_slice()));
    for bytes in [&job, &formatted] {
        let labels = Language::Cpcl.parse(bytes, 8).unwrap();
        let [LabelElement::GraphicField(graphic), LabelElement::BarcodeDatamatrix(dm), field] =
            labels[0].elements.as_slice()
        else {
            panic!("unexpected elements")
        };
        assert_eq!(graphic.data, b"\xff\r\n\x00");
        assert_eq!(dm.data_bytes.as_deref(), Some(b"\xff\x00".as_slice()));
        assert_eq!(text(field), "𠀀");
    }
}

#[test]
fn other_codepages_retain_the_undeclared_text_behavior() {
    for (language, job) in [
        (
            Language::Tspl,
            b"CODEPAGE 936\nCODEPAGE 1252\nTEXT 0,0,\"3\",0,1,1,\"\xff\"\nPRINT 1".as_slice(),
        ),
        (
            Language::Cpcl,
            b"! 0 200 200 240 1\nCOUNTRY GB18030\nCOUNTRY USA\nT 0 0 0 0 \xff\nPRINT",
        ),
    ] {
        let labels = language.parse(job, 8).unwrap();
        assert_eq!(text(&labels[0].elements[0]), "\u{fffd}");
    }
}

#[test]
fn declared_encodings_reject_invalid_sequences_in_text_fields() {
    for (language, command, error) in [
        (
            Language::Tspl,
            b"CODEPAGE 936\nTEXT 0,0,\"3\",0,1,1,\"\x81\"\n".as_slice(),
            "GBK",
        ),
        (
            Language::Tspl,
            b"CODEPAGE 936\nTEXT 0,0,\"3\",0,1,1,\"\x95\x32\x82\x36\"\n",
            "GBK",
        ),
        (
            Language::Tspl,
            b"CODEPAGE UTF-8\nTEXT 0,0,\"3\",0,1,1,\"\xff\"\n",
            "UTF-8",
        ),
        (
            Language::Cpcl,
            b"! 0 200 200 240 1\nENCODING GB18030\nT 4 0 0 0 \x95\x32\x82\n",
            "GB18030",
        ),
        (
            Language::Cpcl,
            b"! 0 200 200 240 1\nCOUNTRY GB18030\nML 20\nT 4 0 0 0\n\xff\nENDML\n",
            "GB18030",
        ),
        (
            Language::Cpcl,
            b"! 0 200 200 240 1\nCOUNTRY UTF-8\nT 4 0 0 0 \xff\n",
            "UTF-8",
        ),
    ] {
        let err = language.parse(command, 8).unwrap_err().to_string();
        assert!(err.contains("invalid") && err.contains(error), "{err}");
    }
}
