use crate::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use crate::elements::barcode_2of5::{Barcode2of5, Barcode2of5WithData};
use crate::elements::barcode_39::{Barcode39, Barcode39WithData};
use crate::elements::barcode_aztec::{BarcodeAztec, BarcodeAztecWithData};
use crate::elements::barcode_datamatrix::{
    BarcodeDatamatrix, BarcodeDatamatrixWithData, DatamatrixRatio,
};
use crate::elements::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use crate::elements::barcode_ean8::{BarcodeEan8, BarcodeEan8WithData};
use crate::elements::barcode_pdf417::{BarcodePdf417, BarcodePdf417WithData};
use crate::elements::barcode_qr::{BarcodeQr, BarcodeQrWithData};
use crate::elements::barcode_upca::{BarcodeUca, BarcodeUcaWithData};
use crate::elements::barcode_upce::{BarcodeUcpe, BarcodeUcpeWithData};
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::graphic_diagonal_line::GraphicDiagonalLine;
use crate::elements::graphic_field::{GraphicField, GraphicFieldFormat};
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::maxicode::{Maxicode, MaxicodeWithData};
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
        let mut results = Vec::new();
        let mut current_elements: Vec<LabelElement> = Vec::new();
        let mut ref_x = 0i32;
        let mut ref_y = 0i32;

        // The input is walked with a byte cursor instead of pre-splitting into
        // lines: `GW` embeds raw binary data that may contain newline bytes,
        // and its data block must be consumed from the byte stream directly.
        let mut cursor = 0usize;
        while cursor < epl_data.len() {
            let line_end = epl_data[cursor..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(epl_data.len(), |p| cursor + p);
            let raw_line = &epl_data[cursor..line_end];
            let next_cursor = if line_end < epl_data.len() {
                line_end + 1
            } else {
                epl_data.len()
            };

            // GW carries raw binary graphic data that may extend past this
            // line's newline; on success the cursor jumps past the data block.
            if let Some((element, end)) =
                parse_epl_graphic_write(epl_data, cursor, line_end, ref_x, ref_y)?
            {
                current_elements.push(element);
                cursor = end;
                continue;
            }

            let line = String::from_utf8_lossy(raw_line);
            let line = line.trim_end_matches('\r').trim();

            if line.is_empty() {
                cursor = next_cursor;
                continue;
            }

            if line == "N" {
                current_elements.clear();
                ref_x = 0;
                ref_y = 0;
                cursor = next_cursor;
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
                cursor = next_cursor;
                continue;
            }

            if line.starts_with('A') {
                if let Some(el) = parse_epl_text(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            if line.starts_with('B') {
                if let Some(el) = parse_epl_barcode(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            if line.starts_with('b') {
                if let Some(el) = parse_epl_2d_barcode(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            if line.starts_with("LO") {
                if let Some(el) = parse_epl_line(line, ref_x, ref_y, LineColor::Black)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            if line.starts_with("LW") {
                if let Some(el) = parse_epl_line(line, ref_x, ref_y, LineColor::White)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            // LE (exclusive-OR line) is intentionally left unparsed: the
            // renderer has no XOR compositing, so no element can express it.

            if line.starts_with("LS") {
                if let Some(el) = parse_epl_diagonal(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
                continue;
            }

            if line.starts_with('X') {
                if let Some(el) = parse_epl_box(line, ref_x, ref_y)? {
                    current_elements.push(el);
                }
                cursor = next_cursor;
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

            cursor = next_cursor;
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
                // Bar code selection per Table 1 of the EPL Programming Guide
                // (14245L-003 Rev A). Codes are exact, multi-character values.
                "3" | "3C" => LabelElement::Barcode39(Barcode39WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: Barcode39 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        check_digit: bc_type == "3C",
                    },
                    width: narrow_bar,
                    width_ratio,
                    position: pos,
                    data: data.to_string(),
                }),
                "0" | "1" | "1A" | "1B" | "1C" | "1E" => {
                    LabelElement::Barcode128(Barcode128WithData {
                        reverse_print: ReversePrint::default(),
                        barcode: Barcode128 {
                            orientation: orient,
                            height,
                            line: show_line,
                            line_above: false,
                            check_digit: false,
                            mode: match bc_type {
                                "0" => BarcodeMode::Ucc,
                                "1E" => BarcodeMode::Ean,
                                // "1A"/"1B"/"1C" pin a Code 128 subset; BarcodeMode
                                // has no subset variants, so Automatic re-encodes
                                // the same data.
                                _ => BarcodeMode::Automatic,
                            },
                        },
                        width: narrow_bar,
                        position: pos,
                        data: data.to_string(),
                    })
                }
                "2" | "2C" | "2D" => LabelElement::Barcode2of5(Barcode2of5WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: Barcode2of5 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        check_digit: bc_type != "2",
                    },
                    width: narrow_bar,
                    width_ratio,
                    position: pos,
                    data: data.to_string(),
                }),
                "E30" => LabelElement::BarcodeEan13(BarcodeEan13WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: BarcodeEan13 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                    },
                    width: narrow_bar,
                    position: pos,
                    data: data.to_string(),
                }),
                "E80" => LabelElement::BarcodeEan8(BarcodeEan8WithData {
                    reverse_print: ReversePrint::default(),
                    barcode: BarcodeEan8 {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                    },
                    width: narrow_bar,
                    position: pos,
                    data: data.to_string(),
                }),
                "UA0" => LabelElement::BarcodeUca(BarcodeUcaWithData {
                    reverse_print: ReversePrint::default(),
                    barcode: BarcodeUca {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                    },
                    width: narrow_bar,
                    position: pos,
                    data: data.to_string(),
                }),
                "UE0" => LabelElement::BarcodeUcpe(BarcodeUcpeWithData {
                    reverse_print: ReversePrint::default(),
                    barcode: BarcodeUcpe {
                        orientation: orient,
                        height,
                        line: show_line,
                        line_above: false,
                        // Same default as ZPL ^B9: check digit in the line.
                        check_digit: true,
                    },
                    width: narrow_bar,
                    position: pos,
                    data: data.to_string(),
                }),
                other => {
                    return Err(format!(
                        "EPL B command: unsupported bar code type \"{}\" ({})",
                        other,
                        epl_barcode_type_name(other)
                    ));
                }
            };

            Ok(Some(el))
        }
        _ => Ok(None),
    }
}

/// Names for EPL2 B-command bar code types that have no labelize encoder;
/// used in the unsupported-type error message.
fn epl_barcode_type_name(code: &str) -> &'static str {
    match code {
        "9" => "Code 93",
        "1D" => "Code 128 with Deutsche Post check digit",
        "K" => "Codabar",
        "E82" => "EAN-8 2-digit add-on",
        "E85" => "EAN-8 5-digit add-on",
        "E32" => "EAN-13 2-digit add-on",
        "E35" => "EAN-13 5-digit add-on",
        "UA2" => "UPC-A 2-digit add-on",
        "UA5" => "UPC-A 5-digit add-on",
        "UE2" => "UPC-E 2-digit add-on",
        "UE5" => "UPC-E 5-digit add-on",
        "2G" => "German Post Code",
        "2U" => "UPC Interleaved 2 of 5",
        "P" => "Postnet",
        "PL" => "Planet",
        "J" => "Japanese Postnet",
        "L" => "Plessey (MSI-1)",
        "M" => "MSI-3",
        _ => "unknown bar code type",
    }
}

fn parse_epl_line(
    line: &str,
    ref_x: i32,
    ref_y: i32,
    line_color: LineColor,
) -> Result<Option<LabelElement>, String> {
    let param_str = &line[2..]; // Skip "LO"/"LW"
    let parts: Vec<&str> = param_str.split(',').collect();

    if parts.len() < 4 {
        return Err(format!(
            "EPL {} command requires 4 parameters, got {}",
            &line[..2],
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
        line_color,
        reverse_print: ReversePrint::default(),
    })))
}

