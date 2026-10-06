use super::label_element::LabelElement;

#[derive(Clone, Debug)]
pub struct LabelInfo {
    pub print_width: i32,
    pub inverted: bool,
    /// Mirror the completed label horizontally (`^PM`).
    pub mirrored: bool,
    pub elements: Vec<LabelElement>,
}
