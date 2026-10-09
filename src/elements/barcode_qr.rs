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
}

#[derive(Clone, Debug)]
pub struct BarcodeQrWithData {
    pub reverse_print: ReversePrint,
    pub barcode: BarcodeQr,
    pub height: i32,
    pub position: LabelPosition,
    pub data: String,
}

impl BarcodeQrWithData {
    pub fn get_input_data(
        &self,
    ) -> Result<(String, QrErrorCorrectionLevel, QrCharacterMode), String> {
        // Only the two format bytes (error-correction char + mode char) are
        // required; a prefix-only field like `QA,` parses to an empty payload,
        // which the renderer skips like Labelary instead of failing the label.
        if self.data.len() < 2 {
            return Err("invalid qr barcode data".to_string());
        }

        let bytes = self.data.as_bytes();
        let level = match bytes[0] {
            b'H' => QrErrorCorrectionLevel::H,
            b'Q' => QrErrorCorrectionLevel::Q,
            b'M' => QrErrorCorrectionLevel::M,
            b'L' => QrErrorCorrectionLevel::L,
            // Unrecognized format indicator: Labelary defaults to H error correction
            _ => QrErrorCorrectionLevel::H,
        };

        // The ECC char and the input-mode designator are always consumed. A
        // recognized letter designator (A/M/…) consumes exactly one separator
        // char after it (the comma in `QA,data`), while an unrecognized one —
        // commonly a digit in a tracking number — consumes nothing and the
        // payload starts at index 2. Pipes are field separators only for the
        // recognized manual modes; Labelary keeps them verbatim otherwise
        // (probe-verified across the QR corpus, see docs/QR_MASK_SELECTION.md).
        let manual = bytes[1] == b'M';
        let skip = if manual || bytes[1].is_ascii_alphabetic() { 3 } else { 2 };
        let mut data = self
            .data
            .get(skip..)
            .ok_or_else(|| "invalid qr barcode data prefix".to_string())?;
        let mut mode = QrCharacterMode::Automatic;

        if manual && !data.is_empty() {
            mode = match data.as_bytes()[0] {
                b'B' => QrCharacterMode::Binary,
                b'N' => QrCharacterMode::Numeric,
                b'A' => QrCharacterMode::Alphanumeric,
                b'K' => QrCharacterMode::Kanji,
                _ => QrCharacterMode::Automatic,
            };
            data = data
                .get(1..)
                .ok_or_else(|| "invalid qr barcode character mode".to_string())?;
        }

        if mode != QrCharacterMode::Binary {
            // `|` is a field separator only for the recognized modes: manual
            // submodes and the explicit automatic `A` designator strip it from
            // the encoded content (matching Labelary); an unrecognized letter
            // or digit designator keeps the payload verbatim (probe-verified,
            // see docs/QR_MASK_SELECTION.md).
            if manual || bytes[1] == b'A' {
                return Ok((data.replace('|', ""), level, mode));
            }
            return Ok((data.to_string(), level, mode));
        }

        if data.len() < 4 {
            return Err("invalid qr barcode byte mode data".to_string());
        }

        let data_len: usize = data
            .get(..4)
            .ok_or_else(|| "invalid qr barcode byte mode data length".to_string())?
            .parse()
            .map_err(|_| "invalid qr barcode byte mode data length".to_string())?;

        let data = &data[4..];
        let data_len = data_len.min(data.len());

        let content = data
            .get(..data_len)
            .ok_or_else(|| "qr barcode byte mode length splits a UTF-8 character".to_string())?;
        Ok((content.to_string(), level, mode))
    }
}
