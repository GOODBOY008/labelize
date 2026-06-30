use super::label_position::LabelPosition;
use super::line_color::LineColor;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct GraphicBox {
    pub reverse_print: ReversePrint,
    pub position: LabelPosition,
    pub width: i32,
    pub height: i32,
    pub border_thickness: i32,
    /// ZPL ^GB corner rounding value, where 1-8 is converted to a side-relative radius.
    pub corner_rounding: i32,
    /// Direct corner radius in dots for languages whose box radius is absolute, such as TSPL.
    pub corner_radius_dots: Option<i32>,
    pub line_color: LineColor,
}
