use super::stream::Syntax;

/// The field byte count describes bytes on the wire. A prefix, line break or
/// delimiter inside the counted payload is data, including the last byte.
pub(super) fn payload(bytes: &[u8], syntax: Syntax) -> Result<Option<(usize, usize)>, String> {
    if bytes.first() == Some(&syntax.control)
        && bytes
            .get(1..3)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"DY"))
    {
        let mut ends = bytes
            .iter()
            .skip(3)
            .take_while(|&&b| b != syntax.format && b != syntax.control)
            .enumerate()
            .filter(|(_, b)| **b == syntax.delimiter)
            .map(|(i, _)| i + 3);
        if let (Some(a), Some(b), Some(c), Some(d), Some(last)) = (
            ends.next(),
            ends.next(),
            ends.next(),
            ends.next(),
            ends.next(),
        ) {
            if bytes[a + 1..b].eq_ignore_ascii_case(b"C") {
                // This header gives the decompressed size, so it cannot frame
                // the wire data without decoding the AR stream.
                return Err("Unsupported AR-compressed object".into());
            }
            if bytes[a + 1..b].eq_ignore_ascii_case(b"B") {
                let n = std::str::from_utf8(&bytes[c + 1..d])
                    .ok()
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .filter(|&n| n <= super::objects::MAX_BYTES)
                    .ok_or("Invalid binary object byte count")?;
                return Ok(Some((last + 1, n)));
            }
        }
        return Ok(None);
    }
    if bytes.first() != Some(&syntax.format)
        || !bytes
            .get(1..4)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"GFB") || s.eq_ignore_ascii_case(b"GFC"))
        || bytes.get(4) != Some(&syntax.delimiter)
    {
        return Ok(None);
    }
    let mut fields = bytes[4..]
        .iter()
        .take_while(|&&b| b != syntax.format && b != syntax.control)
        .enumerate()
        .filter(|(_, b)| **b == syntax.delimiter)
        .map(|(i, _)| i + 4);
    let Some(first) = fields.next() else {
        return Err("Binary graphic delimiter conflicts with the command prefix".into());
    };
    let start = first + 1;
    let Some(stop) = fields.next() else {
        return Ok(None);
    };
    let Some(last) = fields.nth(1) else {
        return Ok(None);
    };
    let count = std::str::from_utf8(&bytes[start..stop])
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|v| (1..=99999).contains(v))
        .ok_or("Binary ^GF byte count must be between 1 and 99999")?;
    Ok(Some((last + 1, count)))
}
