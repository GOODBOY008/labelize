//! Reverse rendering round-trips: image → ZPL/TSPL → forward render → pixels.
//!
//! These tests verify the reverse pipeline end to end: a raster is encoded
//! into printer code, that code is parsed and rendered through the ordinary
//! forward pipeline, and the result must match the binarized source image
//! pixel-for-pixel. No external reference renderer is involved — the source
//! image itself is the reference, which is what makes reverse rendering
//! verifiable offline: if the generated `^GFA`/`BITMAP` payload, polarity,
//! row padding, geometry or `SIZE` were wrong by even one dot, the round
//! trip would not be lossless.

mod common;

use common::render_helpers;
use common::reverse_check::assert_round_trip;
use image::RgbaImage;
use labelize::{
    image_to_tspl, image_to_zpl, tspl_from_image_bytes, zpl_from_image_bytes, DrawerOptions,
    ReverseOptions, TsplParser, ZplParser,
};

/// Real label fixtures (1-bit renders, black/white only) round-tripped
/// through both languages. The unit fixture has the other historical canvas
/// size (812×1624), so both known dimensions are exercised.
const ROUND_TRIP_FIXTURES: &[&str] = &[
    "testdata/labels/amazon.png",
    "testdata/labels/auspost.png",
    "testdata/labels/canadapost.png",
    "testdata/unit/text_fallback_default.png",
];

fn load_fixture(path: &str) -> RgbaImage {
    image::open(render_helpers::testdata_dir().join(path_to_testdata(path)))
        .unwrap_or_else(|e| panic!("failed to open fixture {path}: {e}"))
        .to_rgba8()
}

/// Fixture paths are written relative to `testdata/` so the list above reads
/// like every other test-table in this crate; resolve against the real dir.
fn path_to_testdata(path: &str) -> std::path::PathBuf {
    let stripped = path.strip_prefix("testdata/").unwrap_or(path);
    std::path::PathBuf::from(stripped)
}

fn zpl_round_trip_png(source: &RgbaImage, options: &ReverseOptions) -> Vec<u8> {
    let zpl = image_to_zpl(source, options).expect("ZPL encode failed");
    let labels = ZplParser::new()
        .parse(zpl.as_bytes())
        .expect("generated ZPL must parse");
    assert_eq!(labels.len(), 1, "generated ZPL must produce one label");
    // Generated ZPL leaves the canvas size unset: render at the image's physical
    // size, which is the documented round-trip contract.
    let drawer = DrawerOptions {
        label_width_mm: source.width() as f64 / options.dpmm as f64,
        label_height_mm: source.height() as f64 / options.dpmm as f64,
        dpmm: options.dpmm,
        ..Default::default()
    };
    render_helpers::render_label_to_png(&labels[0], drawer)
}

fn tspl_round_trip_png(source: &RgbaImage, options: &ReverseOptions) -> Vec<u8> {
    let tspl = image_to_tspl(source, options).expect("TSPL encode failed");
    let labels = TsplParser::new()
        .parse(&tspl)
        .expect("generated TSPL must parse");
    assert_eq!(labels.len(), 1, "generated TSPL must produce one label");
    // The generated SIZE declares the canvas; use the label's own dims like
    // every TSPL consumer does.
    let label = &labels[0];
    let drawer = DrawerOptions {
        label_width_mm: label.label_width_mm.expect("SIZE must declare width"),
        label_height_mm: label.label_height_mm.expect("SIZE must declare height"),
        dpmm: options.dpmm,
        ..Default::default()
    };
    render_helpers::render_label_to_png(label, drawer)
}

#[test]
fn zpl_round_trip_is_pixel_identical_on_real_labels() {
    let options = ReverseOptions::default();
    for fixture in ROUND_TRIP_FIXTURES {
        let source = load_fixture(fixture);
        assert_round_trip(&source, &zpl_round_trip_png(&source, &options), fixture);
    }
}

#[test]
fn tspl_round_trip_is_pixel_identical_on_real_labels() {
    let options = ReverseOptions::default();
    for fixture in ROUND_TRIP_FIXTURES {
        let source = load_fixture(fixture);
        assert_round_trip(&source, &tspl_round_trip_png(&source, &options), fixture);
    }
}

