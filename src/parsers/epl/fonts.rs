//! EPL built-in font metrics (the `A` command's font numbers 1-5).
//!
//! EPL fonts are bitmap cells sized in dots; the shared renderer draws them
//! with the scalable font-0 substitute, so the parser maps each font number
//! to a (width, height) cell in dot units and lets width/height magnification
//! scale from there.

static EPL_FONT_SIZES: &[(i32, i32, i32)] = &[
    // (font_num, width, height)
    (1, 8, 12),
    (2, 10, 16),
    (3, 12, 20),
    (4, 14, 24),
    (5, 32, 48),
];

pub fn epl_font_size(font_num: i32) -> (i32, i32) {
    for &(n, w, h) in EPL_FONT_SIZES {
        if n == font_num {
            return (w, h);
        }
    }
    (8, 12) // default to font 1
}
