use super::label_position::LabelPosition;

/// A rectangular region whose final ink is inverted at print time (black
/// becomes white and vice versa), e.g. TSPL `REVERSE x,y,w,h` — text drawn
/// inside the region reads white-on-black. The renderer applies these as a
/// post-pass over the finished label, before any whole-label
/// transformations.
#[derive(Clone, Debug)]
pub struct InvertRegion {
    pub position: LabelPosition,
    pub width: i32,
    pub height: i32,
}
