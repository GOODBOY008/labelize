//! CPCL source formatter: normalize commands one per line without
//! touching counted binary payloads.
//!
//! EG/CG graphics data is byte-counted (not line-delimited) and the 2-D
//! barcode blocks run until their END tags — the formatter walks the same
//! boundaries the parser does so the two can never disagree about where
//! raw data ends. Comments (`;` lines) are preserved verbatim.

use super::parser::CpclLines;

/// Format CPCL source as raw bytes (EG/CG payloads are not UTF-8). Never
/// fails — malformed lines are passed through unchanged rather than
/// rejected.
pub fn format_cpcl(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() + 8);
    let mut lines = CpclLines::new(data);

    while let Some(line) = lines.next_line() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(';') {
            out.extend_from_slice(trimmed.as_bytes());
            out.push(b'\n');
            continue;
        }

        let (cmd_word, rest) = split_first(trimmed);
        let cmd = cmd_word.to_ascii_uppercase();
        match cmd.as_str() {
            // Counted graphics payloads: header plus the declared raw
            // bytes, verbatim (hex digits for EG, raw bytes for CG). The
            // payload may already start on the command line; the rest runs
            // across following lines exactly like the parser reads it.
            "EG" | "CG" => {
                let mut remainder = rest.trim_start();
                let mut params: [&str; 4] = [""; 4];
                let mut ok = true;
                for slot in params.iter_mut() {
                    let ws = remainder.len() - remainder.trim_start().len();
                    let after = &remainder[ws..];
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    *slot = &after[..end];
                    if slot.is_empty() || slot.parse::<f64>().is_err() {
                        ok = false;
                        break;
                    }
                    remainder = &after[end..];
                }
                if ok {
                    let row_bytes = params[0].parse::<usize>().unwrap_or(1).max(1);
                    let height = params[1].parse::<usize>().unwrap_or(1).max(1);
                    let total = row_bytes * height;
                    let need = if cmd == "EG" { total * 2 } else { total };
                    let mut payload: Vec<u8> = remainder
                        .strip_prefix(' ')
                        .map(|s| s.as_bytes().to_vec())
                        .unwrap_or_default();
                    payload = finish_counted(&mut lines, need, cmd == "EG", payload);
                    out.extend_from_slice(cmd_word.as_bytes());
                    for arg in params {
                        out.push(b' ');
                        out.extend_from_slice(arg.as_bytes());
                    }
                    if !payload.is_empty() {
                        out.push(b' ');
                        out.extend_from_slice(&payload);
                    }
                    out.push(b'\n');
                    continue;
                }
            }
            _ => {}
        }

        out.extend_from_slice(trimmed.as_bytes());
        out.push(b'\n');
    }

    out
}

/// Extend `seed` (the command line's own payload prefix) with exactly
/// `need` hex digits (EG) or raw bytes (CG) from the cursor. Hex digits
/// re-emit as parsed; raw payloads stay verbatim. A short payload returns
/// whatever was collected (the header re-emits with a partial payload).
fn finish_counted(lines: &mut CpclLines, need: usize, hex: bool, seed: Vec<u8>) -> Vec<u8> {
    let mut collected = seed;
    if hex {
        collected.retain(|b| b.is_ascii_hexdigit());
        collected.truncate(need);
        while collected.len() < need {
            let Some(line) = lines.next_line() else {
                return collected;
            };
            for b in line.bytes() {
                if b.is_ascii_hexdigit() {
                    collected.push(b);
                    if collected.len() == need {
                        return collected;
                    }
                }
            }
        }
        collected
    } else {
        while collected.len() < need {
            let Some(line) = lines.next_line() else {
                return collected;
            };
            collected.extend_from_slice(line.as_bytes());
        }
        collected.truncate(need);
        collected
    }
}

fn split_first(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(end) => (&line[..end], &line[end..]),
        None => (line, ""),
    }
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
