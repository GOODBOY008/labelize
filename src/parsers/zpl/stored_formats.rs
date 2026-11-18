//! Resolution of recalled ^XF formats into drawable elements.
//!
//! `RecalledFormat` and its field structs live in `elements` (they are part of
//! the shared IR that `LabelElement` references), but everything in this file
//! is ZPL semantics: merging ^FN placeholders with ^SN/^RV data, ^CI charset
//! conversion, and Zebra font-size adjustment all happen here, on the parser
//! side of the language boundary.

use crate::elements::barcode_128::Barcode128WithData;
use crate::elements::barcode_2of5::Barcode2of5WithData;
use crate::elements::barcode_39::Barcode39WithData;
use crate::elements::barcode_aztec::BarcodeAztecWithData;
use crate::elements::barcode_datamatrix::BarcodeDatamatrixWithData;
use crate::elements::barcode_ean13::BarcodeEan13WithData;
use crate::elements::barcode_ean8::BarcodeEan8WithData;
use crate::elements::barcode_pdf417::BarcodePdf417WithData;
use crate::elements::barcode_qr::BarcodeQrWithData;
use crate::elements::barcode_upca::BarcodeUcaWithData;
use crate::elements::barcode_upce::BarcodeUcpeWithData;
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_block::FieldBlock;
use crate::elements::field_info::FieldInfo;
use crate::elements::font::FontInfo;
use crate::elements::graphic_symbol::GraphicSymbol;
use crate::elements::label_element::LabelElement;
use crate::elements::maxicode::MaxicodeWithData;
use crate::elements::stored_format::{RecalledField, RecalledFormat};
use crate::elements::text_field::TextField;
use crate::elements::text_flow::{TextDirection, TextFlow};

pub use super::qr_payload::parse_qr_fd_payload;
use super::{charsets, variable_fields};
use std::collections::HashMap;

pub fn resolve_elements(rf: &RecalledFormat) -> Result<Vec<LabelElement>, String> {
    resolve_copy(&rf.elements, 0, &Default::default())
}

pub(super) fn resolve_copy(
    elements: &[LabelElement],
    index: usize,
    clock: &super::field_clock::FieldClock,
) -> Result<Vec<LabelElement>, String> {
    let references = std::cell::OnceCell::new();
    let mut result = Vec::with_capacity(elements.len());
    for el in elements {
        if let LabelElement::RecalledField(f) = el {
            let mut field = f.clone();
            if let Some(delimiter) = field.stored.field.reference_delimiter {
                let (text_values, byte_values) = references.get_or_init(|| {
                    (
                        numbered_values(elements, index, false),
                        numbered_values(elements, index, true),
                    )
                });
                let source = f.data_bytes.as_deref().unwrap_or(f.data.as_bytes());
                field.data_bytes =
                    Some(variable_fields::interpolate(source, delimiter, byte_values));
                field.data = String::from_utf8(variable_fields::interpolate(
                    f.data.as_bytes(),
                    delimiter,
                    text_values,
                ))
                .expect("text field references preserve UTF-8");
            }
            let markers = f.stored.field.clock_indicators;
            if markers[0].is_some() {
                transform_payload(&mut field, clock.expander(markers));
            }
            if let Some(sequence) = &f.stored.field.sequence {
                transform_payload(&mut field, |data| {
                    variable_fields::serialize(data, sequence, index)
                });
            }
            if let Some(drawable) = resolve_field(&field)? {
                result.push(drawable);
            }
        } else {
            result.push(el.clone());
        }
    }
    Ok(result)
}

// Field text may be a decoded display string while the barcode consumes the
// original printer bytes. Apply the same substitutions to each representation.
fn transform_payload(field: &mut RecalledField, transform: impl Fn(&[u8]) -> Vec<u8>) {
    if let Some(bytes) = &mut field.data_bytes {
        *bytes = transform(bytes);
    }
    field.data = String::from_utf8(transform(field.data.as_bytes()))
        .expect("text field substitutions preserve UTF-8");
}

