use image::RgbaImage;
use super::bit_matrix::BitMatrix;

// Interleaved 2 of 5 patterns: each digit is encoded as 5 bars (2 wide, 3 narrow)
static DIGIT_PATTERNS: [[u8; 5]; 10] = [
    [0, 0, 1, 1, 0], // 0: NNWWN
    [1, 0, 0, 0, 1], // 1: WNNNW
    [0, 1, 0, 0, 1], // 2: NWNNW
    [1, 1, 0, 0, 0], // 3: WWNNN
    [0, 0, 1, 0, 1], // 4: NNWNW
    [1, 0, 1, 0, 0], // 5: WNWNN
    [0, 1, 1, 0, 0], // 6: NWWNN
    [0, 0, 0, 1, 1], // 7: NNNWW
    [1, 0, 0, 1, 0], // 8: WNNWN
    [0, 1, 0, 1, 0], // 9: NWNWN
];

pub fn encode(content: &str, height: i32, wide_bar_ratio: i32, narrow_bar: i32, print_check_digit: bool) -> Result<RgbaImage, String> {
    let mut digits: Vec<u8> = content.chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    if digits.is_empty() {
        return Err("Interleaved 2 of 5: no digits provided".to_string());
    }

    // Calculate check digit if needed
    if print_check_digit || digits.len() % 2 != 0 {
        let mut sum = 0u32;
        for (i, &d) in digits.iter().enumerate() {
            if i % 2 == 0 {
                sum += d as u32 * 3;
            } else {
                sum += d as u32;
            }
        }
        let check = ((10 - (sum % 10)) % 10) as u8;
        digits.push(check);
    }

    // Pad to even length
    if digits.len() % 2 != 0 {
        digits.insert(0, 0);
    }

    let narrow = narrow_bar.max(1) as usize;
    let wide = (narrow_bar * wide_bar_ratio).max(2) as usize;

    // Calculate width
    // Start: nnnn (4 narrow)
    // Each pair: 5 interleaved bars + 5 interleaved spaces = 10 elements
    // Stop: Wnn (wide + narrow + narrow)
    let pair_count = digits.len() / 2;
    let pair_width = 2 * wide + 3 * narrow; // per digit, interleaved bars and spaces
    let total_module_width = 4 * narrow + pair_count * (pair_width * 2) + wide + 2 * narrow;
    let total_width = total_module_width;

    let mut bm = BitMatrix::new(total_width, 1);
    let mut pos = 0;

    // Start pattern: narrow bar, narrow space, narrow bar, narrow space
    bm.set_range(pos, narrow, true); pos += narrow;
    pos += narrow; // space
    bm.set_range(pos, narrow, true); pos += narrow;
    pos += narrow; // space

    // Encode pairs
    for pair_idx in 0..pair_count {
        let d1 = digits[pair_idx * 2] as usize;
        let d2 = digits[pair_idx * 2 + 1] as usize;
        let bars = &DIGIT_PATTERNS[d1];
        let spaces = &DIGIT_PATTERNS[d2];

        for i in 0..5 {
            let bar_w = if bars[i] == 1 { wide } else { narrow };
            let space_w = if spaces[i] == 1 { wide } else { narrow };

            bm.set_range(pos, bar_w, true);
            pos += bar_w;
            pos += space_w; // space
        }
    }

    // Stop pattern: wide bar, narrow space, narrow bar
    bm.set_range(pos, wide, true); pos += wide;
    pos += narrow; // space
    bm.set_range(pos, narrow, true);

    Ok(bm.to_1d_image(1, height.max(1) as usize))
}
