//! CPCL resident font metrics (fonts 0-7, size identifiers 0-7).
//!
//! Values are the Appendix D tables of the Zebra CPCL Programming Manual
//! (cell width × height in dots at the printer's resolution). Fonts 1
//! (Script), 4 (Unison), 5 (Manhattan) and 7 (Warwick) have per-glyph
//! proportional advance tables; they map onto the proportional resident
//! substitute face, while the fixed-cell fonts (0 Standard, 2 OCR-A, 6 MICR)
//! map onto the monospace cell model. Nothing imports the Zebra matrices.

use crate::elements::font_face::FontFace;

pub struct CpclFont {
    /// Character cell width in dots at 1x.
    pub width: f64,
    /// Character cell height in dots at 1x.
    pub height: f64,
    pub face: FontFace,
}

/// Resolve a `TEXT {font} {size}` pair. `size` outside the resident table
/// (and named/downloaded fonts like `GBUNSG24.CPF`) falls back to the
/// Standard font 0 size 0 cell — the manual's default face.
pub fn resident_font(font: &str, size: usize) -> CpclFont {
    let cell = |w: f64, h: f64, face| CpclFont {
        width: w,
        height: h,
        face,
    };
    let mono = FontFace::BitmapMono;
    let prop = FontFace::ResidentBitmap;

    match font.trim().to_ascii_uppercase().as_str() {
        // Standard: fixed 5-dot advance, 9-36 dot heights.
        "0" => match size {
            2 => cell(10.0, 18.0, mono),
            3 | 4 => cell(10.0, 18.0, mono),
            5 | 6 => cell(20.0, 36.0, mono),
            _ => cell(5.0, 9.0, mono),
        },
        // Script: proportional, 48-dot height (the per-glyph advance table
        // is approximated by a half-height cell).
        "1" => cell(24.0, 48.0, prop),
        // OCR-A: square cells.
        "2" => {
            if size == 0 {
                cell(12.0, 12.0, mono)
            } else {
                cell(24.0, 24.0, mono)
            }
        }
        // Unison: square em cells; sizes 2-7 grow past 2x.
        "4" => match size {
            1 => cell(94.0, 94.0, prop),
            2 => cell(45.0, 45.0, prop),
            3 => cell(90.0, 90.0, prop),
            4 => cell(180.0, 180.0, prop),
            5 => cell(270.0, 270.0, prop),
            6 => cell(360.0, 360.0, prop),
            7 => cell(450.0, 450.0, prop),
            _ => cell(47.0, 47.0, prop),
        },
        // Manhattan.
        "5" => match size {
            1 => cell(48.0, 48.0, prop),
            2 => cell(46.0, 46.0, prop),
            3 => cell(92.0, 92.0, prop),
            _ => cell(24.0, 24.0, prop),
        },
        // MICR: fixed 27-dot cell.
        "6" => cell(27.0, 27.0, mono),
        // Warwick.
        "7" => {
            if size == 0 {
                cell(24.0, 24.0, prop)
            } else {
                cell(48.0, 48.0, prop)
            }
        }
        // Named/downloaded fonts and unknown designators: Standard 0-0.
        _ => cell(5.0, 9.0, mono),
    }
}
