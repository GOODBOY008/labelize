//! ZPL source formatter: one command per line, `^XA`/`^XZ` blocks indented.
//!
//! Command boundaries follow the same rules as the parser's command splitter
//! (`^CC`/`^CT` remap included), but unlike the splitter this walk keeps every
//! payload byte verbatim — counted binary graphics bypass text processing in
//! both paths; a formatter must not rewrite their bytes. Only
//! whitespace *between* commands is insignificant and gets dropped.
//!
//! Output is raw bytes, not `String`: `^FD` field data may be any encoding
//! (Latin-1 via `^CI`, hex after `^FH`), and lossy UTF-8 conversion would
//! silently corrupt it.

/// Format ZPL source: each `^`/`~` command on its own line, commands between
/// `^XA` and `^XZ` indented by two spaces, one trailing newline. Never fails
/// — malformed input is reflowed, not rejected.
pub fn format_zpl(data: &[u8]) -> Vec<u8> {
    let mut stream = super::stream::Stream::new(data, Default::default());
    let mut depth = 0usize;
    let mut after_data = false;
    let mut out = Vec::with_capacity(data.len());
    loop {
        let token = match stream.next() {
            Ok(Some(token)) => token,
            Ok(None) => break,
            Err(_) => return data.to_vec(),
        };
        // Formatting whitespace would itself be syntax in these alphabets.
        if [
            stream.syntax.format,
            stream.syntax.control,
            stream.syntax.delimiter,
        ]
        .iter()
        .any(u8::is_ascii_whitespace)
        {
            return data.to_vec();
        }
        // Spaces in data, serial masks and positional field parameters are
        // significant. Indentation before the next command would change them.
        let literal = [b"FD", b"FV", b"SN", b"SF", b"FN", b"FH", b"FC", b"FE"]
            .iter()
            .any(|cmd| token.is_format(cmd));
        let mut raw = token.raw;
        if !token.is_format(b"CC")
            && !token.is_format(b"CD")
            && !token.is_format(b"CT")
            && token.payload.is_none()
        {
            if !literal {
                // Indentation on a new line belongs to the following command.
                // Strip it when formatting an already-formatted stream.
                if let Some(end) = raw.iter().rposition(|b| !matches!(b, b' ' | b'\t')) {
                    if matches!(raw[end], b'\n' | b'\r') {
                        raw = &raw[..=end];
                    }
                }
            }
            while raw.last().is_some_and(|b| matches!(b, b'\n' | b'\r')) {
                raw = &raw[..raw.len() - 1];
            }
        }
        if raw.is_empty() || raw.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let opens = token.is_format(b"XA");
        let closes = token.is_format(b"XZ");
        if closes {
            depth = depth.saturating_sub(1);
        }
        if !opens && !closes && !after_data {
            out.extend(std::iter::repeat_n(b' ', depth * 2));
        }
        out.extend_from_slice(raw);
        out.push(b'\n');
        after_data = literal || token.payload.is_some();
        if opens {
            depth += 1;
        }
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
