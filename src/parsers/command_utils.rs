use crate::elements::barcode_128::BarcodeMode;
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::text_alignment::TextAlignment;

pub fn split_command<'a>(command: &'a str, prefix: &str) -> Vec<&'a str> {
    let data = &command[prefix.len()..];
    data.split(',').collect()
}

pub fn command_text<'a>(command: &'a str, prefix: &str) -> &'a str {
    &command[prefix.len()..]
}

pub fn to_field_orientation(b: u8) -> FieldOrientation {
    match b {
        b'N' => FieldOrientation::Normal,
        b'R' => FieldOrientation::Rotated90,
        b'I' => FieldOrientation::Rotated180,
        b'B' => FieldOrientation::Rotated270,
        _ => FieldOrientation::Normal,
    }
}

pub fn to_field_alignment(s: &str) -> Option<FieldAlignment> {
    match s.trim().parse::<i32>() {
        Ok(0) => Some(FieldAlignment::Left),
        Ok(1) => Some(FieldAlignment::Right),
        Ok(2) => Some(FieldAlignment::Auto),
        _ => None,
    }
}

pub fn to_text_alignment(b: u8) -> TextAlignment {
    match b {
        b'L' => TextAlignment::Left,
        b'R' => TextAlignment::Right,
        b'J' => TextAlignment::Justified,
        b'C' => TextAlignment::Center,
        _ => TextAlignment::Left,
    }
}

pub fn to_barcode_mode(b: u8) -> BarcodeMode {
    match b {
        b'U' => BarcodeMode::Ucc,
        b'A' => BarcodeMode::Automatic,
        b'D' => BarcodeMode::Ean,
        _ => BarcodeMode::No,
    }
}

pub fn to_bool_field(b: u8) -> bool {
    b == b'Y'
}

pub fn to_positive_int(s: &str) -> Option<i32> {
    s.trim().parse::<f64>().ok().map(|v| v.abs().round() as i32)
}

/// Like [`to_positive_int`] but tolerates leading garbage before the number, matching
/// printer/Labelary leniency for malformed numeric parameters (e.g. `^FOB50,660` -> x=50,
/// observed verbatim in courier-generated ZPL). Strict parse is tried first so well-formed
/// input is unaffected.
pub fn to_positive_int_lenient(s: &str) -> Option<i32> {
    let t = s.trim();
    to_positive_int(t).or_else(|| {
        let start = t.find(|c: char| c.is_ascii_digit())?;
        let digits: String = t[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        to_positive_int(&digits)
    })
}

pub fn parse_int(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

pub fn parse_float(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

pub fn parse_int_ceil(s: &str) -> Option<i32> {
    s.trim().parse::<f64>().ok().map(|v| v.ceil() as i32)
}
