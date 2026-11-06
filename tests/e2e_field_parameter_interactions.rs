//! Independent placement and wrapping checks against frozen Labelary captures.
//! Unlike whole-page similarity, isolated geometry cannot hide command errors
//! in a large white background. See testdata/field-parameters/provenance.json.
use image::{GrayImage, ImageBuffer};
use labelize::{DrawerOptions, OutputMode, OutputOptions, ZplParser};

struct Capture {
    source: &'static [u8],
    reference: &'static [u8],
}

macro_rules! capture {
    ($name:literal) => {
        Capture {
            source: include_bytes!(concat!("../testdata/field-parameters/", $name, ".zpl")),
            reference: include_bytes!(concat!("../testdata/field-parameters/", $name, ".png")),
        }
    };
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    x: i32,
    y: i32,
    right: i32,
    bottom: i32,
}

impl Bounds {
    fn width(self) -> i32 {
        self.right - self.x
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn render(capture: Capture, mode: OutputMode) -> (GrayImage, GrayImage) {
    let reference = image::load_from_memory(capture.reference)
        .unwrap()
        .to_luma8();
    let labels = ZplParser::new().parse(capture.source).unwrap();
    assert_eq!(labels.len(), 1);
    let bytes = labelize::render_label_with_output(
        &labels[0],
        DrawerOptions {
            label_width_mm: reference.width() as f64 / 8.0,
            label_height_mm: reference.height() as f64 / 8.0,
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
    assert_eq!(actual.dimensions(), reference.dimensions());
    (actual, reference)
}

fn crop(image: &GrayImage, x: u32, y: u32, width: u32, height: u32) -> GrayImage {
    ImageBuffer::from_fn(width, height, |dx, dy| *image.get_pixel(x + dx, y + dy))
}

fn bounds(image: &GrayImage) -> Bounds {
    let (mut x, mut y, mut right, mut bottom) = (i32::MAX, i32::MAX, 0, 0);
    for (px, py, value) in image.enumerate_pixels() {
        if value[0] < 128 {
            x = x.min(px as i32);
            y = y.min(py as i32);
            right = right.max(px as i32 + 1);
            bottom = bottom.max(py as i32 + 1);
        }
    }
    assert_ne!(x, i32::MAX, "expected isolated text ink");
    Bounds {
        x,
        y,
        right,
        bottom,
    }
}

fn parts(image: &GrayImage, axis: Axis) -> Vec<Bounds> {
    let length = match axis {
        Axis::X => image.width(),
        Axis::Y => image.height(),
    };
    let occupied = |index| match axis {
        Axis::X => (0..image.height()).any(|y| image.get_pixel(index, y)[0] < 128),
        Axis::Y => (0..image.width()).any(|x| image.get_pixel(x, index)[0] < 128),
    };
    let mut start = None;
    let mut result = Vec::new();
    for index in 0..=length {
        let present = index < length && occupied(index);
        match (start, present) {
            (None, true) => start = Some(index),
            (Some(begin), false) => {
                let mut b = match axis {
                    Axis::X => bounds(&crop(image, begin, 0, index - begin, image.height())),
                    Axis::Y => bounds(&crop(image, 0, begin, image.width(), index - begin)),
                };
                match axis {
                    Axis::X => {
                        b.x += begin as i32;
                        b.right += begin as i32;
                    }
                    Axis::Y => {
                        b.y += begin as i32;
                        b.bottom += begin as i32;
                    }
                }
                result.push(b);
                start = None;
            }
            _ => {}
        }
    }
    result
}

fn close(actual: i32, expected: i32, allowance: i32, context: &str) {
    assert!(
        (actual - expected).abs() <= allowance,
        "{context}: actual {actual}, Labelary {expected}, allowance {allowance}"
    );
}

fn positioned(actual: Bounds, reference: Bounds, context: &str) {
    // Font-0 outline/rounding differences are at most three source dots here.
    // Images are never registered or translated to improve this comparison.
    close(actual.x, reference.x, 3, &format!("{context}: left"));
    close(actual.y, reference.y, 3, &format!("{context}: top"));
    close(
        actual.right,
        reference.right,
        3,
        &format!("{context}: right"),
    );
    close(
        actual.bottom,
        reference.bottom,
        3,
        &format!("{context}: bottom"),
    );
}

fn rotated_origins(mode: OutputMode) {
    for (name, capture, axis) in [
        ("reverse 90", capture!("direction_rotation_R_R"), Axis::Y),
        ("vertical 90", capture!("direction_rotation_V_R"), Axis::X),
    ] {
        let (actual, reference) = render(capture, mode);
        for (origin, top, height) in [("FO", 0, 500), ("FT", 500, actual.height() - 500)] {
            let a = parts(&crop(&actual, 0, top, actual.width(), height), axis);
            let r = parts(&crop(&reference, 0, top, reference.width(), height), axis);
            assert_eq!((a.len(), r.len()), (5, 5), "{name} {origin}: five glyphs");
            for (index, (a, r)) in a.iter().zip(&r).enumerate() {
                positioned(*a, *r, &format!("{name} {origin}, glyph {index}"));
            }
        }
    }
}

fn explicit_origins(mode: OutputMode) {
    let (actual, reference) = render(capture!("FO_direction_alignments"), mode);
    for (direction, x, axis) in [
        ("H", 450, Axis::X),
        ("R", 1000, Axis::X),
        ("V", 1550, Axis::Y),
    ] {
        for alignment in 0..=2 {
            let y = 80 + alignment * 300;
            let a = parts(&crop(&actual, x - 300, y - 20, 550, 260), axis);
            let r = parts(&crop(&reference, x - 300, y - 20, 550, 260), axis);
            assert_eq!(
                (a.len(), r.len()),
                (5, 5),
                "{direction}, alignment {alignment}"
            );
            for (index, (a, r)) in a.iter().zip(&r).enumerate() {
                positioned(
                    *a,
                    *r,
                    &format!("{direction}, alignment {alignment}, glyph {index}"),
                );
            }
        }
    }
}

fn block_alignment(mode: OutputMode) {
    let (actual, reference) = render(capture!("FB_alignment_gaps"), mode);
    for (alignment, x) in [("L", 60), ("C", 720), ("R", 1380)] {
        let a0 = bounds(&crop(&actual, x - 5, 75, 600, 90));
        let r0 = bounds(&crop(&reference, x - 5, 75, 600, 90));
        for (index, gap) in [(1, 15), (2, 30)] {
            let y = 80 + index * 160;
            let a = bounds(&crop(&actual, x - 5, y - 5, 600, 90));
            let r = bounds(&crop(&reference, x - 5, y - 5, 600, 90));
            // Existing center-alignment/font calibration is a baseline debt.
            // Only the parameter-induced shift is normalized; a six- or
            // twelve-dot spacing-anchor error cannot fit this one-dot budget.
            close(
                a.x - a0.x,
                r.x - r0.x,
                1,
                &format!("block {alignment}, gap {gap}: spacing-induced origin shift"),
            );
            close(
                a.width() - a0.width(),
                r.width() - r0.width(),
                1,
                &format!("block {alignment}, gap {gap}: spacing-induced width"),
            );
            close(
                a.y - a0.y,
                r.y - r0.y,
                1,
                &format!("block {alignment}, gap {gap}: cap top"),
            );
        }
    }
}

fn block_wrapping(mode: OutputMode) {
    let (actual, reference) = render(capture!("block_gap_comparison"), mode);
    for (index, gap) in [(0, 0), (1, 15), (2, 30)] {
        for (direction, x) in [("H", 120), ("V", 600)] {
            let y = 80 + index * 280;
            let a = parts(&crop(&actual, x - 20, y - 20, 300, 200), Axis::Y);
            let r = parts(&crop(&reference, x - 20, y - 20, 300, 200), Axis::Y);
            assert_eq!(a.len(), r.len(), "block {direction}, gap {gap}: line count");
            assert!(!r.is_empty());
            for (line, (a, r)) in a.iter().zip(&r).enumerate() {
                positioned(
                    *a,
                    *r,
                    &format!("block {direction}, gap {gap}, line {line}"),
                );
            }
        }
    }
    let (actual, reference) = render(capture!("block_R"), mode);
    let a = parts(&actual, Axis::Y);
    let r = parts(&reference, Axis::Y);
    assert_eq!(
        (a.len(), r.len()),
        (2, 2),
        "reverse-before-wrap: GF ED / CBA"
    );
    for (line, (a, r)) in a.iter().zip(&r).enumerate() {
        positioned(*a, *r, &format!("reverse block, line {line}"));
    }
}

#[test]
fn raster_rotated_direction_origins_match_labelary() {
    rotated_origins(OutputMode::Raster);
}
#[test]
fn raster_explicit_origins_match_labelary() {
    explicit_origins(OutputMode::Raster);
}
#[test]
fn raster_spacing_changes_block_alignment_like_labelary() {
    block_alignment(OutputMode::Raster);
}
#[test]
fn raster_directional_blocks_wrap_like_labelary() {
    block_wrapping(OutputMode::Raster);
}

#[cfg(feature = "vector")]
#[test]
fn vector_rotated_direction_origins_match_labelary() {
    rotated_origins(OutputMode::Vector);
}
#[cfg(feature = "vector")]
#[test]
fn vector_explicit_origins_match_labelary() {
    explicit_origins(OutputMode::Vector);
}
#[cfg(feature = "vector")]
#[test]
fn vector_spacing_changes_block_alignment_like_labelary() {
    block_alignment(OutputMode::Vector);
}
#[cfg(feature = "vector")]
#[test]
fn vector_directional_blocks_wrap_like_labelary() {
    block_wrapping(OutputMode::Vector);
}
