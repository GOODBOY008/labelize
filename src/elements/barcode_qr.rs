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

impl BarcodeQrWithData {
    /// Encoder inputs for this element: `(payload, error correction, mode)`.
    ///
    /// Historically this parsed the ZPL `^FD` format prefix (error-correction
    /// char, optional binary length); that resolution now happens in the
    /// parser, and `data`/`error_correction` already hold the resolved
    /// values. The method remains for source compatibility and simply
    /// reports them; the mode is the encoder's automatic choice.
    #[deprecated(
        since = "1.8.0",
        note = "payload prefixes are resolved by the parser; use the `data` and `error_correction` fields"
    )]
    pub fn get_input_data(
        &self,
    ) -> Result<(String, QrErrorCorrectionLevel, QrCharacterMode), String> {
        Ok((
            self.data.clone(),
            self.error_correction,
            QrCharacterMode::Automatic,
        ))
    }
}
