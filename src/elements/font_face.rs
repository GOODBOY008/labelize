//! The renderable font faces the engine substitutes for printer fonts.
//!
//! `FontFace` is the language-neutral rendering class of a text field: it
//! picks the embedded TTF asset AND carries the substitute's calibration
//! (cap scale, advance ratio, line-height factor). Parsers resolve their
//! language-specific font identifiers into a `FontFace` at parse time — the
//! renderer never sees a printer's font names, only faces.
//!
//! Variant set and constants mirror the Zebra substitute model the goldens
//! were calibrated against; a new language maps its own fonts onto these
//! faces (or adds variants) instead of reusing ZPL designators.

use crate::assets::{
    FONT_DEJAVU_SANS_MONO, FONT_DEJAVU_SANS_MONO_BOLD, FONT_ZERO_SUBSTITUTE, FONT_ZPL_GS,
};
use crate::tuning;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFace {
    /// Scalable condensed-bold substitute (ZPL font 0): per-glyph advance
    /// calibration, 0.75-cap em, Roboto Condensed Bold.
    CondensedBold,
    /// Scalable monospace substitute with its own vertical model (ZPL font 1).
    ScalableMono,
    /// Fixed-cell monospace substitute, regular weight (ZPL A, C, E–H).
    /// Matches the historical FontInfo default (name "A").
    #[default]
    BitmapMono,
    /// Fixed-cell monospace substitute, bold, no lowercase glyphs — input is
    /// uppercased (ZPL font B: 9-dot advance, 1.59 cap).
    BitmapBold,
    /// Fixed-cell monospace substitute, bold, ~1.2× advance (ZPL font D).
    BitmapBoldWide,
    /// Resident bitmap matrices with proportional metrics (ZPL P–V).
    ResidentBitmap,
    /// Symbol glyphs (ZPL GS).
    Symbol,
    /// Square-pixel bitmap cell whose substitute scales 1:1 with the
    /// requested cell (TSPL .BF2 resident fonts): no cap correction, no
    /// cell shift — the em is the cell.
    BitmapSquare,
    /// Plain monospace, no cell-shift corrections (ZPL W–Z and unmapped
    /// designators).
    Mono,
}

