use image::RgbaImage;

/// Apply ZPL ^FR (Field Reverse Print) semantics.
///
/// `use_bounding_box`: when true (text/barcode), inverts the entire field
/// bounding box to produce white content on a black background. When false
/// (graphic boxes/lines), inverts only the drawn pixels (standard XOR).
///
/// The distinction matters because ^GB fills its entire bounding box, so
/// bounding-box inversion would cancel out (double invert → no change).
/// Text/barcodes have transparent areas that need to become black background.
pub fn reverse_print(mask: &RgbaImage, background: &mut RgbaImage, use_bounding_box: bool) {
    let alpha_threshold = 30u8;
    let (width, height) = mask.dimensions();

    if use_bounding_box {
        // Find bounding box of opaque pixels in mask.
        let mut min_x = width;
        let mut max_x = 0u32;
        let mut min_y = height;
        let mut max_y = 0u32;

        for y in 0..height {
            for x in 0..width {
                if mask.get_pixel(x, y)[3] >= alpha_threshold {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }

        if min_x > max_x || min_y > max_y {
            return;
        }

        // Step 1: Invert the entire bounding box on the canvas (creates black background).
        // Step 2: Re-invert where mask is opaque (restores original color for content).
        // Net effect: black background + white content (on originally white canvas).
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if x < background.width() && y < background.height() {
                    let bg = background.get_pixel(x, y);
                    let inverted = image::Rgba([255 - bg[0], 255 - bg[1], 255 - bg[2], bg[3]]);
                    background.put_pixel(x, y, inverted);
                }
            }
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if x < background.width() && y < background.height() {
                    if mask.get_pixel(x, y)[3] >= alpha_threshold {
                        let bg = background.get_pixel(x, y);
                        let restored = image::Rgba([255 - bg[0], 255 - bg[1], 255 - bg[2], bg[3]]);
                        background.put_pixel(x, y, restored);
                    }
                }
            }
        }
    } else {
        // Graphic elements: simple pixel-level XOR (original behavior).
        // Inverts canvas pixels where the mask drew something.
        for y in 0..height {
            for x in 0..width {
                let m = mask.get_pixel(x, y);
                if m[3] < alpha_threshold {
                    continue;
                }
                if x < background.width() && y < background.height() {
                    let bg = background.get_pixel(x, y);
                    background.put_pixel(
                        x,
                        y,
                        image::Rgba([255 - bg[0], 255 - bg[1], 255 - bg[2], bg[3]]),
                    );
                }
            }
        }
    }
}
