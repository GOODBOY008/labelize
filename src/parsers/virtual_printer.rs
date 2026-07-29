use std::collections::HashMap;

use crate::elements::barcode_info::BarcodeDimensions;
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_info::FieldInfo;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::label_element::LabelElement;
use crate::elements::label_position::LabelPosition;
use crate::elements::measurement_unit::MeasurementUnit;
use crate::elements::reverse_print::ReversePrint;
use crate::elements::stored_format::StoredFormat;
use crate::elements::stored_graphics::StoredGraphics;

pub struct VirtualPrinter {
    pub stored_graphics: HashMap<String, StoredGraphics>,
    pub stored_formats: HashMap<String, StoredFormat>,
    pub label_home_position: LabelPosition,
    pub next_element_position: LabelPosition,
    pub default_font: FontInfo,
    pub last_field_font: FontInfo,
    pub default_orientation: FieldOrientation,
    pub default_alignment: FieldAlignment,
    pub next_element_alignment: Option<FieldAlignment>,
    pub next_element_field_element: Option<Box<LabelElement>>,
    pub next_element_field_data: String,
    pub next_element_field_number: i32,
    pub next_font: Option<FontInfo>,
    pub next_download_format_name: String,
    pub next_hex_escape_char: u8,
    pub next_element_field_reverse: bool,
    pub label_reverse: bool,
    pub default_barcode_dimensions: BarcodeDimensions,
    pub current_charset: i32,
    pub print_width: i32,
    pub label_inverted: bool,
    /// Printer resolution; must match the resolution the label is rendered at, since
    /// `^MU I`/`^MU M` need it to turn physical units into dots.
    pub dpmm: i32,
    /// Unit of measurement selected by `^MU` (parameter a).
    pub measurement_unit: MeasurementUnit,
    /// dpi conversion factor from `^MU`'s b (format base) and c (desired) parameters.
    pub dpi_conversion: f64,
}

impl Default for VirtualPrinter {
    fn default() -> Self {
        VirtualPrinter {
            stored_graphics: HashMap::new(),
            stored_formats: HashMap::new(),
            label_home_position: LabelPosition::default(),
            next_element_position: LabelPosition::default(),
            default_font: FontInfo {
                name: "A".to_string(),
                ..FontInfo::default()
            },
            last_field_font: FontInfo::default(),
            default_orientation: FieldOrientation::Normal,
            default_alignment: FieldAlignment::Left,
            next_element_alignment: None,
            next_element_field_element: None,
            next_element_field_data: String::new(),
            next_element_field_number: -1,
            next_font: None,
            next_download_format_name: String::new(),
            next_hex_escape_char: 0,
            next_element_field_reverse: false,
            label_reverse: false,
            default_barcode_dimensions: BarcodeDimensions::default(),
            current_charset: 0,
            print_width: 0,
            label_inverted: false,
            dpmm: 8,
            measurement_unit: MeasurementUnit::default(),
            dpi_conversion: 1.0,
        }
    }
}

impl VirtualPrinter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dpmm(dpmm: i32) -> Self {
        VirtualPrinter {
            dpmm: if dpmm > 0 { dpmm } else { 8 },
            ..Self::default()
        }
    }

    /// Factor converting a `^MU`-relative measurement into printer dots. The b/c dpi
    /// conversion applies to dots only -- inches and millimeters are absolute sizes.
    pub fn unit_scale(&self) -> f64 {
        match self.measurement_unit {
            MeasurementUnit::Dots => self.dpi_conversion,
            other => other.dots_per_unit(self.dpmm as f64),
        }
    }

    pub fn set_default_orientation(&mut self, orientation: FieldOrientation) {
        self.default_orientation = orientation;
        self.default_font.orientation = orientation;
        if let Some(ref mut f) = self.next_font {
            f.orientation = orientation;
        }
    }

    pub fn get_next_font_or_default(&self) -> FontInfo {
        self.next_font
            .clone()
            .unwrap_or_else(|| self.default_font.clone())
    }

    pub fn get_next_element_alignment_or_default(&self) -> FieldAlignment {
        self.next_element_alignment
            .unwrap_or(self.default_alignment)
    }

    pub fn get_reverse_print(&self) -> ReversePrint {
        ReversePrint {
            value: self.next_element_field_reverse || self.label_reverse,
        }
    }

    pub fn get_field_info(&self) -> FieldInfo {
        FieldInfo {
            reverse_print: self.get_reverse_print(),
            element: self.next_element_field_element.clone(),
            font: self.get_next_font_or_default(),
            position: self.next_element_position.clone(),
            alignment: self.get_next_element_alignment_or_default(),
            width: self.default_barcode_dimensions.module_width,
            width_ratio: self.default_barcode_dimensions.width_ratio,
            height: self.default_barcode_dimensions.height,
            current_charset: self.current_charset,
        }
    }

    pub fn reset_field_state(&mut self) {
        self.next_element_position = LabelPosition::default();
        self.next_element_field_element = None;
        self.next_element_field_data = String::new();
        self.next_element_field_number = -1;
        self.next_element_alignment = None;
        // Save font used by the last field so ^GS can inherit it when no size specified
        self.last_field_font = self
            .next_font
            .clone()
            .unwrap_or_else(|| self.default_font.clone());
        self.next_font = None;
        self.next_element_field_reverse = false;
        self.next_hex_escape_char = 0;
    }

    pub fn reset_label_state(&mut self) {
        self.next_download_format_name = String::new();
        self.label_inverted = false;
    }
}
