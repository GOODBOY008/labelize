#[derive(Clone, Debug, Default)]
pub struct LabelPosition {
    pub x: i32,
    pub y: i32,
    pub calculate_from_bottom: bool,
    pub automatic_position: bool,
    /// `y` is measured upward from the label's bottom edge (Datamax-style
    /// bottom-left home). The renderer anchors the object's bottom edge
    /// there once the canvas and object heights are known. Orthogonal to
    /// `calculate_from_bottom` (^FT-style baseline anchoring in top-down
    /// coordinates).
    pub from_bottom: bool,
}

impl LabelPosition {
    pub fn add(&self, other: &LabelPosition) -> LabelPosition {
        LabelPosition {
            x: self.x + other.x,
            y: self.y + other.y,
            calculate_from_bottom: self.calculate_from_bottom,
            automatic_position: self.automatic_position,
            from_bottom: self.from_bottom,
        }
    }
}
