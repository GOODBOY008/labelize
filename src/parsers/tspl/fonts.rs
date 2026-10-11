//! TSPL resident font metrics (the `TEXT` command's font numbers 1-8 and
//! the Asian bitmap faces `TST24.BF2`-style).
//!
//! Like EPL, TSPL fonts are bitmap cells sized in dots; the parser maps each
//! font to a cell size plus a [`FontFace`] rendering class and hands the
//! renderer fully precomputed dimensions. TSPL owns its table here — nothing
//! imports the Zebra font matrices.

use crate::elements::font_face::FontFace;

pub struct TsplFont {
    /// Cell width in dots at 1x (the character advance).
    pub base_width: f64,
    /// Cell height in dots at 1x.
    pub base_height: f64,
    pub face: FontFace,
}

/// Resolve a TSPL `TEXT`/`BLOCK` font identifier (case-insensitive; quotes
/// already stripped by the argument splitter). Unknown fonts fall back to
/// font 1, the TSC default.
pub fn resident_font(name: &str) -> TsplFont {
    let upper = name.trim().to_ascii_uppercase();
    let cell = |w: f64, h: f64, face| TsplFont {
        base_width: w,
        base_height: h,
        face,
    };
    match upper.as_str() {
        // Monospaced ASCII fonts 1-8: the manual's width × height cells.
        // Drawn with the monospace substitute and the fixed-cell model
        // (cap-top aligned, cell-height line pitch).
        "2" => cell(12.0, 20.0, FontFace::BitmapMono),
        "3" => cell(16.0, 24.0, FontFace::BitmapMono),
        "4" => cell(24.0, 32.0, FontFace::BitmapMono),
        "5" => cell(32.0, 48.0, FontFace::BitmapMono),
        "6" => cell(14.0, 19.0, FontFace::BitmapMono),
        "7" => cell(21.0, 27.0, FontFace::BitmapMono),
        "8" => cell(14.0, 25.0, FontFace::BitmapMono),
        // Asian bitmap faces (GB/BIG5/SJIS variants share the cell size).
        // Square cells draw 1:1 with no cap correction.
        "TST16.BF2" | "TTT16.BF2" | "TSS16.BF2" => {
            cell(16.0, 16.0, FontFace::BitmapSquare)
        }
        "TST24.BF2" | "TTT24.BF2" | "TSS24.BF2" => {
            cell(24.0, 24.0, FontFace::BitmapSquare)
        }
        // Font 1 and everything unrecognized: TSC defaults to font 1.
        _ => cell(8.0, 12.0, FontFace::BitmapMono),
    }
}
