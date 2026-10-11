//! Zebra font-designator knowledge: matrices, magnification rules, and the
//! designator→[`FontFace`] resolution.
//!
//! Everything in this file describes how ZPL font names behave and belongs
//! exclusively to the ZPL parser; other languages resolve their own font
//! identifiers into `FontFace` values and never call in here.

use std::collections::HashMap;

use crate::elements::font::FontInfo;
use crate::elements::font_face::FontFace;

fn bitmap_font_sizes() -> &'static HashMap<&'static str, [f64; 2]> {
    use std::sync::OnceLock;
    static SIZES: OnceLock<HashMap<&str, [f64; 2]>> = OnceLock::new();
    SIZES.get_or_init(|| {
        let mut m = HashMap::new();
        // Font A: 9 high × 5 body dots + 1 spacing dot = 6 dots advance per character.
        // Verified from Labelary renders: advance = 6×mag px/char at each magnification.
        m.insert("A", [9.0, 6.0]);
        m.insert("B", [11.0, 7.0]);
        m.insert("C", [18.0, 10.0]);
        m.insert("D", [18.0, 10.0]);
        m.insert("E", [28.0, 15.0]);
        m.insert("F", [26.0, 13.0]);
        m.insert("G", [60.0, 40.0]);
        m.insert("H", [21.0, 13.0]);
        m.insert("GS", [24.0, 24.0]);
        m
    })
}

fn pqrs_font_matrices() -> &'static HashMap<&'static str, [f64; 2]> {
    use std::sync::OnceLock;
    static SIZES: OnceLock<HashMap<&str, [f64; 2]>> = OnceLock::new();
    SIZES.get_or_init(|| {
        let mut m = HashMap::new();
        // Zebra resident bitmap fonts P-V: base cell matrices at 8 dpmm (203 dpi),
        // from the official Font Matrices table:
        // https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-font-barcodes-fonts-andbar-codes/r-zpl-font-barcodes-font-matrices.html
        // Unlike A-H (where a missing h/w derives from the other to keep one
        // magnification), Labelary renders P-V with h and w INDEPENDENT: a missing
        // param stays at 1x base (verified: ^AQN,,48 is 2x wide but 1x tall,
        // ^AQN,56, is 1x wide but 2x tall). Magnification is stepwise:
        // mag = round(param / base), min 1 (e.g. Q w=30 -> 1x, w=36 -> 2x).
        m.insert("P", [20.0, 18.0]);
        m.insert("Q", [28.0, 24.0]);
        m.insert("R", [35.0, 31.0]);
        m.insert("S", [40.0, 35.0]);
        m.insert("T", [48.0, 42.0]);
        m.insert("U", [59.0, 53.0]);
        m.insert("V", [80.0, 71.0]);
        m
    })
}

pub fn is_pv_font(name: &str) -> bool {
    matches!(name, "P" | "Q" | "R" | "S" | "T" | "U" | "V")
}

pub fn is_standard_font(font: &FontInfo) -> bool {
    font.name == "0"
        || font.name == "1"
        || bitmap_font_sizes().contains_key(font.name.as_str())
        // Zebra resident scalable fonts (not bitmap, not font-0)
        || matches!(
            font.name.as_str(),
            "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
        )
}

/// Returns true for Zebra bitmap fonts (A-H, GS), false for the scalable font "0".
pub fn is_bitmap_font(font: &FontInfo) -> bool {
    bitmap_font_sizes().contains_key(font.name.as_str())
}

/// Resolve a ZPL font designator into the rendering class plus the
/// per-designator bitmap knobs the renderer consumes (`advance_mult`,
/// `dedot_zero`). Unmapped designators log once and fall back to plain mono.
pub fn resolve_designator(name: &str) -> (FontFace, f64, bool) {
    let face = match name {
        "0" => FontFace::CondensedBold,
        "1" => FontFace::ScalableMono,
        "B" => FontFace::BitmapBold,
        "D" => FontFace::BitmapBoldWide,
        "GS" => FontFace::Symbol,
        "P" | "Q" | "R" | "S" | "T" | "U" | "V" => FontFace::ResidentBitmap,
        "A" | "C" | "E" | "F" | "G" | "H" => FontFace::BitmapMono,
        "W" | "X" | "Y" | "Z" => FontFace::Mono,
        _ => {
            warn_unmapped_font(name);
            FontFace::Mono
        }
    };
    let bitmap = bitmap_font_sizes().contains_key(name);
    (
        face,
        bitmap_advance_mult(name),
        bitmap && bitmap_zero_dedot(name),
    )
}

