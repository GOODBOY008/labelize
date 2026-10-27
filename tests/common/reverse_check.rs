//! Shared assertions for reverse-rendering round trips: the binarization
//! contract and the pixel-identity check. Used by `e2e_reverse.rs` (library
//! round trips) and `e2e_http_encode.rs` (HTTP endpoint round trips), so
//! both surfaces are held to the same contract.

use image::RgbaImage;

/// The documented binarization contract: a pixel is ink when its
/// composite-over-white Rec.601 luminance is strictly below `threshold`.
pub fn binarize(source: &RgbaImage, threshold: u8) -> RgbaImage {
    let mut expected = RgbaImage::from_pixel(
        source.width(),
        source.height(),
        image::Rgba([255, 255, 255, 255]),
    );
    for (x, y, pixel) in source.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let over_white = |c: u8| (c as u32 * a as u32 + 255 * (255 - a as u32) + 127) / 255;
        let luma = (299 * over_white(r) + 587 * over_white(g) + 114 * over_white(b)) / 1000;
        if luma < threshold as u32 {
            expected.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    expected
}

/// Rendered PNG bytes must equal the binarized source exactly — dimensions
/// and every pixel.
pub fn assert_round_trip(source: &RgbaImage, rendered_png: &[u8], what: &str) {
    assert_round_trip_at_threshold(source, rendered_png, 128, what);
}

pub fn assert_round_trip_at_threshold(
    source: &RgbaImage,
    rendered_png: &[u8],
    threshold: u8,
    what: &str,
) {
    let expected = binarize(source, threshold);
    let rendered = image::load_from_memory(rendered_png)
        .unwrap_or_else(|e| panic!("{what}: rendered output is not a PNG: {e}"))
        .to_rgba8();
    assert_eq!(
        rendered.dimensions(),
        expected.dimensions(),
        "{what}: canvas size must match the source image"
    );
    let mut first_mismatch = None;
    let mut mismatched = 0usize;
    for (x, y, pixel) in expected.enumerate_pixels() {
        if rendered.get_pixel(x, y) != pixel {
            mismatched += 1;
            if first_mismatch.is_none() {
                first_mismatch = Some((x, y, *pixel, *rendered.get_pixel(x, y)));
            }
        }
    }
    assert_eq!(
        mismatched, 0,
        "{what}: {mismatched} pixels differ after the round trip (first at {:?})",
        first_mismatch
    );
}
