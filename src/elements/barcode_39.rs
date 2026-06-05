use super::field_alignment::FieldAlignment;
use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct Barcode39 {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub line: bool,
    pub line_above: bool,
    pub line_alignment: FieldAlignment,
    pub check_digit: bool,
}

#[derive(Clone, Debug)]
pub struct Barcode39WithData {
    pub reverse_print: ReversePrint,
    pub barcode: Barcode39,
    pub width: i32,
    pub width_ratio: f64,
    pub position: LabelPosition,
    pub data: String,
}
