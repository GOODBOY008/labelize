//! Verify bytes through the forward renderer, beyond parser element assertions.
use labelize::{format_source, parse_labels, DrawerOptions, Language, Renderer};
use rxing::{common::BitMatrix, datamatrix::decoder::Decoder};
use std::io::Cursor;

fn render(language: Language, source: &[u8], dpmm: i32) -> image::RgbaImage {
    let labels = parse_labels(language, source, dpmm).unwrap();
    let mut png = Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(
            &labels[0],
            &mut png,
            DrawerOptions {
                label_width_mm: 40.0,
                label_height_mm: 40.0,
                dpmm,
                ..Default::default()
            },
        )
        .unwrap();
    image::load_from_memory(png.get_ref()).unwrap().to_rgba8()
}

#[test]
fn cpcl_binary_graphic_pixels_match_every_input_bit_before_and_after_formatting() {
    let payload: Vec<u8> = (0..=255).collect();
    let mut source = b"! 0 203 203 320 1\r\nCG 16 16 0 0 ".to_vec();
    source.extend_from_slice(&payload);
    source.extend_from_slice(b"\r\nPRINT\r\n");
    let (_, formatted) = format_source(&source, Some(Language::Cpcl));
    for bytes in [&source, &formatted] {
        let image = render(Language::Cpcl, bytes, 8);
        for y in 0..16 {
            for x in 0..128 {
                let ink = payload[(y * 16 + x / 8) as usize] & (0x80 >> (x % 8)) != 0;
                assert_eq!(image.get_pixel(x, y)[0] == 0, ink, "pixel ({x}, {y})");
            }
        }
    }
}

#[test]
fn dpl_boxes_keep_independent_side_and_horizontal_thicknesses() {
    let source = b"\x02L\rm\r1X1100000100010B100060001003\rE\r";
    // Metric tenths of a millimetre at 10 dpmm are exactly one pixel.
    let image = render(Language::Dpl, source, 10);
    for y in 0..400 {
        for x in 0..400 {
            let inside = (10..110).contains(&x) && (330..390).contains(&y);
            let edge = !(13..107).contains(&x) || y == 330 || y == 389;
            assert_eq!(
                image.get_pixel(x, y)[0] == 0,
                inside && edge,
                "pixel ({x}, {y})"
            );
        }
    }
}

#[test]
fn dpl_counted_datamatrix_decodes_original_bytes_at_multiple_resolutions() {
    let payload = b"A\0\r\n\x02\xe4\xff  ";
    let mut source = format!(
        "\x02L\r1W1C3300001000100{:04}2000000000",
        payload.len() + 10
    )
    .into_bytes();
    source.extend_from_slice(payload);
    source.extend_from_slice(b"\rE\r");
    let (_, formatted) = format_source(&source, Some(Language::Dpl));
    for dpmm in [8, 12] {
        for bytes in [&source, &formatted] {
            let image = render(Language::Dpl, bytes, dpmm);
            let ink: Vec<_> = image
                .enumerate_pixels()
                .filter(|(_, _, p)| p[0] == 0)
                .collect();
            let x0 = ink.iter().map(|(x, _, _)| *x).min().unwrap();
            let y0 = ink.iter().map(|(_, y, _)| *y).min().unwrap();
            let x1 = ink.iter().map(|(x, _, _)| *x).max().unwrap();
            let y1 = ink.iter().map(|(_, y, _)| *y).max().unwrap();
            let scale = (3.0 * dpmm as f64 / 8.0).round() as u32;
            let mut matrix = BitMatrix::new((x1 - x0 + 1) / scale, (y1 - y0 + 1) / scale).unwrap();
            for y in 0..matrix.getHeight() {
                for x in 0..matrix.getWidth() {
                    matrix.set_bool(
                        x,
                        y,
                        image.get_pixel(x0 + x * scale, y0 + y * scale)[0] == 0,
                    );
                }
            }
            let decoded = Decoder::new()
                .decode(&matrix)
                .expect("independent DataMatrix decode");
            assert_eq!(decoded.getText(), "A\0\r\n\x02äÿ  ");
        }
    }
}

#[test]
fn cpcl_datamatrix_scans_at_both_resolutions_and_resumes_after_terminator() {
    let source = b"! 0 200 200 220 1\r\nPAGE-WIDTH 400\r\nBARCODE DATAMATRIX 10 10 H 3 S 200\r\nABC123\r\nENDDATAMATRIX\r\nLINE 100 100 120 100 2\r\nPRINT\r\n";
    let (_, formatted) = format_source(source, Some(Language::Cpcl));
    for (dpmm, origin, pitch, line_x) in [(8, 10, 3, 100), (12, 15, 5, 150)] {
        for input in [source.as_slice(), &formatted] {
            let image = render(Language::Cpcl, input, dpmm);
            assert_eq!(image.get_pixel(line_x + 1, line_x)[0], 0, "following LINE");
            // The alternating top finder border starts with one black module.
            for (dx, color) in [(0, 0), (pitch - 1, 0), (pitch, 255), (2 * pitch, 0)] {
                assert_eq!(image.get_pixel(origin + dx, origin)[0], color);
            }
            let luma = image::DynamicImage::ImageRgba8(image).into_luma8();
            let (width, height) = luma.dimensions();
            let decoded = rxing::helpers::detect_in_luma(
                luma.into_raw(),
                width,
                height,
                Some(rxing::BarcodeFormat::DATA_MATRIX),
            )
            .expect("scan the rendered label");
            assert_eq!(decoded.getText(), "ABC123");
        }
    }
}

#[test]
fn dpl_download_and_recall_renders_binary_artwork_at_the_bottom_left_anchor() {
    // White/black 1-bit BMP. Its bottom row contains CR LF STX L.
    let source = include_bytes!("../testdata/dpl/downloaded_image.dpl");
    let (language, formatted) = format_source(source, None);
    assert_eq!(language, Language::Dpl);
    let rows = [[0x80u8, 0, 0, 1], [0x0d, 0x0a, 0x02, 0x4c]];
    for (dpmm, x0, row) in [(8, 41, 20), (12, 61, 30)] {
        for bytes in [source.as_slice(), &formatted] {
            let image = render(language, bytes, dpmm);
            let y0 = image.height() - row - 6;
            for (x, y, pixel) in image.enumerate_pixels() {
                let ink = (x0..x0 + 64).contains(&x) && (y0..y0 + 6).contains(&y) && {
                    let bit = (x - x0) / 2;
                    rows[((y - y0) / 3) as usize][(bit / 8) as usize] & (0x80 >> (bit % 8)) != 0
                };
                assert_eq!(pixel[0] == 0, ink, "at {dpmm} dpmm ({x},{y})");
            }
        }
    }
}

#[test]
fn bottom_anchored_artwork_clips_at_the_top_of_the_media() {
    // At 8 dpmm the 39.9 mm row rounds to dot 319 on 40 mm media.
    // The bottom row has its first bit black, the top row its second bit.
    let source = b"\x02IDFLOGO\r80017F\r8001BF\rFFFF\r\x02L\rm\r1Y1100003990000LOGO\rE\r";
    let image = render(Language::Dpl, source, 8);
    let ink: Vec<_> = image
        .enumerate_pixels()
        .filter(|(_, _, p)| p[0] == 0)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert_eq!(ink, [(0, 0)]);
}