#[test]
fn current_field_block_render_survives_image_to_code() {
    use labelize::{encode_image_bytes, render_png, Language, RenderOptions, ReverseLanguage};

    // Render with the current engine so incoming changes to long-word wrapping,
    // final-line overprinting and transparent text buffers enter this pipeline.
    // The fixed Labelary input is covered separately by the reverse goldens.
    let fixture = include_str!("../testdata/unit/fb_single_line_overflow.zpl");
    for orientation in ['N', 'R', 'I', 'B'] {
        for origin in ["FO", "FT"] {
            let zpl = fixture.replace(
                "^FO30,30^A0,60,0",
                &format!("^{origin}300,400^A0{orientation},32,32"),
            );
            for antialias in [false, true] {
                let canvas = RenderOptions {
                    language: Some(Language::Zpl),
                    width_mm: Some(101.625),
                    height_mm: Some(203.25),
                    antialias,
                    ..Default::default()
                };
                let input = render_png(zpl.as_bytes(), canvas.clone()).unwrap();
                let source = image::load_from_memory(&input.bytes).unwrap().to_rgba8();
                assert!(source.pixels().any(|p| p[0] < 128), "nonempty header");
                for (target, language) in [
                    (ReverseLanguage::Zpl, Language::Zpl),
                    (ReverseLanguage::Tspl, Language::Tspl),
                ] {
                    let code = encode_image_bytes(&input.bytes, &ReverseOptions::default(), target)
                        .expect("encode current field-block rendering");
                    let output = render_png(
                        &code,
                        RenderOptions {
                            language: Some(language),
                            // Exercise SIZE for TSPL and the captured canvas for ZPL.
                            width_mm: (target == ReverseLanguage::Zpl).then_some(101.625),
                            height_mm: (target == ReverseLanguage::Zpl).then_some(203.25),
                            antialias: false,
                            ..canvas.clone()
                        },
                    )
                    .unwrap();
                    assert_round_trip(
                        &source,
                        &output.bytes,
                        &format!("field-block {origin}/{orientation}/aa={antialias}/{target:?}"),
                    );
                }
            }
        }
    }
}

#[test]
fn threshold_and_alpha_survive_the_round_trip() {
    // One row carrying every binarization boundary: pure black/white, the
    // strict-threshold pair (127/128), a color (red luma 76 → ink, light
    // green luma 150 → paper), fully transparent black (→ paper) and
    // half-transparent black (luma 127 → ink).
    let mut source = RgbaImage::new(8, 1);
    let cases: [[u8; 4]; 8] = [
        [0, 0, 0, 255],
        [255, 255, 255, 255],
        [127, 127, 127, 255],
        [128, 128, 128, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 0, 0],
        [0, 0, 0, 128],
    ];
    for (x, case) in cases.iter().enumerate() {
        source.put_pixel(x as u32, 0, image::Rgba(*case));
    }
    let options = ReverseOptions::default();
    assert_round_trip(
        &source,
        &zpl_round_trip_png(&source, &options),
        "gray/alpha ZPL",
    );
    assert_round_trip(
        &source,
        &tspl_round_trip_png(&source, &options),
        "gray/alpha TSPL",
    );
}

#[test]
fn odd_width_row_padding_renders_as_paper() {
    // 13 columns force every row to be padded to 2 bytes with 5 non-content
    // bits. The ZPL canvas is deliberately wider than the image so the pad
    // region is visible: it must stay paper in both polarities (ZPL pads
    // with 0-bits, TSPL's complement pads with 1-bits — both = paper).
    let source = RgbaImage::from_pixel(13, 5, image::Rgba([0, 0, 0, 255]));
    let options = ReverseOptions::default();

    let zpl = image_to_zpl(&source, &options).unwrap();
    let labels = ZplParser::new().parse(zpl.as_bytes()).unwrap();
    let wide = DrawerOptions {
        label_width_mm: 3.0, // 24 dots — 11 columns wider than the image
        label_height_mm: 0.625,
        dpmm: 8,
        ..Default::default()
    };
    let png = render_helpers::render_label_to_png(&labels[0], wide);
    let rendered = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(rendered.dimensions(), (24, 5));
    for y in 0..5 {
        for x in 0..13 {
            assert_eq!(
                rendered.get_pixel(x, y),
                &image::Rgba([0, 0, 0, 255]),
                "content column {x} must be ink"
            );
        }
        for x in 13..24 {
            assert_eq!(
                rendered.get_pixel(x, y),
                &image::Rgba([255, 255, 255, 255]),
                "pad column {x} must be paper in both polarities"
            );
        }
    }

    // TSPL canvas comes from SIZE (13 dots wide), so the round trip is
    // checked against the image itself.
    assert_round_trip(
        &source,
        &tspl_round_trip_png(&source, &options),
        "odd-width TSPL",
    );
}

