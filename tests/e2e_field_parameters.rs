//! Independent geometry acceptance against the committed Labelary capture.
//! These checks use isolated fields: sparse whole-page similarity alone would
//! otherwise allow an ignored direction or character-spacing command to pass.
use image::{GrayImage, ImageBuffer};
use labelize::{DrawerOptions, OutputMode, OutputOptions, ZplParser};

const SOURCE: &[u8] = include_bytes!("../testdata/unit/field_parameters.zpl");
const REFERENCE: &[u8] = include_bytes!("../testdata/unit/field_parameters.png");

#[derive(Clone, Debug)]
struct Ink {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

fn images(mode: OutputMode) -> (GrayImage, GrayImage) {
    let labels = ZplParser::new().parse(SOURCE).unwrap();
    assert_eq!(labels.len(), 1);
    let bytes = labelize::render_label_with_output(
        &labels[0],
        DrawerOptions {
            label_width_mm: 101.625,
            label_height_mm: 203.25,
            dpmm: 8,
            ..Default::default()
        },
        OutputOptions {
            mode,
            ..Default::default()
        },
        false,
    )
    .unwrap();
    let actual = image::load_from_memory(&bytes).unwrap().to_luma8();
    let reference = image::load_from_memory(REFERENCE).unwrap().to_luma8();
    assert_eq!(actual.dimensions(), reference.dimensions());
    (actual, reference)
}

fn crop(image: &GrayImage, x: u32, y: u32, width: u32, height: u32) -> GrayImage {
    ImageBuffer::from_fn(width, height, |dx, dy| *image.get_pixel(x + dx, y + dy))
}

fn bounds(image: &GrayImage) -> Ink {
    let (mut x0, mut y0, mut x1, mut y1) = (image.width(), image.height(), 0, 0);
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[0] < 128 {
            found = true;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    assert!(found, "expected nonempty isolated text field");
    let pixels = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| u8::from(image.get_pixel(x, y)[0] < 128)))
        .collect();
    Ink {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
        pixels,
    }
}

fn parts(image: &GrayImage, axis: Axis) -> Vec<Ink> {
    let length = match axis {
        Axis::Horizontal => image.width(),
        Axis::Vertical => image.height(),
    };
    let occupied = |index| match axis {
        Axis::Horizontal => (0..image.height()).any(|y| image.get_pixel(index, y)[0] < 128),
        Axis::Vertical => (0..image.width()).any(|x| image.get_pixel(x, index)[0] < 128),
    };
    let mut start = None;
    let mut result = Vec::new();
    for index in 0..=length {
        let has_ink = index < length && occupied(index);
        match (start, has_ink) {
            (None, true) => start = Some(index),
            (Some(begin), false) => {
                let region = match axis {
                    Axis::Horizontal => crop(image, begin, 0, index - begin, image.height()),
                    Axis::Vertical => crop(image, 0, begin, image.width(), index - begin),
                };
                let mut ink = bounds(&region);
                match axis {
                    Axis::Horizontal => ink.x += begin,
                    Axis::Vertical => ink.y += begin,
                }
                result.push(ink);
                start = None;
            }
            _ => {}
        }
    }
    result
}

fn horizontal(image: &GrayImage, y: u32) -> GrayImage {
    // Annotations end before x=220 and are excluded from every measurement.
    crop(image, 220, y - 5, 560, 55)
}

fn vertical(image: &GrayImage, x: u32) -> GrayImage {
    crop(image, x - 20, 935, 90, 260)
}

fn assert_shape(a: &Ink, b: &Ink, context: &str) {
    assert_eq!((a.width, a.height), (b.width, b.height), "{context}");
    assert_eq!(a.pixels, b.pixels, "{context}: upright glyph mask changed");
}

fn assert_upright_shape(a: &Ink, b: &Ink, context: &str) {
    assert!(
        a.width.abs_diff(b.width) <= 1,
        "{context}: glyph width changed"
    );
    assert!(
        a.height.abs_diff(b.height) <= 1,
        "{context}: glyph height changed"
    );
    // Moving the substitute's outline changes its subpixel sampling phase.
    // Permit a one-dot edge change after a one-dot alignment, while rejecting
    // missing interior strokes, changed glyph order, reflection or rotation.
    let covered = |from: &Ink, to: &Ink, dx: i32, dy: i32| {
        (0..from.height).all(|y| {
            (0..from.width).all(|x| {
                if from.pixels[(y * from.width + x) as usize] == 0 {
                    return true;
                }
                (-1..=1).any(|ey| {
                    (-1..=1).any(|ex| {
                        let tx = x as i32 + dx + ex;
                        let ty = y as i32 + dy + ey;
                        tx >= 0
                            && ty >= 0
                            && tx < to.width as i32
                            && ty < to.height as i32
                            && to.pixels[(ty as u32 * to.width + tx as u32) as usize] != 0
                    })
                })
            })
        })
    };
    assert!(
        (-1..=1).any(|dy| { (-1..=1).any(|dx| covered(a, b, dx, dy) && covered(b, a, -dx, -dy)) }),
        "{context}: expected the same upright glyph within one-dot raster edges"
    );
}

