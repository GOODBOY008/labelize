use super::field_alignment::FieldAlignment;
use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BarcodeMode {
    #[default]
    No = 0,
    Ucc = 1,
    Automatic = 2,
    Ean = 3,
}

#[derive(Clone, Debug)]
pub struct Barcode128 {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub line: bool,
    pub line_above: bool,
    pub line_alignment: FieldAlignment,
    pub check_digit: bool,
    pub mode: BarcodeMode,
}

#[derive(Clone, Debug)]
pub struct Barcode128WithData {
    pub reverse_print: ReversePrint,
    pub barcode: Barcode128,
    pub width: i32,
    pub position: LabelPosition,
    pub data: String,
}
