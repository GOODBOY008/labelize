//! Quoted TSPL arguments share a grammar across the parser and formatter.
//! Counted payloads bypass these helpers.

const TSC_QUOTE: &[u8] = br#"\["]"#;
const LEGACY_QUOTE: &[u8] = br#"\""#;

fn escape_len(data: &[u8]) -> Option<usize> {
    if data.starts_with(TSC_QUOTE) {
        Some(TSC_QUOTE.len())
    } else if data.starts_with(LEGACY_QUOTE) {
        Some(LEGACY_QUOTE.len())
    } else {
        None
    }
}

/// Decode only source escapes. A quote produced by one escape is never
/// rescanned, so the preceding literal backslashes survive exactly.
pub(super) fn decode(mut data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    while let Some((&first, rest)) = data.split_first() {
        if first == b'\\'
            && let Some(len) = escape_len(data)
        {
            result.push(b'"');
            data = &data[len..];
        } else {
            result.push(first);
            data = rest;
        }
    }
    result
}

/// After a legacy backslash-quote at physical EOL, an actual TSPL command
/// (or EOF) wins over a multiline continuation. The documented bracketed
/// quote always remains data, including before a command-like content line.
fn closes_line(after_quote: &[u8]) -> bool {
    let indent = after_quote
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t'))
        .count();
    let after_quote = &after_quote[indent..];
    if !matches!(after_quote.first(), Some(b'\r' | b'\n')) {
        return after_quote.is_empty();
    }
    let next = after_quote.trim_ascii_start();
    let word = next
        .split(u8::is_ascii_whitespace)
        .next()
        .unwrap_or_default();
    word.is_empty() || super::is_command_word(word)
}

pub(super) fn closing_quote(data: &[u8], start: usize) -> Option<usize> {
    closing_quote_with(data, start, |_| 1)
}

/// The callback advances over one encoded character, including any DBCS trail
/// byte that would otherwise look like an ASCII backslash.
pub(super) fn closing_quote_with(
    data: &[u8],
    mut i: usize,
    char_len: impl Fn(&[u8]) -> usize,
) -> Option<usize> {
    while i < data.len() {
        let tail = &data[i..];
        match tail[0] {
            b'"' => return Some(i),
            b'\\' => {
                if let Some(len) = escape_len(tail) {
                    if len == LEGACY_QUOTE.len() && closes_line(&tail[len..]) {
                        return Some(i + 1);
                    }
                    i += len;
                    continue;
                }
            }
            _ => {}
        }
        i += char_len(tail);
    }
    None
}

pub(super) fn command_line_end_with(
    data: &[u8],
    i: usize,
    char_len: impl Fn(&[u8]) -> usize + Copy,
) -> usize {
    separator(data, i, |b| matches!(b, b'\r' | b'\n'), char_len)
}

pub(super) fn argument_end(data: &[u8], start: usize) -> usize {
    separator(data, start, |b| b == b',', |_| 1)
}

fn separator(
    data: &[u8],
    mut i: usize,
    is_separator: impl Fn(u8) -> bool,
    char_len: impl Fn(&[u8]) -> usize + Copy,
) -> usize {
    while i < data.len() {
        let tail = &data[i..];
        match tail[0] {
            c if is_separator(c) => break,
            b'"' => i = closing_quote_with(data, i + 1, char_len).map_or(data.len(), |end| end + 1),
            b'\\' => i += escape_len(tail).unwrap_or(1),
            _ => i += char_len(tail),
        }
    }
    i
}
