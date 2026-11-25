use labelize::{render_png, Language, RenderOptions};
use rxing::{helpers::detect_in_luma, BarcodeFormat};

#[test]
fn retail_and_interleaved_barcodes_scan_at_both_resolutions() {
    let cases = [
        ("EAN8", "9638507", BarcodeFormat::EAN_8, "96385074"),
        ("UPCA", "01234567890", BarcodeFormat::UPC_A, "012345678905"),
        ("25", "001234", BarcodeFormat::ITF, "001234"),
    ];
    for (kind, data, format, expected) in cases {
        let label = format!(
            "SIZE 50 mm,30 mm\nCLS\nBARCODE 10,10,\"{kind}\",40,0,0,2,3,\"{data}\"\nPRINT 1,1\n"
        );
        for dpmm in [8, 12] {
            let options = RenderOptions {
                language: Some(Language::Tspl),
                dpmm,
                ..Default::default()
            };
            let png = render_png(label.as_bytes(), options).unwrap();
            let pixels = image::load_from_memory(&png.bytes).unwrap().to_luma8();
            let (width, height) = pixels.dimensions();
            let scanned = detect_in_luma(pixels.into_raw(), width, height, Some(format))
                .unwrap_or_else(|e| panic!("scan {kind} at {dpmm} dpmm: {e}"));
            assert_eq!(scanned.getText(), expected, "{kind} at {dpmm} dpmm");
        }
    }
}

#[test]
fn plain_interleaved_2_of_5_requires_digit_pairs() {
    for data in ["", "00123", "00A123"] {
        let input = format!("BARCODE 10,10,\"25\",40,0,0,2,3,\"{data}\"");
        let error = Language::Tspl.parse(input.as_bytes(), 8).unwrap_err();
        assert!(error.to_string().contains("even number of decimal digits"));
    }
}
