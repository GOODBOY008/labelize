use super::{
    code39, pdf417_encoding,
    symbols::{code39_modules, micro, EncodedBars},
};
use anyd::{
    codes::pdf417::ec,
    output::{BitMatrix, Encoding},
};

/// ISO/IEC 24728 four-column variants, ordered as in the printer's size table.
/// (row count, error-correction count, first symbol cluster).
const SIZES: [(usize, usize, usize); 10] = [
    (6, 12, 0),
    (8, 14, 0),
    (10, 16, 2),
    (12, 18, 0),
    (15, 21, 0),
    (20, 26, 0),
    (26, 32, 0),
    (32, 38, 2),
    (38, 44, 2),
    (44, 50, 0),
];

fn secondary(payload: &[u8], minimum: usize) -> Result<BitMatrix, String> {
    let mut data = pdf417_encoding::encode_high_level(payload);
    if data.first().is_none_or(|v| *v < 900) {
        data.insert(0, 900);
    }
    // Linked-to-a-non-EAN symbol flag, US 2001/0045461 A1.
    data.insert(0, 918);
    let (index, &(rows, ecc, cluster)) = SIZES
        .iter()
        .enumerate()
        .find(|(_, (rows, ecc, _))| rows * 4 - ecc >= minimum.max(data.len()))
        .ok_or("TLC39 secondary payload exceeds MicroPDF417 capacity")?;
    let mut words: Vec<u32> = data.into_iter().map(u32::from).collect();
    words.resize(rows * 4 - ecc, 900);
    words.extend(ec::encode(&words, ecc));
    let Encoding::Matrix(mut matrix) = micro(b"A", Some(23 + index as u8))? else {
        unreachable!()
    };
    let patterns = pdf417_encoding::cluster_patterns();
    for (y, row) in words.as_chunks::<4>().0.iter().enumerate() {
        for (col, &word) in row.iter().enumerate() {
            let start = 10 + col * 17 + if col >= 2 { 10 } else { 0 };
            let pattern = patterns[(cluster + y) % 3][word as usize];
            for i in 0..17 {
                matrix.set(start + i, y, pattern & (1 << (16 - i)) != 0);
            }
        }
    }
    Ok(matrix)
}

pub(super) fn encode(data: &[u8]) -> Result<EncodedBars, String> {
    let eci = data
        .get(..6)
        .filter(|d| d.iter().all(u8::is_ascii_digit))
        .ok_or("TLC39 requires a six-digit ECI number")?;
    let linear = code39_modules(eci)?;
    if data.get(6) != Some(&b',') {
        return Ok(EncodedBars::Single(Encoding::Linear(linear)));
    }
    let parts = &data[7..];
    if parts.is_empty()
        || parts.len() > 150
        || parts
            .split(|b| *b == b',')
            .any(|s| s.is_empty() || s.len() > 25 || !s.iter().all(u8::is_ascii_alphanumeric))
    {
        return Err("Invalid TLC39 serial number or additional data".into());
    }
    let payload: Vec<_> = parts
        .iter()
        .map(|&b| if b == b',' { b'*' } else { b })
        .collect();
    let estimate = if parts.contains(&b',') {
        2 + parts.len() / 6 * 5 + parts.len() % 6
    } else {
        2 + parts.len().div_ceil(2)
    };
    Ok(EncodedBars::Linked39 {
        linear,
        secondary: secondary(&payload, estimate)?,
        flag: code39::get_pattern('T').unwrap(),
    })
}
