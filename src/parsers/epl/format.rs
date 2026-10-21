//! EPL source formatter: normalize line-based commands without touching
//! binary payloads.
//!
//! EPL is line-oriented, so formatting is normalization: trim whitespace
//! around text commands, drop blank lines, LF line endings, one trailing
//! newline. `GW` blocks carry raw binary (which may itself contain newlines
//! and whitespace bytes) and are copied through byte-for-byte — the boundary
//! walk reuses the parser's [`gw_span`] so the two can never disagree about
//! where a graphic's data ends.

use super::parser::gw_span;

/// Format EPL source as raw bytes (GW payloads are not UTF-8). Never fails —
/// malformed lines are passed through unchanged rather than rejected.
pub fn format_epl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut cursor = 0usize;

    while cursor < data.len() {
        let line_end = data[cursor..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(data.len(), |p| cursor + p);

        // A complete GW block is emitted verbatim (header + binary payload)
        // and the cursor jumps past it, mirroring the parser's byte walk.
        // Malformed GW lines fall through to text normalization.
        if let Ok(Some(span)) = gw_span(data, cursor, line_end) {
            out.extend_from_slice(&data[cursor..span.data_end]);
            out.push(b'\n');
            cursor = span.data_end;
            continue;
        }

        let raw = &data[cursor..line_end];
        // Same trim the parser applies to text lines (\r suffix, then all
        // ASCII whitespace); the parser's parse() ignores it, printers do not
        // see it in quoted data because it is outside the quotes.
        let text = trim_epl_line(raw);
        if !text.is_empty() {
            out.extend_from_slice(text);
            out.push(b'\n');
        }

        cursor = if line_end < data.len() {
            line_end + 1
        } else {
            data.len()
        };
    }

    out
}

fn trim_epl_line(raw: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = raw.len();
    while start < end && raw[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && raw[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &raw[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_normalizes_lines() {
        let formatted = format_epl(b"N\r\nq812\r\n\r\n\r\nA50,50,0,1,1,1,N,\"Hi\"   \r\nP1\r\n");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "N\nq812\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n"
        );
    }

    #[test]
    fn quoted_data_keeps_inner_spaces() {
        let formatted = format_epl(b"A10,10,0,2,1,1,N,\"  spaced  \"\nP1\n");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "A10,10,0,2,1,1,N,\"  spaced  \"\nP1\n"
        );
    }

    #[test]
    fn gw_binary_payload_is_byte_exact() {
        // 2x2-byte graphic whose data deliberately contains \n, spaces and
        // non-UTF-8 bytes: nothing inside the payload may be touched.
        let payload: [u8; 4] = [0xA5, b'\n', b' ', 0x00];
        let mut src = b"N\nGW10,10,2,2,".to_vec();
        src.extend_from_slice(&payload);
        src.extend_from_slice(b"\nP1\n");

        let formatted = format_epl(&src);
        let expected_prefix = b"N\nGW10,10,2,2,";
        assert!(formatted.starts_with(expected_prefix));
        let after_header = &formatted[expected_prefix.len()..];
        assert_eq!(&after_header[..4], &payload, "payload preserved byte-exact");
        assert!(formatted.ends_with(b"P1\n"));
    }

    #[test]
    fn formatted_output_parses_equivalently() {
        use crate::parsers::epl::parser::EplParser;
        let src = b"N\nq812\nA50,50,0,3,1,1,N,\"HELLO\"\nB60,120,0,3,2,4,80,B,\"ABC\"\nP1\n";
        let before = EplParser::new().parse(src).unwrap();
        let after = EplParser::new().parse(&format_epl(src)).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].elements.len(), after[0].elements.len());
    }
}
