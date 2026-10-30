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
    /// ECC 200 data-stream escape character (the producing language's
    /// escape flavor, e.g. ZPL's ^BX `>` sequences); zero means the bytes
    /// are literal and no escape processing applies.
    pub escape: u8,
    pub ratio: Option<DatamatrixRatio>,
}

#[derive(Clone, Debug)]
pub struct BarcodeDatamatrixWithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeDatamatrix,
    pub position: LabelPosition,
    pub data: String,
    /// Raw bytes the producer decoded, pre-escape. Legacy encoders and
    /// literal ECC 200 fields (`escape == 0`) consume these without a
    /// Unicode round trip; `None` falls back to `data`'s bytes.
    pub data_bytes: Option<Vec<u8>>,
}
