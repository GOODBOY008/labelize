use super::field_alignment::FieldAlignment;
use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct BarcodeEan13 {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub line: bool,
    pub line_above: bool,
    pub line_alignment: FieldAlignment,
}

#[derive(Clone, Debug)]
pub struct BarcodeEan13WithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeEan13,
    pub width: i32,
    pub position: LabelPosition,
    pub data: String,
}
