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
