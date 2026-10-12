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
use crate::elements::field_block::FieldBlock;
use crate::elements::field_info::FieldInfo;
use crate::elements::font::FontInfo;
use crate::elements::graphic_symbol::GraphicSymbol;
use crate::elements::label_element::LabelElement;
use crate::elements::maxicode::MaxicodeWithData;
use crate::elements::stored_format::{RecalledField, RecalledFormat};
use crate::elements::text_field::TextField;

use super::charsets;
pub use super::qr_payload::parse_qr_fd_payload;

pub fn resolve_elements(rf: &RecalledFormat) -> Result<Vec<LabelElement>, String> {
    let mut res = Vec::with_capacity(rf.elements.len());
    for element in &rf.elements {
        match element {
            LabelElement::RecalledField(rf) => {
                if let Some(resolved) = resolve_field(rf)? {
                    res.push(resolved);
                }
            }
            other => {
                res.push(other.clone());
            }
        }
    }
    Ok(res)
}

fn resolve_field(f: &RecalledField) -> Result<Option<LabelElement>, String> {
    let field = &f.stored.field;
    let text = &f.data;

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
                data: text.clone(),
            })))
        }
        Some(LabelElement::Barcode39Config(bc)) => {
            Ok(Some(LabelElement::Barcode39(Barcode39WithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                width: field.width,
                width_ratio: field.width_ratio,
                position: field.position.clone(),
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
    let font = super::zebra_fonts::with_adjusted_sizes(&FontInfo {
        name: "GS".to_string(),
        face,
        advance_mult,
        dedot_zero,
        width: gs.width,
        height: gs.height,
        orientation: gs.orientation,
    });

    Ok(Some(LabelElement::Text(TextField {
        font,
        position: field.position.clone(),
        alignment: field.alignment,
        text: gs_text,
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
    let unicode_text = charsets::to_unicode_text(&text, field.current_charset)
        .map_err(|e| format!("failed to convert to unicode text: {}", e))?;

    Ok(Some(LabelElement::Text(TextField {
        font: super::zebra_fonts::with_adjusted_sizes(&field.font),
        position: field.position.clone(),
        alignment: field.alignment,
        text: unicode_text,
        block: fb.cloned(),
        reverse_print: field.reverse_print.clone(),
    })))
}
