use super::bit_matrix::BitMatrix;
use image::{Rgba, RgbaImage};

// EAN-8 (^B8): 67 modules — start guard 101, four L-parity digits, center guard
// 01010, four R-parity digits, end guard 101. All left digits use L parity; the
// check digit is computed over the first seven digits.

static L_PATTERNS: [[u8; 7]; 10] = [
    [0, 0, 0, 1, 1, 0, 1],
    [0, 0, 1, 1, 0, 0, 1],
    [0, 0, 1, 0, 0, 1, 1],
    [0, 1, 1, 1, 1, 0, 1],
    [0, 1, 0, 0, 0, 1, 1],
    [0, 1, 1, 0, 0, 0, 1],
    [0, 1, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 0, 1, 1],
    [0, 1, 1, 0, 1, 1, 1],
    [0, 0, 0, 1, 0, 1, 1],
];

static R_PATTERNS: [[u8; 7]; 10] = [
    [1, 1, 1, 0, 0, 1, 0],
    [1, 1, 0, 0, 1, 1, 0],
    [1, 1, 0, 1, 1, 0, 0],
    [1, 0, 0, 0, 0, 1, 0],
    [1, 0, 1, 1, 1, 0, 0],
    [1, 0, 0, 1, 1, 1, 0],
    [1, 0, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 1, 0, 0],
    [1, 0, 0, 1, 0, 0, 0],
    [1, 1, 1, 0, 1, 0, 0],
];

#[derive(Clone, Debug)]
pub struct Ean8Symbol {
    pub image: RgbaImage,
    /// The eight displayed digits (seven data digits + check).
    pub digits: [u8; 8],
    pub check_digit: u8,
    /// Height of the guard-bar extension below the data bars.
    pub guard_height: u32,
}

/// EAN-8 check digit over the first seven digits: positions 1,3,5,7 x3.
fn calculate_checksum(digits: &[u8; 7]) -> u8 {
    let mut sum = 0u32;
    for (i, &d) in digits.iter().enumerate() {
        if i % 2 == 0 {
            sum += d as u32 * 3;
        } else {
            sum += d as u32;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

pub fn encode(content: &str, height: i32, bar_width: i32) -> Result<Ean8Symbol, String> {
    let digits: Vec<u8> = content
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    let mut d7 = [0u8; 7];
    match digits.len() {
        7 => d7.copy_from_slice(&digits[..7]),
        // 8-digit input: the eighth digit is a check digit that gets recomputed
        // (like the EAN-13 encoder does).
        8 => d7.copy_from_slice(&digits[..7]),
        n => return Err(format!("EAN-8: expected 7 or 8 digits, got {}", n)),
    }
    let check = calculate_checksum(&d7);

    // 67 modules: 3 start + 4x7 left + 5 center + 4x7 right + 3 end.
    let module_count = 67usize;
    let mut bm = BitMatrix::new(module_count, 1);
    let mut pos = 0;

    // Start guard 101
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;
    bm.set(pos, 0, true);
    pos += 1;

    // Left digits (L parity)
    for i in 0..4 {
        let pattern = &L_PATTERNS[d7[i] as usize];
        for &bit in pattern {
            if bit == 1 {
                bm.set(pos, 0, true);
            }
            pos += 1;
        }
    }

    // Center guard 01010
    pos += 1;
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;

    // Right digits (R parity): digits 5..7 + check digit.
    let right_digits = [d7[4], d7[5], d7[6], check];
    for &digit in &right_digits {
        let pattern = &R_PATTERNS[digit as usize];
        for &bit in pattern {
            if bit == 1 {
                bm.set(pos, 0, true);
            }
            pos += 1;
        }
    }

    // End guard 101
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;
    bm.set(pos, 0, true);

    let bw = bar_width.max(1) as usize;
    let h = height.max(1) as usize;
    // Fixed ~12-dot guard extension (Labelary-measured, height-independent).
    let guard_extension = 12usize;
    let total_height = h + guard_extension;
    let iw = module_count * bw;
    let black = Rgba([0, 0, 0, 255]);
    let mut img = RgbaImage::from_pixel(iw as u32, total_height as u32, Rgba([0, 0, 0, 0]));

    let is_guard_module = |m: usize| m <= 2 || (31..=35).contains(&m) || m >= 64;
    for m in 0..module_count {
        if bm.get(m, 0) {
            let bar_h = if is_guard_module(m) { total_height } else { h };
            for b in 0..bw {
                let px = (m * bw + b) as u32;
                for py in 0..bar_h as u32 {
                    if px < img.width() {
                        img.put_pixel(px, py, black);
                    }
                }
            }
        }
    }

    let mut d8 = [0u8; 8];
    d8[..7].copy_from_slice(&d7);
    d8[7] = check;
    Ok(Ean8Symbol {
        image: img,
        digits: d8,
        check_digit: check,
        guard_height: guard_extension as u32,
    })
}
