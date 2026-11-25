use labelize::{format_source, render_png, Language, RenderOptions};
use rxing::{helpers::detect_in_luma, BarcodeFormat};

fn jobs(field: &str) -> [Vec<u8>; 2] {
    let lines = [
        "SIZE 60 mm,40 mm".to_owned(),
        "DIRECTION 1".to_owned(),
        "CLS".to_owned(),
        format!(r#"TEXT 10,10,"3",0,1,1,"{field}""#),
        format!(r#"QRCODE 70,100,M,4,A,0,M2,S7,"{field}""#),
        r#"TEXT 10,55,"3",0,1,1,"NEXT""#.to_owned(),
        "PRINT 1,1".to_owned(),
        String::new(),
    ];
    let source = lines.join("\r\n").into_bytes();
    let (_, formatted) = format_source(&source, Some(Language::Tspl));
    assert_eq!(formatted, lines.join("\n").as_bytes());
    [source, formatted]
}

const CASES: &[(&str, &str)] = &[(r#"A\\["]B"#, "A\\\"B"), (r"A\", "A\\")];

#[test]
fn qr_scans_to_the_literal_bytes() {
    for &(field, expected) in CASES {
        for job in jobs(field) {
            for dpmm in [8, 12] {
                let options = RenderOptions {
                    language: Some(Language::Tspl),
                    dpmm,
                    ..Default::default()
                };
                let png = render_png(&job, options).unwrap();
                let pixels = image::load_from_memory(&png.bytes).unwrap().into_luma8();
                let (width, height) = pixels.dimensions();
                let qr = detect_in_luma(
                    pixels.into_raw(),
                    width,
                    height,
                    Some(BarcodeFormat::QR_CODE),
                )
                .unwrap();
                assert_eq!(qr.getText().as_bytes(), expected.as_bytes(), "{dpmm} dpmm");
            }
        }
    }
}

#[cfg(feature = "vector")]
#[test]
fn vector_pdf_preserves_literal_and_following_fields() {
    let output = labelize::OutputOptions {
        mode: labelize::OutputMode::Vector,
        ..Default::default()
    };
    for &(field, expected) in CASES {
        for job in jobs(field) {
            for dpmm in [8, 12] {
                let options = RenderOptions {
                    language: Some(Language::Tspl),
                    dpmm,
                    ..Default::default()
                };
                let rendered = labelize::render_pdf_with_output(&job, options, output).unwrap();
                let pdf = lopdf::Document::load_mem(&rendered.bytes).unwrap();
                assert_eq!(
                    pdf.extract_text(&[1]).unwrap().replace('\n', ""),
                    format!("{expected}NEXT"),
                    "{dpmm} dpmm"
                );
            }
        }
    }
}
