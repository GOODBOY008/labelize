use super::field_orientation::FieldOrientation;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FontInfo {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub orientation: FieldOrientation,
}

impl Default for FontInfo {
    fn default() -> Self {
        FontInfo {
            name: "A".to_string(),
            width: 0.0,
            height: 0.0,
            orientation: FieldOrientation::Normal,
        }
    }
}

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

impl FontInfo {
    pub fn get_size(&self) -> f64 {
        self.height
    }

    pub fn get_scale_x(&self) -> f64 {
        if self.height != 0.0 {
            self.get_width_to_height_ratio() * self.width / self.height
        } else {
            1.0
        }
    }

    pub fn is_standard_font(&self) -> bool {
        self.name == "0"
            || bitmap_font_sizes().contains_key(self.name.as_str())
            // Zebra resident scalable fonts (not bitmap, not font-0)
            || matches!(
                self.name.as_str(),
                "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
            )
    }

    /// Returns true for Zebra bitmap fonts (A-H, GS), false for the scalable font "0".
    pub fn is_bitmap_font(&self) -> bool {
        bitmap_font_sizes().contains_key(self.name.as_str())
    }

    pub fn with_adjusted_sizes(&self) -> FontInfo {
        let mut font = self.clone();
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

    fn get_width_to_height_ratio(&self) -> f64 {
        if self.name == "GS" {
            1.0
        } else if self.name == "0" {
            // Zebra font 0 (smooth scalable) width-to-height ratio. Our Helvetica Bold
            // substitute runs narrower than Zebra's CG Triumvirate, so glyph shapes need
            // widening; per-character spacing is corrected separately by
            // `tuning::font0_advance_delta`. See `tuning::FONT0_RATIO` for the calibration.
            crate::tuning::FONT0_RATIO
        } else if self.name == "D" {
            // Zebra font D's actual character advance is ~1.2× the nominal 10-dot cell width.
            // Empirically calibrated against Labelary: at 1x (w=10), Zebra font D renders
            // ~12px per character advance vs our DejaVu's ~10px.
            // 1.931 × 1.2 = 2.317
            2.317
        } else if is_pv_font(self.name.as_str()) {
            // Fonts P-V are proportional (unlike monospace A-H), so our DejaVu Sans
            // Mono Bold substitute needs a narrower advance to match the average
            // glyph width. Measured against Labelary at 1x base ("HHHH" vs
            // "Font X Normal"): H advance is ~0.54x base_w while average advance
            // over mixed text is ~0.41x base_w. With DejaVu advancing at
            // 0.518x scale.x, advance = width x ratio x 0.518, so ratio ~= 0.80
            // lands between H-width and average-width (sweep 0.70-0.95 against the
            // font_p/q/s references converged on 0.80).
            0.80
        } else {
            // Bitmap fonts A-H use DejaVu Sans Mono (Regular or Bold).
            // ab_glyph scales advances as:  h_advance = h_advance_unscaled / height_unscaled * scale_x
            // where height_unscaled = ascender - descender + line_gap (≠ units_per_em).
            // For DejaVu Sans Mono: height_unscaled = 1901 + 483 = 2384, h_advance_unscaled ≈ 1235.
            // Ratio = height_unscaled / h_advance_unscaled ≈ 2384/1235 = 1.931, so that
            //   advance = scale_x * (1235/2384) = ratio * w * (1235/2384) ≈ w.
            1.931
        }
    }
}
