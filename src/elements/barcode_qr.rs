use super::field_orientation::FieldOrientation;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrErrorCorrectionLevel {
    H,
    Q,
    M,
    L,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrCharacterMode {
    Automatic,
    Binary,
    Numeric,
    Alphanumeric,
    Kanji,
}

#[derive(Clone, Debug)]
pub struct BarcodeQr {
    pub magnification: i32,
    /// Field rotation (TSPL `QRCODE` parameter; ZPL rotates QR via ^FW,
    /// which is not modeled yet and stays Normal).
    pub orientation: FieldOrientation,
}

#[derive(Clone, Debug)]
pub struct BarcodeQrWithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeQr,
    pub height: i32,
    pub position: LabelPosition,
    /// Raw QR payload — format prefixes, field-separator conventions, and
    /// binary length prefixes are the producing language's concern and are
    /// resolved by the parser before the element is built.
    pub data: String,
    /// Error-correction level resolved by the parser (ZPL takes it from the
    /// ^FD format char, other languages from their own parameters).
    pub error_correction: QrErrorCorrectionLevel,
}