/// Refresh the render-facing fields (`face`, `advance_mult`, `dedot_zero`)
/// of `font` from its — by then final — designator. Call this once the name
/// has settled (^CW remap, numeric fallbacks, default-font inheritance).
pub(crate) fn apply_face(font: &mut FontInfo) {
    let (face, advance_mult, dedot_zero) = resolve_designator(&font.name);
    font.face = face;
    font.advance_mult = advance_mult;
    font.dedot_zero = dedot_zero;
}

/// Font names with no built-in mapping silently fall back to DejaVu Sans Mono.
/// Surface that once per name -- not once per rendered field -- so rendering
/// many labels (e.g. through the HTTP service) doesn't flood stderr.
fn warn_unmapped_font(name: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let first_time = WARNED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map(|mut seen| seen.insert(name.to_string()))
        .unwrap_or(false);
    if first_time {
        eprintln!(
            "labelize: no mapping for ZPL font '{name}'; substituting DejaVu Sans Mono (layout may differ)"
        );
    }
}

/// Apply Zebra's font-size derivation rules to `font` (magnification
/// step-off for bitmap fonts, P-V matrix independence, scalable fill-ins).
pub fn with_adjusted_sizes(font: &FontInfo) -> FontInfo {
    let mut font = font.clone();

    if font.name == "1" {
        // Zebra scalable font 1 — Labelary substitutes a monospace face
        // (see tuning::FONT1_* for the probed model). With the height slot
        // empty the width value feeds both axes with a doubled em
        // (^A1,,10,40 renders identical to ^A1,,10,10); a missing width
        // derives from the height, mirroring the font-0 rule.
        if font.width == 0.0 && font.height == 0.0 {
            font.width = 10.0;
            font.height = 10.0;
        } else {
            if font.width == 0.0 {
                font.width = font.height;
            }
            if font.height == 0.0 {
                font.height = 2.0 * font.width;
            }
        }
        return font;
    }

    let sizes = bitmap_font_sizes();

    if let Some(org_size) = sizes.get(font.name.as_str()) {
        // Bitmap font.
        // Font B empirics (measured against Labelary at magnifications 1-9): the width
        // parameter selects magnification on the 11-dot cell height -- NOT the 7-dot glyph
        // width -- and the character advance is 9 dots per magnification (7-dot glyph plus
        // 2-dot intercharacter gap). With the generic rules a ^CFB,80 route code rendered
        // ~26% smaller than Labelary/Zebra output.
        let advance = if font.name == "B" { 9.0 } else { org_size[1] };
        let width_mag_base = if font.name == "B" {
            org_size[0]
        } else {
            org_size[1]
        };
        if font.width == 0.0 && font.height == 0.0 {
            font.width = advance;
            font.height = org_size[0];
            return font;
        }

        if font.width == 0.0 {
            font.width = advance * (font.height / org_size[0]).round().max(1.0);
        } else {
            font.width = advance * (font.width / width_mag_base).round().max(1.0);
        }

        if font.height == 0.0 {
            font.height = org_size[0] * (font.width / advance).round().max(1.0);
        } else {
            font.height = org_size[0] * (font.height / org_size[0]).round().max(1.0);
        }

        font
    } else if let Some(org_size) = pqrs_font_matrices().get(font.name.as_str()) {
        // Zebra resident bitmap fonts P-V. Base cell matrices at 8 dpmm from the
        // official Font Matrices table (P 20x18, Q 28x24, R 35x31, S 40x35,
        // T 48x42, U 59x53, V 80x71). Magnification is stepwise and INDEPENDENT
        // for h and w: mag_h = round(h / base_h), mag_w = round(w / base_w),
        // min 1; a missing (0) param stays at 1x base instead of deriving from
        // the other (verified against Labelary: ^AQN,,48 is 2x wide / 1x tall,
        // ^AQN,56, is 1x wide / 2x tall, ^AQN,,30 stays 1x/1x).
        let base_h = org_size[0];
        let base_w = org_size[1];
        let mag_h = if font.height == 0.0 {
            1.0
        } else {
            (font.height / base_h).round().max(1.0)
        };
        let mag_w = if font.width == 0.0 {
            1.0
        } else {
            (font.width / base_w).round().max(1.0)
        };
        font.height = base_h * mag_h;
        font.width = base_w * mag_w;
        font
    } else {
        // Scalable font (font 0)
        if font.width == 0.0 {
            font.width = font.height;
        }
        if font.height == 0.0 {
            font.height = font.width;
        }
        font.width = font.width.max(10.0);
        font.height = font.height.max(10.0);
        font
    }
}

