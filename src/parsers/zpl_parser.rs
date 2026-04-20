use crate::elements::barcode_128::{Barcode128, BarcodeMode};
use crate::elements::barcode_2of5::Barcode2of5;
use crate::elements::barcode_39::Barcode39;
use crate::elements::barcode_aztec::BarcodeAztec;
use crate::elements::barcode_datamatrix::{BarcodeDatamatrix, DatamatrixRatio};
use crate::elements::barcode_ean13::BarcodeEan13;
use crate::elements::barcode_pdf417::BarcodePdf417;
use crate::elements::barcode_qr::BarcodeQr;
use crate::elements::field_block::FieldBlock;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::graphic_circle::GraphicCircle;
use crate::elements::graphic_diagonal_line::GraphicDiagonalLine;
use crate::elements::graphic_field::{GraphicField, GraphicFieldFormat};
use crate::elements::graphic_symbol::GraphicSymbol;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::maxicode::Maxicode;
use crate::elements::reverse_print::ReversePrint;
use crate::elements::stored_format::{RecalledFieldData, StoredField, StoredFormat};
use crate::hex;

use super::command_utils::*;
use super::fs::*;
use super::virtual_printer::VirtualPrinter;

pub struct ZplParser {
    printer: VirtualPrinter,
}

impl Default for ZplParser {
    fn default() -> Self {
        ZplParser {
            printer: VirtualPrinter::new(),
        }
    }
}

impl ZplParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(&mut self, zpl_data: &[u8]) -> Result<Vec<LabelInfo>, String> {
        let mut results = Vec::new();
        let mut result_elements: Vec<LabelElement> = Vec::new();

        let commands = split_zpl_commands(zpl_data)?;
        let mut current_recalled_format: Option<crate::elements::stored_format::RecalledFormat> =
            None;

        for command in &commands {
            let upper = command.to_uppercase();

            if upper.starts_with("^XA") {
                self.printer.reset_label_state();
                current_recalled_format = None;
                continue;
            }

            if upper.starts_with("^XZ") {
                // Resolve any active recalled format
                if let Some(ref rf) = current_recalled_format {
                    let resolved = rf.resolve_elements()?;
                    result_elements.extend(resolved);
                }

                if result_elements.is_empty() {
                    continue;
                }

                if self.printer.next_download_format_name.is_empty() {
                    results.push(LabelInfo {
                        print_width: self.printer.print_width,
                        inverted: self.printer.label_inverted,
                        elements: result_elements.clone(),
                    });
                } else {
                    self.printer.stored_formats.insert(
                        self.printer.next_download_format_name.clone(),
                        StoredFormat {
                            inverted: self.printer.label_inverted,
                            elements: result_elements.clone(),
                        },
                    );
                }

                result_elements.clear();
                continue;
            }

            // Try each command parser
            if let Some(el) = self.parse_command(command)? {
                // Handle template swap
                if let LabelElement::RecalledFormat(rf) = el {
                    if let Some(ref prev_rf) = current_recalled_format {
                        let resolved = prev_rf.resolve_elements()?;
                        result_elements.extend(resolved);
                    }
                    self.printer.label_inverted = rf.inverted;
                    current_recalled_format = Some(rf);
                    continue;
                }

                // If template in use, add elements to template
                if let Some(ref mut rf) = current_recalled_format {
                    rf.add_element(el);
                    continue;
                }

                result_elements.push(el);
            }
        }

