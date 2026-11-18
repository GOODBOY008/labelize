use crate::elements::field_info::FieldSequence;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn serialize(value: &str, rule: &FieldSequence, index: usize) -> String {
    match rule {
        FieldSequence::Numeric { increment, pad } => {
            let bytes = value.as_bytes();
            let Some(last) = bytes.iter().rposition(u8::is_ascii_digit) else {
                return value.into();
            };
            let end = last + 1;
            let width = bytes[..end]
                .iter()
                .rev()
                .take(12)
                .take_while(|b| b.is_ascii_digit())
                .count();
            let start = end - width;
            let number: i128 = value[start..end].parse().unwrap_or(0);
            let next =
                (number + i128::from(*increment) * index as i128).rem_euclid(1_000_000_000_000);
            let width = if *pad { width } else { 0 };
            format!("{}{:0width$}{}", &value[..start], next, &value[end..])
        }
        FieldSequence::Masked { mask, increment } => {
            if index == 0 {
                return value.into();
            }
            let mut masks = mask.chars().rev();
            let mut increments = increment.chars().rev();
            let mut carry = u64::from(increment.is_empty()) * index as u64;
            let mut parts = Vec::new();
            for cluster in value.graphemes(true).rev() {
                let c = cluster.chars().next().unwrap();
                if matches!(c, '\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
                {
                    parts.push(cluster.to_string());
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
                    _ => {
                        parts.push(cluster.to_string());
                        continue;
                    }
                };
                let digit = |c: char| {
                    alphabet
                        .iter()
                        .position(|&b| char::from(b).eq_ignore_ascii_case(&c))
                };
                let Some(current) = digit(c) else {
                    parts.push(cluster.to_string());
                    continue;
                };
                let sum =
                    current as u64 + add.and_then(digit).unwrap_or(0) as u64 * index as u64 + carry;
                carry = sum / alphabet.len() as u64;
                let next = alphabet[(sum % alphabet.len() as u64) as usize] as char;
                parts.push(format!("{next}{}", &cluster[c.len_utf8()..]));
            }
            parts.reverse();
            parts.concat()
        }
    }
}

pub(super) fn interpolate(text: &str, delimiter: char, values: &HashMap<i32, String>) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some((before, tail)) = rest.split_once(delimiter) {
        result.push_str(before);
        let Some((expression, after)) = tail.split_once(delimiter) else {
            result.push(delimiter);
            rest = tail;
            break;
        };
        let mut parts = expression.split(',');
        let field = parts.next().and_then(|p| p.parse::<i32>().ok());
        let Some(id) = field else {
            result.push(delimiter);
            rest = tail;
            continue;
        };
        let value = values.get(&id).map(String::as_str).unwrap_or("");
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (None, None, None, None) => result.push_str(value),
            (Some(dir), Some(pos), Some(len), None) => {
                if let (Ok(pos), Ok(len)) = (pos.parse::<usize>(), len.parse::<usize>()) {
                    let chars: Vec<_> = value.graphemes(true).collect();
                    if pos > 0 && pos <= chars.len() {
                        let (start, end) = if dir.eq_ignore_ascii_case("f") {
                            (pos - 1, (pos - 1).saturating_add(len).min(chars.len()))
                        } else if dir.eq_ignore_ascii_case("b") {
                            let end = chars.len() + 1 - pos;
                            (end.saturating_sub(len), end)
                        } else {
                            (0, 0)
                        };
                        result.extend(chars[start..end].iter().copied());
                    }
                }
            }
            _ => {}
        }
        rest = after;
    }
    result.push_str(rest);
    result
}
