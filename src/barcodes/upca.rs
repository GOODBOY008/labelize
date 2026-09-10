use super::bit_matrix::BitMatrix;
use image::{Rgba, RgbaImage};

// UPC-A (^BU): a 95-module EAN-13-style symbol built from an 11-digit code
// (NS + manufacturer + product). The left half encodes the first six digits with
// L parity, the right half the remaining five plus the computed check digit.
// Calibrated against Labelary: the left half is the input's first six digits
// (number system included) with fixed L parity -- Labelary does not apply the
// EAN-13 first-digit parity table to ^BU.

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
pub struct UpcaSymbol {
    pub image: RgbaImage,
    /// The eleven encoded digits (NS + manufacturer + product).
    pub digits: [u8; 11],
    /// Check digit for the interpretation line.
    pub check_digit: u8,
    /// Height of the guard-bar extension below the data bars.
    pub guard_height: u32,
}

/// Check digit over an 11-digit UPC-A string ("NS M P"): weights alternate x3
/// starting from the last digit, per the EAN/UPC standard (same rule as UPC-E).
/// Verified against Labelary: "01234567890" -> 5, "11234567890" -> 2.
fn upca_check_digit(digits: &[u8]) -> u8 {
    let mut sum = 0u32;
    let n = digits.len();
    for (i, &d) in digits.iter().enumerate() {
        if (n - i).is_multiple_of(2) {
            sum += d as u32;
        } else {
            sum += d as u32 * 3;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

pub fn encode(content: &str, height: i32, bar_width: i32) -> Result<UpcaSymbol, String> {
    let digits: Vec<u8> = content
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    // 11 digits = NS + M5 + P5 (check computed); 12 digits = full UPC-A (the given
    // check digit is recomputed, matching Labelary's ^BU which uses the first 11).
    let body: [u8; 11] = match digits.len() {
        11 | 12 => digits[..11].try_into().expect("11 digits"),
        n => return Err(format!("UPC-A: expected 11 or 12 digits, got {}", n)),
    };

    let check = upca_check_digit(&body);

    // 95 modules: 3 start + 6x7 left + 5 center + 6x7 right + 3 end.
    let module_count = 95usize;
    let mut bm = BitMatrix::new(module_count, 1);
    let mut pos = 0;

    // Start guard 101
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;
    bm.set(pos, 0, true);
    pos += 1;

    // Left digits: the first six input digits, L parity (Labelary behavior).
    for i in 0..6 {
        let pattern = &L_PATTERNS[body[i] as usize];
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

    // Right digits: remaining five digits + check digit, R parity.
    let right_digits = [body[6], body[7], body[8], body[9], body[10], check];
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

    let is_guard_module = |m: usize| m <= 2 || (45..=49).contains(&m) || m >= 92;
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

    Ok(UpcaSymbol {
        image: img,
        digits: body,
        check_digit: check,
        guard_height: guard_extension as u32,
    })
}