        Ok(results)
    }

    fn parse_command(&mut self, command: &str) -> Result<Option<LabelElement>, String> {
        // Match on command prefix (first 3 chars typically)
        let upper = command.to_uppercase();

        // Label home
        if upper.starts_with("^LH") {
            self.parse_label_home(command);
            return Ok(None);
        }
        // Label reverse print
        if upper.starts_with("^LR") {
            let text = command_text(command, "^LR");
            self.printer.label_reverse = text == "Y";
            return Ok(None);
        }
        // Print orientation
        if upper.starts_with("^PO") {
            let text = command_text(command, "^PO");
            self.printer.label_inverted = text == "I";
            return Ok(None);
        }
        // Print width
        if upper.starts_with("^PW") {
            let parts = split_command(command, "^PW");
            if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
                self.printer.print_width = v.max(2);
            }
            return Ok(None);
        }
        // Change charset
        if upper.starts_with("^CI") {
            let parts = split_command(command, "^CI");
            if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
                self.printer.current_charset = v;
            }
            return Ok(None);
        }
        // Change default font
        if upper.starts_with("^CF") {
            self.parse_change_default_font(command);
            return Ok(None);
        }
        // Change font
        if upper.starts_with("^A") && !upper.starts_with("^A@") {
            self.parse_change_font(command);
            return Ok(None);
        }
        // Field orientation
        if upper.starts_with("^FW") {
            self.parse_field_orientation(command);
            return Ok(None);
        }
        // Field origin
        if upper.starts_with("^FO") {
            self.parse_field_origin(command);
            return Ok(None);
        }
        // Field typeset
        if upper.starts_with("^FT") {
            self.parse_field_typeset(command);
            return Ok(None);
        }
        // Field block
        if upper.starts_with("^FB") {
            self.parse_field_block(command);
            return Ok(None);
        }
        // Field data
        if upper.starts_with("^FD") {
            self.parse_field_data(command)?;
            return Ok(None);
        }
        // Field value
        if upper.starts_with("^FV") {
            self.printer.next_element_field_data = command_text(command, "^FV").to_string();
            return Ok(None);
        }
        // Field number
        if upper.starts_with("^FN") {
            let number = command_text(command, "^FN");
            if let Ok(v) = number.parse::<i32>() {
                if v >= 0 {
                    self.printer.next_element_field_number = v;
                }
            }
            return Ok(None);
        }
        // Field reverse print
        if upper.starts_with("^FR") {
            self.printer.next_element_field_reverse = true;
            return Ok(None);
        }
        // Hex escape
        if upper.starts_with("^FH") {
            let text = command_text(command, "^FH");
            self.printer.next_hex_escape_char = if text.is_empty() {
                b'_'
            } else {
                text.as_bytes()[0]
            };
            return Ok(None);
        }
        // Field separator - this resolves the current field
        if upper.starts_with("^FS") {
            return self.parse_field_separator();
        }

        // Barcode commands
        if upper.starts_with("^BC") {
            self.parse_barcode_128(command);
            return Ok(None);
        }
        if upper.starts_with("^BE") {
            self.parse_barcode_ean13(command);
            return Ok(None);
        }
        if upper.starts_with("^B2") {
            self.parse_barcode_2of5(command);
            return Ok(None);
        }
        if upper.starts_with("^B3") {
            self.parse_barcode_39(command);
            return Ok(None);
        }
        if upper.starts_with("^B7") {
            self.parse_barcode_pdf417(command);
            return Ok(None);
        }
        if upper.starts_with("^BO") {
            self.parse_barcode_aztec(command);
            return Ok(None);
        }
        if upper.starts_with("^BX") {
            self.parse_barcode_datamatrix(command);
            return Ok(None);
        }
        if upper.starts_with("^BQ") {
            self.parse_barcode_qr(command);
            return Ok(None);
        }
        if upper.starts_with("^BD") {
            self.parse_maxicode(command);
            return Ok(None);
        }
        if upper.starts_with("^BY") {
            self.parse_barcode_field_defaults(command);
            return Ok(None);
        }

        // Graphic commands
        if upper.starts_with("^GB") {
            return self.parse_graphic_box(command);
        }
        if upper.starts_with("^GC") {
            return self.parse_graphic_circle(command);
        }
        if upper.starts_with("^GD") {
            return self.parse_graphic_diagonal_line(command);
        }
        if upper.starts_with("^GF") {
            return self.parse_graphic_field(command);
        }
        if upper.starts_with("^GS") {
            self.parse_graphic_symbol(command);
            return Ok(None);
        }

        // Download/recall
        if upper.starts_with("~DG") {
            self.parse_download_graphics(command)?;
            return Ok(None);
        }
        if upper.starts_with("^IL") {
            return self.parse_image_load(command);
        }
        if upper.starts_with("^XG") {
            return self.parse_recall_graphics(command);
        }
        if upper.starts_with("^DF") {
            self.parse_download_format(command)?;
            return Ok(None);
        }
        if upper.starts_with("^XF") {
            return self.parse_recall_format(command);
        }

        Ok(None)
    }

    fn parse_label_home(&mut self, command: &str) {
        let parts = split_command(command, "^LH");
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            self.printer.label_home_position.x = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            self.printer.label_home_position.y = v;
        }
    }

    fn parse_change_default_font(&mut self, command: &str) {
        let parts = split_command(command, "^CF");
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                self.printer.default_font.name = s.to_uppercase();
            }
        }
        let has_height = parts.get(1).and_then(|s| parse_int(s)).is_some();
        let has_width = parts.get(2).and_then(|s| parse_int(s)).is_some();
        if let Some(s) = parts.get(1) {
            if let Some(v) = parse_int(s) {
                self.printer.default_font.height = v as f64;
            }
        }
        // Per ZPL spec: "Defining only the height or width forces the magnification to be
        // proportional to the parameter defined." When only height is given (no explicit width),
        // reset width to 0 so with_adjusted_sizes() derives it proportionally from height.
        if has_height && !has_width {
            self.printer.default_font.width = 0.0;
        }
        if let Some(s) = parts.get(2) {
            if let Some(v) = parse_int(s) {
                self.printer.default_font.width = v as f64;
            }
        }
    }

    fn parse_change_font(&mut self, command: &str) {
        let parts = split_command(command, "^A");
        if parts.is_empty() || parts[0].is_empty() {
            self.printer.next_font = None;
            return;
        }

        let first = parts[0].as_bytes();
        let mut font = crate::elements::font::FontInfo {
            name: (first[0] as char).to_uppercase().to_string(),
            orientation: self.printer.default_font.orientation,
            ..Default::default()
        };

        if !font.is_standard_font() {
            // Numeric font names (1-9) are user-installed fonts on Zebra printers.
            // Fall back to font "0" (proportional) rather than font "A" (monospaced).
            if font.name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                font.name = "0".to_string();
            } else {
                font.name = self.printer.default_font.name.clone();
            }
        }

        // After font name character, check if next char is a valid orientation letter.
        // If it's a digit or missing, the remainder is height (^A048,40 = font 0, h=48, w=40).
        let (extra_height_str, height_part_idx, width_part_idx) = if first.len() > 1 {
            let second = first[1];
            if matches!(
                second,
                b'N' | b'R' | b'I' | b'B' | b'n' | b'r' | b'i' | b'b'
            ) {
                font.orientation = to_field_orientation(second);
                (None, 1usize, 2usize)
            } else {
                // No valid orientation: remaining chars in first part are height digits
                let height_str = std::str::from_utf8(&first[1..]).unwrap_or("");
                (Some(height_str.to_string()), usize::MAX, 1usize)
            }
        } else {
            (None, 1, 2)
        };

        if let Some(hs) = extra_height_str {
            if let Some(v) = parse_int(&hs) {
                font.height = v as f64;
            }
        } else if let Some(s) = parts.get(height_part_idx) {
            if let Some(v) = parse_int(s) {
                font.height = v as f64;
            }
        }
        if let Some(s) = parts.get(width_part_idx) {
            if let Some(v) = parse_int(s) {
                font.width = v as f64;
            }
        }

        self.printer.next_font = Some(font);
    }

    fn parse_field_orientation(&mut self, command: &str) {
        let parts = split_command(command, "^FW");
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                self.printer
                    .set_default_orientation(to_field_orientation(s.as_bytes()[0]));
            }
        }
        if let Some(s) = parts.get(1) {
            if let Some(val) = to_field_alignment(s) {
                self.printer.default_alignment = val;
            }
        }
    }

    fn parse_field_origin(&mut self, command: &str) {
        let parts = split_command(command, "^FO");
        let mut pos = LabelPosition {
            calculate_from_bottom: false,
            ..Default::default()
        };
        if let Some(v) = parts.first().and_then(|s| to_positive_int(s)) {
            pos.x = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| to_positive_int(s)) {
            pos.y = v;
        }
        if let Some(s) = parts.get(2) {
            if let Some(val) = to_field_alignment(s) {
                self.printer.next_element_alignment = Some(val);
            }
        }
        self.printer.next_element_position = pos.add(&self.printer.label_home_position);
    }

    fn parse_field_typeset(&mut self, command: &str) {
        let parts = split_command(command, "^FT");
        let mut pos = LabelPosition {
            calculate_from_bottom: true,
            automatic_position: true,
            ..Default::default()
        };
        if let Some(v) = parts.first().and_then(|s| to_positive_int(s)) {
            pos.x = v;
            pos.automatic_position = false;
        }
        if let Some(v) = parts.get(1).and_then(|s| to_positive_int(s)) {
            pos.y = v;
            pos.automatic_position = false;
        }
        if let Some(s) = parts.get(2) {
            if let Some(val) = to_field_alignment(s) {
                self.printer.next_element_alignment = Some(val);
            }
        }
        self.printer.next_element_position = pos.add(&self.printer.label_home_position);
    }

    fn parse_field_block(&mut self, command: &str) {
        let parts = split_command(command, "^FB");
        let mut block = FieldBlock {
            max_width: 0,
            max_lines: 1,
            line_spacing: 0,
            alignment: crate::elements::text_alignment::TextAlignment::Left,
            hanging_indent: 0,
        };
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            block.max_width = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            block.max_lines = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            block.line_spacing = v;
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                block.alignment = to_text_alignment(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(4).and_then(|s| parse_int(s)) {
            block.hanging_indent = v;
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::FieldBlockConfig(block)));
    }

    fn parse_field_data(&mut self, command: &str) -> Result<(), String> {
        let mut text = command_text(command, "^FD").to_string();
        if self.printer.next_hex_escape_char != 0 {
            text = hex::decode_escaped_string(&text, self.printer.next_hex_escape_char)
                .map_err(|e| format!("failed to decode escaped hex string: {}", e))?;
        }
        self.printer.next_element_field_data = text;
        Ok(())
    }

    fn parse_field_separator(&mut self) -> Result<Option<LabelElement>, String> {
        let result = if self.printer.next_element_field_number < 0 {
            // Not a template field -> resolve immediately via RecalledField
            let rf = crate::elements::stored_format::RecalledField {
                stored: StoredField {
                    number: self.printer.next_element_field_number,
                    field: self.printer.get_field_info(),
                },
                data: self.printer.next_element_field_data.clone(),
            };
            // Resolve immediately
            resolve_recalled_field(&rf)?
        } else if self.printer.next_download_format_name.is_empty() {
            Some(LabelElement::RecalledFieldData(RecalledFieldData {
                number: self.printer.next_element_field_number,
                data: self.printer.next_element_field_data.clone(),
            }))
        } else {
            Some(LabelElement::StoredField(StoredField {
                number: self.printer.next_element_field_number,
                field: self.printer.get_field_info(),
            }))
        };
        self.printer.reset_field_state();
        Ok(result)
    }

    // Barcode parsers
    fn parse_barcode_128(&mut self, command: &str) {
        let parts = split_command(command, "^BC");
        let mut bc = Barcode128 {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            line: true,
            line_above: false,
            check_digit: false,
            mode: BarcodeMode::No,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil(s)) {
            bc.height = v;
        }
        if let Some(s) = parts.get(2) {
            if !s.is_empty() {
                bc.line = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                bc.line_above = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(4) {
            if !s.is_empty() {
                bc.check_digit = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(5) {
            if !s.is_empty() {
                bc.mode = to_barcode_mode(s.as_bytes()[0]);
            }
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::Barcode128Config(bc)));
    }

    fn parse_barcode_ean13(&mut self, command: &str) {
        let parts = split_command(command, "^BE");
        let mut bc = BarcodeEan13 {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            line: true,
            line_above: false,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil(s)) {
            bc.height = v;
        }
        if let Some(s) = parts.get(2) {
            if !s.is_empty() {
                bc.line = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                bc.line_above = to_bool_field(s.as_bytes()[0]);
            }
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::BarcodeEan13Config(bc)));
    }

    fn parse_barcode_2of5(&mut self, command: &str) {
        let parts = split_command(command, "^B2");
        let mut bc = Barcode2of5 {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            line: true,
            line_above: false,
            check_digit: false,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil(s)) {
            bc.height = v;
        }
        if let Some(s) = parts.get(2) {
            if !s.is_empty() {
                bc.line = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                bc.line_above = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(4) {
            if !s.is_empty() {
                bc.check_digit = to_bool_field(s.as_bytes()[0]);
            }
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::Barcode2of5Config(bc)));
    }

    fn parse_barcode_39(&mut self, command: &str) {
        let parts = split_command(command, "^B3");
        let mut bc = Barcode39 {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            line: true,
            line_above: false,
            check_digit: false,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(1) {
            if !s.is_empty() {
                bc.check_digit = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int_ceil(s)) {
            bc.height = v;
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                bc.line = to_bool_field(s.as_bytes()[0]);
            }
        }
        if let Some(s) = parts.get(4) {
            if !s.is_empty() {
                bc.line_above = to_bool_field(s.as_bytes()[0]);
            }
        }
        self.printer.next_element_field_element = Some(Box::new(LabelElement::Barcode39Config(bc)));
    }

    fn parse_barcode_pdf417(&mut self, command: &str) {
        let parts = split_command(command, "^B7");
        let mut bc = BarcodePdf417 {
            orientation: self.printer.default_orientation,
            row_height: 0,
            security: 0,
            columns: 0,
            rows: 0,
            truncate: false,
            module_width: self.printer.default_barcode_dimensions.module_width,
            by_height: self.printer.default_barcode_dimensions.height,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            bc.row_height = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            bc.security = v;
        }
        if let Some(v) = parts.get(3).and_then(|s| parse_int(s)) {
            bc.columns = v;
        }
        if let Some(v) = parts.get(4).and_then(|s| parse_int(s)) {
            bc.rows = v;
        }
        if let Some(s) = parts.get(5) {
            if !s.is_empty() {
                bc.truncate = to_bool_field(s.as_bytes()[0]);
            }
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::BarcodePdf417Config(bc)));
    }

    fn parse_barcode_aztec(&mut self, command: &str) {
        let parts = split_command(command, "^BO");
        let mut bc = BarcodeAztec {
            orientation: self.printer.default_orientation,
            magnification: 0,
            size: 0,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            bc.magnification = v;
        }
        if let Some(v) = parts.get(3).and_then(|s| parse_int(s)) {
            bc.size = v;
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::BarcodeAztecConfig(bc)));
    }

    fn parse_barcode_datamatrix(&mut self, command: &str) {
        let parts = split_command(command, "^BX");
        let mut bc = BarcodeDatamatrix {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            quality: 0,
            columns: 0,
            rows: 0,
            format: 6,
            escape: b'~',
            ratio: Some(DatamatrixRatio::Square),
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil(s)) {
            bc.height = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            bc.quality = v;
        }
        if let Some(v) = parts.get(3).and_then(|s| parse_int(s)) {
            bc.columns = v;
        }
        if let Some(v) = parts.get(4).and_then(|s| parse_int(s)) {
            bc.rows = v;
        }
        if let Some(v) = parts.get(5).and_then(|s| parse_int(s)) {
            if v > 0 {
                bc.format = v;
            }
        }
        if let Some(s) = parts.get(6) {
            if !s.is_empty() {
                bc.escape = s.as_bytes()[0];
            }
        }
        if let Some(v) = parts.get(7).and_then(|s| parse_int(s)) {
            if v == 1 {
                bc.ratio = Some(DatamatrixRatio::Square);
            } else if v == 2 {
                bc.ratio = Some(DatamatrixRatio::Rectangular);
            }
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::BarcodeDatamatrixConfig(bc)));
    }

    fn parse_barcode_qr(&mut self, command: &str) {
        let parts = split_command(command, "^BQ");
        let mut bc = BarcodeQr { magnification: 1 };
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            bc.magnification = v.clamp(1, 100);
        }
        self.printer.next_element_field_element = Some(Box::new(LabelElement::BarcodeQrConfig(bc)));
    }

    fn parse_maxicode(&mut self, command: &str) {
        let parts = split_command(command, "^BD");
        let mut mc = Maxicode { mode: 0 };
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            mc.mode = v;
        }
        self.printer.next_element_field_element = Some(Box::new(LabelElement::MaxicodeConfig(mc)));
    }

    fn parse_barcode_field_defaults(&mut self, command: &str) {
        let parts = split_command(command, "^BY");
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            self.printer.default_barcode_dimensions.module_width = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_float(s)) {
            self.printer.default_barcode_dimensions.width_ratio = v.clamp(2.0, 3.0);
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            self.printer.default_barcode_dimensions.height = v;
        }
    }

    // Graphic commands
    fn parse_graphic_box(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GB");
        let mut gb = GraphicBox {
            position: self.printer.next_element_position.clone(),
            width: 1,
            height: 1,
            border_thickness: 1,
            corner_rounding: 0,
            line_color: LineColor::Black,
            reverse_print: self.printer.get_reverse_print(),
        };
        if let Some(v) = parts.get(2).and_then(|s| to_positive_int(s)) {
            if v > 0 {
                gb.border_thickness = v;
            }
        }
        if let Some(v) = parts.first().and_then(|s| to_positive_int(s)) {
            if v > 0 {
                gb.width = v.max(gb.border_thickness);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| to_positive_int(s)) {
            if v > 0 {
                gb.height = v.max(gb.border_thickness);
            }
        }
        if parts.get(3).is_some_and(|s| *s == "W") {
            gb.line_color = LineColor::White;
        }
        if let Some(v) = parts.get(4).and_then(|s| parse_int(s)) {
            if v > 0 && v < 9 {
                gb.corner_rounding = v;
            }
        }
        Ok(Some(LabelElement::GraphicBox(gb)))
    }

    fn parse_graphic_circle(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GC");
        let mut gc = GraphicCircle {
            position: self.printer.next_element_position.clone(),
            circle_diameter: 3,
            border_thickness: 1,
            line_color: LineColor::Black,
            reverse_print: self.printer.get_reverse_print(),
        };
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            gc.circle_diameter = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            gc.border_thickness = v;
        }
        if parts.get(2).is_some_and(|s| *s == "W") {
            gc.line_color = LineColor::White;
        }
        Ok(Some(LabelElement::GraphicCircle(gc)))
    }

    fn parse_graphic_diagonal_line(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GD");
        let mut gd = GraphicDiagonalLine {
            position: self.printer.next_element_position.clone(),
            width: 3,
            height: 3,
            border_thickness: 1,
            line_color: LineColor::Black,
            top_to_bottom: false,
            reverse_print: self.printer.get_reverse_print(),
        };
        // Parse thickness first — w and h default to max(t, 3) per spec
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            gd.border_thickness = v.max(1);
        }
        let default_wh = gd.border_thickness.max(3);
        gd.width = default_wh;
        gd.height = default_wh;
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            gd.width = v.max(3);
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            gd.height = v.max(3);
        }
        if parts.get(3).is_some_and(|s| *s == "W") {
            gd.line_color = LineColor::White;
        }
        // R (default) = right-leaning / = top_to_bottom false
        // L = left-leaning \ = top_to_bottom true
        if parts.get(4).is_some_and(|s| *s == "L" || *s == "\\") {
            gd.top_to_bottom = true;
        }
        Ok(Some(LabelElement::DiagonalLine(gd)))
    }

    fn parse_graphic_field(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GF");
        let mut gf = GraphicField {
            position: self.printer.next_element_position.clone(),
            magnification_x: 1,
            magnification_y: 1,
            reverse_print: self.printer.get_reverse_print(),
            format: GraphicFieldFormat::Hex,
            data_bytes: 0,
            total_bytes: 0,
            row_bytes: 0,
            data: Vec::new(),
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                match s.as_bytes()[0] {
                    b'A' => gf.format = GraphicFieldFormat::Hex,
                    b'B' => gf.format = GraphicFieldFormat::Raw,
                    b'C' => gf.format = GraphicFieldFormat::AR,
                    _ => {}
                }
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            gf.data_bytes = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            gf.total_bytes = v;
        }
        if let Some(v) = parts.get(3).and_then(|s| parse_int(s)) {
            gf.row_bytes = v.min(9999999);
        }
        if parts.len() > 4 {
            let data = parts[4..].join(",").trim().to_string();
            match gf.format {
                GraphicFieldFormat::Hex => {
                    gf.data = hex::decode_graphic_field_data(&data, gf.row_bytes)
                        .map_err(|e| format!("failed to decode hex string: {}", e))?;
                }
                GraphicFieldFormat::Raw => {
                    gf.data = data.into_bytes();
                }
                _ => {}
            }
        }
        Ok(Some(LabelElement::GraphicField(gf)))
    }

    fn parse_graphic_symbol(&mut self, command: &str) {
        let parts = split_command(command, "^GS");
        let mut gs = GraphicSymbol {
            width: self.printer.default_font.width,
            height: self.printer.default_font.height,
            orientation: self.printer.default_orientation,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                gs.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            gs.height = v as f64;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            gs.width = v as f64;
        }
        self.printer.next_element_field_element =
            Some(Box::new(LabelElement::GraphicSymbolConfig(gs)));
    }

    fn parse_download_graphics(&mut self, command: &str) -> Result<(), String> {
        let data = &command["~DG".len()..];
        let parts: Vec<&str> = data.splitn(4, ',').collect();

        let mut path = STORED_GRAPHICS_DEFAULT_PATH.to_string();
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                path = s.to_string();
            }
        }

        let mut graphics = crate::elements::stored_graphics::StoredGraphics {
            total_bytes: 0,
            row_bytes: 1,
            data: Vec::new(),
        };

        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            graphics.total_bytes = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            graphics.row_bytes = v.min(9999999);
        }
        if let Some(s) = parts.get(3) {
            graphics.data = hex::decode_graphic_field_data(s, graphics.row_bytes)
                .map_err(|e| format!("failed to decode embedded graphics: {}", e))?;
        }

        let path = ensure_extension(&path, "GRF");
        self.printer.stored_graphics.insert(path, graphics);
        Ok(())
    }

    fn parse_image_load(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^IL");
        let mut gf = GraphicField {
            position: LabelPosition::default(),
            magnification_x: 1,
            magnification_y: 1,
            reverse_print: ReversePrint::default(),
            format: GraphicFieldFormat::Hex,
            data_bytes: 0,
            total_bytes: 0,
            row_bytes: 0,
            data: Vec::new(),
        };

        let mut path = STORED_GRAPHICS_DEFAULT_PATH.to_string();
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                path = s.to_string();
            }
        }

        if let Some(v) = self.printer.stored_graphics.get(&path) {
            gf.data = v.data.clone();
            gf.data_bytes = v.total_bytes;
            gf.total_bytes = v.total_bytes;
            gf.row_bytes = v.row_bytes;
            Ok(Some(LabelElement::GraphicField(gf)))
        } else {
            Ok(None)
        }
    }

    fn parse_recall_graphics(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^XG");
        let mut gf = GraphicField {
            position: self.printer.next_element_position.clone(),
            magnification_x: 1,
            magnification_y: 1,
            reverse_print: self.printer.get_reverse_print(),
            format: GraphicFieldFormat::Hex,
            data_bytes: 0,
            total_bytes: 0,
            row_bytes: 0,
            data: Vec::new(),
        };

        let mut path = STORED_GRAPHICS_DEFAULT_PATH.to_string();
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                path = s.to_string();
            }
        }

        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            if (0..=10).contains(&v) {
                gf.magnification_x = v;
            }
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            if (0..=10).contains(&v) {
                gf.magnification_y = v;
            }
        }

        if let Some(v) = self.printer.stored_graphics.get(&path) {
            gf.data = v.data.clone();
            gf.data_bytes = v.total_bytes;
            gf.total_bytes = v.total_bytes;
            gf.row_bytes = v.row_bytes;
            Ok(Some(LabelElement::GraphicField(gf)))
        } else {
            Ok(None)
        }
    }

    fn parse_download_format(&mut self, command: &str) -> Result<(), String> {
        let path_text = command_text(command, "^DF");
        let path = if path_text.is_empty() {
            STORED_FORMAT_DEFAULT_PATH.to_string()
        } else {
            path_text.to_string()
        };
        validate_device(&path)?;
        self.printer.next_download_format_name = ensure_extension(&path, "ZPL");
        Ok(())
    }

    fn parse_recall_format(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let path_text = command_text(command, "^XF");
        let path = if path_text.is_empty() {
            STORED_FORMAT_DEFAULT_PATH.to_string()
        } else {
            path_text.to_string()
        };
        validate_device(&path)?;
        let key = ensure_extension(&path, "ZPL");
        if let Some(v) = self.printer.stored_formats.get(&key) {
            Ok(Some(LabelElement::RecalledFormat(v.to_recalled_format())))
        } else {
            Ok(None)
        }
    }
}

