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
fn automatic_columns_do_not_widen_a_fixed_row_symbol_to_fit_the_payload() {
    use DatamatrixRatio::Rectangular;
    // ZD421 D04/P16: short c0/r12 matches c26/r12; the long input prints
    // only with explicit c36/r12. Labelary independently gives the same result.
    assert_eq!(
        datamatrix::encode_with_ratio("ABC", 1, 12, 0, Some(Rectangular)).unwrap(),
        datamatrix::encode_with_ratio("ABC", 1, 12, 26, Some(Rectangular)).unwrap()
    );
    assert!(datamatrix::encode_with_ratio(PAYLOAD, 1, 12, 0, Some(Rectangular)).is_err());
    assert_eq!(
        datamatrix::encode_with_ratio(PAYLOAD, 1, 12, 36, Some(Rectangular))
            .unwrap()
            .dimensions(),
        (36, 12)
    );
    // Labelary controls: the same fixed-row selection also occurs at r8.
    assert!(datamatrix::encode_with_ratio("aB!cD?eF", 1, 8, 0, Some(Rectangular)).is_err());
    assert_eq!(
        datamatrix::encode_with_ratio("aB!cD?eF", 1, 8, 32, Some(Rectangular))
            .unwrap()
            .dimensions(),
        (32, 8)
    );
    // Same observation at r16: c0 does not widen from 36 to 48 columns.
    let long16 = "aB!cD?eF#gH%iJ&kLaB!cD?eF#gH%iJ&kL";
    for columns in [0, 36] {
        assert!(datamatrix::encode_with_ratio(long16, 1, 16, columns, Some(Rectangular)).is_err());
    }
    assert_eq!(
        datamatrix::encode_with_ratio(long16, 1, 16, 48, Some(Rectangular))
            .unwrap()
            .dimensions(),
        (48, 16)
    );
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
fn fixed_row_capacity_boundary_matches_labelary() {
    use DatamatrixRatio::Rectangular;
    // 8 letters + 16 digits = 16 ASCII data codewords, exactly 26x12 capacity.
    // Adding one digit needs a 17th word; c0/r12 must not silently widen.
    let fits = "ABCD1234567890123490efgh";
    let over = "ABCD12345678901234901efgh";
    let expected = datamatrix::encode_with_ratio(fits, 1, 12, 26, Some(Rectangular)).unwrap();
    assert_eq!(expected.dimensions(), (26, 12));
    assert_eq!(
        datamatrix::encode_with_ratio(fits, 1, 12, 0, Some(Rectangular)).unwrap(),
        expected
    );
    for columns in [0, 26] {
        assert!(datamatrix::encode_with_ratio(over, 1, 12, columns, Some(Rectangular)).is_err());
    }
    assert_eq!(
        datamatrix::encode_with_ratio(over, 1, 12, 36, Some(Rectangular))
            .unwrap()
            .dimensions(),
        (36, 12)
    );
    // Exercise the omitted escape argument exactly as in the user/Labelary probe.
    assert_eq!(
        ink_bounds(&render_bx("N,4,200,0,12,6,,2", fits).unwrap()),
        (10, 20, 104, 48)
    );
    let omitted = render_bx("N,4,200,0,12,6,,2", over).unwrap();
    assert!(omitted.pixels().all(|p| p[3] == 0 || p[0] >= 128));
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
        ("N,1,200,0,12,,_,2", "ABC", 26, 12),
    ] {
        let image = render_bx(parameters, content).unwrap_or_else(|e| panic!("{parameters}: {e}"));
        assert_eq!(ink_bounds(&image), (10, 20, width, height), "{parameters}");
    }
}

#[test]
fn zpl_row_only_capacity_failure_skips_only_that_field() {
    // Compare against the exact same label with the rejected field absent;
    // include fields before/after and a subsequent label to catch state leakage.
    let rejected = format!("^FO40,80^BXN,4,200,0,12,6,_,2^FD{PAYLOAD}^FS");
    let source = format!("^XA^FO10,10^GB30,20,3^FS{rejected}^FO10,220^GB40,20,4^FS^FO180,80^BXN,2,200,0,12,6,_,2^FDABC^FS^XZ^XA^FO20,20^BXN,2,200,36,12,6,_,2^FD{PAYLOAD}^FS^XZ");
    let expected = source.replace(&rejected, "");
    let draw = |source: &str| {
        ZplParser::new()
            .parse(source.as_bytes())
            .unwrap()
            .iter()
            .map(|label| {
                let mut bytes = Vec::new();
                Renderer::new()
                    .draw_label_as_png(label, &mut bytes, DrawerOptions::default())
                    .unwrap();
                bytes
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(draw(&source), draw(&expected));
    assert!(
        render_bx("N,1,200,0,12,6,_,2", "_d999").is_err(),
        "invalid escapes must not be hidden as capacity failures"
    );
}

#[test]
fn special_codewords_obey_width_shape_and_partial_size_without_fallback() {
    use DatamatrixRatio::{Rectangular, Square};
    for data in [b"_1ABC".as_slice(), b"_5009ABC", b"_2001001001ABC"] {
        let img =
            datamatrix::encode_zpl_with_ratio(data, 1, 12, 36, b'_', Some(Rectangular)).unwrap();
        assert_eq!(img.dimensions(), (36, 12));
        // ^BX renderer must send ratio and width into the function-codeword path too.
        let field = render_bx("N,2,200,36,12,6,_,2", std::str::from_utf8(data).unwrap()).unwrap();
        assert_eq!(ink_bounds(&field), (10, 20, 72, 24));
        assert!(datamatrix::encode_zpl_with_ratio(data, 1, 12, 36, b'_', Some(Square)).is_err());
        let short =
            datamatrix::encode_zpl_with_ratio(data, 1, 12, 0, b'_', Some(Rectangular)).unwrap();
        assert_eq!(short.dimensions(), (26, 12));
    }
    let long = b"_1aB!cD?eF#gH%iJ&kL";
    assert!(datamatrix::encode_zpl_with_ratio(long, 1, 12, 0, b'_', Some(Rectangular)).is_err());
    assert_eq!(
        datamatrix::encode_zpl_with_ratio(long, 1, 12, 36, b'_', Some(Rectangular))
            .unwrap()
            .dimensions(),
        (36, 12)
    );
    let omitted = render_bx("N,1,200,0,12,6,_,2", std::str::from_utf8(long).unwrap()).unwrap();
    assert!(omitted.pixels().all(|p| p[3] == 0 || p[0] >= 128));
    assert!(datamatrix::encode_zpl_with_ratio(b"_1ABC", 1, 10, 10, b'_', Some(Square)).is_err());
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