impl FontFace {
    /// The embedded TTF asset drawn for this face.
    pub fn ttf_data(self) -> &'static [u8] {
        match self {
            FontFace::CondensedBold => FONT_ZERO_SUBSTITUTE,
            FontFace::BitmapBold | FontFace::BitmapBoldWide | FontFace::ResidentBitmap => {
                FONT_DEJAVU_SANS_MONO_BOLD
            }
            FontFace::Symbol => FONT_ZPL_GS,
            FontFace::BitmapSquare
            | FontFace::ScalableMono
            | FontFace::BitmapMono
            | FontFace::Mono => FONT_DEJAVU_SANS_MONO,
        }
    }

    /// Cap-height correction applied to the em so substitute caps land where
    /// the printer's face puts them (font A: 7 of 9 dot rows, etc.).
    pub fn cap_scale(self) -> f64 {
        match self {
            FontFace::BitmapBold => 1.59,
            FontFace::ResidentBitmap => 0.85,
            FontFace::ScalableMono => tuning::FONT1_CAP_SCALE,
            FontFace::BitmapSquare => 1.0,
            _ => 7.0 / 6.0,
        }
    }

    /// Width-to-height ratio feeding `scale.x` (advance model of the face).
    pub fn width_ratio(self) -> f64 {
        match self {
            FontFace::CondensedBold => tuning::FONT0_RATIO,
            FontFace::ScalableMono => tuning::FONT1_RATIO,
            FontFace::BitmapBoldWide => 2.317,
            FontFace::ResidentBitmap => 0.80,
            FontFace::Symbol | FontFace::BitmapSquare => 1.0,
            _ => 1.931,
        }
    }

    /// Line-height factor for multi-line blocks, as a fraction of the em.
    pub fn line_height_factor(self) -> f64 {
        match self {
            FontFace::ScalableMono => tuning::FONT1_LINE_HEIGHT,
            FontFace::CondensedBold => tuning::FONT0_LINE_HEIGHT,
            _ => 1.0,
        }
    }

    /// Pen-axis nudge for faces whose left side bearing drifts a pixel.
    pub fn pen_x_offset(self) -> f64 {
        match self {
            FontFace::ScalableMono => tuning::FONT1_X_OFFSET,
            _ => 0.0,
        }
    }

    /// Field data has no lowercase on this face — draw it uppercased.
    pub fn uppercases_input(self) -> bool {
        matches!(self, FontFace::BitmapBold)
    }

    /// True when the em is enlarged by `cap_scale` and the draw origin shifts
    /// up to align cap tops with the field origin (all fixed-cell faces and
    /// the scalable mono substitute).
    pub fn uses_cell_shift(self) -> bool {
        matches!(
            self,
            FontFace::BitmapMono
                | FontFace::BitmapBold
                | FontFace::BitmapBoldWide
                | FontFace::ResidentBitmap
                | FontFace::Symbol
                | FontFace::ScalableMono
        )
        // BitmapSquare is intentionally absent: its em IS the cell, so the
        // substitute needs no cap-top realignment.
    }

    /// True when `^FB` line pitch divides by `cap_scale` (fixed-cell faces
    /// whose printer line pitch is the cell height, not the enlarged em).
    pub fn divides_line_height_by_cap(self) -> bool {
        matches!(
            self,
            FontFace::BitmapMono
                | FontFace::BitmapBold
                | FontFace::BitmapBoldWide
                | FontFace::ResidentBitmap
                | FontFace::Symbol
        )
    }

    /// Cap tops sit this far *below* the field origin, as a fraction of the
    /// cell (resident bitmaps anchor low; measured on Labelary).
    pub fn cap_anchor_drop(self) -> f64 {
        match self {
            FontFace::ResidentBitmap => 0.15,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_map_to_their_substitute_assets() {
        assert_eq!(FontFace::CondensedBold.ttf_data(), FONT_ZERO_SUBSTITUTE);
        assert_eq!(FontFace::BitmapBold.ttf_data(), FONT_DEJAVU_SANS_MONO_BOLD);
        assert_eq!(
            FontFace::BitmapBoldWide.ttf_data(),
            FONT_DEJAVU_SANS_MONO_BOLD
        );
        assert_eq!(
            FontFace::ResidentBitmap.ttf_data(),
            FONT_DEJAVU_SANS_MONO_BOLD
        );
        assert_eq!(FontFace::Symbol.ttf_data(), FONT_ZPL_GS);
        assert_eq!(FontFace::ScalableMono.ttf_data(), FONT_DEJAVU_SANS_MONO);
        assert_eq!(FontFace::BitmapMono.ttf_data(), FONT_DEJAVU_SANS_MONO);
        assert_eq!(FontFace::Mono.ttf_data(), FONT_DEJAVU_SANS_MONO);
    }

    #[test]
    fn calibration_matches_the_zebra_substitute_model() {
        assert_eq!(FontFace::BitmapBold.cap_scale(), 1.59);
        assert_eq!(FontFace::ResidentBitmap.cap_scale(), 0.85);
        assert_eq!(FontFace::ScalableMono.cap_scale(), tuning::FONT1_CAP_SCALE);
        assert_eq!(FontFace::BitmapMono.cap_scale(), 7.0 / 6.0);
        assert_eq!(FontFace::BitmapBoldWide.width_ratio(), 2.317);
        assert_eq!(FontFace::ResidentBitmap.width_ratio(), 0.80);
        assert_eq!(FontFace::CondensedBold.width_ratio(), tuning::FONT0_RATIO);
        assert_eq!(FontFace::ScalableMono.width_ratio(), tuning::FONT1_RATIO);
        assert_eq!(
            FontFace::ScalableMono.pen_x_offset(),
            tuning::FONT1_X_OFFSET
        );
        assert!(FontFace::BitmapBold.uppercases_input());
        assert!(!FontFace::CondensedBold.uppercases_input());
        // Only the two scalable faces skip the fixed-cell corrections.
        assert!(!FontFace::CondensedBold.uses_cell_shift());
        assert!(!FontFace::Mono.uses_cell_shift());
        assert!(FontFace::BitmapMono.uses_cell_shift());
        // The scalable mono shifts but keeps an undivided line pitch.
        assert!(FontFace::ScalableMono.uses_cell_shift());
        assert!(!FontFace::ScalableMono.divides_line_height_by_cap());
        assert_eq!(FontFace::ResidentBitmap.cap_anchor_drop(), 0.15);
    }
}
