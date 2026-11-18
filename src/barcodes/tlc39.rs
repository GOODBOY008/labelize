use super::{
    code39, pdf417_encoding,
    symbols::{checked_image, micro, raster},
};
use crate::elements::barcode_symbol::BarcodeSymbol;
use anyd::{
    codes::pdf417::ec,
    output::{BitMatrix, Encoding},
};
use image::{Rgba, RgbaImage};

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

pub(super) fn encode(
    spec: &BarcodeSymbol,
    data: &[u8],
    micro_width: u32,
    micro_height: u32,
) -> Result<RgbaImage, String> {
    let eci = data
        .get(..6)
        .filter(|d| d.iter().all(u8::is_ascii_digit))
        .ok_or("TLC39 requires a six-digit ECI number")?;
    let primary = code39::encode(
        std::str::from_utf8(eci).unwrap(),
        spec.height as i32,
        spec.ratio,
        spec.module as i32,
    )?;
    if data.get(6) != Some(&b',') {
        return Ok(primary);
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
    let secondary = raster(
        Encoding::Matrix(secondary(&payload, estimate)?),
        micro_width,
        micro_height,
        None,
        None,
        true,
    )?;
    let extension = (f64::from(spec.height) * (0.214 - (spec.ratio - 2.0) * 0.0165)) as u32;
    let top = extension.max(secondary.height() + spec.module);
    let gap = 10 * spec.module;
    let wide = (f64::from(spec.module) * spec.ratio).round() as u32;
    let flag = code39::get_pattern('T').unwrap();
    let flag_width = 6 * spec.module + 3 * wide;
    let width = (primary.width() + gap + flag_width).max(spec.module + secondary.width());
    let mut image = checked_image(width, top + spec.height + extension)?;
    image::imageops::overlay(&mut image, &primary, 0, i64::from(top));
    image::imageops::overlay(
        &mut image,
        &secondary,
        i64::from(spec.module),
        i64::from(top - secondary.height() - spec.module),
    );
    let mut x = primary.width() + gap;
    for (i, bar) in flag.iter().enumerate() {
        let w = if *bar == 0 { spec.module } else { wide };
        if i % 2 == 0 {
            for y in top - extension..image.height() {
                for xx in x..x + w {
                    image.put_pixel(xx, y, Rgba([0, 0, 0, 255]))
                }
            }
        }
        x += w;
    }
    Ok(image)
}
