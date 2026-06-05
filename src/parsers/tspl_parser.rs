use crate::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use crate::elements::barcode_39::{Barcode39, Barcode39WithData};
use crate::elements::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use crate::elements::barcode_pdf417::{BarcodePdf417, BarcodePdf417WithData};
use crate::elements::barcode_qr::{BarcodeQr, BarcodeQrWithData};
use crate::elements::drawer_options::DrawerOptions;
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::graphic_circle::GraphicCircle;
use crate::elements::graphic_ellipse::GraphicEllipse;
use crate::elements::graphic_field::{GraphicField, GraphicFieldFormat, GraphicFieldMode};
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::reverse_print::ReversePrint;
use crate::elements::text_field::TextField;

#[derive(Clone, Debug)]
pub struct TsplParsedLabel {
    pub label: LabelInfo,
    pub drawer_options: DrawerOptions,
}

pub struct TsplParser;

impl Default for TsplParser {
    fn default() -> Self {
        Self
    }
}

impl TsplParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, tspl_data: &[u8]) -> Result<Vec<LabelInfo>, String> {
        Ok(self
            .parse_with_options(tspl_data, DrawerOptions::default())?
            .into_iter()
            .map(|parsed| parsed.label)
            .collect())
    }

    pub fn parse_with_options(
        &self,
        tspl_data: &[u8],
        base_options: DrawerOptions,
    ) -> Result<Vec<TsplParsedLabel>, String> {
        let commands = split_tspl_commands(tspl_data)?;
        let mut state = TsplState::new(base_options.with_defaults());
        let mut labels = Vec::new();

        for command in commands {
            match command {
                TsplCommand::Line(line) => self.parse_line(&line, &mut state, &mut labels)?,
                TsplCommand::Bitmap { header, data } => {
                    self.parse_bitmap(&header, data, &mut state)?
                }
            }
        }

        if !state.elements.is_empty() {
            labels.push(state.to_label());
        }

        Ok(labels)
    }

    fn parse_line(
        &self,
        line: &str,
        state: &mut TsplState,
        labels: &mut Vec<TsplParsedLabel>,
    ) -> Result<(), String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        let name = command_name(line);
        match name.as_str() {
            "SIZE" => parse_size(line, state),
            "DIRECTION" => parse_direction(line, state),
            "REFERENCE" => parse_reference(line, state),
            "SHIFT" => parse_shift(line, state),
            "CLS" => {
                state.elements.clear();
                Ok(())
            }
            "PRINT" => {
                if !state.elements.is_empty() {
                    labels.push(state.to_label());
                    state.elements.clear();
                }
                Ok(())
            }
            "TEXT" => parse_text(line, state),
            "BAR" => parse_bar(line, state),
            "BOX" => parse_box(line, state),
            "CIRCLE" => parse_circle(line, state),
            "ELLIPSE" => parse_ellipse(line, state),
            "ERASE" => parse_region_box(line, state, LineColor::White, false),
            "REVERSE" => parse_region_box(line, state, LineColor::Black, true),
            "BARCODE" => parse_barcode(line, state),
            "QRCODE" => parse_qrcode(line, state),
            "PDF417" => parse_pdf417(line, state),
            "MPDF417" | "PUTPCX" | "PUTBMP" | "DMATRIX" | "AZTEC" => {
                Err(format!("unsupported TSPL command {}", name))
            }
            _ if is_device_noop(&name) => Ok(()),
            _ => Ok(()),
        }
    }

    fn parse_bitmap(
        &self,
        header: &str,
        data: Vec<u8>,
        state: &mut TsplState,
    ) -> Result<(), String> {
        let args = command_args(header, "BITMAP");
        if args.len() < 5 {
            return Err(format!(
                "TSPL BITMAP command requires at least 5 parameters, got {}",
                args.len()
            ));
        }

        let x = parse_i32_arg(&args[0]).unwrap_or(0);
        let y = parse_i32_arg(&args[1]).unwrap_or(0);
        let row_bytes = parse_i32_arg(&args[2]).unwrap_or(1).max(1);
        let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
        let mode = match parse_i32_arg(&args[4]).unwrap_or(0) {
            1 => GraphicFieldMode::Or,
            2 => GraphicFieldMode::Xor,
            _ => GraphicFieldMode::Overwrite,
        };
        let total_bytes = row_bytes * height;

        state
            .elements
            .push(LabelElement::GraphicField(GraphicField {
                reverse_print: ReversePrint::default(),
                position: state.position(x, y),
                format: GraphicFieldFormat::Raw,
                mode,
                data_bytes: total_bytes,
                total_bytes,
                row_bytes,
                data,
                magnification_x: 1,
                magnification_y: 1,
            }));
        Ok(())
    }
}

