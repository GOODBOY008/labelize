use image::RgbaImage;
use super::bit_matrix::BitMatrix;

// EAN-13 encoding patterns
// L-codes (odd parity), G-codes (even parity), R-codes
static L_PATTERNS: [[u8; 7]; 10] = [
    [0,0,0,1,1,0,1],[0,0,1,1,0,0,1],[0,0,1,0,0,1,1],[0,1,1,1,1,0,1],
    [0,1,0,0,0,1,1],[0,1,1,0,0,0,1],[0,1,0,1,1,1,1],[0,1,1,1,0,1,1],
    [0,1,1,0,1,1,1],[0,0,0,1,0,1,1],
];

static G_PATTERNS: [[u8; 7]; 10] = [
    [0,1,0,0,1,1,1],[0,1,1,0,0,1,1],[0,0,1,1,0,1,1],[0,1,0,0,0,0,1],
    [0,0,1,1,1,0,1],[0,1,1,1,0,0,1],[0,0,0,0,1,0,1],[0,0,1,0,0,0,1],
    [0,0,0,1,0,0,1],[0,0,1,0,1,1,1],
];

static R_PATTERNS: [[u8; 7]; 10] = [
    [1,1,1,0,0,1,0],[1,1,0,0,1,1,0],[1,1,0,1,1,0,0],[1,0,0,0,0,1,0],
    [1,0,1,1,1,0,0],[1,0,0,1,1,1,0],[1,0,1,0,0,0,0],[1,0,0,0,1,0,0],
    [1,0,0,1,0,0,0],[1,1,1,0,1,0,0],
];

// First digit encoding: which pattern (L or G) to use for digits 2-7
static FIRST_DIGIT_PATTERNS: [[u8; 6]; 10] = [
    [0,0,0,0,0,0], // 0: LLLLLL
    [0,0,1,0,1,1], // 1: LLGLGG
    [0,0,1,1,0,1], // 2: LLGGLG
    [0,0,1,1,1,0], // 3: LLGGGL
    [0,1,0,0,1,1], // 4: LGLLGG
    [0,1,1,0,0,1], // 5: LGGLGL (actually LGGLLG)
    [0,1,1,1,0,0], // 6: LGGGLL
    [0,1,0,1,0,1], // 7: LGLGLG
    [0,1,0,1,1,0], // 8: LGLGGL
    [0,1,1,0,1,0], // 9: LGGLGL
];

fn calculate_checksum(digits: &[u8; 12]) -> u8 {
    let mut sum = 0u32;
    for (i, &d) in digits.iter().enumerate() {
        if i % 2 == 0 {
            sum += d as u32;
        } else {
            sum += d as u32 * 3;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

pub fn encode(content: &str, height: i32, bar_width: i32) -> Result<RgbaImage, String> {
    let digits: Vec<u8> = content.chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    if digits.len() < 12 {
        return Err(format!("EAN-13: need at least 12 digits, got {}", digits.len()));
    }

    let mut d12 = [0u8; 12];
    d12.copy_from_slice(&digits[..12]);
    let check = calculate_checksum(&d12);

    let first_digit = d12[0] as usize;
    let parity_pattern = &FIRST_DIGIT_PATTERNS[first_digit];

    // Total width: 3 (start) + 6*7 (left) + 5 (center) + 6*7 (right) + 3 (end) = 95 modules
    let module_count = 95;
    let total_width = module_count;

    let mut bm = BitMatrix::new(total_width, 1);
    let mut pos = 0;

    // Start guard: 101
    bm.set(pos, 0, true); pos += 1;
    pos += 1; // space
    bm.set(pos, 0, true); pos += 1;

    // Left side (digits 2-7)
    for i in 0..6 {
        let digit = d12[i + 1] as usize;
        let pattern = if parity_pattern[i] == 0 {
            &L_PATTERNS[digit]
        } else {
            &G_PATTERNS[digit]
        };
        for &bit in pattern {
            if bit == 1 { bm.set(pos, 0, true); }
            pos += 1;
        }
    }

    // Center guard: 01010
    pos += 1;
    bm.set(pos, 0, true); pos += 1;
    pos += 1;
    bm.set(pos, 0, true); pos += 1;
    pos += 1;

    // Right side (digits 8-12 + checksum)
    let right_digits = [d12[7], d12[8], d12[9], d12[10], d12[11], check];
    for &digit in &right_digits {
        let pattern = &R_PATTERNS[digit as usize];
        for &bit in pattern {
            if bit == 1 { bm.set(pos, 0, true); }
            pos += 1;
        }
    }

    // End guard: 101
    bm.set(pos, 0, true); pos += 1;
    pos += 1;
    bm.set(pos, 0, true);

    let bw = bar_width.max(1) as usize;
    Ok(bm.to_1d_image(bw, height.max(1) as usize))
}