#[test]
fn non_default_dpmm_round_trip_is_pixel_identical() {
    // 60×48 dots at 12 dpmm = exactly 5×4 mm, so both the explicit ZPL
    // canvas and the TSPL SIZE reproduce the dimensions without rounding.
    let mut source = RgbaImage::from_pixel(60, 48, image::Rgba([255, 255, 255, 255]));
    for y in 0..48 {
        for x in 0..60 {
            // Deterministic ink pattern rather than a flat field.
            if (x / 3 + y / 4) % 2 == 0 {
                source.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
    }
    let options = ReverseOptions {
        dpmm: 12,
        ..ReverseOptions::default()
    };
    assert_round_trip(
        &source,
        &zpl_round_trip_png(&source, &options),
        "12dpmm ZPL",
    );
    assert_round_trip(
        &source,
        &tspl_round_trip_png(&source, &options),
        "12dpmm TSPL",
    );
}

#[test]
fn round_trip_from_encoded_image_bytes() {
    // The *_from_image_bytes helpers are the SDK entry points; exercise them
    // through a real PNG in memory.
    let mut source = RgbaImage::from_pixel(40, 24, image::Rgba([255, 255, 255, 255]));
    for x in 5..35 {
        source.put_pixel(x, 6, image::Rgba([0, 0, 0, 255]));
        source.put_pixel(x, 17, image::Rgba([0, 0, 0, 255]));
    }
    for y in 6..18 {
        source.put_pixel(5, y, image::Rgba([0, 0, 0, 255]));
        source.put_pixel(34, y, image::Rgba([0, 0, 0, 255]));
    }

    let mut png = Vec::new();
    labelize::encode_png(&source, &mut png).expect("PNG encode failed");

    let options = ReverseOptions::default();
    let zpl = zpl_from_image_bytes(&png, &options).expect("ZPL from PNG bytes failed");
    let labels = ZplParser::new().parse(zpl.as_bytes()).unwrap();
    let drawer = DrawerOptions {
        label_width_mm: 5.0,
        label_height_mm: 3.0,
        ..Default::default()
    };
    let rendered = render_helpers::render_label_to_png(&labels[0], drawer);
    assert_round_trip(&source, &rendered, "PNG-bytes ZPL");

    let tspl = tspl_from_image_bytes(&png, &options).expect("TSPL from PNG bytes failed");
    let labels = TsplParser::new().parse(&tspl).unwrap();
    let label = &labels[0];
    let rendered = render_helpers::render_label_to_png(
        label,
        DrawerOptions {
            label_width_mm: label.label_width_mm.unwrap(),
            label_height_mm: label.label_height_mm.unwrap(),
            ..Default::default()
        },
    );
    assert_round_trip(&source, &rendered, "PNG-bytes TSPL");
}

#[test]
fn tspl_binary_control_bytes_survive_the_round_trip() {
    // These bytes must remain payload even though they resemble commands,
    // delimiters, line breaks and end-of-file markers.
    let payload = b"\r\n,\0\xFFPRINT 1,1\r\nSIZE 10 mm,10 mm\r\n^XA^XZ";
    let mut source = RgbaImage::new(8, payload.len() as u32);
    for (y, byte) in payload.iter().enumerate() {
        for x in 0..8 {
            let value = if byte & (0x80 >> x) == 0 { 0 } else { 255 };
            source.put_pixel(x, y as u32, image::Rgba([value, value, value, 255]));
        }
    }
    let options = ReverseOptions::default();
    let code = image_to_tspl(&source, &options).unwrap();
    assert!(code.windows(payload.len()).any(|w| w == payload));
    assert_round_trip(
        &source,
        &tspl_round_trip_png(&source, &options),
        "binary TSPL",
    );
}

#[test]
fn custom_thresholds_and_fractional_dot_sizes_round_trip() {
    use common::reverse_check::assert_round_trip_at_threshold;
    let source = RgbaImage::from_fn(13, 7, |x, y| {
        let gray = ((x * 19 + y * 41) % 256) as u8;
        image::Rgba([gray, gray, gray, 255])
    });
    for dpmm in [6, 7, 12, 24, 24000] {
        for threshold in [0, 1, 128, 255] {
            let options = ReverseOptions { dpmm, threshold };
            assert_round_trip_at_threshold(
                &source,
                &tspl_round_trip_png(&source, &options),
                threshold,
                "TSPL precise SIZE",
            );
            assert_round_trip_at_threshold(
                &source,
                &zpl_round_trip_png(&source, &options),
                threshold,
                "ZPL threshold",
            );
        }
    }
}

#[test]
fn encoded_jpeg_honors_exif_orientation() {
    let source = image::RgbImage::from_fn(24, 16, |x, y| {
        let gray = if x < 8 && y < 8 { 0 } else { 255 };
        image::Rgb([gray; 3])
    });
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
        .encode_image(&source)
        .unwrap();
    // EXIF little-endian IFD: orientation (0x0112), SHORT, one value, 6
    // (90 degrees clockwise). Insert its APP1 segment after JPEG's SOI.
    let exif = b"Exif\0\0II\x2A\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut oriented = jpeg[..2].to_vec();
    oriented.extend_from_slice(&[0xFF, 0xE1]);
    oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    oriented.extend_from_slice(exif);
    oriented.extend_from_slice(&jpeg[2..]);
    let decoded = image::load_from_memory(&jpeg).unwrap().to_rgba8();
    let expected = image::imageops::rotate90(&decoded);
    assert_eq!(expected.dimensions(), (16, 24));
    for language in [
        labelize::ReverseLanguage::Zpl,
        labelize::ReverseLanguage::Tspl,
    ] {
        let code =
            labelize::encode_image_bytes(&oriented, &ReverseOptions::default(), language).unwrap();
        let output = labelize::render_png(
            &code,
            labelize::RenderOptions {
                language: Some(if language == labelize::ReverseLanguage::Zpl {
                    labelize::Language::Zpl
                } else {
                    labelize::Language::Tspl
                }),
                width_mm: Some(2.0),
                height_mm: Some(3.0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_round_trip(&expected, &output.bytes, "EXIF JPEG");
    }
}
