//! DPL font metrics: the internal bit-mapped fonts 0-8 (Table C-2, 203
//! dpi cells) and the smooth font 9 point-size table (Table C-3). Nothing
//! imports the Zebra matrices.

use crate::elements::font_face::FontFace;

pub struct DplFont {
    /// Character cell height in dots at 1x.
    pub height: f64,
    /// Character cell width in dots at 1x.
    pub width: f64,
    pub face: FontFace,
}

/// Resolve the internal bit-mapped font `0`-`8` (Table C-2, 203 dpi).
/// Fonts 3-6 carry uppercase-only character sets; font 7 is OCR-A and
/// font 8 OCR-B.
pub fn bitmap_font(b: u8) -> Option<DplFont> {
    let cell = |h: f64, w: f64| DplFont {
        height: h,
        width: w,
        face: FontFace::BitmapMono,
    };
    match b {
        b'0' => Some(cell(7.0, 5.0)),
        b'1' => Some(cell(13.0, 7.0)),
        b'2' => Some(cell(18.0, 10.0)),
        b'3' => Some(cell(27.0, 14.0)),
        b'4' => Some(cell(36.0, 18.0)),
        b'5' => Some(cell(52.0, 18.0)),
        b'6' => Some(cell(64.0, 32.0)),
        b'7' => Some(cell(32.0, 15.0)),
        b'8' => Some(cell(28.0, 15.0)),
        _ => None,
    }
}

/// Smooth font 9 (internal CG Triumvirate) point sizes selected by the
/// `eee` field: `Ann` or `0nn` forms (Table C-3). Returns the point size.
pub fn smooth_font_points(eee: &str) -> Option<f64> {
    let table: [(&str, &str, f64); 9] = [
        ("A06", "001", 6.0),
        ("A08", "002", 8.0),
        ("A10", "003", 10.0),
        ("A12", "004", 12.0),
        ("A14", "005", 14.0),
        ("A18", "006", 18.0),
        ("A24", "007", 24.0),
        ("A30", "008", 30.0),
        ("A36", "009", 36.0),
    ];
    for (ann, nn, pts) in table {
        if eee.eq_ignore_ascii_case(ann) || eee == nn {
            return Some(pts);
        }
    }
    None
}

/// Base-25 multiplier digit: `1`-`9` are 1-9, `A`-`O` are 10-24. A `0`
/// selects the command's default and reports 0 — callers substitute their
/// own default.
pub fn multiplier(ch: u8) -> f64 {
    match ch {
        b'0' => 0.0,
        b'1'..=b'9' => (ch - b'0') as f64,
        b'A'..=b'O' => (ch - b'A' + 10) as f64,
        b'a'..=b'o' => (ch - b'a' + 10) as f64,
        _ => 1.0,
    }
}
