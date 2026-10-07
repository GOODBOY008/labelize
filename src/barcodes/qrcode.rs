use image::{Rgba, RgbaImage};
use qrcode::types::EcLevel;
use qrcode::{bits::encode_auto, canvas::Canvas, ec::construct_codewords};

use crate::elements::barcode_qr::QrErrorCorrectionLevel;

/// Generate a QR code image using a proper QR code encoder.
pub fn encode(
    content: &str,
    magnification: i32,
    ec_level: QrErrorCorrectionLevel,
) -> Result<RgbaImage, String> {
    if content.is_empty() {
        return Err("QR code: empty content".to_string());
    }

    let mag = magnification.max(1) as u32;

    let ec = match ec_level {
        QrErrorCorrectionLevel::L => EcLevel::L,
        QrErrorCorrectionLevel::M => EcLevel::M,
        QrErrorCorrectionLevel::Q => EcLevel::Q,
        QrErrorCorrectionLevel::H => EcLevel::H,
    };

    let bits = encode_auto(content.as_bytes(), ec)
        .map_err(|e| format!("QR code encoding failed: {}", e))?;
    let version = bits.version();
    let side = version.width() as u32;
    let (data, ecc) = construct_codewords(&bits.into_bytes(), version, ec)
        .map_err(|e| format!("QR code encoding failed: {e}"))?;
    let mut canvas = Canvas::new(version, ec);
    canvas.draw_all_functional_patterns();
    canvas.draw_data(&data, &ecc);
    let modules = super::qr_mask::select(&canvas, side as usize);

    // Render to image with quiet zone — ZPL ^BQ includes a 4-module quiet zone
    let quiet_zone = 4u32;
    let img_side = side * mag + 2 * quiet_zone * mag;
    let mut img = RgbaImage::from_pixel(img_side, img_side, Rgba([0, 0, 0, 0]));

    let black = Rgba([0, 0, 0, 255]);
    for (idx, &color) in modules.iter().enumerate() {
        let row = idx as u32 / side;
        let col = idx as u32 % side;
        if color == qrcode::types::Color::Dark {
            let px = (col + quiet_zone) * mag;
            let py = (row + quiet_zone) * mag;
            for dy in 0..mag {
                for dx in 0..mag {
                    if px + dx < img_side && py + dy < img_side {
                        img.put_pixel(px + dx, py + dy, black);
                    }
                }
            }
        }
    }

    Ok(img)
}
