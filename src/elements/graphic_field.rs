use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicFieldFormat {
    Hex = 1,
    Raw = 2,
    AR = 3,
}

/// How a graphic field combines with what is already on the canvas.
/// On a 1-bit canvas `Or` is indistinguishable from `Overwrite` (black is
/// absorbing); `Xor` inverts overlapped ink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphicFieldMode {
    #[default]
    Overwrite,
    Or,
    Xor,
}

#[derive(Clone, Debug)]
pub struct GraphicField {
    pub reverse_print: ReversePrint,
    pub position: LabelPosition,
    pub format: GraphicFieldFormat,
    pub data_bytes: i32,
    pub total_bytes: i32,
    pub row_bytes: i32,
    pub data: Vec<u8>,
    pub magnification_x: i32,
    pub magnification_y: i32,
    /// Blend with existing canvas content (default: overwrite).
    pub mode: GraphicFieldMode,
    /// Polarity of the packed data: cleared bits print black (TSC BITMAP
    /// convention). The default — set bits print black — matches ^GF/GW.
    pub zero_bits_black: bool,
}
