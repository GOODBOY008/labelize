//! DPL source formatter: normalize records one per line without touching
//! byte-counted payloads.
//!
//! Records split on the field-data terminator (CR by default — the `T`
//! command reassigns it), exactly like the parser's cursor, so the two can
//! never disagree about where binary-safe data (PDF-417 `Z` byte counts,
//! DataMatrix `W1C`) ends. `<STX>` system lines pass through verbatim.

use super::parser::DplCursor;

/// Format DPL source as raw bytes. Never fails — malformed lines pass
/// through unchanged rather than being rejected.
pub fn format_dpl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut cursor = DplCursor::new(data);

    while let Some(line) = cursor.next_record(b'\r') {
        if line.is_empty() {
            continue;
        }
        let trimmed: &[u8] = {
            // Trim trailing whitespace only: a record's data field may in
            // principle carry leading spaces, and the leading bytes are
            // structural (rotation digit / command letter).
            let mut end = line.len();
            while end > 0 && line[end - 1].is_ascii_whitespace() {
                end -= 1;
            }
            &line[..end]
        };
        if trimmed.is_empty() {
            continue;
        }
        out.extend_from_slice(trimmed);
        out.push(b'\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_records_one_per_line() {
        let src = b"\x02L\rD11\r191100801200100ORDER 1001\rQ0001\rE\r";
        let formatted = format_dpl(src);
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "\u{2}L\nD11\n191100801200100ORDER 1001\nQ0001\nE\n"
        );
    }

    #[test]
    fn formatted_output_parses_equivalently() {
        use crate::parsers::dpl::parser::DplParser;
        let src =
            b"\x02L\rD11\r191100801200100ORDER 1001\r1E21040015002401Z999AA10123456784\rQ0001\rE\r";
        let before = DplParser::with_dpmm(8).parse(src).unwrap();
        let after = DplParser::with_dpmm(8).parse(&format_dpl(src)).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].elements.len(), after[0].elements.len());
    }
}
