use super::label_element::LabelElement;

#[derive(Clone, Debug, Default)]
pub struct LabelInfo {
    pub print_width: i32,
    pub inverted: bool,
    /// Mirror the completed label horizontally (`^PM`).
    pub mirrored: bool,
    /// Physical label width declared by the producing language (TSPL's
    /// `SIZE`); `None` when the language doesn't carry one. Entry points
    /// fall back to this behind an explicit user override.
    pub label_width_mm: Option<f64>,
    /// Physical label height declared by the producing language.
    pub label_height_mm: Option<f64>,
    pub elements: Vec<LabelElement>,
}
