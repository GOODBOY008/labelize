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
        if self.data.len() < 4 {
            return Err("invalid qr barcode data".to_string());
        }

        let bytes = self.data.as_bytes();
        let mut data = &self.data[3..];
        let mut mode = QrCharacterMode::Automatic;
        let level = match bytes[0] {
            b'H' => QrErrorCorrectionLevel::H,
            b'Q' => QrErrorCorrectionLevel::Q,
            b'M' => QrErrorCorrectionLevel::M,
            b'L' => QrErrorCorrectionLevel::L,
            // Unrecognized format indicator: Labelary defaults to H error correction
            _ => QrErrorCorrectionLevel::H,
        };

        if bytes[1] == b'M' && !data.is_empty() {
            mode = match data.as_bytes()[0] {
                b'B' => QrCharacterMode::Binary,
                b'N' => QrCharacterMode::Numeric,
                b'A' => QrCharacterMode::Alphanumeric,
                b'K' => QrCharacterMode::Kanji,
                _ => QrCharacterMode::Automatic,
            };
            data = &data[1..];
        }

        if mode != QrCharacterMode::Binary {
            // Zebra printers treat `|` as a field separator in QR data; it is stripped
            // from the encoded content (not included in the QR symbol), matching Labelary behavior.
            let content = data.replace('|', "");
            return Ok((content, level, mode));
        }

        if data.len() < 5 {
            return Err("invalid qr barcode byte mode data".to_string());
        }

        let data_len: usize = data[0..4]
            .parse()
            .map_err(|_| "invalid qr barcode byte mode data length".to_string())?;

        let data = &data[4..];
        let data_len = data_len.min(data.len());

        Ok((data[..data_len].to_string(), level, mode))
    }
}