/// `LS,x1,y1,thickness,x2,y2` -- Line Draw Diagonal between two points.
fn parse_epl_diagonal(line: &str, ref_x: i32, ref_y: i32) -> Result<Option<LabelElement>, String> {
    let parts: Vec<&str> = line[2..].split(',').collect();
    if parts.len() < 5 {
        return Err(format!(
            "EPL LS command requires 5 parameters, got {}",
            parts.len()
        ));
    }

    let x1: i32 = parts[0].trim().parse().unwrap_or(0);
    let y1: i32 = parts[1].trim().parse().unwrap_or(0);
    let thickness: i32 = parts[2].trim().parse::<i32>().unwrap_or(1).max(1);
    let x2: i32 = parts[3].trim().parse().unwrap_or(0);
    let y2: i32 = parts[4].trim().parse().unwrap_or(0);

    let left = x1.min(x2);
    let top = y1.min(y2);
    // dx and dy with the same sign draw a "\" (top-left to bottom-right);
    // opposite signs draw a "/".
    let top_to_bottom = (x2 - x1).signum() == (y2 - y1).signum();

    Ok(Some(LabelElement::DiagonalLine(GraphicDiagonalLine {
        reverse_print: ReversePrint::default(),
        position: LabelPosition {
            x: left + ref_x,
            y: top + ref_y,
            ..Default::default()
        },
        width: (x2 - x1).abs(),
        height: (y2 - y1).abs(),
        border_thickness: thickness,
        line_color: LineColor::Black,
        top_to_bottom,
    })))
}

