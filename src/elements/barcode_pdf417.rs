use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct BarcodePdf417 {
    pub orientation: FieldOrientation,
    pub row_height: i32,
    pub security: i32,
    pub columns: i32,
    pub rows: i32,
    pub truncate: bool,
}

#[derive(Clone, Debug)]
pub struct BarcodePdf417WithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodePdf417,
    pub position: LabelPosition,
    pub data: String,
    pub module_width: i32,
    pub by_height: i32,
}
