use image::{Rgba, RgbaImage};
use labelize::DrawerOptions;
use labelize::encode_pdf;

#[test]
fn pdf_output_is_non_empty() {
    let img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
    let opts = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 50.0,
        dpmm: 8,
        ..Default::default()
    };
    let mut buf = Vec::new();
    encode_pdf(&img, &opts, &mut buf).expect("encode_pdf failed");
    assert!(!buf.is_empty(), "PDF output should be non-empty");
}

#[test]
fn pdf_starts_with_pdf_header() {
    let img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
    let opts = DrawerOptions::default();
    let mut buf = Vec::new();
    encode_pdf(&img, &opts, &mut buf).expect("encode_pdf failed");
    let header = std::str::from_utf8(&buf[..5]).unwrap_or("");
    assert_eq!(header, "%PDF-", "PDF should start with %PDF- header");
}

#[test]
fn pdf_contains_mediabox() {
    let img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
    let opts = DrawerOptions {
        label_width_mm: 102.0,
        label_height_mm: 152.0,
        dpmm: 8,
        ..Default::default()
    };
    let mut buf = Vec::new();
    encode_pdf(&img, &opts, &mut buf).expect("encode_pdf failed");
    let content = String::from_utf8_lossy(&buf);
    assert!(
        content.contains("MediaBox"),
        "PDF should contain MediaBox"
    );
}