fn assert_gap(base: &GrayImage, spaced: &GrayImage, gap: i32, context: &str) {
    let a = parts(base, Axis::Horizontal);
    let b = parts(spaced, Axis::Horizontal);
    assert_eq!((a.len(), b.len()), (5, 5), "{context}: five letters");
    for (index, (a, b)) in a.iter().zip(&b).enumerate() {
        assert_eq!(
            b.x as i32 - a.x as i32,
            index as i32 * gap,
            "{context}: glyph {index} must move by exactly index * gap"
        );
        assert_shape(a, b, context);
    }
    assert_eq!(
        bounds(spaced).width as i32 - bounds(base).width as i32,
        4 * gap,
        "{context}: five letters have four gaps"
    );
}

fn assert_origin_and_glyph_size(actual: &GrayImage, reference: &GrayImage, context: &str) {
    let a = parts(actual, Axis::Horizontal);
    let b = parts(reference, Axis::Horizontal);
    assert_eq!((a.len(), b.len()), (5, 5), "{context}");
    assert!(a[0].x.abs_diff(b[0].x) <= 3, "{context}: initial x origin");
    assert!(a[0].y.abs_diff(b[0].y) <= 3, "{context}: cap-top origin");
    for (a, b) in a.iter().zip(&b) {
        assert!(
            a.width.abs_diff(b.width) <= 6,
            "{context}: substitute glyph width"
        );
        assert!(
            a.height.abs_diff(b.height) <= 6,
            "{context}: substitute glyph height"
        );
    }
}

fn horizontal_acceptance(mode: OutputMode) {
    let (actual, reference) = images(mode);
    for (font, top) in [("font 0", 60), ("font A", 440)] {
        let a0 = horizontal(&actual, top);
        let r0 = horizontal(&reference, top);
        assert_origin_and_glyph_size(&a0, &r0, font);
        // Compare parameter-induced displacement separately from the known
        // substitute-font advances; existing font shapes are not the oracle.
        for (offset, gap) in [(60, 5), (120, 15), (180, 30)] {
            assert_gap(&r0, &horizontal(&reference, top + offset), gap, "Labelary");
            assert_gap(&a0, &horizontal(&actual, top + offset), gap, font);
        }
    }
}

fn reverse_acceptance(mode: OutputMode) {
    let (actual, reference) = images(mode);
    for (font, normal_y, reverse_y) in [("font 0", 60, 300), ("font A", 440, 680)] {
        for (image, renderer) in [(&actual, "Labelize"), (&reference, "Labelary")] {
            let normal = parts(&horizontal(image, normal_y), Axis::Horizontal);
            let reverse = parts(&horizontal(image, reverse_y + 60), Axis::Horizontal);
            assert_eq!((normal.len(), reverse.len()), (5, 5), "{renderer} {font}");
            for (expected, actual) in normal.iter().rev().zip(&reverse) {
                assert_upright_shape(expected, actual, &format!("{renderer} {font}: EDCBA"));
            }
            let plain = bounds(&horizontal(image, reverse_y));
            let spaced = bounds(&horizontal(image, reverse_y + 60));
            assert_eq!(
                spaced.width - plain.width,
                40,
                "{renderer} {font}: reverse gap"
            );
        }
        let a = parts(&horizontal(&actual, reverse_y + 60), Axis::Horizontal);
        let r = parts(&horizontal(&reference, reverse_y + 60), Axis::Horizontal);
        // The first logical glyph A is the rightmost visible glyph.
        assert!(a[4].x.abs_diff(r[4].x) <= 6, "{font}: reverse anchor");
        assert!(a[4].y.abs_diff(r[4].y) <= 3, "{font}: reverse cap top");
    }
}

