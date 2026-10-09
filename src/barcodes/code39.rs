use super::bit_matrix::BitMatrix;
use image::RgbaImage;

// Code 39 patterns: each character maps to 9 bars (wide/narrow)
// Format: bar, space, bar, space, bar, space, bar, space, bar
// 0 = narrow, 1 = wide
static CHAR_MAP: &[(char, [u8; 9])] = &[
    ('0', [0, 0, 0, 1, 1, 0, 1, 0, 0]),
    ('1', [1, 0, 0, 1, 0, 0, 0, 0, 1]),
    ('2', [0, 0, 1, 1, 0, 0, 0, 0, 1]),
    ('3', [1, 0, 1, 1, 0, 0, 0, 0, 0]),
    ('4', [0, 0, 0, 1, 1, 0, 0, 0, 1]),
    ('5', [1, 0, 0, 1, 1, 0, 0, 0, 0]),
    ('6', [0, 0, 1, 1, 1, 0, 0, 0, 0]),
    ('7', [0, 0, 0, 1, 0, 0, 1, 0, 1]),
    ('8', [1, 0, 0, 1, 0, 0, 1, 0, 0]),
    ('9', [0, 0, 1, 1, 0, 0, 1, 0, 0]),
    ('A', [1, 0, 0, 0, 0, 1, 0, 0, 1]),
    ('B', [0, 0, 1, 0, 0, 1, 0, 0, 1]),
    ('C', [1, 0, 1, 0, 0, 1, 0, 0, 0]),
    ('D', [0, 0, 0, 0, 1, 1, 0, 0, 1]),
    ('E', [1, 0, 0, 0, 1, 1, 0, 0, 0]),
    ('F', [0, 0, 1, 0, 1, 1, 0, 0, 0]),
    ('G', [0, 0, 0, 0, 0, 1, 1, 0, 1]),
    ('H', [1, 0, 0, 0, 0, 1, 1, 0, 0]),
    ('I', [0, 0, 1, 0, 0, 1, 1, 0, 0]),
    ('J', [0, 0, 0, 0, 1, 1, 1, 0, 0]),
    ('K', [1, 0, 0, 0, 0, 0, 0, 1, 1]),
    ('L', [0, 0, 1, 0, 0, 0, 0, 1, 1]),
    ('M', [1, 0, 1, 0, 0, 0, 0, 1, 0]),
    ('N', [0, 0, 0, 0, 1, 0, 0, 1, 1]),
    ('O', [1, 0, 0, 0, 1, 0, 0, 1, 0]),
    ('P', [0, 0, 1, 0, 1, 0, 0, 1, 0]),
    ('Q', [0, 0, 0, 0, 0, 0, 1, 1, 1]),
    ('R', [1, 0, 0, 0, 0, 0, 1, 1, 0]),
    ('S', [0, 0, 1, 0, 0, 0, 1, 1, 0]),
    ('T', [0, 0, 0, 0, 1, 0, 1, 1, 0]),
    ('U', [1, 1, 0, 0, 0, 0, 0, 0, 1]),
    ('V', [0, 1, 1, 0, 0, 0, 0, 0, 1]),
    ('W', [1, 1, 1, 0, 0, 0, 0, 0, 0]),
    ('X', [0, 1, 0, 0, 1, 0, 0, 0, 1]),
    ('Y', [1, 1, 0, 0, 1, 0, 0, 0, 0]),
    ('Z', [0, 1, 1, 0, 1, 0, 0, 0, 0]),
    ('-', [0, 1, 0, 0, 0, 0, 1, 0, 1]),
    ('.', [1, 1, 0, 0, 0, 0, 1, 0, 0]),
    (' ', [0, 1, 1, 0, 0, 0, 1, 0, 0]),
    ('$', [0, 1, 0, 1, 0, 1, 0, 0, 0]),
    ('/', [0, 1, 0, 1, 0, 0, 0, 1, 0]),
    ('+', [0, 1, 0, 0, 0, 1, 0, 1, 0]),
    ('%', [0, 0, 0, 1, 0, 1, 0, 1, 0]),
    ('*', [0, 1, 0, 0, 1, 0, 1, 0, 0]),
];

fn get_pattern(ch: char) -> Option<[u8; 9]> {
    let upper = ch.to_ascii_uppercase();
    CHAR_MAP.iter().find(|(c, _)| *c == upper).map(|(_, p)| *p)
}

pub fn encode(
    content: &str,
    height: i32,
    wide_bar_ratio: f64,
    narrow_bar: i32,
) -> Result<RgbaImage, String> {
    let narrow = narrow_bar.max(1) as usize;
    let wide = ((narrow_bar as f64) * wide_bar_ratio).round().max(2.0) as usize;
    let interchar_gap = narrow;

    // Calculate width (no quiet zones — Labelary convention)
    let char_count = content.len() + 2; // + start/stop asterisks
    let bars_per_char = 6 * narrow + 3 * wide; // 6 narrow + 3 wide per character
    let total_width = char_count * bars_per_char + (char_count - 1) * interchar_gap;

    let mut bm = BitMatrix::new(total_width, 1);
    let mut pos = 0;

    let full_content = format!("*{}*", content.to_ascii_uppercase());
    for (ci, ch) in full_content.chars().enumerate() {
        if ci > 0 {
            pos += interchar_gap;
        }
        let pattern =
            get_pattern(ch).ok_or_else(|| format!("Code 39: invalid character '{}'", ch))?;
        for (j, &bit) in pattern.iter().enumerate() {
            let w = if bit == 1 { wide } else { narrow };
            let is_bar = j % 2 == 0;
            for _ in 0..w {
                if is_bar {
                    bm.set(pos, 0, true);
                }
                pos += 1;
            }
        }
    }

    Ok(bm.to_1d_image(1, height.max(1) as usize))
}
