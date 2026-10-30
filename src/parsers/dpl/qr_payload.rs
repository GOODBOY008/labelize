//! QR payload conventions for DPL `W1D` (manual-format) fields.
//!
//! The manual form is `[model,]<EC>[mask]<mode>,<segment>…` — the same
//! segment shape as the CPCL/ZPL conventions, optionally prefixed by a
//! model selector (`1,`/`2,`) and by a structured-append header
//! (`D aatt pp`, currently rejected). This module resolves that syntax; the QR element
//! carries only the raw payload plus the resolved level.

use crate::elements::barcode_qr::QrErrorCorrectionLevel;

pub fn resolve_dpl_qr_payload(data: &str) -> Result<(String, QrErrorCorrectionLevel), String> {
    let mut data = data;
    if data.is_empty() {
        return Ok((String::new(), QrErrorCorrectionLevel::M));
    }

    // The encoder cannot preserve structured-append metadata.
    if data.starts_with('D') {
        return Err("DPL QR structured append is not supported".to_string());
    }
    for model in ["1,", "2,"] {
        if let Some(rest) = data.strip_prefix(model) {
            data = rest;
            break;
        }
    }

    let bytes = data.as_bytes();
    let level = match bytes.first() {
        Some(b'H') => QrErrorCorrectionLevel::H,
        Some(b'Q') => QrErrorCorrectionLevel::Q,
        Some(b'L') => QrErrorCorrectionLevel::L,
        _ => QrErrorCorrectionLevel::M,
    };

    // Optional mask digit between the level and the mode letter.
    let mut idx = 1;
    if bytes.get(idx).is_some_and(|b| b.is_ascii_digit()) {
        idx += 1;
    }
    let mode = bytes.get(idx).copied().unwrap_or(b',');
    if matches!(mode, b'a' | b'm') {
        return Err("DPL QR hex-ASCII input is not supported".to_string());
    }
    let manual = matches!(mode, b'M' | b'm');
    if matches!(mode, b'A' | b'a' | b'M' | b'm') {
        idx += 1;
    }
    let rest = data
        .get(idx..)
        .and_then(|s| s.strip_prefix(','))
        .ok_or("invalid DPL QR data format")?;

    if !manual {
        return Ok((rest.to_string(), level));
    }

    let payload = super::super::qr_segments::decode_manual_segments(rest)?;
    Ok((payload, level))
}