struct TsplState {
    elements: Vec<LabelElement>,
    options: DrawerOptions,
    reference_x: i32,
    reference_y: i32,
    shift_x: i32,
    shift_y: i32,
    inverted: bool,
}

impl TsplState {
    fn new(options: DrawerOptions) -> Self {
        Self {
            elements: Vec::new(),
            options,
            reference_x: 0,
            reference_y: 0,
            shift_x: 0,
            shift_y: 0,
            inverted: false,
        }
    }

    fn position(&self, x: i32, y: i32) -> LabelPosition {
        LabelPosition {
            x: x + self.reference_x + self.shift_x,
            y: y + self.reference_y + self.shift_y,
            calculate_from_bottom: false,
            automatic_position: false,
        }
    }

    fn to_label(&self) -> TsplParsedLabel {
        TsplParsedLabel {
            drawer_options: self.options.clone(),
            label: LabelInfo {
                print_width: 0,
                inverted: self.inverted,
                elements: self.elements.clone(),
            },
        }
    }
}

enum TsplCommand {
    Line(String),
    Bitmap { header: String, data: Vec<u8> },
}

fn split_tspl_commands(data: &[u8]) -> Result<Vec<TsplCommand>, String> {
    let mut commands = Vec::new();
    let mut idx = 0usize;

    while idx < data.len() {
        while idx < data.len() && matches!(data[idx], b'\r' | b'\n') {
            idx += 1;
        }
        if idx >= data.len() {
            break;
        }

        let line_start = idx;
        let token_end = first_token_end(data, line_start);
        let token = String::from_utf8_lossy(&data[line_start..token_end]).to_uppercase();

        if token == "BITMAP" {
            let payload_start = bitmap_payload_start(data, line_start)
                .ok_or_else(|| "TSPL BITMAP command missing bitmap data separator".to_string())?;
            let header = String::from_utf8_lossy(&data[line_start..payload_start]).to_string();
            let args = command_args(&header, "BITMAP");
            if args.len() < 4 {
                return Err(format!(
                    "TSPL BITMAP command requires at least 4 size parameters, got {}",
                    args.len()
                ));
            }
            let row_bytes = parse_i32_arg(&args[2]).unwrap_or(1).max(1) as usize;
            let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1) as usize;
            let data_len = row_bytes * height;
            let payload_end = payload_start.saturating_add(data_len).min(data.len());
            commands.push(TsplCommand::Bitmap {
                header,
                data: data[payload_start..payload_end].to_vec(),
            });
            idx = payload_end;
            while idx < data.len() && matches!(data[idx], b'\r' | b'\n') {
                idx += 1;
            }
            continue;
        }

        idx = command_line_end(data, line_start);
        let line = String::from_utf8_lossy(&data[line_start..idx]).to_string();
        commands.push(TsplCommand::Line(line));
    }

    Ok(commands)
}

