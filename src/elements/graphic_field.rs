use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicFieldFormat {
    Hex = 1,
    Raw = 2,
    AR = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicFieldMode {
    Overwrite,
    Or,
    Xor,
}

#[derive(Clone, Debug)]
pub struct GraphicField {
    pub reverse_print: ReversePrint,
    pub position: LabelPosition,
    pub format: GraphicFieldFormat,
    pub mode: GraphicFieldMode,
    pub data_bytes: i32,
    pub total_bytes: i32,
    pub row_bytes: i32,
    pub data: Vec<u8>,
    pub magnification_x: i32,
    pub magnification_y: i32,
}
