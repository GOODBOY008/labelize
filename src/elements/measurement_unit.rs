/// Unit of measurement selected by `^MU` (`a` parameter).
///
/// Applies to every dot-valued ZPL parameter (coordinates, box/line geometry, font
/// heights, barcode module width and height) until a new `^MU` is issued.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeasurementUnit {
    #[default]
    Dots,
    Inches,
    Millimeters,
}

impl MeasurementUnit {
    pub fn from_byte(b: u8) -> Option<MeasurementUnit> {
        match b.to_ascii_uppercase() {
            b'D' => Some(MeasurementUnit::Dots),
            b'I' => Some(MeasurementUnit::Inches),
            b'M' => Some(MeasurementUnit::Millimeters),
            _ => None,
        }
    }

    /// Dots per unit for a printer of the given resolution.
    pub fn dots_per_unit(&self, dpmm: f64) -> f64 {
        match self {
            MeasurementUnit::Dots => 1.0,
            MeasurementUnit::Inches => dpmm * 25.4,
            MeasurementUnit::Millimeters => dpmm,
        }
    }
}
