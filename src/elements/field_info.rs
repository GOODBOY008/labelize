use super::field_alignment::FieldAlignment;
use super::font::FontInfo;
use super::label_element::LabelElement;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;
use super::text_flow::TextFlow;

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub reverse_print: ReversePrint,
    pub element: Option<Box<LabelElement>>,
    pub font: FontInfo,
    pub position: LabelPosition,
    pub alignment: FieldAlignment,
    pub width: i32,
    pub width_ratio: f64,
    pub height: i32,
    pub current_charset: i32,
    pub text_flow: TextFlow,
}
