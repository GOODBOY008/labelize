use crate::elements::barcode_128::{Barcode128, BarcodeMode};
use crate::elements::barcode_2of5::Barcode2of5;
use crate::elements::barcode_39::Barcode39;
use crate::elements::barcode_aztec::BarcodeAztec;
use crate::elements::barcode_datamatrix::{BarcodeDatamatrix, DatamatrixRatio};
use crate::elements::barcode_ean13::BarcodeEan13;
use crate::elements::barcode_ean8::BarcodeEan8;
use crate::elements::barcode_pdf417::BarcodePdf417;
use crate::elements::barcode_qr::BarcodeQr;
use crate::elements::barcode_upca::BarcodeUca;
use crate::elements::barcode_upce::BarcodeUcpe;
use crate::elements::field_block::FieldBlock;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::graphic_circle::GraphicCircle;
use crate::elements::graphic_diagonal_line::GraphicDiagonalLine;
use crate::elements::graphic_ellipse::GraphicEllipse;
use crate::elements::graphic_field::{GraphicField, GraphicFieldFormat};
use crate::elements::graphic_symbol::GraphicSymbol;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::maxicode::Maxicode;
use crate::elements::measurement_unit::MeasurementUnit;
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

    /// Parser for a printer of the given resolution (dots per millimeter). Pass the same
    /// dpmm the label is rendered at, or `^MU I`/`^MU M` formats come out at the wrong
    /// scale; everything else is resolution-independent.
    pub fn with_dpmm(dpmm: i32) -> Self {
        ZplParser {
            printer: VirtualPrinter::with_dpmm(dpmm),
        }
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
                    // ^LT/^LS apply when the label is emitted, never when a format is
                    // stored: recalled formats keep raw positions and shift with the
                    // recall format's own ^LT/^LS (Labelary behavior, verified). This
                    // also makes the shift retroactive for fields placed before the
                    // command appeared in the format.
                    let mut shifted = result_elements.clone();
                    apply_label_offsets(
                        &mut shifted,
                        self.printer.label_shift,
                        self.printer.label_top,
                    );
                    let quantity =
                        self.printer.print_quantity.max(1) * self.printer.print_copies.max(1);
                    for _ in 0..quantity {
                        results.push(LabelInfo {
                            print_width: self.printer.print_width,
                            inverted: self.printer.label_inverted,
                            elements: shifted.clone(),
                        });
                    }
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
        // Label top
        if upper.starts_with("^LT") {
            self.parse_label_top(command);
            return Ok(None);
        }
        // Label shift
        if upper.starts_with("^LS") {
            self.parse_label_shift(command);
            return Ok(None);
        }
        // Label length (recorded; rendering size comes from the canvas, so ^LL
        // has no visible effect, matching Labelary)
        if upper.starts_with("^LL") {
            let scale = self.printer.unit_scale();
            if let Some(v) = command_text(command, "^LL")
                .trim()
                .parse::<f64>()
                .ok()
                .map(|v| (v * scale).round() as i32)
            {
                if v > 0 {
                    self.printer.label_length = v;
                }
            }
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
            let scale = self.printer.unit_scale();
            if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
                self.printer.print_width = v.max(2);
            }
            return Ok(None);
        }
        // Units of measurement
        if upper.starts_with("^MU") {
            self.parse_measurement_units(command);
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
        // Change font by name
        if upper.starts_with("^A@") {
            self.parse_change_font_named(command);
            return Ok(None);
        }
        // Change font
        if upper.starts_with("^A") && !upper.starts_with("^A@") {
            self.parse_change_font(command);
            return Ok(None);
        }
        // Font identifier assignment
        if upper.starts_with("^CW") {
            self.parse_font_identifier(command);
            return Ok(None);
        }
        // Comment — payload is ignored (recognized explicitly so future command
        // additions can never intercept comment text).
        if upper.starts_with("^FX") {
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
        if upper.starts_with("^B8") {
            self.parse_barcode_ean8(command);
            return Ok(None);
        }
        if upper.starts_with("^BU") {
            self.parse_barcode_upca(command);
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
        if upper.starts_with("^B9") {
            self.parse_barcode_upce(command);
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
        if upper.starts_with("^GE") {
            return self.parse_graphic_ellipse(command);
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
        if upper.starts_with("^ID") {
            self.parse_image_delete(command);
            return Ok(None);
        }
        if upper.starts_with("^IM") {
            self.parse_image_move(command);
            return Ok(None);
        }
        if upper.starts_with("^IS") {
            self.parse_image_save(command);
            return Ok(None);
        }
        if upper.starts_with("~EG") {
            self.parse_erase_graphic(command);
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

        // Print quantity and serial state (serial markers in ^FD render literally,
        // matching Labelary, so ^SN/^SF only record stream state)
        if upper.starts_with("^PQ") {
            self.parse_print_quantity(command);
            return Ok(None);
        }
        if upper.starts_with("^SN") {
            self.printer.serial_number = command_text(command, "^SN").to_string();
            return Ok(None);
        }
        if upper.starts_with("^SF") {
            self.printer.serial_format = command_text(command, "^SF").to_string();
            return Ok(None);
        }

        Ok(None)
    }

    fn parse_label_home(&mut self, command: &str) {
        let parts = split_command(command, "^LH");
        let scale = self.printer.unit_scale();
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            self.printer.label_home_position.x = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_scaled(s, scale)) {
            self.printer.label_home_position.y = v;
        }
    }

    /// `^LTv` -- label top. Shifts every element's y by +v at label emission.
    /// Zebra/Labelary accept only |v| <= 120 dots and ignore larger values
    /// (calibrated: `^LT121` and `^LT-121` render unshifted).
    fn parse_label_top(&mut self, command: &str) {
        let parts = split_command(command, "^LT");
        let scale = self.printer.unit_scale();
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            if v.abs() <= 120 {
                self.printer.label_top = v;
            }
        }
    }

    /// `^LSv` -- label shift. Positive shifts content left, negative right;
    /// per-element x is clamped at 0 (Labelary behavior).
    fn parse_label_shift(&mut self, command: &str) {
        let parts = split_command(command, "^LS");
        let scale = self.printer.unit_scale();
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            self.printer.label_shift = v;
        }
    }

    /// `^MUa,b,c` -- unit of measurement (a) plus dpi conversion (b -> c). Per spec b and
    /// c are ignored unless both are given; matching values reset the conversion. The mode
    /// carries over from field to field until the next `^MU`.
    fn parse_measurement_units(&mut self, command: &str) {
        let parts = split_command(command, "^MU");
        if let Some(unit) = parts
            .first()
            .and_then(|s| s.trim().as_bytes().first().copied())
            .and_then(MeasurementUnit::from_byte)
        {
            self.printer.measurement_unit = unit;
        }
        let base = parts.get(1).and_then(|s| parse_float(s));
        let target = parts.get(2).and_then(|s| parse_float(s));
        if let (Some(base), Some(target)) = (base, target) {
            if base > 0.0 && target > 0.0 {
                self.printer.dpi_conversion = target / base;
            }
        }
    }

    fn parse_change_default_font(&mut self, command: &str) {
        let parts = split_command(command, "^CF");
        // The font designator is a single character; real printers and Labelary tolerate height
        // digits glued onto it (^CFB0,30 = font B, height 0, width 30) -- the same leniency
        // parse_change_font() already applies to ^A. Taking the whole first part as the name
        // ("B0") matches no font and silently falls back to the default TTF, shredding layouts
        // authored for the intended font (observed on production courier labels).
        let (extra_height, height_idx, width_idx) = match parts.first() {
            Some(s) if !s.is_empty() => {
                let first = s.as_bytes();
                let name = (first[0] as char).to_uppercase().to_string();
                let mut probe = self.printer.default_font.clone();
                probe.name = name.clone();
                if probe.is_standard_font() {
                    self.printer.default_font.name = name;
                } else if (first[0] as char).is_ascii_digit() {
                    // Numeric font names (1-9) are user-installed fonts on Zebra printers.
                    // Fall back to font "0" (proportional), as parse_change_font does.
                    self.printer.default_font.name = "0".to_string();
                }
                if first.len() > 1 {
                    let glued = std::str::from_utf8(&first[1..]).unwrap_or("").to_string();
                    (Some(glued), usize::MAX, 1usize)
                } else {
                    (None, 1, 2)
                }
            }
            _ => (None, 1, 2),
        };

        let scale = self.printer.unit_scale();
        let height = match &extra_height {
            Some(hs) => parse_int_scaled(hs, scale),
            None => parts
                .get(height_idx)
                .and_then(|s| parse_int_scaled(s, scale)),
        };
        let width = parts
            .get(width_idx)
            .and_then(|s| parse_int_scaled(s, scale));
        if let Some(v) = height {
            self.printer.default_font.height = v as f64;
        }
        // Per ZPL spec: "Defining only the height or width forces the magnification to be
        // proportional to the parameter defined." When only height is given (no explicit width),
        // reset width to 0 so with_adjusted_sizes() derives it proportionally from height.
        if height.is_some() && width.is_none() {
            self.printer.default_font.width = 0.0;
        }
        if let Some(v) = width {
            self.printer.default_font.width = v as f64;
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

        // ^CW can remap a font identifier (the ^A first character) to a font name.
        if let Some(mapped) = self.printer.font_map.get(&font.name).cloned() {
            font.name = self.resolve_font_name(&mapped);
        }

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

        let scale = self.printer.unit_scale();
        if let Some(hs) = extra_height_str {
            if let Some(v) = parse_int_scaled(&hs, scale) {
                font.height = v as f64;
            }
        } else if let Some(s) = parts.get(height_part_idx) {
            if let Some(v) = parse_int_scaled(s, scale) {
                font.height = v as f64;
            }
        }
        if let Some(s) = parts.get(width_part_idx) {
            if let Some(v) = parse_int_scaled(s, scale) {
                font.width = v as f64;
            }
        }

        self.printer.next_font = Some(font);
    }

    /// `^A@f,h,w` -- change font by name. `f` may be a built-in designator or a
    /// downloadable font name (device prefix, extension); names we cannot render
    /// degrade to the default font like `^A`'s numeric fallback does.
    fn parse_change_font_named(&mut self, command: &str) {
        let parts = split_command(command, "^A@");
        if parts.is_empty() || parts[0].is_empty() {
            self.printer.next_font = None;
            return;
        }
        let mut font = crate::elements::font::FontInfo {
            name: self.resolve_font_name(parts[0]),
            orientation: self.printer.default_font.orientation,
            ..Default::default()
        };

        let scale = self.printer.unit_scale();
        if let Some(s) = parts.get(1) {
            if let Some(v) = parse_int_scaled(s, scale) {
                font.height = v as f64;
            }
        }
        if let Some(s) = parts.get(2) {
            if let Some(v) = parse_int_scaled(s, scale) {
                font.width = v as f64;
            }
        }
        self.printer.next_font = Some(font);
    }

    /// `^CWx,f` -- assign a font name to a font identifier character (A-Z, 0-9);
    /// later `^A` with that identifier uses the mapped font.
    fn parse_font_identifier(&mut self, command: &str) {
        let parts = split_command(command, "^CW");
        if let Some(id) = parts.first() {
            if let Some(c) = id.trim().chars().next() {
                if c.is_ascii_alphanumeric() {
                    let name = parts
                        .get(1)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        self.printer
                            .font_map
                            .insert(c.to_uppercase().to_string(), name);
                    }
                }
            }
        }
    }

    /// Resolve a font name (from `^A@` or a `^CW`-mapped name) to a renderable
    /// built-in designator. Downloadable font names are unrecognizable here and fall
    /// back to the default font.
    fn resolve_font_name(&self, name: &str) -> String {
        let base = name.rsplit(':').next().unwrap_or(name);
        let base = base.split('.').next().unwrap_or(base);
        let mut chars = base.chars();
        let single = match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => return self.printer.default_font.name.clone(),
        };
        let probe = crate::elements::font::FontInfo {
            name: single.to_uppercase().to_string(),
            ..Default::default()
        };
        if probe.is_standard_font() {
            probe.name
        } else if single.is_ascii_digit() {
            // Numeric font names (1-9) are user-installed fonts: fall back to font 0.
            "0".to_string()
        } else {
            self.printer.default_font.name.clone()
        }
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
        let scale = self.printer.unit_scale();
        let mut pos = LabelPosition {
            calculate_from_bottom: false,
            ..Default::default()
        };
        if let Some(v) = parts
            .first()
            .and_then(|s| to_positive_int_lenient_scaled(s, scale))
        {
            pos.x = v;
        }
        if let Some(v) = parts
            .get(1)
            .and_then(|s| to_positive_int_lenient_scaled(s, scale))
        {
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
        let scale = self.printer.unit_scale();
        let mut pos = LabelPosition {
            calculate_from_bottom: true,
            automatic_position: true,
            ..Default::default()
        };
        if let Some(v) = parts
            .first()
            .and_then(|s| to_positive_int_lenient_scaled(s, scale))
        {
            pos.x = v;
            pos.automatic_position = false;
        }
        if let Some(v) = parts
            .get(1)
            .and_then(|s| to_positive_int_lenient_scaled(s, scale))
        {
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
        let scale = self.printer.unit_scale();
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            block.max_width = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int(s)) {
            block.max_lines = v;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int_scaled(s, scale)) {
            block.line_spacing = v;
        }
        if let Some(s) = parts.get(3) {
            if !s.is_empty() {
                block.alignment = to_text_alignment(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(4).and_then(|s| parse_int_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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

    /// `^B8o,h,f,g` -- EAN-8. Defaults: orientation N, height from ^BY,
    /// interpretation line on (below), not above.
    fn parse_barcode_ean8(&mut self, command: &str) {
        let parts = split_command(command, "^B8");
        let scale = self.printer.unit_scale();
        let mut bc = BarcodeEan8 {
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
            Some(Box::new(LabelElement::BarcodeEan8Config(bc)));
    }

    /// `^BUo,h,f,g` -- UPC-A. Defaults like ^B8.
    fn parse_barcode_upca(&mut self, command: &str) {
        let parts = split_command(command, "^BU");
        let scale = self.printer.unit_scale();
        let mut bc = BarcodeUca {
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
            Some(Box::new(LabelElement::BarcodeUcaConfig(bc)));
    }

    fn parse_barcode_2of5(&mut self, command: &str) {
        let parts = split_command(command, "^B2");
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(2).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
        // Zebra defines Mode 2 as the default when the ^BD mode parameter is omitted.
        let mut mc = Maxicode { mode: 2 };
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            mc.mode = v;
        }
        self.printer.next_element_field_element = Some(Box::new(LabelElement::MaxicodeConfig(mc)));
    }

    /// `^B9o,h,f,g,e` -- UPC-E. Defaults: orientation N, height from ^BY,
    /// interpretation line on (below), not above, check digit printed.
    fn parse_barcode_upce(&mut self, command: &str) {
        let parts = split_command(command, "^B9");
        let scale = self.printer.unit_scale();
        let mut bc = BarcodeUcpe {
            orientation: self.printer.default_orientation,
            height: self.printer.default_barcode_dimensions.height,
            line: true,
            line_above: false,
            check_digit: true,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                bc.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_ceil_scaled(s, scale)) {
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
            Some(Box::new(LabelElement::BarcodeUcpeConfig(bc)));
    }

    fn parse_barcode_field_defaults(&mut self, command: &str) {
        let parts = split_command(command, "^BY");
        let scale = self.printer.unit_scale();
        // Never below one dot: a sub-dot module collapses the barcode.
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            self.printer.default_barcode_dimensions.module_width = v.max(1);
        }
        // Parameter b is a ratio, not a measurement: never scaled.
        if let Some(v) = parts.get(1).and_then(|s| parse_float(s)) {
            self.printer.default_barcode_dimensions.width_ratio = v.clamp(2.0, 3.0);
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int_scaled(s, scale)) {
            self.printer.default_barcode_dimensions.height = v;
        }
    }

    // Graphic commands
    fn parse_graphic_box(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GB");
        let scale = self.printer.unit_scale();
        let mut gb = GraphicBox {
            position: self.printer.next_element_position.clone(),
            width: 1,
            height: 1,
            border_thickness: 1,
            corner_rounding: 0,
            line_color: LineColor::Black,
            reverse_print: self.printer.get_reverse_print(),
        };
        if let Some(v) = parts.get(2).and_then(|s| to_positive_int_scaled(s, scale)) {
            if v > 0 {
                gb.border_thickness = v;
            }
        }
        if let Some(v) = parts.first().and_then(|s| to_positive_int_scaled(s, scale)) {
            if v > 0 {
                gb.width = v.max(gb.border_thickness);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| to_positive_int_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
        let mut gc = GraphicCircle {
            position: self.printer.next_element_position.clone(),
            circle_diameter: 3,
            border_thickness: 1,
            line_color: LineColor::Black,
            reverse_print: self.printer.get_reverse_print(),
        };
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            gc.circle_diameter = v;
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_scaled(s, scale)) {
            gc.border_thickness = v;
        }
        if parts.get(2).is_some_and(|s| *s == "W") {
            gc.line_color = LineColor::White;
        }
        Ok(Some(LabelElement::GraphicCircle(gc)))
    }

    fn parse_graphic_ellipse(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GE");
        let scale = self.printer.unit_scale();
        let mut ge = GraphicEllipse {
            position: self.printer.next_element_position.clone(),
            width: 3,
            height: 3,
            border_thickness: 1,
            line_color: LineColor::Black,
            reverse_print: self.printer.get_reverse_print(),
        };
        if let Some(v) = parts.first().and_then(|s| to_positive_int_scaled(s, scale)) {
            if v > 0 {
                ge.width = v;
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| to_positive_int_scaled(s, scale)) {
            if v > 0 {
                ge.height = v;
            }
        }
        if let Some(v) = parts.get(2).and_then(|s| to_positive_int_scaled(s, scale)) {
            if v > 0 {
                ge.border_thickness = v;
            }
        }
        if parts.get(3).is_some_and(|s| *s == "W") {
            ge.line_color = LineColor::White;
        }
        Ok(Some(LabelElement::GraphicEllipse(ge)))
    }

    fn parse_graphic_diagonal_line(&self, command: &str) -> Result<Option<LabelElement>, String> {
        let parts = split_command(command, "^GD");
        let scale = self.printer.unit_scale();
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
        if let Some(v) = parts.get(2).and_then(|s| parse_int_scaled(s, scale)) {
            gd.border_thickness = v.max(1);
        }
        let default_wh = gd.border_thickness.max(3);
        gd.width = default_wh;
        gd.height = default_wh;
        if let Some(v) = parts.first().and_then(|s| parse_int_scaled(s, scale)) {
            gd.width = v.max(3);
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_scaled(s, scale)) {
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
        let scale = self.printer.unit_scale();
        // When ^GS has no explicit size, inherit from last rendered field's font
        // (Labelary behavior: GS follows the most recent ^A font dimensions)
        let fallback = if self.printer.last_field_font.height > 0.0 {
            self.printer.last_field_font.clone()
        } else {
            self.printer.default_font.clone()
        };
        let mut gs = GraphicSymbol {
            width: fallback.width,
            height: fallback.height,
            orientation: self.printer.default_orientation,
        };
        if let Some(s) = parts.first() {
            if !s.is_empty() {
                gs.orientation = to_field_orientation(s.as_bytes()[0]);
            }
        }
        if let Some(v) = parts.get(1).and_then(|s| parse_int_scaled(s, scale)) {
            gs.height = v as f64;
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int_scaled(s, scale)) {
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

    /// `^IDname` -- delete a stored object. Without an extension the raw key, then
    /// the `.GRF`, then the `.ZPL` store are tried (spec: bare name deletes all
    /// stored objects of that name).
    fn parse_image_delete(&mut self, command: &str) {
        let name = command_text(command, "^ID").trim();
        if name.is_empty() {
            return;
        }
        let key = name.to_string();
        if self.printer.stored_graphics.remove(&key).is_some() {
            return;
        }
        if self.printer.stored_formats.remove(&key).is_some() {
            return;
        }
        let grf = ensure_extension(&key, "GRF");
        if self.printer.stored_graphics.remove(&grf).is_some() {
            return;
        }
        let zpl = ensure_extension(&key, "ZPL");
        self.printer.stored_formats.remove(&zpl);
    }

    /// `^IMa,b` -- move (rename) a stored object from a to b. The extension picks
    /// the store: `.ZPL` moves formats, anything else moves graphics.
    fn parse_image_move(&mut self, command: &str) {
        let parts = split_command(command, "^IM");
        if parts.len() < 2 {
            return;
        }
        let (from, to) = (parts[0].trim(), parts[1].trim());
        if let Some(key) = storage_key(from, "ZPL") {
            if let Some(v) = self.printer.stored_formats.remove(&key) {
                self.printer
                    .stored_formats
                    .insert(storage_key(to, "ZPL").unwrap_or_default(), v);
            }
            return;
        }
        let from_key = ensure_extension(from, "GRF");
        let to_key = ensure_extension(to, "GRF");
        if let Some(g) = self.printer.stored_graphics.remove(&from_key) {
            self.printer.stored_graphics.insert(to_key, g);
        }
    }

    /// `^ISa,b` -- copy a stored object from a to b (both names remain usable).
    fn parse_image_save(&mut self, command: &str) {
        let parts = split_command(command, "^IS");
        if parts.len() < 2 {
            return;
        }
        let (from, to) = (parts[0].trim(), parts[1].trim());
        if let Some(key) = storage_key(from, "ZPL") {
            if let Some(v) = self.printer.stored_formats.get(&key).cloned() {
                self.printer
                    .stored_formats
                    .insert(storage_key(to, "ZPL").unwrap_or_default(), v);
            }
            return;
        }
        let from_key = ensure_extension(from, "GRF");
        let to_key = ensure_extension(to, "GRF");
        if let Some(g) = self.printer.stored_graphics.get(&from_key).cloned() {
            self.printer.stored_graphics.insert(to_key, g);
        }
    }

    /// `~EGname` -- erase stored graphics; a blank name erases all of them.
    fn parse_erase_graphic(&mut self, command: &str) {
        let name = command_text(command, "~EG").trim();
        if name.is_empty() {
            self.printer.stored_graphics.clear();
        } else {
            let key = ensure_extension(name, "GRF");
            self.printer.stored_graphics.remove(&key);
        }
    }

    /// `^PQa,b,c,d` -- labels to print (a) and copies of the same label (c).
    /// The format emits a * c identical labels (serials render literally, so
    /// copies are pixel-identical, matching Labelary).
    fn parse_print_quantity(&mut self, command: &str) {
        let parts = split_command(command, "^PQ");
        if let Some(v) = parts.first().and_then(|s| parse_int(s)) {
            if v > 0 {
                self.printer.print_quantity = v;
            }
        }
        if let Some(v) = parts.get(2).and_then(|s| parse_int(s)) {
            if v > 0 {
                self.printer.print_copies = v;
            }
        }
    }
}

/// Storage key for a `.ZPL` extension check on `^IM`/`^IS` targets: returns the
/// normalized key when the name carries a `.ZPL` extension, else None (graphics).
fn storage_key(name: &str, ext: &str) -> Option<String> {
    if name.to_ascii_uppercase().ends_with(&format!(".{}", ext)) {
        Some(ensure_extension(name, ext))
    } else {
        None
    }
}

/// Apply `^LS`/`^LT` offsets to every positioned element at label emission time:
/// `^LS` shifts x left by `shift_x` (clamped at 0 per element), `^LT` shifts y down
/// by `top_y` (clamped at 0) — exactly Labelary's rendering. Stored formats keep
/// raw positions; only emitted labels are shifted, so a recalled format shifts with
/// the recall format's own `^LT`/`^LS`.
fn apply_label_offsets(elements: &mut [LabelElement], shift_x: i32, top_y: i32) {
    if shift_x == 0 && top_y == 0 {
        return;
    }
    for el in elements {
        let pos = match el {
            LabelElement::Text(t) => &mut t.position,
            LabelElement::GraphicBox(g) => &mut g.position,
            LabelElement::GraphicCircle(g) => &mut g.position,
            LabelElement::DiagonalLine(g) => &mut g.position,
            LabelElement::GraphicField(g) => &mut g.position,
            LabelElement::Barcode128(b) => &mut b.position,
            LabelElement::BarcodeEan13(b) => &mut b.position,
            LabelElement::Barcode2of5(b) => &mut b.position,
            LabelElement::Barcode39(b) => &mut b.position,
            LabelElement::BarcodePdf417(b) => &mut b.position,
            LabelElement::BarcodeAztec(b) => &mut b.position,
            LabelElement::BarcodeDatamatrix(b) => &mut b.position,
            LabelElement::BarcodeQr(b) => &mut b.position,
            LabelElement::Maxicode(m) => &mut m.position,
            LabelElement::StoredField(sf) => &mut sf.field.position,
            _ => continue,
        };
        pos.x = (pos.x - shift_x).max(0);
        pos.y = (pos.y + top_y).max(0);
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