fn first_token_end(data: &[u8], start: usize) -> usize {
    let mut idx = start;
    while idx < data.len() && !data[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}

fn bitmap_payload_start(data: &[u8], start: usize) -> Option<usize> {
    let mut commas = 0usize;
    let mut idx = start;
    while idx < data.len() {
        if data[idx] == b',' {
            commas += 1;
            if commas == 5 {
                return Some(idx + 1);
            }
        }
        if matches!(data[idx], b'\r' | b'\n') {
            return None;
        }
        idx += 1;
    }
    None
}

fn command_line_end(data: &[u8], start: usize) -> usize {
    let mut idx = start;
    let mut in_quotes = false;

    while idx < data.len() {
        if data[idx] == b'"' && !is_escaped_quote_byte(data, idx) {
            in_quotes = !in_quotes;
        } else if matches!(data[idx], b'\r' | b'\n') && !in_quotes {
            break;
        }
        idx += 1;
    }

    idx
}

fn is_escaped_quote_byte(data: &[u8], idx: usize) -> bool {
    idx > 0 && data[idx - 1] == b'\\'
        || idx >= 2
            && data[idx - 2] == b'\\'
            && data[idx - 1] == b'['
            && data.get(idx + 1) == Some(&b']')
}

fn command_name(line: &str) -> String {
    line.split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
}

fn command_args(line: &str, name: &str) -> Vec<String> {
    let rest = line.get(name.len()..).unwrap_or("").trim_start();
    split_tspl_args(rest)
}

fn split_tspl_args(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        let escaped_quote = ch == '"'
            && (i > 0 && chars[i - 1] == '\\'
                || i >= 2
                    && chars[i - 2] == '\\'
                    && chars[i - 1] == '['
                    && chars.get(i + 1) == Some(&']'));

        if ch == '"' && !escaped_quote {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch == ',' && !in_quotes {
            args.push(clean_arg(&current));
            current.clear();
        } else {
            current.push(ch);
        }
        i += 1;
    }

    if !current.is_empty() || input.ends_with(',') {
        args.push(clean_arg(&current));
    }

    args
}

fn clean_arg(arg: &str) -> String {
    let trimmed = arg.trim();
    let unquoted = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    unquoted.replace("\\[\"]", "\"").replace("\\\"", "\"")
}

fn parse_i32_arg(arg: &str) -> Option<i32> {
    arg.trim().parse::<f64>().ok().map(|v| v.round() as i32)
}

fn parse_f64_arg(arg: &str) -> Option<f64> {
    arg.trim().parse::<f64>().ok()
}

fn parse_size(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "SIZE");
    if args.is_empty() {
        return Err("TSPL SIZE command requires at least a width".to_string());
    }
    if let Some(width) = parse_dimension_mm(&args[0], state.options.dpmm) {
        state.options.label_width_mm = width;
    }
    if let Some(height_arg) = args.get(1) {
        if let Some(height) = parse_dimension_mm(height_arg, state.options.dpmm) {
            state.options.label_height_mm = height;
        }
    }
    Ok(())
}

fn parse_dimension_mm(arg: &str, dpmm: i32) -> Option<f64> {
    let value_text: String = arg
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    let value = parse_f64_arg(&value_text)?;
    let lower = arg.trim().to_ascii_lowercase();
    if lower.ends_with("mm") {
        Some(value)
    } else if lower.ends_with("dot") || lower.ends_with("dots") {
        Some(value / dpmm.max(1) as f64)
    } else {
        Some(value * 25.4)
    }
}

fn parse_direction(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "DIRECTION");
    if let Some(v) = args.first().and_then(|arg| parse_i32_arg(arg)) {
        state.inverted = v == 0;
    }
    Ok(())
}

fn parse_reference(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "REFERENCE");
    if let Some(v) = args.first().and_then(|arg| parse_i32_arg(arg)) {
        state.reference_x = v;
    }
    if let Some(v) = args.get(1).and_then(|arg| parse_i32_arg(arg)) {
        state.reference_y = v;
    }
    Ok(())
}

