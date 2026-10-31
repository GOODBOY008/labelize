//! Counted QR segments shared by frontends using N/A/B/K payload syntax.

pub(super) fn decode_manual_segments(mut rest: &str) -> Result<String, String> {
    let mut payload = String::new();
    while !rest.is_empty() {
        let mode = rest.as_bytes()[0].to_ascii_uppercase();
        if !matches!(mode, b'N' | b'A' | b'B' | b'K') {
            return Err("invalid QR segment mode".to_string());
        }
        rest = &rest[1..];
        if mode == b'B' {
            let count = rest
                .get(..4)
                .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or("QR binary segment requires a four-digit byte count")?;
            rest = &rest[4..];
            // A comma inside the declared bytes belongs to the payload.
            let data = rest
                .get(..count)
                .ok_or("QR binary segment has an invalid byte count or UTF-8 boundary")?;
            payload.push_str(data);
            rest = &rest[count..];
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            payload.push_str(&rest[..end]);
            rest = &rest[end..];
        }
        if !rest.is_empty() {
            rest = rest
                .strip_prefix(',')
                .ok_or("QR segments must be separated by a comma")?;
        }
    }
    Ok(payload)
}
