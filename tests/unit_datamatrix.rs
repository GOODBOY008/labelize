use image::RgbaImage;
use labelize::barcodes::datamatrix;
use labelize::elements::barcode_datamatrix::DatamatrixRatio;
use labelize::{DrawerOptions, Renderer, ZplParser};

const PAYLOAD: &str = "ABCD12345678901234567890efgh";

#[test]
fn fixed_rectangle_preserves_both_dimensions() {
    for content in ["A", PAYLOAD] {
        let img = datamatrix::encode(content, 1, 12, 36).unwrap();
        assert_eq!(img.dimensions(), (36, 12), "{content}");
    }
}

#[test]
fn fixed_size_too_small_returns_error() {
    assert!(datamatrix::encode(PAYLOAD, 1, 10, 10).is_err());
}

#[test]
fn partial_rectangle_accepts_content_that_fits_after_compression() {
    let img = datamatrix::encode_with_ratio(PAYLOAD, 1, 12, 0, Some(DatamatrixRatio::Rectangular))
        .unwrap();
    assert_eq!(img.dimensions(), (36, 12));
}

#[test]
fn automatic_and_partial_dimensions_respect_shape() {
    use DatamatrixRatio::{Rectangular, Square};
    for (rows, columns, ratio, width, height) in [
        (0, 0, None, 10, 10),
        (0, 0, Some(Square), 10, 10),
        (0, 0, Some(Rectangular), 18, 8),
        (26, 26, Some(Square), 26, 26),
        (26, 0, Some(Square), 26, 26),
        (0, 26, Some(Square), 26, 26),
        (26, 0, None, 26, 26),
        (0, 26, None, 26, 26),
        (12, 36, Some(Rectangular), 36, 12),
        (12, 0, Some(Rectangular), 26, 12),
        (0, 36, Some(Rectangular), 36, 12),
    ] {
        let img = datamatrix::encode_with_ratio("A", 1, rows, columns, ratio).unwrap();
        assert_eq!(
            img.dimensions(),
            (width, height),
            "{rows}, {columns}, {ratio:?}"
        );
    }
}

#[test]
fn invalid_or_conflicting_constraints_return_errors() {
    use DatamatrixRatio::{Rectangular, Square};
    for (rows, columns, ratio) in [
        (-1, 0, None),
        (0, -1, None),
        (11, 11, None),
        (12, 14, None),
        (200, 200, None),
        (12, 36, Some(Square)),
        (18, 18, Some(Rectangular)),
        (26, 0, Some(Rectangular)),
    ] {
        assert!(datamatrix::encode_with_ratio("A", 1, rows, columns, ratio).is_err());
    }
    // A rectangle-only request must not fall back to a larger square.
    assert!(datamatrix::encode_with_ratio(&"A".repeat(200), 1, 0, 0, Some(Rectangular)).is_err());
    // A partial fixed size must not be relaxed when the payload does not fit.
    for (rows, columns) in [(10, 0), (0, 10)] {
        assert!(datamatrix::encode_with_ratio(PAYLOAD, 1, rows, columns, Some(Square)).is_err());
    }
}

#[test]
fn magnification_scales_modules_without_changing_the_symbol() {
    let modules = datamatrix::encode(PAYLOAD, 1, 12, 36).unwrap();
    for mag in [2, 3, 5] {
        let scaled = datamatrix::encode(PAYLOAD, mag, 12, 36).unwrap();
        assert_eq!(scaled.dimensions(), (36 * mag as u32, 12 * mag as u32));
        for (x, y, pixel) in scaled.enumerate_pixels() {
            assert_eq!(pixel, modules.get_pixel(x / mag as u32, y / mag as u32));
        }
    }
}

fn render_bx(parameters: &str, content: &str) -> Result<RgbaImage, String> {
    let zpl = format!("^XA^FO10,20^BX{parameters}^FD{content}^FS^XZ");
    let labels = ZplParser::new().parse(zpl.as_bytes())?;
    assert_eq!(labels.len(), 1);
    let options = DrawerOptions {
        label_width_mm: 60.0,
        label_height_mm: 40.0,
        dpmm: 8,
        ..Default::default()
    };
    let mut png = Vec::new();
    Renderer::new().draw_label_as_png(&labels[0], &mut png, options)?;
    Ok(image::load_from_memory(&png).unwrap().to_rgba8())
}

fn ink_bounds(image: &RgbaImage) -> (u32, u32, u32, u32) {
    let mut min_x = image.width();
    let mut min_y = image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[0] < 128 && pixel[3] > 0 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    assert!(min_x <= max_x && min_y <= max_y, "no barcode rendered");
    (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}

#[test]
fn zpl_renderer_honors_dimensions_ratio_and_magnification() {
    for (parameters, content, width, height) in [
        ("N,1,200,36,12,,_,2", PAYLOAD, 36, 12),
        ("N,3,200,36,12,,_,2", PAYLOAD, 108, 36),
        ("N,1,200,0,0,,_,2", "A", 18, 8),
        ("N,2,200,0,0,,_,2", "A", 36, 16),
        ("N,1,200,0,0,,_,1", "A", 10, 10),
        ("N,1,200", "A", 10, 10),
        ("N,1,200,36,0,,_,2", PAYLOAD, 36, 12),
        ("N,1,200,0,12,,_,2", PAYLOAD, 36, 12),
    ] {
        let image = render_bx(parameters, content).unwrap_or_else(|e| panic!("{parameters}: {e}"));
        assert_eq!(ink_bounds(&image), (10, 20, width, height), "{parameters}");
    }
}

#[test]
fn zpl_renderer_propagates_invalid_size_errors() {
    for parameters in [
        "N,1,200,10,10,,_,1",
        "N,1,200,14,12,,_,2",
        "N,1,200,36,12,,_,1",
        "N,1,200,18,18,,_,2",
    ] {
        let error = render_bx(parameters, PAYLOAD)
            .err()
            .expect("invalid dimensions must fail");
        assert!(error.contains("DataMatrix"), "{parameters}: {error}");
    }
}
