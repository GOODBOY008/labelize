//! QR payload conventions for CPCL `B QR` fields.
//!
//! CPCL carries the error-correction level, an optional mask number and
//! the data-input mode in the data lines (`MA,QR Code`, `H0M,N0123…`,
//! `LM,AQRcode,N0123…,B0006qrcode`), mirroring the ZPL `^FD` format. This
//! module resolves that syntax; the QR element carries only the raw
//! payload plus the resolved level.

use crate::elements::barcode_qr::QrErrorCorrectionLevel;

/// Resolve CPCL QR data lines into the raw payload and error-correction
/// level.
///
/// Automatic mode (`<EC>[mask]A,<data>`) passes the data through as-is.
/// Manual mode (`<EC>[mask]M,<segment>…`) concatenates the comma-
/// separated segments; each segment is a character-mode letter (`N`
/// numeric, `A` alphanumeric, `B` binary, `K` kanji) followed by its data,
/// and binary segments carry a four-digit decimal byte count that is
/// stripped (the encoder measures the payload itself). The mode letters
/// only govern how the payload is read — the encoder re-selects
/// compaction automatically.
pub fn resolve_cpcl_qr_payload(data: &str) -> Result<(String, QrErrorCorrectionLevel), String> {
    if data.is_empty() {
        return Ok((String::new(), QrErrorCorrectionLevel::M));
    }

    if data.starts_with('D') {
        return Err("CPCL QR structured append is not supported".to_string());
    }

    let bytes = data.as_bytes();
    let level = match bytes[0] {
        b'H' => QrErrorCorrectionLevel::H,
        b'Q' => QrErrorCorrectionLevel::Q,
        b'M' => QrErrorCorrectionLevel::M,
        b'L' => QrErrorCorrectionLevel::L,
        _ => QrErrorCorrectionLevel::M,
    };

    // Optional mask digit (0-8) between the level and the mode letter.
    let mut idx = 1;
    if bytes.get(idx).is_some_and(|b| b.is_ascii_digit()) {
        idx += 1;
    }
    let mode = bytes.get(idx).copied().unwrap_or(b',');
    let manual = matches!(mode, b'M' | b'm');
    if matches!(mode, b'A' | b'a' | b'M' | b'm') {
        idx += 1;
    }
    let rest = data
        .get(idx..)
        .and_then(|s| s.strip_prefix(','))
        .ok_or("invalid CPCL QR data format")?;

    if !manual {
        return Ok((rest.to_string(), level));
    }

    let payload = super::super::qr_segments::decode_manual_segments(rest)?;
    Ok((payload, level))
}