/// `X,x1,y1,thickness,x2,y2` -- Box Draw defined by its two corners and the
/// border thickness.
fn parse_epl_box(line: &str, ref_x: i32, ref_y: i32) -> Result<Option<LabelElement>, String> {
    let parts: Vec<&str> = line[1..].split(',').collect();
    if parts.len() < 5 {
        return Err(format!(
            "EPL X command requires 5 parameters, got {}",
            parts.len()
        ));
    }

    let x1: i32 = parts[0].trim().parse().unwrap_or(0);
    let y1: i32 = parts[1].trim().parse().unwrap_or(0);
    let thickness: i32 = parts[2].trim().parse().unwrap_or(1).max(1);
    let x2: i32 = parts[3].trim().parse().unwrap_or(0);
    let y2: i32 = parts[4].trim().parse().unwrap_or(0);

    Ok(Some(LabelElement::GraphicBox(GraphicBox {
        position: LabelPosition {
            x: x1.min(x2) + ref_x,
            y: y1.min(y2) + ref_y,
            ..Default::default()
        },
        width: (x2 - x1).abs(),
        height: (y2 - y1).abs(),
        border_thickness: thickness,
        corner_rounding: 0,
        line_color: LineColor::Black,
        reverse_print: ReversePrint::default(),
    })))
}

/// `GW,x,y,width_bytes,lines,<raw binary>` -- Direct Graphic Write.
///
/// The graphic data is raw binary (one bit per dot, 8 dots per byte per row)
/// that may contain newline bytes, so the data block is consumed straight
/// from the byte stream. Returns the element and the byte offset just past
/// the data block, or `None` when the line is not a GW command.
fn parse_epl_graphic_write(
    epl_data: &[u8],
    line_start: usize,
    line_end: usize,
    ref_x: i32,
    ref_y: i32,
) -> Result<Option<(LabelElement, usize)>, String> {
    let raw_line = &epl_data[line_start..line_end];
    let Some(first) = raw_line.iter().position(|b| !b.is_ascii_whitespace()) else {
        return Ok(None);
    };
    if !raw_line[first..].starts_with(b"GW") {
        return Ok(None);
    }

    // Scan the four comma-separated parameters within this line; the binary
    // data begins immediately after the fourth comma.
    let header = &raw_line[first + 2..];
    let mut comma_positions = header
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b',')
        .map(|(i, _)| i);
    let (Some(c1), Some(c2), Some(c3), Some(c4)) = (
        comma_positions.next(),
        comma_positions.next(),
        comma_positions.next(),
        comma_positions.next(),
    ) else {
        return Err(
            "EPL GW command requires 4 parameters: x, y, width (bytes), length (lines)".to_string(),
        );
    };

    let x = epl_parse_int(&header[..c1]).unwrap_or(0);
    let y = epl_parse_int(&header[c1 + 1..c2]).unwrap_or(0);
    let width_bytes = epl_parse_int(&header[c2 + 1..c3]).unwrap_or(0);
    let lines = epl_parse_int(&header[c3 + 1..c4]).unwrap_or(0);
    if width_bytes < 0 || lines < 0 {
        return Err("EPL GW command parameters must not be negative".to_string());
    }

    let data_start = line_start + first + 2 + c4 + 1;
    // Checked arithmetic: absurd dimensions must error, not wrap around the
    // truncation check (usize is 32-bit on some targets).
    let dimensions_overflow = || "EPL GW graphic dimensions overflow".to_string();
    let total = (width_bytes as usize)
        .checked_mul(lines as usize)
        .ok_or_else(dimensions_overflow)?;
    let data_end = data_start
        .checked_add(total)
        .ok_or_else(dimensions_overflow)?;
    if data_end > epl_data.len() {
        return Err(format!(
            "EPL GW graphic data truncated: need {} bytes, have {}",
            total,
            epl_data.len().saturating_sub(data_start)
        ));
    }

    let element = LabelElement::GraphicField(GraphicField {
        reverse_print: ReversePrint::default(),
        position: LabelPosition {
            x: x + ref_x,
            y: y + ref_y,
            ..Default::default()
        },
        format: GraphicFieldFormat::Raw,
        data_bytes: total as i32,
        total_bytes: total as i32,
        row_bytes: width_bytes,
        data: epl_data[data_start..data_end].to_vec(),
        magnification_x: 1,
        magnification_y: 1,
    });

    Ok(Some((element, data_end)))
}

