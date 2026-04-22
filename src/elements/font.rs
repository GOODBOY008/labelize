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
        self.name == "0" || bitmap_font_sizes().contains_key(self.name.as_str())
    }

    /// Returns true for Zebra bitmap fonts (A-H, GS), false for the scalable font "0".
    pub fn is_bitmap_font(&self) -> bool {
        bitmap_font_sizes().contains_key(self.name.as_str())
    }

    pub fn with_adjusted_sizes(&self) -> FontInfo {
        let mut font = self.clone();
        let sizes = bitmap_font_sizes();

        if let Some(org_size) = sizes.get(font.name.as_str()) {
            // Bitmap font
            if font.width == 0.0 && font.height == 0.0 {
                font.width = org_size[1];
                font.height = org_size[0];
                return font;
            }

            if font.width == 0.0 {
                font.width = org_size[1] * (font.height / org_size[0]).round().max(1.0);
            } else {
                font.width = org_size[1] * (font.width / org_size[1]).round().max(1.0);
            }

            if font.height == 0.0 {
                font.height = org_size[0] * (font.width / org_size[1]).round().max(1.0);
            } else {
                font.height = org_size[0] * (font.height / org_size[0]).round().max(1.0);
            }

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
            // Zebra font 0 (smooth scalable) width-to-height ratio.
            // Per ZPL docs: "setting height and width equally produces characters that appear most balanced"
            // This means when h=w, scale_x should be 1.0 (balanced/square proportions).
            0.9
        } else if self.name == "D" {
            // Zebra font D's actual character advance is ~1.2× the nominal 10-dot cell width.
            // Empirically calibrated against Labelary: at 1x (w=10), Zebra font D renders
            // ~12px per character advance vs our DejaVu's ~10px.
            // 1.931 × 1.2 = 2.317
            2.317
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
