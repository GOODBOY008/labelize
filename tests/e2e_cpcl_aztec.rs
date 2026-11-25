//! CPCL Aztec data is a printer byte string, including Unicode spacing and legacy text bytes.
use anyd::{codes::aztec::scan, GrayFrame};
use labelize::{format_source, render_png, Language, PrintTemplate, RenderOptions, TemplateValues};

fn scan_job(job: &[u8], expected: &[u8]) {
    let (_, formatted) = format_source(job, Some(Language::Cpcl));
    for data in [job, &formatted] {
        for dpmm in [8, 12] {
            let png = render_png(
                data,
                RenderOptions {
                    dpmm,
                    ..Default::default()
                },
            )
            .unwrap();
            let pixels = image::load_from_memory(&png.bytes).unwrap().into_luma8();
            let frame = GrayFrame::new(
                pixels.as_raw(),
                pixels.width() as usize,
                pixels.height() as usize,
            )
            .unwrap();
            assert_eq!(scan(&frame).unwrap().payload_bytes(), expected);
        }
    }
}

#[test]
fn filled_unicode_spaces_next_to_end_tag_are_printed_as_data() {
    let header = b"! 0 200 200 240 1\r\nPAGE-WIDTH 240\r\nB AZTEC 20 20 XD 6\r\n";
    let directory = format!(
        "LABELIZE-TPL/1 cpcl UTF-8\n{} 9 1 aztec aztec-line -\n\n",
        header.len()
    );
    let template = PrintTemplate::load(
        Language::Cpcl,
        &[
            directory.as_bytes(),
            header,
            b"{{value}}\r\nENDAZTEC\r\nPRINT\r\n",
        ]
        .concat(),
    )
    .unwrap();
    for value in [
        "\u{a0}ENDAZTEC",
        "endaztec\u{2007}",
        "\u{202f}EndAztec",
        "保温杯",
    ] {
        let record = TemplateValues::from([("value".to_owned(), value.to_owned())]);
        let job = template.fill(&record).unwrap();
        scan_job(&job, value.as_bytes());
    }
}

#[test]
fn legacy_font_codepage_leaves_native_aztec_payload_unchanged() {
    // C0 F1 CE EF is "礼物" in GB18030. Encoding selects a font code page;
    // the symbol still carries the exact bytes in the printer file.
    let payload = b"\xc0\xf1\xce\xef";
    let job = [
        b"! 0 200 200 240 1\r\nPAGE-WIDTH 240\r\nENCODING GB18030\r\nB AZTEC 20 20 XD 6\r\n"
            .as_slice(),
        payload,
        b"\r\n \tendaztec\t\r\nPRINT\r\n",
    ]
    .concat();
    scan_job(&job, payload);
}