fn numbered_values(elements: &[LabelElement], index: usize, raw: bool) -> HashMap<i32, Vec<u8>> {
    let mut values = HashMap::new();
    for el in elements {
        let (number, text, bytes, sequence) = match el {
            LabelElement::RecalledField(f) if f.stored.number >= 0 => (
                f.stored.number,
                &f.data,
                &f.data_bytes,
                &f.stored.field.sequence,
            ),
            LabelElement::RecalledFieldData(f) => (f.number, &f.data, &f.data_bytes, &f.sequence),
            _ => continue,
        };
        let data = if raw {
            bytes.as_deref().unwrap_or(text.as_bytes())
        } else {
            text.as_bytes()
        };
        let value = sequence.as_ref().map_or_else(
            || data.to_vec(),
            |rule| variable_fields::serialize(data, rule, index),
        );
        values.insert(number, value);
    }
    values
}

/// v1.7.x method form of [`resolve_elements`]. The inherent impl lives in
/// the parser module (methods may be defined anywhere in the defining
/// crate) so `elements` stays free of parser references; the body is a
/// thin delegate — extend the free function, not this.
impl RecalledFormat {
    pub fn resolve_elements(&self) -> Result<Vec<LabelElement>, String> {
        resolve_elements(self)
    }
}

pub(super) fn resolve_field(f: &RecalledField) -> Result<Option<LabelElement>, String> {
    let field = &f.stored.field;
    let original;
    let text = if let (Some(_), Some(bytes)) = (&field.character_map, &f.data_bytes) {
        original = bytes.iter().map(|&b| char::from(b)).collect::<String>();
        &original
    } else {
        &f.data
    };

    if field.element.is_none() && text.is_empty() {
        return Ok(None);
    }

    match field.element.as_deref() {
        Some(LabelElement::MaxicodeConfig(mc)) => {
            Ok(Some(LabelElement::Maxicode(MaxicodeWithData {
                reverse_print: field.reverse_print.clone(),
                code: mc.clone(),
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodePdf417Config(bc)) => {
            Ok(Some(LabelElement::BarcodePdf417(BarcodePdf417WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::Barcode128Config(bc)) => {
            Ok(Some(LabelElement::Barcode128(Barcode128WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                position: field.position.clone(),
                line_alignment: FieldAlignment::Center,
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeEan13Config(bc)) => {
            Ok(Some(LabelElement::BarcodeEan13(BarcodeEan13WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeEan8Config(bc)) => {
            Ok(Some(LabelElement::BarcodeEan8(BarcodeEan8WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeUcaConfig(bc)) => {
            Ok(Some(LabelElement::BarcodeUca(BarcodeUcaWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::Barcode2of5Config(bc)) => {
            Ok(Some(LabelElement::Barcode2of5(Barcode2of5WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                width_ratio: field.width_ratio,
                position: field.position.clone(),
                line_alignment: FieldAlignment::Center,
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeSymbolConfig(symbol)) => Ok(Some(LabelElement::BarcodeSymbol(
            crate::elements::barcode_symbol::BarcodeSymbolWithData {
                symbol: symbol.clone(),
                data: super::barcode_commands::field_bytes(
                    &symbol.kind,
                    f.data_bytes.as_deref().unwrap_or(text.as_bytes()),
                )?,
                position: field.position.clone(),
                reverse_print: field.reverse_print.clone(),
            },
        ))),
        Some(LabelElement::Barcode39Config(bc)) => {
            Ok(Some(LabelElement::Barcode39(Barcode39WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                width_ratio: field.width_ratio,
                position: field.position.clone(),
                line_alignment: FieldAlignment::Center,
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeAztecConfig(bc)) => {
            Ok(Some(LabelElement::BarcodeAztec(BarcodeAztecWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::BarcodeDatamatrixConfig(bc)) => Ok(Some(
            LabelElement::BarcodeDatamatrix(BarcodeDatamatrixWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                position: field.position.clone(),
                data: text.clone(),
                data_bytes: f.data_bytes.clone(),
            }),
        )),
        Some(LabelElement::BarcodeQrConfig(bc)) => {
            let (data, error_correction, _) = parse_qr_fd_payload(text)?;
            Ok(Some(LabelElement::BarcodeQr(BarcodeQrWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                height: field.height,
                position: field.position.clone(),
                data,
                error_correction,
            })))
        }
        Some(LabelElement::BarcodeUcpeConfig(bc)) => {
            Ok(Some(LabelElement::BarcodeUcpe(BarcodeUcpeWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                position: field.position.clone(),
                data: text.clone(),
            })))
        }
        Some(LabelElement::GraphicSymbolConfig(gs)) => {
            to_graphic_symbol_text_field(text, field, gs)
        }
        Some(LabelElement::FieldBlockConfig(fb)) => to_text_field(text, field, Some(fb)),
        _ => to_text_field(text, field, None),
    }
}

fn to_graphic_symbol_text_field(
    text: &str,
    field: &FieldInfo,
    gs: &GraphicSymbol,
) -> Result<Option<LabelElement>, String> {
    let gs_text = to_gs_text(text);
    if gs_text.is_empty() {
        return Ok(None);
    }

    let (face, advance_mult, dedot_zero) = super::zebra_fonts::resolve_designator("GS");
    let source_font = FontInfo {
        name: "GS".to_string(),
        resources: None,
        face,
        advance_mult,
        dedot_zero,
        width: gs.width,
        height: gs.height,
        orientation: gs.orientation,
    };
    let font = super::zebra_fonts::with_adjusted_sizes(&source_font);
    let flow = resolve_text_flow(field.text_flow, &source_font, &font);

    Ok(Some(LabelElement::Text(TextField {
        font,
        position: field.position.clone(),
        alignment: field.alignment,
        text: gs_text,
        flow,
        reverse_print: field.reverse_print.clone(),
        block: None,
    })))
}

fn to_gs_text(text: &str) -> String {
    let mut res = String::new();
    for c in text.chars() {
        if c == ' ' {
            res.push(c);
            continue;
        }
        if ('A'..='E').contains(&c) {
            res.push(c);
        }
        break;
    }
    res
}

fn to_text_field(
    text: &str,
    field: &FieldInfo,
    fb: Option<&FieldBlock>,
) -> Result<Option<LabelElement>, String> {
    let text = text.replace("\\&", "\n");
    let text = if let Some(map) = &field.character_map {
        text.chars()
            .map(|c| u8::try_from(c).map_or(c, |b| char::from(map[b as usize])))
            .collect()
    } else {
        text
    };
    let unicode_text = charsets::to_unicode_text(&text, field.current_charset)
        .map_err(|e| format!("failed to convert to unicode text: {}", e))?;
    let font = super::zebra_fonts::with_adjusted_sizes(&field.font);
    let mut block = fb.cloned();
    let mut unicode_text = unicode_text;
    if let Some(block) = block.as_mut().filter(|b| b.max_height.is_some()) {
        let height = block.max_height.unwrap_or(1);
        let cell = if field.font.height > 0.0 {
            field.font.height
        } else {
            font.height
        }
        .max(1.0);
        // Bounded text blocks have a quarter-cell gap. The last line may
        // be partly visible; the physical rectangle clips its ink.
        block.line_spacing = (cell * 0.25).round() as i32;
        block.max_lines = (f64::from(height) / (cell + f64::from(block.line_spacing)))
            .ceil()
            .max(1.0) as i32;
        block.alignment = match field.alignment {
            FieldAlignment::Right => crate::elements::text_alignment::TextAlignment::Right,
            FieldAlignment::Center => crate::elements::text_alignment::TextAlignment::Center,
            _ => crate::elements::text_alignment::TextAlignment::Left,
        };
        unicode_text = unicode_text.replace('\u{00ad}', "").replace("<< >", "<");
    }
    let flow = resolve_text_flow(field.text_flow, &field.font, &font);

    Ok(Some(LabelElement::Text(TextField {
        font,
        position: field.position.clone(),
        alignment: field.alignment,
        text: unicode_text,
        flow,
        block,
        reverse_print: field.reverse_print.clone(),
    })))
}

fn resolve_text_flow(mut flow: TextFlow, source_font: &FontInfo, font: &FontInfo) -> TextFlow {
    // Labelary drops excessive spacing instead of clamping it. Resolve this
    // after the final font selection, so command order and stored fields use
    // the same limit. An omitted width uses the font's derived cell width.
    let width = if source_font.width > 0.0 {
        source_font.width
    } else {
        font.width
    };
    let limit_height = super::zebra_fonts::spacing_limit_height(source_font, font);
    if flow.character_spacing as f64 > 10.0 * limit_height {
        flow.character_spacing = 0;
    }
    flow.wrap_spacing = flow.character_spacing as f64 * 10.0 / width.max(1.0);
    flow.origin_width = if matches!(
        flow.direction,
        TextDirection::Reverse | TextDirection::Vertical
    ) {
        if source_font.name == "0" {
            0.75 * font.width
        } else {
            font.width
        }
    } else {
        0.0
    };
    flow
}
