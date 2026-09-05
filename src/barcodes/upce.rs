use super::bit_matrix::BitMatrix;
use image::{Rgba, RgbaImage};

// UPC-E encoding. The symbol is 51 modules: start guard 101, six 7-module digits
// (L or G parity selected by number system + check digit), end guard 010101.
// The check digit is NOT encoded as a digit -- it selects the digit parity, so the
// decoder derives it from the parity pattern.

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

static G_PATTERNS: [[u8; 7]; 10] = [
    [0, 1, 0, 0, 1, 1, 1],
    [0, 1, 1, 0, 0, 1, 1],
    [0, 0, 1, 1, 0, 1, 1],
    [0, 1, 0, 0, 0, 0, 1],
    [0, 0, 1, 1, 1, 0, 1],
    [0, 1, 1, 1, 0, 0, 1],
    [0, 0, 0, 0, 1, 0, 1],
    [0, 0, 1, 0, 0, 0, 1],
    [0, 0, 0, 1, 0, 0, 1],
    [0, 0, 1, 0, 1, 1, 1],
];

/// Digit parity as a function of number system digit and check digit: bit set = G
/// (even) parity, clear = L (odd). Table from the UPC-E standard (ZXing
/// NUMSYS_AND_CHECK_DIGIT_PATTERNS), verified against Labelary renders for all
/// 20 (number system, check digit) combinations.
static PARITY_PATTERNS: [[u8; 6]; 10] = [
    [1, 1, 1, 0, 0, 0], // C0
    [1, 1, 0, 1, 0, 0], // C1
    [1, 1, 0, 0, 1, 0], // C2
    [1, 1, 0, 0, 0, 1], // C3
    [1, 0, 1, 1, 0, 0], // C4
    [1, 0, 0, 1, 1, 0], // C5
    [1, 0, 0, 0, 1, 1], // C6
    [1, 0, 1, 0, 1, 0], // C7
    [1, 0, 1, 0, 0, 1], // C8
    [1, 0, 0, 1, 0, 1], // C9
];

#[derive(Clone, Debug)]
pub struct UpceSymbol {
    pub image: RgbaImage,
    /// Number system digit (0 or 1) carried by the parity pattern.
    pub number_system: u8,
    /// The six encoded digits.
    pub digits: [u8; 6],
    /// Check digit (derived from the parity pattern for display).
    pub check_digit: u8,
    /// Height of the data bars (the image includes the guard extension below).
    pub data_height: u32,
}

/// Check digit of an 11- or 12-digit UPC-A string ("NS M P" or "NS M P C").
/// Weights alternate x3 starting from the LAST digit, per the EAN/UPC standard
/// (verified against Labelary: e.g. UPC-E 100007 -> check 8).
fn upca_check_digit(digits: &[u8]) -> u8 {
    let mut sum = 0u32;
    let n = digits.len();
    for (i, &d) in digits.iter().enumerate() {
        // (n - i) odd -> weight 3; counting from the end, the last digit is x3.
        if (n - i).is_multiple_of(2) {
            sum += d as u32;
        } else {
            sum += d as u32 * 3;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

/// Zero-compress a 12-digit UPC-A ("NS M M M M M P P P P P C") to its six-digit
/// UPC-E form plus number system, using the standard suppression rules.
fn compress(upca: &[u8; 12]) -> (u8, [u8; 6]) {
    let ns = upca[0];
    let m = &upca[1..6];
    let p = &upca[6..11];
    let e = if m[2..5] == [0, 0, 0] || m[2..5] == [1, 0, 0] || m[2..5] == [2, 0, 0] {
        // Manufacturer ends 000/100/200: e = M1 M2 M3 P3 P4 P5
        [m[0], m[1], m[2], p[2], p[3], p[4]]
    } else if m[3..5] == [0, 0] {
        // Manufacturer ends 00 (not 000/100/200): NS + M1M2M3 + P4P5 + 3
        [m[0], m[1], m[2], p[3], p[4], 3]
    } else if m[4] == 0 {
        // Manufacturer ends 0 (not 00): NS + M1M2M3M4 + P5 + 4
        [m[0], m[1], m[2], m[3], p[4], 4]
    } else {
        // No zeros: NS + M1..M5 + P5
        [m[0], m[1], m[2], m[3], m[4], p[4]]
    };
    (ns, e)
}

/// Expand six UPC-E digits back to an 11-digit UPC-A ("NS M P") for check digit
/// computation. (Inverse of the suppression rules; only used internally.)
fn expand(ns: u8, e: &[u8; 6]) -> [u8; 11] {
    let mut d = [0u8; 11];
    d[0] = ns;
    match e[5] {
        0..=2 => {
            d[1] = e[0];
            d[2] = e[1];
            d[3] = e[5];
            d[6] = e[2];
            d[7] = e[3];
            d[8] = e[4];
        }
        3 => {
            d[1] = e[0];
            d[2] = e[1];
            d[3] = e[2];
            d[7] = e[3];
            d[8] = e[4];
        }
        4 => {
            d[1] = e[0];
            d[2] = e[1];
            d[3] = e[2];
            d[4] = e[3];
            d[9] = e[4];
        }
        _ => {
            d[1] = e[0];
            d[2] = e[1];
            d[3] = e[2];
            d[4] = e[3];
            d[5] = e[4];
            d[10] = e[5];
        }
    }
    d
}

pub fn encode(content: &str, height: i32, bar_width: i32) -> Result<UpceSymbol, String> {
    let digits: Vec<u8> = content
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    let (ns, code): (u8, [u8; 6]) = match digits.len() {
        6 => (0, digits.as_slice().try_into().expect("6 digits")),
        7 => (digits[0], digits[1..].try_into().expect("6 digits")),
        // 11-digit input: UPC-A without check digit (NS + M + P), zero-compressed.
        // 12-digit input: full UPC-A (NS + M + P + C); the given check digit is
        // recomputed like ^BE does, per the repo's EAN-13 convention.
        11 | 12 => {
            let mut upca = [0u8; 12];
            upca[..11].copy_from_slice(&digits[..11]);
            compress(&upca)
        }
        n => return Err(format!("UPC-E: expected 6, 7, 11 or 12 digits, got {}", n)),
    };

    // Check digit from the expanded UPC-A (weights x3 from the last digit).
    let expanded = expand(ns, &code);
    let check = upca_check_digit(&expanded);
    let parity = &PARITY_PATTERNS[check as usize];

    // 51 modules: 3 start guard + 6x7 digits + 6 end guard (010101).
    let module_count = 51usize;
    let mut bm = BitMatrix::new(module_count, 1);
    let mut pos = 0;

    // Start guard 101
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1; // space
    bm.set(pos, 0, true);
    pos += 1;

    for i in 0..6 {
        let digit = code[i] as usize;
        let pattern = if parity[i] == 1 {
            &G_PATTERNS[digit]
        } else {
            &L_PATTERNS[digit]
        };
        for &bit in pattern {
            if bit == 1 {
                bm.set(pos, 0, true);
            }
            pos += 1;
        }
    }

    // End guard 010101
    pos += 1;
    bm.set(pos, 0, true);
    pos += 1;
    pos += 1;
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

    // Guard modules: 0..2 (start) and 45..50 (end).
    let is_guard_module = |m: usize| m <= 2 || m >= 45;
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

    Ok(UpceSymbol {
        image: img,
        number_system: ns,
        digits: code,
        check_digit: check,
        data_height: h as u32,
    })
}
