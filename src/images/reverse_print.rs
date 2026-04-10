use image::RgbaImage;

/// Apply ZPL `^FR` (Field Reverse) to the canvas using the rendered mask.
///
/// The mask is rendered on a fully transparent background; only the element's
/// own pixels are opaque (alpha >= 30).  The algorithm:
///
/// 1. Compute the bounding box of all opaque pixels — this approximates the
///    field area that ^FR must invert.
/// 2. Fill that bounding box with black on the canvas (field background).
/// 3. For each opaque mask pixel, write the inverted mask colour onto the
///    canvas (dark stroke → white; light area → dark), producing white
///    text/graphics on a black field background.
///
/// This matches Zebra hardware behaviour: the field content is XOR'd with the
/// existing label background (white label → black field with white content).
pub fn reverse_print(mask: &RgbaImage, background: &mut RgbaImage) {
    let alpha_threshold = 30u8;
    let (width, height) = mask.dimensions();

    // Pass 1: compute bounding box of rendered element pixels.
    let mut min_x = u32::MAX;
    let mut max_x = 0u32;
    let mut min_y = u32::MAX;
    let mut max_y = 0u32;

    for y in 0..height {
        for x in 0..width {
            if mask.get_pixel(x, y)[3] >= alpha_threshold {
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
            }
        }
    }

    if min_x > max_x {
        return; // nothing was rendered, nothing to invert
    }

    // Pass 2: XOR semantics — iterate only over the bounding box.
    //
    // For each pixel inside the field area:
    //   • transparent in mask (field background) → invert the canvas pixel.
    //     On a white canvas this produces a black background; on a canvas that
    //     was already blackened by a previous ^FR field it restores white,
    //     matching Zebra's true XOR behaviour for overlapping ^FR fields.
    //   • opaque in mask (element stroke) → leave the canvas pixel unchanged.
    //     After the background inversion the canvas is black in that area, so
    //     the unmodified original canvas value (white) becomes the "ink" colour.
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let m = mask.get_pixel(x, y);
            if m[3] < alpha_threshold {
                // Background pixel: invert canvas (white→black, black→white).
                let bg = background.get_pixel(x, y);
                background.put_pixel(
                    x,
                    y,
                    image::Rgba([255 - bg[0], 255 - bg[1], 255 - bg[2], bg[3]]),
                );
            }
            // Stroke pixels: canvas unchanged → appears as the inverse of the
            // background (i.e. white text / white bars on black background).
        }
    }
}
