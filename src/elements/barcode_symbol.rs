use super::{
    field_orientation::FieldOrientation, label_position::LabelPosition, reverse_print::ReversePrint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostalCode {
    Postnet,
    Planet,
    IntelligentMail,
}

#[derive(Clone, Debug)]
pub enum BarcodeKind {
    Code11 {
        checks: u8,
    },
    Code49 {
        mode: Option<u8>,
    },
    Code93,
    Codablock {
        mode: u8,
        columns: usize,
        rows: usize,
        checksum: bool,
        height_is_total: bool,
    },
    MicroPdf417 {
        variant: u8,
    },
    Industrial2of5,
    Standard2of5,
    Codabar {
        start: u8,
        stop: u8,
    },
    Logmars,
    Msi {
        check: u8,
    },
    Plessey,
    DataBar {
        variant: u8,
        segments: usize,
        separator: u32,
    },
    EanSupplement,
    Tlc39 {
        micro_width: u32,
        micro_height: u32,
    },
    Postal(PostalCode),
}

#[derive(Clone, Debug)]
pub struct BarcodeSymbol {
    pub kind: BarcodeKind,
    pub orientation: FieldOrientation,
    pub module: u32,
    pub height: u32,
    pub ratio: f64,
    pub caption: bool,
    pub caption_above: bool,
    pub caption_checks: bool,
}

#[derive(Clone, Debug)]
pub struct BarcodeSymbolWithData {
    pub symbol: BarcodeSymbol,
    pub data: Vec<u8>,
    pub position: LabelPosition,
    pub reverse_print: ReversePrint,
}