/// The multiplier decouples ONLY the pen advance: glyph ink is still laid out
/// with the substitute's own shape width (`scale.x` unchanged), so glyph
/// rasterization and the vertical model are untouched. Fonts 0, 1 and P–V have
/// their own calibrated advance models and return 1.0 here.
pub(crate) fn bitmap_advance_mult(name: &str) -> f64 {
    match name {
        "C" | "G" => 1.2,
        "E" => 4.0 / 3.0,
        "F" => 16.0 / 13.0,
        "H" => 19.0 / 13.0,
        _ => 1.0,
    }
}

/// Whether the substitute face's dotted zero must be removed for this bitmap
/// font. Labelary's reference zeros are not uniform: measured over the
/// `HHHHHHHHHH 0000000000` probe rows, every font except E and H carries an
/// interior zero dot in the reference bitmaps, so the DejaVu substitute's dot
/// STAYS; E and H have empty interiors at every probed size and removing the
/// dot there is a pure win (see tuning history for the per-font verdicts and
/// the corpus check that kept A/C/D/F/G keeping their dots).
pub(crate) fn bitmap_zero_dedot(name: &str) -> bool {
    matches!(name, "E" | "H")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face_of(name: &str) -> FontFace {
        resolve_designator(name).0
    }

    #[test]
    fn designators_resolve_to_their_faces() {
        assert_eq!(face_of("0"), FontFace::CondensedBold);
        assert_eq!(face_of("1"), FontFace::ScalableMono);
        assert_eq!(face_of("B"), FontFace::BitmapBold);
        assert_eq!(face_of("D"), FontFace::BitmapBoldWide);
        assert_eq!(face_of("GS"), FontFace::Symbol);
        assert_eq!(face_of("P"), FontFace::ResidentBitmap);
        assert_eq!(face_of("V"), FontFace::ResidentBitmap);
        assert_eq!(face_of("A"), FontFace::BitmapMono);
        assert_eq!(face_of("H"), FontFace::BitmapMono);
        assert_eq!(face_of("W"), FontFace::Mono);
    }

    #[test]
    fn unmapped_designators_fall_back_to_plain_mono() {
        assert_eq!(face_of("2"), FontFace::Mono);
        assert_eq!(face_of("E:TT0003M_.FNT"), FontFace::Mono);
    }

    #[test]
    fn bitmap_knobs_carry_the_per_font_tables() {
        let (_, adv, _) = resolve_designator("C");
        assert_eq!(adv, 1.2);
        let (_, adv, _) = resolve_designator("E");
        assert_eq!(adv, 4.0 / 3.0);
        let (_, _, dedot) = resolve_designator("E");
        assert!(dedot);
        let (_, _, dedot) = resolve_designator("A");
        assert!(!dedot);
        // Scalable faces have no bitmap knobs.
        let (_, adv, dedot) = resolve_designator("0");
        assert_eq!((adv, dedot), (1.0, false));
    }

    #[test]
    fn apply_face_refreshes_from_the_settled_name() {
        let mut font = FontInfo::default();
        font.name = "0".to_string();
        apply_face(&mut font);
        assert_eq!(font.face, FontFace::CondensedBold);
        font.name = "B".to_string();
        apply_face(&mut font);
        assert_eq!(font.face, FontFace::BitmapBold);
        assert!(!font.dedot_zero);
    }
}
