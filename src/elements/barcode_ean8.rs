use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct BarcodeEan8 {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub line: bool,
    pub line_above: bool,
}

#[derive(Clone, Debug)]
pub struct BarcodeEan8WithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeEan8,
    pub width: i32,
    pub position: LabelPosition,
    pub data: String,
}
