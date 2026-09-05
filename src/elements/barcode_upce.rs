use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct BarcodeUcpe {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub line: bool,
    pub line_above: bool,
    /// Print the check digit in the interpretation line (`^B9` parameter e).
    /// The check digit is always encoded (it selects the digit parity).
    pub check_digit: bool,
}

#[derive(Clone, Debug)]
pub struct BarcodeUcpeWithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeUcpe,
    pub width: i32,
    pub position: LabelPosition,
    pub data: String,
}