fn parse_shift(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "SHIFT");
    match args.as_slice() {
        [y] => {
            state.shift_y = parse_i32_arg(y).unwrap_or(0);
        }
        [x, y, ..] => {
            state.shift_x = parse_i32_arg(x).unwrap_or(0);
            state.shift_y = parse_i32_arg(y).unwrap_or(0);
        }
        _ => {}
    }
    Ok(())
}

fn parse_text(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "TEXT");
    if args.len() < 7 {
        return Err(format!(
            "TSPL TEXT command requires at least 7 parameters, got {}",
            args.len()
        ));
    }

    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let (font_name, base_w, base_h) = tspl_font_info(&args[2]);
    let orientation = tspl_rotation(&args[3]);
    let x_mult = parse_i32_arg(&args[4]).unwrap_or(1).clamp(1, 10);
    let y_mult = parse_i32_arg(&args[5]).unwrap_or(1).clamp(1, 10);
    let (alignment, content_idx) = if args.len() >= 8 {
        (tspl_alignment(&args[6]), 7usize)
    } else {
        (FieldAlignment::Left, 6usize)
    };
    let text = args.get(content_idx).cloned().unwrap_or_default();
    if text.is_empty() {
        return Ok(());
    }

    state.elements.push(LabelElement::Text(TextField {
        reverse_print: ReversePrint::default(),
        font: FontInfo {
            name: font_name,
            width: (base_w * x_mult) as f64,
            height: (base_h * y_mult) as f64,
            orientation,
        }
        .with_adjusted_sizes(),
        position: state.position(x, y),
        alignment,
        text,
        block: None,
    }));
    Ok(())
}

fn tspl_font_info(font: &str) -> (String, i32, i32) {
    let font = font.trim_matches('"').to_ascii_uppercase();
    match font.as_str() {
        "2" => ("TSPL_2".to_string(), 12, 20),
        "3" => ("TSPL_3".to_string(), 16, 24),
        "4" => ("TSPL_4".to_string(), 24, 32),
        "5" => ("TSPL_5".to_string(), 32, 48),
        "6" => ("TSPL_6".to_string(), 14, 19),
        "7" => ("TSPL_7".to_string(), 21, 27),
        "8" => ("TSPL_8".to_string(), 14, 25),
        "TST16.BF2" | "TTT16.BF2" | "TSS16.BF2" => (font, 16, 16),
        "TST24.BF2" | "TTT24.BF2" | "TSS24.BF2" => (font, 24, 24),
        _ => ("TSPL_1".to_string(), 8, 12),
    }
}

fn tspl_alignment(arg: &str) -> FieldAlignment {
    match parse_i32_arg(arg).unwrap_or(0) {
        2 => FieldAlignment::Center,
        3 => FieldAlignment::Right,
        _ => FieldAlignment::Left,
    }
}

