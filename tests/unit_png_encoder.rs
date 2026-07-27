use image::{Rgba, RgbaImage};
use labelize::encode_png;

fn round_trip(img: &RgbaImage) -> image::GrayImage {
    let mut buf = Vec::new();
    encode_png(img, &mut buf).expect("encode_png failed");
    let decoded = image::load_from_memory(&buf).expect("decode PNG");
    decoded.to_luma8()
}

#[test]
fn round_trip_preserves_dimensions() {
    let img = RgbaImage::from_pixel(100, 50, Rgba([255, 255, 255, 255]));
    let gray = round_trip(&img);
    assert_eq!(gray.width(), 100);
    assert_eq!(gray.height(), 50);
}

#[test]
fn grayscale_values_preserved() {
    let img = RgbaImage::from_pixel(10, 10, Rgba([200, 200, 200, 255]));
    let gray = round_trip(&img);
    for p in gray.pixels() {
        assert_eq!(p[0], 200, "grayscale value should be preserved");
    }
}

#[test]
fn black_pixels_preserved() {
    let img = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
    let gray = round_trip(&img);
    for p in gray.pixels() {
        assert_eq!(p[0], 0, "black should be preserved");
    }
}

#[test]
fn white_pixels_preserved() {
    let img = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
    let gray = round_trip(&img);
    for p in gray.pixels() {
        assert_eq!(p[0], 255, "white should be preserved");
    }
}

#[test]
fn mixed_pixel_values() {
    let mut img = RgbaImage::new(4, 1);
    img.put_pixel(0, 0, Rgba([0, 0, 0, 255])); // black
    img.put_pixel(1, 0, Rgba([128, 128, 128, 255])); // mid-gray
    img.put_pixel(2, 0, Rgba([200, 200, 200, 255])); // light gray
    img.put_pixel(3, 0, Rgba([255, 255, 255, 255])); // white

    let gray = round_trip(&img);
    assert_eq!(gray.get_pixel(0, 0)[0], 0);
    assert_eq!(gray.get_pixel(1, 0)[0], 128);
    assert_eq!(gray.get_pixel(2, 0)[0], 200);
    assert_eq!(gray.get_pixel(3, 0)[0], 255);
}

#[test]
fn single_pixel_image() {
    let img = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
    let gray = round_trip(&img);
    assert_eq!(gray.width(), 1);
    assert_eq!(gray.height(), 1);
}
