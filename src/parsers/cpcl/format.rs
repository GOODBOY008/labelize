//! CPCL source formatter: normalize commands one per line without
//! touching counted binary payloads.
//!
//! EG/CG graphics data is byte-counted (not line-delimited) and the 2-D
//! barcode blocks run until their END tags — the formatter walks the same
//! boundaries the parser does so the two can never disagree about where
//! raw data ends. Comments (`;` lines) are preserved verbatim.

use super::parser::{CpclCommand, CpclLines};

/// Format CPCL source as raw bytes (EG/CG payloads are not UTF-8). Never
/// fails — malformed lines are passed through unchanged rather than
/// rejected.
pub fn format_cpcl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut lines = CpclLines::new(data);

    loop {
        let command = match lines.next_command() {
            Ok(Some(command)) => command,
            Ok(None) => break,
            Err(_) => return data.to_vec(),
        };
        match command {
            CpclCommand::Graphic {
                header, data, hex, ..
            } => {
                out.extend_from_slice(header.trim_ascii_start());
                if hex {
                    const DIGITS: &[u8] = b"0123456789ABCDEF";
                    for byte in data {
                        out.push(DIGITS[(byte >> 4) as usize]);
                        out.push(DIGITS[(byte & 15) as usize]);
                    }
                } else {
                    out.extend_from_slice(&data);
                }
                out.push(b'\n');
            }
            CpclCommand::Line(line) => {
                let trimmed = line.trim_ascii_start();
                if trimmed.is_empty() {
                    continue;
                }
                out.extend_from_slice(trimmed);
                out.push(b'\n');
                let text = String::from_utf8_lossy(trimmed);
                let mut words = text.split_whitespace();
                let name = words.next().unwrap_or("").to_ascii_uppercase();
                let end_tags: &[&str] = match name.as_str() {
                    "ML" | "MULTILINE" => &["ENDML", "ENDMULTILINE"],
                    "B" | "BARCODE" | "VB" | "VBARCODE" => {
                        match words.next().unwrap_or("").to_ascii_uppercase().as_str() {
                            "QR" => &["ENDQR"],
                            "PDF-417" | "PDF417" => &["ENDPDF"],
                            "AZTEC" => &["ENDAZTEC"],
                            "MAXICODE" => &["ENDMAXICODE"],
                            _ => &[],
                        }
                    }
                    _ => &[],
                };
                if !end_tags.is_empty() {
                    while let Some(raw) = lines.next_line_bytes() {
                        out.extend_from_slice(raw);
                        out.push(b'\n');
                        if end_tags
                            .iter()
                            .any(|tag| raw.trim_ascii().eq_ignore_ascii_case(tag.as_bytes()))
                        {
                            break;
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_whitespace_and_blank_runs() {
        let formatted = format_cpcl(
            b"! 0 200 200 210 1\r\n\r\nTEXT 4 0 30 40 Hello World\r\nFORM\r\nPRINT\r\n",
        );
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "! 0 200 200 210 1\nTEXT 4 0 30 40 Hello World\nFORM\nPRINT\n"
        );
    }

    #[test]
    fn comments_are_preserved() {
        let formatted = format_cpcl(b"; Center justify\r\nTEXT 5 1 0 5 A COMMENT\r\n");
        assert_eq!(
            String::from_utf8_lossy(&formatted),
            "; Center justify\nTEXT 5 1 0 5 A COMMENT\n"
        );
    }

    #[test]
    fn eg_hex_payload_is_byte_exact_across_lines() {
        let src = b"! 0 200 200 210 1\nEG 2 16 90 45 F0F0F0F0F0F0F0F00F0F0F0F0F0F0F0F\nF0F0F0F0F0F0F0F00F0F0F0F0F0F0F0F\nFORM\nPRINT\n";
        let formatted = format_cpcl(src);
        // The 64 hex digits re-emit as one space-delimited payload chunk.
        assert!(formatted.starts_with(
            &b"! 0 200 200 210 1\nEG 2 16 90 45 F0F0F0F0F0F0F0F00F0F0F0F0F0F0F0FF0F0F0F0F0F0F0F00F0F0F0F0F0F0F0F\nFORM\nPRINT\n"[..]
        ));
    }

    #[test]
    fn formatted_output_parses_equivalently() {
        use crate::parsers::cpcl::parser::CpclParser;
        let src = b"! 0 200 200 210 1\nCENTER\nTEXT 4 0 0 40 Hello\nB 128 1 1 50 30 120 1234567\nFORM\nPRINT\n";
        let before = CpclParser::with_dpmm(8).parse(src).unwrap();
        let after = CpclParser::with_dpmm(8).parse(&format_cpcl(src)).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].elements.len(), after[0].elements.len());
    }
}
