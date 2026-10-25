//! QR payload conventions for DPL `W1D` (manual-format) fields.
//!
//! The manual form is `[model,]<EC>[mask]<mode>,<segment>…` — the same
//! segment shape as the CPCL/ZPL conventions, optionally prefixed by a
//! model selector (`1,`/`2,`) and by a structured-append header
//! (`D aatt pp`). This module resolves that syntax; the QR element
//! carries only the raw payload plus the resolved level.

use crate::elements::barcode_qr::QrErrorCorrectionLevel;

pub fn resolve_dpl_qr_payload(data: &str) -> Result<(String, QrErrorCorrectionLevel), String> {
    let mut data = data.trim();
    if data.is_empty() {
        return Ok((String::new(), QrErrorCorrectionLevel::M));
    }

    // Optional leading model selector (`1,` or `2,`) and the structured
    // append header (`D aatt pp,`) are accepted and skipped — the encoder
    // emits model-2 symbols and append series are out of scope.
    if let Some(rest) = data.strip_prefix('D') {
        if let Some((_, tail)) = rest.split_once(',') {
            data = tail.trim_start();
        }
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
    let manual = matches!(bytes.get(idx), Some(b'M') | Some(b'm'));
    idx += 1;

    let rest = data
        .get(idx..)
        .ok_or_else(|| "invalid DPL QR data format".to_string())?
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
