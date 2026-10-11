#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FieldAlignment {
    #[default]
    Left = 0,
    Right = 1,
    Auto = 2,
    /// Centered on the field origin (TSPL alignment parameter 2; ZPL ^FB
    /// centering is TextAlignment and does not use this).
    Center = 3,
}