fn vertical_acceptance(mode: OutputMode) {
    let (actual, reference) = images(mode);
    for (font, normal_y, first_x, spaced_x) in [("font 0", 60, 80, 220), ("font A", 440, 460, 620)]
    {
        for (image, renderer) in [(&actual, "Labelize"), (&reference, "Labelary")] {
            let normal = parts(&horizontal(image, normal_y), Axis::Horizontal);
            let first = vertical(image, first_x);
            let spaced = vertical(image, spaced_x);
            // ROIs are equal relative to their field origins, so this compares
            // the complete field, including placements and background pixels.
            assert_eq!(first, spaced, "{renderer} {font}: vertical gap is ignored");
            let glyphs = parts(&first, Axis::Vertical);
            assert_eq!(glyphs.len(), 5, "{renderer} {font}: five vertical glyphs");
            for (expected, actual) in normal.iter().zip(&glyphs) {
                assert_upright_shape(expected, actual, &format!("{renderer} {font}: A/B/C/D/E"));
            }
        }
        let a = parts(&vertical(&actual, first_x), Axis::Vertical);
        let r = parts(&vertical(&reference, first_x), Axis::Vertical);
        for (a, r) in a.iter().zip(&r) {
            assert!(a.y.abs_diff(r.y) <= 3, "{font}: vertical cell pitch");
            assert!(a.x.abs_diff(r.x) <= 6, "{font}: vertical alignment");
        }
    }
}

fn reset_and_negative_acceptance(mode: OutputMode) {
    let (actual, reference) = images(mode);
    for (font, normal_y, negative_y, reset_y) in
        [("font 0", 60, 1265, 1385), ("font A", 440, 1325, 1445)]
    {
        for (image, renderer) in [(&actual, "Labelize"), (&reference, "Labelary")] {
            let normal = horizontal(image, normal_y);
            let negative = horizontal(image, negative_y);
            assert_eq!(
                bounds(&normal).width as i32 - bounds(&negative).width as i32,
                20,
                "{renderer} {font}: negative five-dot gap"
            );
            assert_eq!(
                normal,
                horizontal(image, reset_y),
                "{renderer} {font}: field separator restores default flow"
            );
        }
    }
}

fn absolute_character_positions(mode: OutputMode) {
    let (actual, reference) = images(mode);
    // Check absolute positions against the independent capture as well as
    // parameter-induced shifts: a wrong natural advance still adds index * gap
    // correctly, but accumulates a visible error across the field.
    // Retain the existing three-dot origin allowance for substitute bearings,
    // applying it to every character instead of only the first one.
    for y in [
        60, 120, 180, 240, 300, 360, 440, 500, 560, 620, 680, 740, 1385, 1445,
    ] {
        let a = parts(&horizontal(&actual, y), Axis::Horizontal);
        let r = parts(&horizontal(&reference, y), Axis::Horizontal);
        assert_eq!((a.len(), r.len()), (5, 5), "row {y}: five letters");
        for (index, (a, r)) in a.iter().zip(&r).enumerate() {
            assert!(
                a.x.abs_diff(r.x) <= 3,
                "row {y}, glyph {index}: x={}, Labelary x={}",
                a.x,
                r.x
            );
        }
    }
    for y in [1265, 1325] {
        let a = bounds(&horizontal(&actual, y));
        let r = bounds(&horizontal(&reference, y));
        assert!(a.x.abs_diff(r.x) <= 2, "row {y}: negative-gap origin");
        assert!(
            a.width.abs_diff(r.width) <= 3,
            "row {y}: negative-gap width={}, Labelary width={}",
            a.width,
            r.width
        );
    }
}

#[test]
fn raster_absolute_character_positions_match_labelary() {
    absolute_character_positions(OutputMode::Raster);
}

#[cfg(feature = "vector")]
#[test]
fn vector_absolute_character_positions_match_labelary() {
    absolute_character_positions(OutputMode::Vector);
}

#[test]
fn raster_horizontal_gaps_match_labelary_displacements() {
    horizontal_acceptance(OutputMode::Raster);
}

#[test]
fn raster_reverse_order_and_spacing_match_labelary() {
    reverse_acceptance(OutputMode::Raster);
}

#[test]
fn raster_vertical_order_pitch_and_ignored_gap_match_labelary() {
    vertical_acceptance(OutputMode::Raster);
}

#[test]
fn raster_negative_spacing_and_field_reset_match_labelary() {
    reset_and_negative_acceptance(OutputMode::Raster);
}

#[cfg(feature = "vector")]
#[test]
fn vector_horizontal_gaps_match_labelary_displacements() {
    horizontal_acceptance(OutputMode::Vector);
}

#[cfg(feature = "vector")]
#[test]
fn vector_reverse_order_and_spacing_match_labelary() {
    reverse_acceptance(OutputMode::Vector);
}

#[cfg(feature = "vector")]
#[test]
fn vector_vertical_order_pitch_and_ignored_gap_match_labelary() {
    vertical_acceptance(OutputMode::Vector);
}

#[cfg(feature = "vector")]
#[test]
fn vector_negative_spacing_and_field_reset_match_labelary() {
    reset_and_negative_acceptance(OutputMode::Vector);
}