fn epl_parse_int(bytes: &[u8]) -> Option<i32> {
    std::str::from_utf8(bytes).ok()?.trim().parse().ok()
}

/// `b,x,y,<A|D|M|P|Q>[,prefix-params...],"DATA"` -- 2D bar codes.
///
/// Optional parameters are letter-prefixed tokens (e.g. `s4`, `o1`); per the
/// manual the commas between them are optional, so a token may hold several
/// concatenated params. Unsupported options are ignored.
fn parse_epl_2d_barcode(
    line: &str,
    ref_x: i32,
    ref_y: i32,
) -> Result<Option<LabelElement>, String> {
    let data_start = line.find('"');
    let data_end = line.rfind('"');
    match (data_start, data_end) {
        (Some(s), Some(e)) if e > s => {
            let content = &line[s + 1..e];
            if content.is_empty() {
                return Ok(None);
            }

            let param_str = line[1..s].trim_end_matches(',');
            let parts: Vec<&str> = param_str.split(',').collect();
            if parts.len() < 3 {
                return Err(format!(
                    "EPL b command requires at least 3 parameters, got {}",
                    parts.len()
                ));
            }

            let x: i32 = parts[0].trim().parse().unwrap_or(0);
            let y: i32 = parts[1].trim().parse().unwrap_or(0);
            let bc_type = parts[2].trim();
            let params = parse_epl_2d_params(&parts[3..]);
            let pos = LabelPosition {
                x: x + ref_x,
                y: y + ref_y,
                ..Default::default()
            };

            let el = match bc_type {
                "A" => {
                    let mut magnification = 3i32; // manual default for d
                    let mut size = 0i32;
                    for (p, v) in &params {
                        match p {
                            'd' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    magnification = n.clamp(1, 55);
                                }
                            }
                            'e' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    // 1-99 = EC%, 101-104 compact layers,
                                    // 201-232 full layers, 300 rune symbol.
                                    size = match n {
                                        1..=99 | 101..=104 | 201..=232 | 300 => n,
                                        _ => 0,
                                    };
                                }
                            }
                            // f (flg format), m (menu), r (inverse) unsupported
                            _ => {}
                        }
                    }
                    LabelElement::BarcodeAztec(BarcodeAztecWithData {
                        reverse_print: ReversePrint::default(),
                        barcode: BarcodeAztec {
                            orientation: FieldOrientation::Normal,
                            magnification,
                            size,
                        },
                        position: pos,
                        data: content.to_string(),
                    })
                }
                "D" => {
                    let mut columns = 0i32;
                    let mut rows = 0i32;
                    let mut module = 5i32; // manual default for h
                    for (p, v) in &params {
                        match p {
                            'c' => columns = v.parse().unwrap_or(0),
                            'r' => rows = v.parse().unwrap_or(0),
                            'h' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    module = n.clamp(1, 40);
                                }
                            }
                            // v (inverse) unsupported
                            _ => {}
                        }
                    }
                    LabelElement::BarcodeDatamatrix(BarcodeDatamatrixWithData {
                        reverse_print: ReversePrint::default(),
                        barcode: BarcodeDatamatrix {
                            orientation: FieldOrientation::Normal,
                            height: module,
                            quality: 0,
                            columns,
                            rows,
                            format: 6,
                            escape: b'~',
                            ratio: Some(DatamatrixRatio::Square),
                        },
                        position: pos,
                        data: content.to_string(),
                    })
                }
                "M" => {
                    let mut mode = None;
                    for (p, v) in &params {
                        if *p == 'm' {
                            mode = v.parse::<i32>().ok().filter(|m| matches!(m, 2 | 3 | 4 | 6));
                        }
                        // Associated-symbol numbering is unsupported (and its
                        // exact prefix is undocumented); other params ignored.
                    }
                    LabelElement::Maxicode(MaxicodeWithData {
                        reverse_print: ReversePrint::default(),
                        code: Maxicode {
                            mode: mode.unwrap_or_else(|| auto_maxicode_mode(content)),
                        },
                        position: pos,
                        data: content.to_string(),
                    })
                }
                "P" => {
                    let mut security = 0i32;
                    let mut module_width = 6i32; // manual: auto selects 6
                    let mut row_height = 0i32;
                    let mut rows = 0i32;
                    let mut columns = 0i32;
                    let mut truncate = false;
                    let mut orientation = FieldOrientation::Normal;
                    for (p, v) in &params {
                        match p {
                            's' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    security = n.clamp(1, 8);
                                }
                            }
                            'x' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    module_width = n.clamp(2, 9);
                                }
                            }
                            'y' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    row_height = n.clamp(4, 99);
                                }
                            }
                            'r' => rows = v.parse().unwrap_or(0),
                            'l' => columns = v.parse().unwrap_or(0),
                            // EPL documents r/l as auto-select maxima; they are
                            // applied as exact dimensions (ZPL ^B7 semantics).
                            't' => truncate = v == "1",
                            'o' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    orientation = epl_rotation(n);
                                }
                            }
                            // w/h (max print box), c (compaction), p (human
                            // readable), f (origin) unsupported
                            _ => {}
                        }
                    }
                    if row_height == 0 {
                        // Manual default: bar height = 4 x module width.
                        row_height = 4 * module_width.max(1);
                    }
                    LabelElement::BarcodePdf417(BarcodePdf417WithData {
                        reverse_print: ReversePrint::default(),
                        barcode: BarcodePdf417 {
                            orientation,
                            row_height,
                            security,
                            columns,
                            rows,
                            truncate,
                            module_width,
                            by_height: 0,
                        },
                        position: pos,
                        data: content.to_string(),
                    })
                }
                "Q" => {
                    let mut magnification = 3i32; // manual default for s
                    let mut ecc = 'M';
                    for (p, v) in &params {
                        match p {
                            's' => {
                                if let Ok(n) = v.parse::<i32>() {
                                    magnification = n.clamp(1, 100);
                                }
                            }
                            'e' => {
                                if let Some(c) = v.chars().next() {
                                    if matches!(c, 'L' | 'M' | 'Q' | 'H') {
                                        ecc = c;
                                    }
                                }
                            }
                            // m (code model: Model 2 only), i (input mode),
                            // D (structured append) unsupported
                            _ => {}
                        }
                    }
                    if content.len() > 9999 {
                        return Err("EPL b QR command data exceeds 9999 bytes".to_string());
                    }
                    // The QR element carries its ECC level and character mode
                    // embedded in the data (ZPL ^FD convention); synthesize a
                    // binary-mode payload so EPL data passes through untouched
                    // (no Zebra `|` field-separator stripping).
                    let data = format!("{ecc}M,B{:04}{content}", content.len());
                    LabelElement::BarcodeQr(BarcodeQrWithData {
                        reverse_print: ReversePrint::default(),
                        barcode: BarcodeQr { magnification },
                        height: 0,
                        position: pos,
                        data,
                    })
                }
                other => {
                    return Err(format!(
                        "EPL b command: unsupported 2D bar code type \"{}\" (expected A, D, M, P or Q)",
                        other
                    ));
                }
            };

            Ok(Some(el))
        }
        _ => Ok(None),
    }
}

