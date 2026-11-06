/// Direction in which upright glyphs are placed before field rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Horizontal,
    Vertical,
    Reverse,
}

/// Resolved text placement in printer dots, independent of input language.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextFlow {
    pub direction: TextDirection,
    pub character_spacing: i32,
    /// Extra advance used when measuring line wrapping, independent of ink spacing.
    pub wrap_spacing: f64,
    /// Nominal width used for the reverse pen origin and vertical field rotation.
    pub origin_width: f64,
}
