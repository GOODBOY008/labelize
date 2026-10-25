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
    let data = data.trim();
    if data.is_empty() {
        return Ok((String::new(), QrErrorCorrectionLevel::M));
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
    let manual = matches!(bytes.get(idx), Some(b'M') | Some(b'm'));
    idx += 1; // consume the mode letter

    let rest = data
        .get(idx..)
        .ok_or_else(|| "invalid CPCL QR data format".to_string())?
        .trim_start_matches(',');

    if !manual {
        return Ok((rest.to_string(), level));
    }

    let mut payload = String::new();
    for segment in rest.split(',') {
        if segment.is_empty() {
            continue;
        }
        match segment.as_bytes()[0] {
            b'B' | b'b' => {
                // Binary: four-digit decimal byte count, then the bytes.
                let count = segment
                    .get(1..5)
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                let content = segment.get(5..).unwrap_or("");
                let end = count.min(content.len());
                payload.push_str(content.get(..end).unwrap_or(content));
            }
            _ => payload.push_str(segment.get(1..).unwrap_or(segment)),
        }
    }
    Ok((payload, level))
}
