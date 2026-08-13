#[derive(Clone, Debug)]
pub struct DrawerOptions {
    pub label_width_mm: f64,
    pub label_height_mm: f64,
    pub dpmm: i32,
    pub enable_inverted_labels: bool,
    /// Preserve the renderer's antialiased greys instead of thresholding to 1-bit.
    /// Off by default: a thermal printer is bi-level, so 1-bit is the faithful
    /// default. Turn it on to match Labelary's antialiased PNG preview.
    pub antialias: bool,
}

impl Default for DrawerOptions {
    fn default() -> Self {
        DrawerOptions {
            label_width_mm: 101.6,
            label_height_mm: 203.2,
            dpmm: 8,
            enable_inverted_labels: true,
            antialias: false,
        }
    }
}

impl DrawerOptions {
    pub fn with_defaults(mut self) -> Self {
        if self.label_width_mm == 0.0 {
            self.label_width_mm = 101.6;
        }
        if self.label_height_mm == 0.0 {
            self.label_height_mm = 203.2;
        }
        if self.dpmm == 0 {
            self.dpmm = 8;
        }
        self
    }
}
