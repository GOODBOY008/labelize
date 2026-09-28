use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatamatrixRatio {
    Square = 1,
    Rectangular = 2,
}

#[derive(Clone, Debug)]
pub struct BarcodeDatamatrix {
    pub orientation: FieldOrientation,
    pub height: i32,
    pub quality: i32,
    pub columns: i32,
    pub rows: i32,
    pub format: i32,
    /// ECC 200 field-data escape; zero disables ZPL escapes (for EPL).
    pub escape: u8,
    pub ratio: Option<DatamatrixRatio>,
}

#[derive(Clone, Debug)]
pub struct BarcodeDatamatrixWithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeDatamatrix,
    pub position: LabelPosition,
    pub data: String,
    /// Original ZPL field bytes after ^FH processing, before barcode escapes.
    /// Legacy encoders consume these without a Unicode round trip.
    pub data_bytes: Option<Vec<u8>>,
}
