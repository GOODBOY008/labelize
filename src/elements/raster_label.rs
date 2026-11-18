use std::sync::Arc;

use super::{
    label_info::LabelInfo, label_position::LabelPosition, reverse_print::ReversePrint, LabelElement,
};

/// A frozen label rendered as a bitmap when the caller's canvas is known.
/// Cloning a placement shares the immutable source rather than its full tree.
#[derive(Clone, Debug)]
pub struct RasterLabel {
    label: Arc<LabelInfo>,
    depth: u8,
    pub source_dpmm: i32,
    pub position: LabelPosition,
    /// Page-relative loads include the source's print margins. Field placements
    /// use the destination's printable origin as their starting point.
    pub page_relative: bool,
    pub magnification_x: i32,
    pub magnification_y: i32,
    pub reverse_print: ReversePrint,
}

impl RasterLabel {
    pub fn new(label: LabelInfo, source_dpmm: i32) -> Result<Self, String> {
        // Each child already knows its depth. Several placements can share a
        // source, so following their subtrees here would repeat earlier work.
        let depth = 1 + label
            .elements
            .iter()
            .filter_map(|element| match element {
                LabelElement::RasterLabel(child) => Some(child.depth),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if depth > 16 {
            return Err("Saved image nesting exceeds 16 levels".into());
        }
        Ok(Self {
            label: Arc::new(label),
            depth,
            source_dpmm,
            position: LabelPosition::default(),
            page_relative: false,
            magnification_x: 1,
            magnification_y: 1,
            reverse_print: ReversePrint::default(),
        })
    }

    pub fn label(&self) -> &LabelInfo {
        &self.label
    }
}
