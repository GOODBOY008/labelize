//! TSPL source formatter: normalize commands one per line without touching
//! binary payloads.
//!
//! The boundary walk reuses the parser's command splitter helpers
//! ([`first_token_end`], [`bitmap_payload_start`], [`bitmap_hex_payload`],
//! [`pdf417_raw_layout`], [`raw_content_end`], [`command_line_end`]) so
//! formatter and parser can never disagree about where `BITMAP`/`PDF417
//! L<len>` raw data ends — reflowing those blocks with a line-based pass
//! would corrupt the payload.

use super::parser::{
    bitmap_hex_payload, bitmap_payload_start, command_args, command_line_end, first_token_end,
    parse_i32_arg, pdf417_raw_layout, raw_content_end,
};

/// Format TSPL source as raw bytes (BITMAP payloads are not UTF-8). Never
/// fails — malformed lines are passed through unchanged rather than rejected.
pub fn format_tspl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut idx = 0usize;

    while idx < data.len() {
        while idx < data.len() && matches!(data[idx], b'\r' | b'\n') {
            idx += 1;
        }
        if idx >= data.len() {
            break;
        }

        let line_start = idx;
        let token_end = first_token_end(data, line_start);
        let token = String::from_utf8_lossy(&data[line_start..token_end]).to_uppercase();

        // BITMAP: header line plus `row_bytes * height` raw bytes (or the
        // hex-text form's digit run), verbatim.
        if token == "BITMAP" {
            if let Some(payload_start) = bitmap_payload_start(data, line_start) {
                let header = String::from_utf8_lossy(&data[line_start..payload_start]).to_string();
                let args = command_args(&header, "BITMAP");
                if args.len() >= 4 {
                    let row_bytes = parse_i32_arg(&args[2]).unwrap_or(1).max(1) as usize;
                    let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1) as usize;
                    let payload_end =
                        match bitmap_hex_payload(data, payload_start, row_bytes * height) {
                            Some((hex_end, _)) => {
                                // Keep the digit run verbatim; drop trailing
                                // separator spaces the scan absorbed.
                                let mut end = hex_end;
                                while end > payload_start && matches!(data[end - 1], b' ' | b'\t') {
                                    end -= 1;
                                }
                                end
                            }
                            None => payload_start
                                .saturating_add(row_bytes * height)
                                .min(data.len()),
                        };
                    out.extend_from_slice(&data[line_start..payload_end]);
                    out.push(b'\n');
                    idx = payload_end;
                    continue;
                }
            }
        }

        // PDF417 with an `L<len>` raw expression: header plus exactly <len>
        // content bytes (with the splitter's structural-stop rule), verbatim.
        if matches!(token.as_str(), "PDF417" | "MPDF417") {
            if let Some((len, _header_end, content_start)) = pdf417_raw_layout(data, line_start) {
                let content_end = raw_content_end(
                    data,
                    content_start,
                    len.min(data.len().saturating_sub(content_start)),
                );
                out.extend_from_slice(&data[line_start..content_end]);
                out.push(b'\n');
                idx = content_end;
                continue;
            }
        }

        // Plain command line (may span physical lines inside quotes).
        let line_end = command_line_end(data, line_start);
        let line = trim_tspl_line(&data[line_start..line_end]);
        if !line.is_empty() {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
        idx = line_end;
    }

    out
}

fn trim_tspl_line(raw: &[u8]) -> &[u8] {
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
    fn normalizes_whitespace_and_blank_runs() {
        let formatted = format_tspl(b"SIZE 101.6 mm,152.4 mm\r\n\r\n\r\nCLS\r\nTEXT 40,40,\"3\",0,1,1,\"HI\"\r\nPRINT 1\r\n");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "SIZE 101.6 mm,152.4 mm\nCLS\nTEXT 40,40,\"3\",0,1,1,\"HI\"\nPRINT 1\n"
        );
    }

    #[test]
    fn bitmap_payload_is_byte_exact() {
        // 1 byte per row, 2 rows: payload deliberately contains \n, spaces
        // and non-UTF-8 bytes — none of them may be touched or re-decoded.
        let payload: [u8; 2] = [0x80, b'\n'];
        let mut src = b"SIZE 60 mm,30 mm\nCLS\nBITMAP 10,10,1,2,0,".to_vec();
        src.extend_from_slice(&payload);
        src.extend_from_slice(b"\nPRINT 1\n");

        let formatted = format_tspl(&src);
        let expected_prefix = b"SIZE 60 mm,30 mm\nCLS\nBITMAP 10,10,1,2,0,";
        assert!(formatted.starts_with(expected_prefix));
        let after_header = &formatted[expected_prefix.len()..];
        assert_eq!(&after_header[..2], &payload, "payload preserved byte-exact");
        assert!(formatted.ends_with(b"\nPRINT 1\n"));
    }

    #[test]
    fn bitmap_hex_payload_is_verbatim() {
        // Hex-text form (issue #108): the digit run is preserved exactly,
        // including the manual-table space separators, and PRINT survives.
        let formatted = format_tspl(
            b"SIZE 60 mm,30 mm\r\nCLS\r\nBITMAP 10,10,2,2,0,07 FF 1F 8F\r\nPRINT 1\r\n",
        );
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "SIZE 60 mm,30 mm\nCLS\nBITMAP 10,10,2,2,0,07 FF 1F 8F\nPRINT 1\n"
        );
    }

    #[test]
    fn formatted_output_parses_equivalently() {
        use crate::parsers::tspl::parser::TsplParser;
        let src = b"SIZE 101.6 mm,152.4 mm\nCLS\nTEXT 40,40,\"3\",0,1,1,\"HI\"\nBARCODE 40,120,\"128\",90,1,0,2,3,\"12345\"\nPRINT 1\n";
        let before = TsplParser::with_dpmm(8).parse(src).unwrap();
        let after = TsplParser::with_dpmm(8).parse(&format_tspl(src)).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].elements.len(), after[0].elements.len());
    }
}