/// Flattens the optional 2D parameters into `(prefix letter, value)` pairs.
/// Tokens may contain several concatenated params (`m2s3eMiA`) since commas
/// between them are optional.
fn parse_epl_2d_params(tokens: &[&str]) -> Vec<(char, String)> {
    let mut out = Vec::new();
    for token in tokens {
        let bytes = token.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c.is_ascii_alphabetic() {
                // A value is either a run of digits (`d4`, `e208`) or a single
                // letter (`eM`, `iA`); the next prefix letter always starts a
                // new parameter, so concatenated tokens split unambiguously.
                let mut j = i + 1;
                if j < bytes.len() && bytes[j].is_ascii_digit() {
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                } else if j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                out.push((c, token[i + 1..j].to_string()));
                i = j;
            } else {
                i += 1;
            }
        }
    }
    out
}

/// EPL MaxiCode automatic mode selection: with the AIM header present, an
/// all-numeric postal code selects Mode 2, anything else Mode 3; without the
/// header the data is a standard symbol (Mode 4).
fn auto_maxicode_mode(data: &str) -> i32 {
    const HEADER: &str = "[)>\u{1e}01\u{1d}";
    match data.find(HEADER) {
        Some(p) if p >= 6 => {
            let postal = &data[6..p];
            if !postal.is_empty() && postal.bytes().all(|b| b.is_ascii_digit()) {
                2
            } else {
                3
            }
        }
        _ => 4,
    }
}
