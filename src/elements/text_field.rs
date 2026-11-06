use super::field_alignment::FieldAlignment;
use super::field_block::FieldBlock;
use super::font::FontInfo;
use super::label_position::LabelPosition;
use super::reverse_print::ReversePrint;
use super::text_flow::TextFlow;

#[derive(Clone, Debug)]
pub struct TextField {
    pub reverse_print: ReversePrint,
    pub font: FontInfo,
    pub position: LabelPosition,
    pub alignment: FieldAlignment,
    pub text: String,
    pub flow: TextFlow,
    pub block: Option<FieldBlock>,
}
