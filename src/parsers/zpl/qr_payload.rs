//! ^FD payload conventions for `^BQ` QR fields.
//!
//! ZPL embeds the error-correction level (and an optional character-mode
//! descriptor) in the field data: `QA,HELLO`, `HM,N123`, `QM,B0012…`. This
//! module owns that syntax; the QR element carries only the raw payload plus
//! the resolved level.

use crate::elements::barcode_qr::{QrCharacterMode, QrErrorCorrectionLevel};

/// Split a `^BQ` field's data into the raw QR payload and error-correction
/// level (plus the character mode, which only governs how the payload is
/// read here — the encoder re-selects automatically).
///
/// Only the two format bytes (error-correction char + mode char) are
/// required; a prefix-only field like `QA,` parses to an empty payload,
/// which the renderer skips like Labelary instead of failing the label.
/// Binary mode (`QM,B…`) prefixes the raw bytes with a four-digit decimal
/// length; an overlong declared length truncates to the available bytes.
pub fn parse_qr_fd_payload(
    field: &str,
) -> Result<(String, QrErrorCorrectionLevel, QrCharacterMode), String> {
    // An entirely empty ^FD is a no-field skip decided by the renderer, not a
    // payload error.
    if field.is_empty() {
        return Ok((String::new(), QrErrorCorrectionLevel::H, QrCharacterMode::Automatic));
    }
    if field.len() < 2 {
        return Err("invalid qr barcode data".to_string());
    }

    let bytes = field.as_bytes();
    let mut data = field
        .get(3..)
        .ok_or_else(|| "invalid qr barcode data prefix".to_string())?;
    let mut mode = QrCharacterMode::Automatic;
    let level = match bytes[0] {
        b'H' => QrErrorCorrectionLevel::H,
        b'Q' => QrErrorCorrectionLevel::Q,
        b'M' => QrErrorCorrectionLevel::M,
        b'L' => QrErrorCorrectionLevel::L,
        // Unrecognized format indicator: Labelary defaults to H error correction
        _ => QrErrorCorrectionLevel::H,
    };

    if bytes[1] == b'M' && !data.is_empty() {
        mode = match data.as_bytes()[0] {
            b'B' => QrCharacterMode::Binary,
            b'N' => QrCharacterMode::Numeric,
            b'A' => QrCharacterMode::Alphanumeric,
            b'K' => QrCharacterMode::Kanji,
            _ => QrCharacterMode::Automatic,
        };
        data = data
            .get(1..)
            .ok_or_else(|| "invalid qr barcode character mode".to_string())?;
    }

    if mode != QrCharacterMode::Binary {
        // Zebra printers treat `|` as a field separator in QR data; it is stripped
        // from the encoded content (not included in the QR symbol), matching Labelary behavior.
        let content = data.replace('|', "");
        return Ok((content, level, mode));
    }

    if data.len() < 5 {
        return Err("invalid qr barcode byte mode data".to_string());
    }

    let data_len: usize = data
        .get(..4)
        .ok_or_else(|| "invalid qr barcode byte mode data length".to_string())?
        .parse()
        .map_err(|_| "invalid qr barcode byte mode data length".to_string())?;

    let data = &data[4..];
    let data_len = data_len.min(data.len());

    let content = data
        .get(..data_len)
        .ok_or_else(|| "qr barcode byte mode length splits a UTF-8 character".to_string())?;
    Ok((content.to_string(), level, mode))
}
