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

fn at_line_end(data: &[u8]) -> bool {
    data.iter()
        .find(|b| !matches!(b, b' ' | b'\t'))
        .is_none_or(|b| matches!(b, b'\r' | b'\n'))
}

fn trim_horizontal(data: &[u8]) -> &[u8] {
    let indent = data
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t'))
        .count();
    &data[indent..]
}

fn line_word(line: &[u8]) -> &[u8] {
    trim_horizontal(line)
        .split(u8::is_ascii_whitespace)
        .next()
        .unwrap_or_default()
}

fn starts_opaque_record(line: &[u8]) -> bool {
    let line = trim_horizontal(line);
    let word = line_word(line);
    if word.eq_ignore_ascii_case(b"REM") {
        return true;
    }
    // Data commands have a comma-separated header. A plain-text line
    // beginning "PDF417 matters" can still close a legacy multiline field.
    let raw_command = [b"BITMAP".as_slice(), b"AZTEC", b"PDF417", b"MPDF417"]
        .iter()
        .any(|cmd| word.eq_ignore_ascii_case(cmd));
    raw_command
        && line[word.len()..]
            .iter()
            .find(|b| matches!(b, b',' | b'\r' | b'\n' | b'"'))
            == Some(&b',')
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
    // At physical EOL, \" can be either a final backslash and delimiter,
    // or a legacy quote in a multiline value. Save that delimiter while
    // looking for a real closing quote. Never search through binary records
    // or REM contents for evidence that the text continues.
    let mut fallback = None;
    let mut line_start = i;
    while i < data.len() {
        let tail = &data[i..];
        match tail[0] {
            b'"' => {
                if fallback.is_some()
                    && (!at_line_end(&tail[1..])
                        || (data[line_start..i].trim_ascii_end().ends_with(b",")
                            && super::parses_arguments(line_word(&data[line_start..i]))))
                {
                    // An opening quote in the next command's arguments.
                    return fallback;
                }
                return Some(i);
            }
            b'\r' | b'\n' if fallback.is_some() => {
                i += 1 + usize::from(tail.starts_with(b"\r\n"));
                line_start = i;
                if starts_opaque_record(&data[i..]) {
                    return fallback;
                }
                continue;
            }
            b'\\' => {
                if let Some(len) = escape_len(tail) {
                    if len == LEGACY_QUOTE.len() && at_line_end(&tail[len..]) {
                        fallback.get_or_insert(i + 1);
                    }
                    i += len;
                    continue;
                }
            }
            _ => {}
        }
        i += char_len(tail);
    }
    fallback
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
