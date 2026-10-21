//! ZPL source formatter: one command per line, `^XA`/`^XZ` blocks indented.
//!
//! Command boundaries follow the same rules as the parser's command splitter
//! (`^CC`/`^CT` remap included), but unlike the splitter this walk keeps every
//! payload byte verbatim — the splitter drops transport CR/LF/TAB everywhere
//! because rendering ignores them; a formatter must not rewrite data. Only
//! whitespace *between* commands is insignificant and gets dropped.
//!
//! Output is raw bytes, not `String`: `^FD` field data may be any encoding
//! (Latin-1 via `^CI`, hex after `^FH`), and lossy UTF-8 conversion would
//! silently corrupt it.

/// Format ZPL source: each `^`/`~` command on its own line, commands between
/// `^XA` and `^XZ` indented by two spaces, one trailing newline. Never fails
/// — malformed input is reflowed, not rejected.
pub fn format_zpl(data: &[u8]) -> Vec<u8> {
    let mut caret = b'^';
    let mut tilde = b'~';
    let mut depth: i32 = 0;
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 16);
    let mut buf: Vec<u8> = Vec::new();

    fn starts_with_ignoring_case(bytes: &[u8], prefix: &[u8]) -> bool {
        bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    fn flush(buf: &mut Vec<u8>, out: &mut Vec<u8>, depth: &mut i32, after_data: bool) -> bool {
        if buf.is_empty() {
            return after_data;
        }
        // Leading whitespace between commands is insignificant (the parser
        // trims it). Trailing CR/LF is transport noise the parser strips
        // everywhere, so it is dropped too; trailing spaces/tabs may be ^FD
        // payload and are kept byte-for-byte.
        let start = buf
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(buf.len());
        let mut end = buf.len();
        while end > start && matches!(buf[end - 1], b'\n' | b'\r') {
            end -= 1;
        }
        if start >= end {
            buf.clear();
            return after_data;
        }
        let cmd = &buf[start..end];
        let is_xa = starts_with_ignoring_case(cmd, b"^XA");
        let is_xz = starts_with_ignoring_case(cmd, b"^XZ");
        let is_data =
            starts_with_ignoring_case(cmd, b"^FD") || starts_with_ignoring_case(cmd, b"^FV");
        if is_xz {
            *depth = (*depth - 1).max(0);
        }
        // The parser drops the newline but keeps spaces between commands, so
        // indenting a line right after ^FD/^FV would append spaces to the
        // field data itself (empty ^FD^FS pairs gained "  " payloads). The
        // line directly after a data command stays flush-left.
        if is_xa || is_xz || after_data {
            out.extend_from_slice(cmd);
        } else {
            for _ in 0..*depth {
                out.extend_from_slice(b"  ");
            }
            out.extend_from_slice(cmd);
        }
        if is_xa {
            *depth += 1;
        }
        out.push(b'\n');
        buf.clear();
        is_data
    }

    let mut after_data = false;
    for &ch in data {
        // A completed `^CCx`/`^CTx` at exactly four bytes is itself a
        // boundary: the remapped character takes effect for the bytes that
        // follow (mirrors the parser's splitter state machine).
        let mut is_ct = false;
        let mut is_cc = false;
        if buf.len() == 4 {
            is_ct = buf[0] == caret && &buf[1..3] == b"CT";
            is_cc = buf[0] == caret && &buf[1..3] == b"CC";
        }

        if ch == caret || ch == tilde || is_ct || is_cc {
            if is_ct && buf.len() >= 4 {
                tilde = buf[3];
            } else if is_cc && buf.len() >= 4 {
                caret = buf[3];
            }
            after_data = flush(&mut buf, &mut out, &mut depth, after_data);
        }
        buf.push(ch);
    }
    if !buf.is_empty() {
        flush(&mut buf, &mut out, &mut depth, after_data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_command_per_line_with_indent() {
        let formatted = format_zpl(b"^XA^CF0,60^FO50,50^FDHello World^FS^XZ");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "^XA\n  ^CF0,60\n  ^FO50,50\n  ^FDHello World\n^FS\n^XZ\n"
        );
    }

    #[test]
    fn transport_newlines_between_commands_are_absorbed() {
        let formatted = format_zpl(b"^XA\n\n^FO50,50\n^FDHi^FS\n^XZ\r\n");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "^XA\n  ^FO50,50\n  ^FDHi\n^FS\n^XZ\n"
        );
    }

    #[test]
    fn fd_payload_bytes_survive_verbatim() {
        // A trailing space, a tab and non-UTF-8 (Latin-1) bytes inside field
        // data are payload and must not be trimmed, reflowed, or lossy-decoded.
        let formatted = format_zpl(b"^XA^FDtab\t kept \xFF\xFE^FS^XZ");
        assert_eq!(
            formatted,
            b"^XA\n  ^FDtab\t kept \xFF\xFE\n^FS\n^XZ\n".to_vec()
        );
    }

    #[test]
    fn formatted_output_parses_identically() {
        use crate::parsers::zpl::parser::ZplParser;
        let src = b"^XA^LH20,10^FO50,50^A0N,30,30^FDHello^FS^FO50,120^BCN,80,Y,N,N^FD12345^FS^XZ";
        let before = ZplParser::with_dpmm(8).parse(src).unwrap();
        let after = ZplParser::with_dpmm(8).parse(&format_zpl(src)).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].elements.len(), after[0].elements.len());
    }

    #[test]
    fn caret_remap_boundaries_are_honored() {
        // ^CT! remaps the tilde introducer to '!'; the formatter must split
        // commands at '!' afterwards, like the parser does.
        let formatted = format_zpl(b"^XA^CT!^FO10,10^FDx^FS!DG...^XZ");
        let text = String::from_utf8_lossy(&formatted);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.contains(&"  !DG..."), "got: {text}");
    }

    #[test]
    fn multiple_formats_indent_per_block() {
        let formatted = format_zpl(b"^XA^FDa^FS^XZ^XA^FDb^FS^XZ");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "^XA\n  ^FDa\n^FS\n^XZ\n^XA\n  ^FDb\n^FS\n^XZ\n"
        );
    }
}
