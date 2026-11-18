/// Recognize a complete binary graphic header, before any payload bytes.
/// The prefix may have been changed by the stream's control commands.
pub(super) fn payload_length(header: &[u8]) -> Option<Result<usize, String>> {
    if header.len() < 5
        || !(header[1..4].eq_ignore_ascii_case(b"GFB") || header[1..4].eq_ignore_ascii_case(b"GFC"))
        || header.last() != Some(&b',')
        || header.iter().filter(|&&b| b == b',').count() != 4
    {
        return None;
    }
    Some((|| {
        let count = header.split(|&b| b == b',').nth(1).unwrap_or_default();
        let count = std::str::from_utf8(count)
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|n| (1..=99999).contains(n))
            .ok_or("Binary ^GF byte count must be between 1 and 99999")?;
        Ok(count)
    })())
}
