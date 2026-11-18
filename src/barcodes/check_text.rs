//! Human-readable check characters printed alongside the encoded checks.
const BASE: &[u8; 43] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

pub(crate) fn code93_pair(shift: u8, letter: u8) -> Option<u8> {
    match (shift, letter) {
        (0, b'A'..=b'Z') => Some(letter - 64),
        (1, b'A'..=b'E') => Some(letter - 38),
        (1, b'F'..=b'J') => Some(letter - 11),
        (1, b'K'..=b'O') => Some(letter + 16),
        (1, b'P'..=b'T') => Some(letter + 43),
        (1, b'U') => Some(0),
        (1, b'V') => Some(b'@'),
        (1, b'W') => Some(96),
        (1, b'X'..=b'Z') => Some(127),
        (2, b'A'..=b'O') => Some(letter - 32),
        (2, b'Z') => Some(b':'),
        (3, b'A'..=b'Z') => Some(letter + 32),
        _ => None,
    }
}
pub(super) fn code93(data: &[u8]) -> String {
    let mut values = Vec::new();
    for &b in data {
        if let Some(n) = BASE.iter().position(|&c| c == b) {
            values.push(n);
            continue;
        }
        let pair = (0..4).find_map(|shift| {
            (b'A'..=b'Z')
                .find(|&ch| code93_pair(shift, ch) == Some(b))
                .map(|ch| (shift, ch))
        });
        if let Some((shift, ch)) = pair {
            values.extend([43 + shift as usize, 10 + (ch - b'A') as usize]);
        }
    }
    let mut result = String::new();
    for weights in [20, 15] {
        let value = values
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &v)| (1 + i % weights) * v)
            .sum::<usize>()
            % 47;
        values.push(value);
        result.push(if value < 43 {
            BASE[value]
        } else {
            b"&'()"[value - 43]
        } as char);
    }
    result
}
pub(super) fn mod43(data: &[u8]) -> char {
    let sum: usize = data
        .iter()
        .filter_map(|b| BASE.iter().position(|c| c == b))
        .sum();
    BASE[sum % 43] as char
}
pub(super) fn msi(data: &[u8], scheme: u8) -> String {
    let mut digits: Vec<u32> = data.iter().map(|c| u32::from(c - b'0')).collect();
    let count = match scheme {
        b'A' => 0,
        b'C' | b'D' => 2,
        _ => 1,
    };
    let mut result = String::new();
    for i in 0..count {
        let digit = if i == 0 && scheme == b'D' {
            let s: u32 = digits
                .iter()
                .rev()
                .enumerate()
                .map(|(i, v)| v * (2 + i % 6) as u32)
                .sum();
            (11 - s % 11) % 11
        } else {
            let s: u32 = digits
                .iter()
                .rev()
                .enumerate()
                .map(|(i, &n)| {
                    if i % 2 == 0 {
                        n * 2 / 10 + n * 2 % 10
                    } else {
                        n
                    }
                })
                .sum();
            (10 - s % 10) % 10
        };
        digits.push(digit);
        result.push_str(&digit.to_string());
    }
    result
}
pub(super) fn plessey(data: &[u8]) -> String {
    let mut bits: Vec<u8> = data
        .iter()
        .flat_map(|&c| {
            let v = (c as char).to_digit(16).unwrap_or(0) as u8;
            (0..4).map(move |n| (v >> n) & 1)
        })
        .collect();
    let data_bits = bits.len();
    bits.resize(data_bits + 8, 0);
    let poly = [1, 1, 1, 1, 0, 1, 0, 0, 1];
    for i in 0..data_bits {
        if bits[i] != 0 {
            for (j, &p) in poly.iter().enumerate() {
                bits[i + j] ^= p
            }
        }
    }
    bits[data_bits..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| {
            char::from_digit(
                chunk
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| u32::from(b) << i)
                    .sum(),
                16,
            )
            .unwrap()
            .to_ascii_uppercase()
        })
        .collect()
}
