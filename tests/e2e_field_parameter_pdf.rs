//! Public-API exports for the independent native-text/viewer regressions.
//! Python checks structure, search and geometry in e2e/vector/test_field_parameters.py.
#![cfg(feature = "vector")]

use labelize::{Language, OutputMode, OutputOptions, RenderOptions};
use std::path::Path;

#[test]
fn export_field_parameter_native_pdf_probes() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("output/vector-golden");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, command, text, x, y) in [
        ("h", "^FPH,15", "FIELDGAP", 80, 80),
        ("r", "^FPR,10", "REVERSE", 400, 160),
        ("v", "^FPV,30", "VERTICAL", 80, 240),
    ] {
        let source = format!("^XA^PW640^LL640^FO{x},{y}^A0N,40,32{command}^FD{text}^FS^XZ\n");
        let options = RenderOptions {
            language: Some(Language::Zpl),
            width_mm: Some(80.0),
            height_mm: Some(80.0),
            dpmm: 8,
            antialias: false,
        };
        let pdf = labelize::render_pdf_with_output(
            source.as_bytes(),
            options.clone(),
            OutputOptions {
                mode: OutputMode::Vector,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pdf.labels, 1);
        assert!(pdf.bytes.starts_with(b"%PDF-"));
        let png = labelize::render_png_with_output(
            source.as_bytes(),
            options,
            OutputOptions {
                mode: OutputMode::Vector,
                scale: 3.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(png.labels, 1);
        let image = image::load_from_memory(&png.bytes).unwrap();
        assert_eq!((image.width(), image.height()), (1920, 1920));
        let stem = format!("field-parameter-{name}");
        std::fs::write(directory.join(format!("{stem}.zpl")), source).unwrap();
        std::fs::write(directory.join(format!("{stem}.pdf")), pdf.bytes).unwrap();
        std::fs::write(directory.join(format!("{stem}-3x.png")), png.bytes).unwrap();
    }
}