fn parse_bar(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "BAR");
    if args.len() < 4 {
        return Err(format!(
            "TSPL BAR command requires 4 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let width = parse_i32_arg(&args[2]).unwrap_or(1).max(1);
    let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
    state.elements.push(LabelElement::GraphicBox(filled_box(
        state,
        x,
        y,
        width,
        height,
        LineColor::Black,
        false,
    )));
    Ok(())
}

fn parse_box(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "BOX");
    if args.len() < 5 {
        return Err(format!(
            "TSPL BOX command requires at least 5 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let x_end = parse_i32_arg(&args[2]).unwrap_or(x);
    let y_end = parse_i32_arg(&args[3]).unwrap_or(y);
    let thickness = parse_i32_arg(&args[4]).unwrap_or(1).max(1);
    let radius = args.get(5).and_then(|arg| parse_i32_arg(arg)).unwrap_or(0);
    state.elements.push(LabelElement::GraphicBox(GraphicBox {
        reverse_print: ReversePrint::default(),
        position: state.position(x, y),
        width: (x_end - x).abs().max(thickness),
        height: (y_end - y).abs().max(thickness),
        border_thickness: thickness,
        corner_rounding: radius.clamp(0, 8),
        line_color: LineColor::Black,
    }));
    Ok(())
}

fn parse_circle(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "CIRCLE");
    if args.len() < 4 {
        return Err(format!(
            "TSPL CIRCLE command requires 4 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let diameter = parse_i32_arg(&args[2]).unwrap_or(1).max(1);
    let thickness = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
    state
        .elements
        .push(LabelElement::GraphicCircle(GraphicCircle {
            reverse_print: ReversePrint::default(),
            position: state.position(x, y),
            circle_diameter: diameter,
            border_thickness: thickness,
            line_color: LineColor::Black,
        }));
    Ok(())
}

fn parse_ellipse(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "ELLIPSE");
    if args.len() < 5 {
        return Err(format!(
            "TSPL ELLIPSE command requires 5 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let width = parse_i32_arg(&args[2]).unwrap_or(1).max(1);
    let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
    let thickness = parse_i32_arg(&args[4]).unwrap_or(1).max(1);
    state
        .elements
        .push(LabelElement::GraphicEllipse(GraphicEllipse {
            reverse_print: ReversePrint::default(),
            position: state.position(x, y),
            width,
            height,
            border_thickness: thickness,
            line_color: LineColor::Black,
        }));
    Ok(())
}

fn parse_region_box(
    line: &str,
    state: &mut TsplState,
    color: LineColor,
    reverse: bool,
) -> Result<(), String> {
    let name = command_name(line);
    let args = command_args(line, &name);
    if args.len() < 4 {
        return Err(format!(
            "TSPL {} command requires 4 parameters, got {}",
            name,
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let width = parse_i32_arg(&args[2]).unwrap_or(1).max(1);
    let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
    state.elements.push(LabelElement::GraphicBox(filled_box(
        state, x, y, width, height, color, reverse,
    )));
    Ok(())
}

fn filled_box(
    state: &TsplState,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: LineColor,
    reverse: bool,
) -> GraphicBox {
    GraphicBox {
        reverse_print: ReversePrint { value: reverse },
        position: state.position(x, y),
        width,
        height,
        border_thickness: width.min(height).max(1),
        corner_rounding: 0,
        line_color: color,
    }
}

fn parse_barcode(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "BARCODE");
    if args.len() < 9 {
        return Err(format!(
            "TSPL BARCODE command requires at least 9 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let code_type = args[2].trim().to_ascii_uppercase();
    let height = parse_i32_arg(&args[3]).unwrap_or(1).max(1);
    let human_readable = parse_i32_arg(&args[4]).unwrap_or(0);
    let line = human_readable > 0;
    let line_alignment = tspl_alignment(&args[4]);
    let orientation = tspl_rotation(&args[5]);
    let narrow = parse_i32_arg(&args[6]).unwrap_or(1).max(1);
    let wide = parse_i32_arg(&args[7]).unwrap_or(narrow).max(narrow);
    let content_idx = if args.len() >= 10 { 9 } else { 8 };
    let data = args.get(content_idx).cloned().unwrap_or_default();
    let position = state.position(x, y);
    let width_ratio = (wide as f64 / narrow as f64).max(1.0);

    let element = match code_type.as_str() {
        "128" | "128M" => LabelElement::Barcode128(Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height,
                line,
                line_above: false,
                line_alignment,
                check_digit: false,
                mode: BarcodeMode::Automatic,
            },
            width: narrow,
            position,
            data,
        }),
        "EAN128" | "EAN128M" => LabelElement::Barcode128(Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height,
                line,
                line_above: false,
                line_alignment,
                check_digit: false,
                mode: BarcodeMode::Ucc,
            },
            width: narrow,
            position,
            data,
        }),
        "39" | "39C" => LabelElement::Barcode39(Barcode39WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode39 {
                orientation,
                height,
                line,
                line_above: false,
                line_alignment,
                check_digit: code_type == "39C",
            },
            width: narrow,
            width_ratio,
            position,
            data,
        }),
        "EAN13" => LabelElement::BarcodeEan13(BarcodeEan13WithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeEan13 {
                orientation,
                height,
                line,
                line_above: false,
                line_alignment,
            },
            width: narrow,
            position,
            data,
        }),
        _ => return Err(format!("unsupported TSPL barcode type {}", code_type)),
    };

    state.elements.push(element);
    Ok(())
}

fn parse_qrcode(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "QRCODE");
    if args.len() < 7 {
        return Err(format!(
            "TSPL QRCODE command requires at least 7 parameters, got {}",
            args.len()
        ));
    }
    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let level = args[2].chars().next().unwrap_or('H').to_ascii_uppercase();
    let magnification = parse_i32_arg(&args[3]).unwrap_or(1).clamp(1, 10);
    let mode = args[4].chars().next().unwrap_or('A').to_ascii_uppercase();
    let orientation = tspl_rotation(&args[5]);
    let content = args.last().cloned().unwrap_or_default();

    state
        .elements
        .push(LabelElement::BarcodeQr(BarcodeQrWithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeQr {
                magnification,
                orientation,
            },
            height: 0,
            position: state.position(x, y),
            data: format!("{}{},{}", level, mode, content),
        }));
    Ok(())
}

fn parse_pdf417(line: &str, state: &mut TsplState) -> Result<(), String> {
    let args = command_args(line, "PDF417");
    if args.len() < 6 {
        return Err(format!(
            "TSPL PDF417 command requires at least 6 parameters, got {}",
            args.len()
        ));
    }

    let x = parse_i32_arg(&args[0]).unwrap_or(0);
    let y = parse_i32_arg(&args[1]).unwrap_or(0);
    let expected_height = parse_i32_arg(&args[3]).unwrap_or(10).max(1);
    let orientation = tspl_rotation(&args[4]);
    let content = args.last().cloned().unwrap_or_default();
    let mut barcode = BarcodePdf417 {
        orientation,
        row_height: 0,
        security: 0,
        columns: 0,
        rows: 0,
        truncate: false,
        module_width: 2,
        by_height: expected_height,
    };

    for option in args.iter().skip(5).take(args.len().saturating_sub(6)) {
        apply_pdf417_option(option, &mut barcode);
    }

    state
        .elements
        .push(LabelElement::BarcodePdf417(BarcodePdf417WithData {
            reverse_print: ReversePrint::default(),
            barcode,
            position: state.position(x, y),
            data: content,
        }));
    Ok(())
}

fn apply_pdf417_option(option: &str, barcode: &mut BarcodePdf417) {
    let upper = option.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return;
    }
    let value = upper
        .get(1..)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    match upper.as_bytes()[0] {
        b'E' => barcode.security = value.clamp(0, 8),
        b'W' => barcode.module_width = value.max(1),
        b'H' => barcode.row_height = value.max(1),
        b'R' => barcode.rows = value,
        b'C' => barcode.columns = value,
        b'T' => barcode.truncate = value == 1,
        _ => {}
    }
}

fn tspl_rotation(arg: &str) -> FieldOrientation {
    match parse_i32_arg(arg).unwrap_or(0) {
        1 | 90 => FieldOrientation::Rotated90,
        2 | 180 => FieldOrientation::Rotated180,
        3 | 270 => FieldOrientation::Rotated270,
        _ => FieldOrientation::Normal,
    }
}

fn is_device_noop(name: &str) -> bool {
    matches!(
        name,
        "GAP"
            | "BLINE"
            | "GAPDETECT"
            | "BLINEDETECT"
            | "AUTODETECT"
            | "OFFSET"
            | "SPEED"
            | "DENSITY"
            | "CODEPAGE"
            | "FEED"
            | "BACKFEED"
            | "FORMFEED"
            | "HOME"
            | "SELFTEST"
            | "INITIALPRINTER"
            | "DOWNLOAD"
            | "EOP"
            | "FILES"
            | "KILL"
            | "RUN"
            | "OUT"
            | "SET"
    )
}