/// Resolve a RecalledField into a drawable LabelElement (used in field separator)
fn resolve_recalled_field(
    f: &crate::elements::stored_format::RecalledField,
) -> Result<Option<LabelElement>, String> {
    use crate::elements::stored_format::RecalledFormat;

    // Build a temporary RecalledFormat with a single field and resolve it
    let rf = RecalledFormat {
        inverted: false,
        elements: vec![LabelElement::RecalledField(f.clone())],
        field_refs: std::collections::HashMap::new(),
    };

    let resolved = rf.resolve_elements()?;
    Ok(resolved.into_iter().next())
}

fn split_zpl_commands(zpl_data: &[u8]) -> Result<Vec<String>, String> {
    let data_str = String::from_utf8_lossy(zpl_data);
    let data = data_str.replace(['\n', '\r', '\t'], "");

    let mut caret = '^';
    let mut tilde = '~';

    let mut buff = String::new();
    let mut results = Vec::new();

    for ch in data.chars() {
        let mut is_ct = false;
        let mut is_cc = false;
        if buff.len() == 4 {
            is_ct = buff.contains("CT") && buff.starts_with(caret);
            is_cc = buff.contains("CC") && buff.starts_with(caret);
        }

        if ch == caret || ch == tilde || is_ct || is_cc {
            let normalized = normalize_command(&buff, tilde, caret);

            if is_ct && normalized.len() >= 4 {
                tilde = normalized.chars().nth(3).unwrap_or('~');
            } else if is_cc && normalized.len() >= 4 {
                caret = normalized.chars().nth(3).unwrap_or('^');
            } else if !normalized.is_empty() {
                results.push(normalized);
            }

            buff.clear();
        }

        buff.push(ch);
    }

    if !buff.is_empty() {
        let normalized = normalize_command(&buff, tilde, caret);
        if !normalized.is_empty() {
            results.push(normalized);
        }
    }

    Ok(results)
}

fn normalize_command(command: &str, tilde: char, caret: char) -> String {
    if command.is_empty() {
        return String::new();
    }
    let mut cmd = command.to_string();
    let first = cmd.chars().next().unwrap();
    if caret != '^' && first == caret {
        cmd = format!("^{}", &cmd[first.len_utf8()..]);
    }
    if tilde != '~' && first == tilde {
        cmd = format!("~{}", &cmd[first.len_utf8()..]);
    }
    cmd.trim_start().to_string()
}
