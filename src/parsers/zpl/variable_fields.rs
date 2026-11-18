use crate::elements::field_info::FieldSequence;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Each valid Unicode grapheme occupies one position. In binary data each
/// undecodable byte is an opaque position and is copied verbatim.
fn grapheme_boundaries(data: &[u8]) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let mut offset = 0;
    for chunk in data.utf8_chunks() {
        boundaries.extend(
            chunk
                .valid()
                .grapheme_indices(true)
                .map(|(i, _)| offset + i),
        );
        offset += chunk.valid().len();
        boundaries.extend(offset..offset + chunk.invalid().len());
        offset += chunk.invalid().len();
    }
    boundaries.push(data.len());
    boundaries
}

pub(super) fn serialize(value: &[u8], rule: &FieldSequence, index: usize) -> Vec<u8> {
    match rule {
        FieldSequence::Numeric { increment, pad } => {
            let Some(last) = value.iter().rposition(u8::is_ascii_digit) else {
                return value.to_vec();
            };
            let end = last + 1;
            let width = value[..end]
                .iter()
                .rev()
                .take(12)
                .take_while(|b| b.is_ascii_digit())
                .count();
            let start = end - width;
            let number = value[start..end]
                .iter()
                .fold(0i128, |n, &b| n * 10 + i128::from(b - b'0'));
            let next =
                (number + i128::from(*increment) * index as i128).rem_euclid(1_000_000_000_000);
            let width = if *pad { width } else { 0 };
            let mut result = value[..start].to_vec();
            result.extend(format!("{next:0width$}").bytes());
            result.extend_from_slice(&value[end..]);
            result
        }
        FieldSequence::Masked { mask, increment } => {
            let mut result = value.to_vec();
            if index == 0 {
                return result;
            }
            let mut masks = mask.chars().rev();
            let mut increments = increment.chars().rev();
            let mut carry = u64::from(increment.is_empty()) * index as u64;
            for span in grapheme_boundaries(value).windows(2).rev() {
                let cluster = &value[span[0]..span[1]];
                let first = std::str::from_utf8(cluster)
                    .ok()
                    .and_then(|s| s.chars().next());
                if matches!(
                    first,
                    Some(
                        '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}',
                    )
                ) {
                    continue;
                }
                let mask = masks.next().unwrap_or('%');
                let add = increments.next();
                let alphabet: &[u8] = match mask {
                    'd' | 'D' => b"0123456789",
                    'o' | 'O' => b"01234567",
                    'h' => b"0123456789abcdef",
                    'H' => b"0123456789ABCDEF",
                    'a' => b"abcdefghijklmnopqrstuvwxyz",
                    'A' => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    'n' => b"0123456789abcdefghijklmnopqrstuvwxyz",
                    'N' => b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    _ => continue,
                };
                let digit = |c: char| {
                    alphabet
                        .iter()
                        .position(|&b| char::from(b).eq_ignore_ascii_case(&c))
                };
                let Some(current) = first.and_then(digit) else {
                    continue;
                };
                let sum =
                    current as u64 + add.and_then(digit).unwrap_or(0) as u64 * index as u64 + carry;
                carry = sum / alphabet.len() as u64;
                result[span[0]] = alphabet[(sum % alphabet.len() as u64) as usize];
            }
            result
        }
    }
}

fn split_marker<'a>(data: &'a [u8], marker: &[u8]) -> Option<(&'a [u8], &'a [u8])> {
    let start = data.windows(marker.len()).position(|w| w == marker)?;
    Some((&data[..start], &data[start + marker.len()..]))
}

pub(super) fn interpolate(data: &[u8], delimiter: char, values: &HashMap<i32, Vec<u8>>) -> Vec<u8> {
    let mut marker = [0; 4];
    let marker = delimiter.encode_utf8(&mut marker).as_bytes();
    let mut result = Vec::with_capacity(data.len());
    let mut rest = data;
    while let Some((before, tail)) = split_marker(rest, marker) {
        result.extend_from_slice(before);
        let Some((expression, after)) = split_marker(tail, marker) else {
            result.extend_from_slice(marker);
            rest = tail;
            break;
        };
        let mut parts = std::str::from_utf8(expression).unwrap_or("").split(',');
        let Some(id) = parts.next().and_then(|p| p.parse::<i32>().ok()) else {
            result.extend_from_slice(marker);
            rest = tail;
            continue;
        };
        let value = values.get(&id).map(Vec::as_slice).unwrap_or_default();
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (None, None, None, None) => result.extend_from_slice(value),
            (Some(dir), Some(pos), Some(len), None) => {
                if let (Ok(pos), Ok(len)) = (pos.parse::<usize>(), len.parse::<usize>()) {
                    let offsets = grapheme_boundaries(value);
                    let count = offsets.len() - 1;
                    if pos > 0 && pos <= count {
                        let (start, end) = if dir.eq_ignore_ascii_case("f") {
                            (pos - 1, (pos - 1).saturating_add(len).min(count))
                        } else if dir.eq_ignore_ascii_case("b") {
                            let end = count + 1 - pos;
                            (end.saturating_sub(len), end)
                        } else {
                            (0, 0)
                        };
                        result.extend_from_slice(&value[offsets[start]..offsets[end]]);
                    }
                }
            }
            _ => {}
        }
        rest = after;
    }
    result.extend_from_slice(rest);
    result
}
