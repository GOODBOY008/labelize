//! Language-neutral font descriptor.
//!
//! A parser resolves its language's font identifier into `face` plus the
//! resolved metrics (`advance_mult`, `dedot_zero`) at parse time; the
//! renderer draws from those fields and never looks at `name`. Producer
//! conventions for sizes (Zebra magnification matrices, EPL font-number
//! cells, …) live in the parsers, not here.

use super::field_orientation::FieldOrientation;
use super::font_face::FontFace;

#[derive(Clone, Debug, Default)]
pub struct TextProperties {
    pub show_missing: bool,
    pub bidirectional: bool,
    pub shape: bool,
    pub open_type: bool,
}

#[derive(Clone, Debug)]
pub struct FontResources {
    pub primary: Option<std::sync::Arc<[u8]>>,
    pub fallbacks: Vec<std::sync::Arc<[u8]>>,
    pub properties: TextProperties,
    pub native_cell: Option<(u16, u16)>,
}

#[derive(Clone, Debug)]
pub struct FontInfo {
    /// Producer-owned font key (ZPL designator, EPL font number, …) — kept
    /// for diagnostics and ^CW-style remapping during parsing only.
    pub name: String,
    /// Rendering class (embedded asset + calibration), resolved by the parser.
    pub face: FontFace,
    pub resources: Option<std::sync::Arc<FontResources>>,
    /// Pen-advance multiplier for fixed-cell faces, resolved by the parser
    /// from its font tables; 1.0 for faces with their own advance model.
    pub advance_mult: f64,
    /// Whether the substitute face's dotted zero is removed for this font
    /// (reference renderers differ per font); resolved by the parser.
    pub dedot_zero: bool,
    pub width: f64,
    pub height: f64,
    pub orientation: FieldOrientation,
}

impl Default for FontInfo {
    fn default() -> Self {
        FontInfo {
            name: "A".to_string(),
            face: FontFace::default(),
            resources: None,
            advance_mult: 1.0,
            dedot_zero: false,
            width: 0.0,
            height: 0.0,
            orientation: FieldOrientation::Normal,
        }
    }
}

/// True for the resident scalable font designators (ZPL fonts P–V, the
/// matrix-font family). Kept on the descriptor for downstream diagnostics;
/// Zebra's calibration tables live in `parsers::zpl::zebra_fonts`.
pub fn is_pv_font(name: &str) -> bool {
    matches!(name, "P" | "Q" | "R" | "S" | "T" | "U" | "V")
}

impl FontInfo {
    pub fn get_size(&self) -> f64 {
        self.height
    }

    pub fn get_scale_x(&self) -> f64 {
        if self.height != 0.0 {
            self.face.width_ratio() * self.width / self.height
        } else {
            1.0
        }
    }
}
