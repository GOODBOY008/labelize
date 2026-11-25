use labelize::{Language, RenderOptions};

const LABEL: &[u8] = br#"N
q400
Q200,0
A20,20,0,3,1,1,N,"A\"B\\C"
B20,60,0,1,2,3,100,N,"A\"B\\C"
P1
"#;
const VALUE: &str = r#"A"B\C"#;

fn options(dpmm: i32) -> RenderOptions {
    RenderOptions {
        language: Some(Language::Epl),
        width_mm: Some(50.0),
        height_mm: Some(25.0),
        dpmm,
        ..Default::default()
    }
}

#[test]
fn code128_scans_to_the_literal_value() {
    for dpmm in [8, 12] {
        let png = labelize::render_png(LABEL, options(dpmm)).unwrap();
        let pixels = image::load_from_memory(&png.bytes).unwrap().to_luma8();
        let (width, height) = pixels.dimensions();
        let scanned = rxing::helpers::detect_in_luma(
            pixels.into_raw(),
            width,
            height,
            Some(rxing::BarcodeFormat::CODE_128),
        )
        .unwrap();
        assert_eq!(scanned.getText(), VALUE, "{dpmm} dpmm");
    }
}

#[cfg(feature = "vector")]
#[test]
fn searchable_pdf_contains_the_literal_text() {
    let output = labelize::OutputOptions {
        mode: labelize::OutputMode::Vector,
        ..Default::default()
    };
    for dpmm in [8, 12] {
        let rendered = labelize::render_pdf_with_output(LABEL, options(dpmm), output).unwrap();
        let pdf = lopdf::Document::load_mem(&rendered.bytes).unwrap();
        assert_eq!(
            pdf.extract_text(&[1]).unwrap().replace('\n', ""),
            VALUE,
            "{dpmm} dpmm"
        );
    }
}
