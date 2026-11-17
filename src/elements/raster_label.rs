use std::sync::Arc;

use super::{label_info::LabelInfo, label_position::LabelPosition, reverse_print::ReversePrint};

/// A frozen label rendered as a bitmap when the caller's canvas is known.
/// Cloning a placement shares the immutable source rather than its full tree.
#[derive(Clone, Debug)]
pub struct RasterLabel {
    pub label: Arc<LabelInfo>,
    pub source_dpmm: i32,
    pub position: LabelPosition,
    pub magnification_x: i32,
    pub magnification_y: i32,
    pub reverse_print: ReversePrint,
}
