//! DPL source formatter: normalize records one per line without touching
//! byte-counted payloads.
//!
//! Records split on the field-data terminator (CR by default — the `T`
//! command reassigns it for one field), exactly like the parser's cursor, so the two can
//! never disagree about where binary-safe data (PDF-417 `Z` byte counts,
//! DataMatrix `W1C`) ends. `<STX>` system lines pass through verbatim.

use super::parser::DplCursor;

/// Format DPL source as raw bytes. Never fails — malformed lines pass
/// through unchanged rather than being rejected.
pub fn format_dpl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut cursor = DplCursor::new(data);

    loop {
        let record = match cursor.next_record() {
            Ok(Some(record)) => record,
            Ok(None) => break,
            // Formatting malformed input must never discard or rewrite data.
            Err(_) => return data.to_vec(),
        };
        out.extend_from_slice(record.data);
        match record.terminator {
            Some(b'\r' | b'\n') | None => out.push(b'\n'),
            Some(terminator) => {
                out.push(terminator);
                out.push(b'\n');
            }
        }
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
