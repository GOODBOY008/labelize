//! Check the asymmetric finder layout independently of a scanner's auto-rotation.
use image::GrayImage;
use labelize::{render_png_with_output, OutputMode, OutputOptions, RenderOptions};
use rxing::{helpers::detect_in_luma_slice, BarcodeFormat};

const MODULE_SIZE: u32 = 4;
const SIDE: u32 = 21; // version 1 for this short payload
const PAYLOAD: &str = "SO-00123";

fn render(source: &str, dpmm: i32, mode: OutputMode) -> GrayImage {
    let output = render_png_with_output(
        source.as_bytes(),
        RenderOptions {
            width_mm: Some(40.0),
            height_mm: Some(40.0),
            dpmm,
            ..Default::default()
        },
        OutputOptions {
            mode,
            ..Default::default()
        },
    )
    .unwrap();
    image::load_from_memory(&output.bytes).unwrap().into_luma8()
}

fn check_symbol(image: &GrayImage, origin: u32, missing_corner: usize) {
    // Read the whole four-module margin and the complete 7x7 finder patterns.
    let border = 4 * MODULE_SIZE;
    let end = origin + SIDE * MODULE_SIZE;
    for y in origin - border..end + border {
        for x in origin - border..end + border {
            if !(origin..end).contains(&x) || !(origin..end).contains(&y) {
                assert_eq!(image.get_pixel(x, y)[0], 255, "quiet zone at ({x},{y})");
            }
        }
    }
    let far = end - 7 * MODULE_SIZE;
    // Corner indices run clockwise from the top left.
    for (index, (left, top)) in [(origin, origin), (far, origin), (far, far), (origin, far)]
        .into_iter()
        .enumerate()
    {
        let is_finder = (0..7).all(|y| {
            (0..7).all(|x| {
                let black = x == 0
                    || y == 0
                    || x == 6
                    || y == 6
                    || ((2..=4).contains(&x) && (2..=4).contains(&y));
                (image.get_pixel(
                    left + x * MODULE_SIZE + MODULE_SIZE / 2,
                    top + y * MODULE_SIZE + MODULE_SIZE / 2,
                )[0] == 0)
                    == black
            })
        });
        assert_eq!(
            is_finder,
            index != missing_corner,
            "finder at corner {index}"
        );
    }
    let decoded = detect_in_luma_slice(
        image.as_raw(),
        image.width(),
        image.height(),
        Some(BarcodeFormat::QR_CODE),
    )
    .unwrap();
    assert_eq!(decoded.getText(), PAYLOAD);
}

#[test]
fn qr_finders_follow_tspl_rotation_and_cpcl_vertical_orientation() {
    for mode in [
        OutputMode::Raster,
        #[cfg(feature = "vector")]
        OutputMode::Vector,
    ] {
        for dpmm in [8, 12] {
            for (degrees, missing) in [(0, 2), (90, 3), (180, 0), (270, 1)] {
                let source = format!("SIZE 40 mm,40 mm\nDIRECTION 1\nCLS\nQRCODE 60,60,M,4,A,{degrees},M2,S7,\"{PAYLOAD}\"\nPRINT 1,1");
                check_symbol(&render(&source, dpmm, mode), 60, missing);
            }
            let dpi = if dpmm == 8 { 200 } else { 300 };
            for (command, missing) in [("BARCODE", 2), ("VBARCODE", 1)] {
                let source = format!("! 0 {dpi} {dpi} 320 1\r\n{command} QR 60 60 M 2 U 4\r\nMA,{PAYLOAD}\r\nENDQR\r\nPRINT\r\n");
                check_symbol(&render(&source, dpmm, mode), 60, missing);
            }
        }
    }
}

#[test]
fn zpl_qr_stays_upright_even_with_field_rotation() {
    // Zebra's ^BQ supports only normal orientation and is unaffected by ^FW.
    for anchor in ["^FO60,60", "^FT60,180"] {
        let reference = format!("^XA{anchor}^BQN,2,4^FDMA,{PAYLOAD}^FS^XZ");
        let rotated = format!("^XA^FWR{anchor}^BQR,2,4^FDMA,{PAYLOAD}^FS^XZ");
        assert_eq!(
            render(&rotated, 8, OutputMode::Raster),
            render(&reference, 8, OutputMode::Raster)
        );
    }
}
