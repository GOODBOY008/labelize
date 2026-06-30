use crate::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use crate::elements::barcode_2of5::{Barcode2of5, Barcode2of5WithData};
use crate::elements::barcode_39::{Barcode39, Barcode39WithData};
use crate::elements::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::reverse_print::ReversePrint;
use crate::elements::text_field::TextField;

pub struct EplParser;

impl Default for EplParser {
    fn default() -> Self {
        Self
    }
}

impl EplParser {
    pub fn new() -> Self {
        EplParser
    }

    pub fn parse(&self, epl_data: &[u8]) -> Result<Vec<LabelInfo>, String> {
        let data_str = String::from_utf8_lossy(epl_data);
        let lines: Vec<&str> = data_str.split('\n').collect();

        let mut results = Vec::new();
        let mut current_elements: Vec<LabelElement> = Vec::new();
        let mut ref_x = 0i32;
        let mut ref_y = 0i32;

        for raw_line in &lines {
            let line = raw_line.trim_end_matches('\r').trim();
            if line.is_empty() {
                continue;
            }

            if line == "N" {
                current_elements.clear();
                ref_x = 0;
                ref_y = 0;
                continue;
            }

            if is_epl_reference_point(line) {
                let parts: Vec<&str> = line[1..].splitn(2, ',').collect();
                if let Some(s) = parts.first() {
                    ref_x = s.trim().parse().unwrap_or(0);
                }
                if let Some(s) = parts.get(1) {
                    ref_y = s.trim().parse().unwrap_or(0);
                }
                continue;
            }

            if line.starts_with('A') {
                if let Some(el) = parse_epl_text(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                continue;
            }

            if line.starts_with('B') {
                if let Some(el) = parse_epl_barcode(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                continue;
            }

            if line.starts_with("LO") {
                if let Some(el) = parse_epl_line(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                continue;
            }

            if is_epl_print_command(line) {
                if !current_elements.is_empty() {
                    results.push(LabelInfo {
                        print_width: 0,
                        inverted: false,
                        elements: current_elements.clone(),
                    });
                }
                current_elements.clear();
            }
        }

        // Handle labels without trailing P
        if !current_elements.is_empty() {
            results.push(LabelInfo {
                print_width: 0,
                inverted: false,
                elements: current_elements,
            });
        }

        Ok(results)
    }
}

fn is_epl_reference_point(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() > 1 && bytes[0] == b'R' && bytes[1].is_ascii_digit()
}

fn is_epl_print_command(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.is_empty() || bytes[0] != b'P' {
        return false;
    }
    if bytes.len() == 1 {
        return true;
    }
    bytes[1..].iter().all(|b| b.is_ascii_digit())
}

fn epl_rotation(rotation: i32) -> FieldOrientation {
    match rotation {
        1 => FieldOrientation::Rotated90,
        2 => FieldOrientation::Rotated180,
        3 => FieldOrientation::Rotated270,
        _ => FieldOrientation::Normal,
    }
}

static EPL_FONT_SIZES: &[(i32, i32, i32)] = &[
    // (font_num, width, height)
    (1, 8, 12),
    (2, 10, 16),
    (3, 12, 20),
    (4, 14, 24),
    (5, 32, 48),
];

fn epl_font_size(font_num: i32) -> (i32, i32) {
    for &(n, w, h) in EPL_FONT_SIZES {
        if n == font_num {
            return (w, h);
        }
    }
    (8, 12) // default to font 1
}

fn parse_epl_text(line: &str, ref_x: i32, ref_y: i32) -> Result<Option<LabelElement>, String> {
    let data_start = line.find('"');
    let data_end = line.rfind('"');
    match (data_start, data_end) {
        (Some(s), Some(e)) if e > s => {
            let text = &line[s + 1..e];
            if text.is_empty() {
                return Ok(None);
            }

            let param_str = line[1..s].trim_end_matches(',');
            let parts: Vec<&str> = param_str.split(',').collect();

            if parts.len() < 7 {
                return Err(format!(
                    "EPL A command requires at least 7 parameters, got {}",
                    parts.len()
                ));
            }

            let x: i32 = parts[0].trim().parse().unwrap_or(0);
            let y: i32 = parts[1].trim().parse().unwrap_or(0);
            let rotation: i32 = parts[2].trim().parse().unwrap_or(0);
            let font_num: i32 = parts[3].trim().parse().unwrap_or(1);
            let h_mult: i32 = parts[4].trim().parse::<i32>().unwrap_or(1).max(1);
            let v_mult: i32 = parts[5].trim().parse::<i32>().unwrap_or(1).max(1);
            let reverse = parts[6].trim();

            let (base_w, base_h) = epl_font_size(font_num);

            let font_height = (base_h * v_mult) as f64;
            let font_width = if h_mult != v_mult {
                font_height * (h_mult * base_w) as f64 / (v_mult * base_h) as f64
            } else {
                font_height
            };

            Ok(Some(LabelElement::Text(TextField {
                reverse_print: ReversePrint {
                    value: reverse == "R",
                },
                font: FontInfo {
                    name: "0".to_string(),
                    width: font_width,
                    height: font_height,
                    orientation: epl_rotation(rotation),
                },
                position: LabelPosition {
                    x: x + ref_x,
                    y: y + ref_y,
                    ..Default::default()
                },
                text: text.to_string(),
                alignment: Default::default(),
                block: None,
            })))
        }
        _ => Ok(None),
    }
}

fn parse_epl_barcode(line: &str, ref_x: i32, ref_y: i32) -> Result<Option<LabelElement>, String> {
    let data_start = line.find('"');
    let data_end = line.rfind('"');
    match (data_start, data_end) {
        (Some(s), Some(e)) if e > s => {
            let data = &line[s + 1..e];
            if data.is_empty() {
                return Ok(None);
            }

            let param_str = line[1..s].trim_end_matches(',');
            let parts: Vec<&str> = param_str.split(',').collect();

            if parts.len() < 8 {
                return Err(format!(
                    "EPL B command requires at least 8 parameters, got {}",
                    parts.len()
                ));
            }

            let x: i32 = parts[0].trim().parse().unwrap_or(0);
            let y: i32 = parts[1].trim().parse().unwrap_or(0);
            let rotation: i32 = parts[2].trim().parse().unwrap_or(0);
            let bc_type = parts[3].trim();
            let narrow_bar: i32 = parts[4].trim().parse::<i32>().unwrap_or(1).max(1);
            let wide_bar: i32 = parts[5].trim().parse().unwrap_or(2);
            let height: i32 = parts[6].trim().parse::<i32>().unwrap_or(10).max(1);
            let human_readable = parts[7].trim();

            let pos = LabelPosition {
                x: x + ref_x,
                y: y + ref_y,
                ..Default::default()
            };
            let orient = epl_rotation(rotation);
            let show_line = human_readable == "B";
            let width_ratio = (wide_bar as f64 / narrow_bar as f64).max(2.0);

            let el = match bc_type {
                "0" => LabelElement::Barcode39(Barcode39WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: Barcode39 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        line_alignment: FieldAlignment::Center,
                        check_digit: false,
                    },
                    width: narrow_bar,
                    width_ratio,
                    position: pos,
                    data: data.to_string(),
                }),
                "B" => LabelElement::BarcodeEan13(BarcodeEan13WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: BarcodeEan13 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        line_alignment: FieldAlignment::Center,
                    },
                    width: narrow_bar,
                    position: pos,
                    data: data.to_string(),
                }),
                "G" | "H" => LabelElement::Barcode2of5(Barcode2of5WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: Barcode2of5 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        line_alignment: FieldAlignment::Center,
                        check_digit: false,
                    },
                    width: narrow_bar,
                    width_ratio,
                    position: pos,
                    data: data.to_string(),
                }),
                _ => {
                    // Default to Code 128 Auto
                    LabelElement::Barcode128(Barcode128WithData {
                        reverse_print: ReversePrint::default(),
                        barcode: Barcode128 {
                            orientation: orient,
                            height,
                            line: show_line,
                            line_above: false,
                            line_alignment: FieldAlignment::Center,
                            check_digit: false,
                            mode: BarcodeMode::Automatic,
                        },
                        width: narrow_bar,
                        position: pos,
                        data: data.to_string(),
                    })
                }
            };

            Ok(Some(el))
        }
        _ => Ok(None),
    }
}

fn parse_epl_line(line: &str, ref_x: i32, ref_y: i32) -> Result<Option<LabelElement>, String> {
    let param_str = &line[2..]; // Skip "LO"
    let parts: Vec<&str> = param_str.split(',').collect();

    if parts.len() < 4 {
        return Err(format!(
            "EPL LO command requires 4 parameters, got {}",
            parts.len()
        ));
    }

    let x: i32 = parts[0].trim().parse().unwrap_or(0);
    let y: i32 = parts[1].trim().parse().unwrap_or(0);
    let width: i32 = parts[2].trim().parse::<i32>().unwrap_or(1).max(1);
    let height: i32 = parts[3].trim().parse::<i32>().unwrap_or(1).max(1);

    Ok(Some(LabelElement::GraphicBox(GraphicBox {
        position: LabelPosition {
            x: x + ref_x,
            y: y + ref_y,
            ..Default::default()
        },
        width,
        height,
        border_thickness: width.min(height),
        corner_rounding: 0,
        corner_radius_dots: None,
        line_color: LineColor::Black,
        reverse_print: ReversePrint::default(),
    })))
}
