use image::{Rgba, RgbaImage};
use qrcode::bits::Bits;
use qrcode::types::{EcLevel, QrError, Version};
use qrcode::QrCode;

use crate::elements::barcode_qr::{QrCharacterMode, QrErrorCorrectionLevel};

/// Generate a QR code image using a proper QR code encoder.
pub fn encode(
    content: &str,
    magnification: i32,
    ec_level: QrErrorCorrectionLevel,
) -> Result<RgbaImage, String> {
    encode_with_mode(content, magnification, ec_level, QrCharacterMode::Automatic)
}

/// Encode automatic data, or a single explicitly selected Numeric, Alphanumeric,
/// or Byte segment. Kanji requires a separate Shift-JIS input path and is rejected.
pub fn encode_with_mode(
    content: &str,
    magnification: i32,
    ec_level: QrErrorCorrectionLevel,
    mode: QrCharacterMode,
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

    let code = encode_segments(content.as_bytes(), ec, mode)?;

    let modules = code.to_colors();
    let side = code.width() as u32;

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

fn encode_segments(data: &[u8], ec: EcLevel, mode: QrCharacterMode) -> Result<QrCode, String> {
    match mode {
        QrCharacterMode::Automatic => {
            return QrCode::with_error_correction_level(data, ec)
                .map_err(|e| format!("QR code encoding failed: {e}"));
        }
        QrCharacterMode::Numeric if !data.iter().all(u8::is_ascii_digit) => {
            return Err("QR numeric mode requires ASCII digits 0-9".to_string());
        }
        QrCharacterMode::Alphanumeric
            if !data.iter().all(|b| {
                b.is_ascii_digit() || b.is_ascii_uppercase() || b" $%*+-./:".contains(b)
            }) =>
        {
            return Err("QR alphanumeric mode requires 0-9, A-Z, space, or $%*+-./:".to_string());
        }
        QrCharacterMode::Kanji => {
            return Err(
                "QR manual Kanji mode is not supported (requires Shift-JIS data)".to_string(),
            );
        }
        _ => {}
    }

    // Validate before calling Bits: numeric encoding assumes digits, and the
    // alphanumeric encoder silently maps unsupported characters to zero.
    // Try standard QR versions in order while retaining the explicit segment.
    for version in 1..=40 {
        let mut bits = Bits::new(Version::Normal(version));
        let result = match mode {
            QrCharacterMode::Numeric => bits.push_numeric_data(data),
            QrCharacterMode::Alphanumeric => bits.push_alphanumeric_data(data),
            QrCharacterMode::Binary => bits.push_byte_data(data),
            _ => unreachable!("automatic and Kanji modes handled above"),
        }
        .and_then(|()| bits.push_terminator(ec));
        match result {
            Ok(()) => {
                return QrCode::with_bits(bits, ec)
                    .map_err(|e| format!("QR code encoding failed: {e}"))
            }
            Err(QrError::DataTooLong) => continue,
            Err(e) => return Err(format!("QR code encoding failed: {e}")),
        }
    }
    Err("QR code encoding failed: data too long for the requested character mode".to_string())
}
