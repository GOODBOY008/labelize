use std::collections::HashMap;

use super::barcode_128::Barcode128WithData;
use super::barcode_2of5::Barcode2of5WithData;
use super::barcode_39::Barcode39WithData;
use super::barcode_aztec::BarcodeAztecWithData;
use super::barcode_datamatrix::BarcodeDatamatrixWithData;
use super::barcode_ean13::BarcodeEan13WithData;
use super::barcode_pdf417::BarcodePdf417WithData;
use super::barcode_qr::BarcodeQrWithData;
use super::field_block::FieldBlock;
use super::field_info::FieldInfo;
use super::font::FontInfo;
use super::graphic_symbol::GraphicSymbol;
use super::label_element::LabelElement;
use super::maxicode::MaxicodeWithData;
use super::reverse_print::ReversePrint;
use super::text_field::TextField;
use crate::encodings;

#[derive(Clone, Debug)]
pub struct StoredFormat {
    pub inverted: bool,
    pub elements: Vec<LabelElement>,
}

impl StoredFormat {
    pub fn to_recalled_format(&self) -> RecalledFormat {
        let mut rf = RecalledFormat {
            inverted: self.inverted,
            elements: Vec::new(),
            field_refs: HashMap::new(),
        };
        for el in &self.elements {
            rf.add_element(el.clone());
        }
        rf
    }
}

#[derive(Clone, Debug)]
pub struct StoredField {
    pub number: i32,
    pub field: FieldInfo,
}

#[derive(Clone, Debug)]
pub struct RecalledFieldData {
    pub number: i32,
    pub data: String,
}

#[derive(Clone, Debug)]
pub struct RecalledField {
    pub stored: StoredField,
    pub data: String,
}

#[derive(Clone, Debug)]
pub struct RecalledFormat {
    pub inverted: bool,
    pub elements: Vec<LabelElement>,
    pub field_refs: HashMap<i32, Vec<usize>>, // indices into elements
}

impl RecalledFormat {
    pub fn add_element(&mut self, element: LabelElement) -> bool {
        match element {
            LabelElement::StoredField(sf) => {
                let idx = self.elements.len();
                self.elements
                    .push(LabelElement::RecalledField(RecalledField {
                        stored: sf.clone(),
                        data: String::new(),
                    }));
                self.field_refs.entry(sf.number).or_default().push(idx);
                true
            }
            LabelElement::RecalledFieldData(rfd) => {
                if let Some(indices) = self.field_refs.remove(&rfd.number) {
                    for idx in indices {
                        if let LabelElement::RecalledField(ref mut rf) = self.elements[idx] {
                            rf.data = rfd.data.clone();
                        }
                    }
                } else {
                    self.elements
                        .push(LabelElement::RecalledField(RecalledField {
                            stored: StoredField {
                                number: rfd.number,
                                field: FieldInfo {
                                    reverse_print: ReversePrint::default(),
                                    element: None,
                                    font: FontInfo::default(),
                                    position: Default::default(),
                                    alignment: Default::default(),
                                    width: 0,
                                    width_ratio: 0.0,
                                    height: 0,
                                    current_charset: 0,
                                },
                            },
                            data: rfd.data,
                        }));
                }
                true
            }
            other => {
                self.elements.push(other);
                true
            }
        }
    }

    pub fn resolve_elements(&self) -> Result<Vec<LabelElement>, String> {
        let mut res = Vec::with_capacity(self.elements.len());
        for element in &self.elements {
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
            }),
        )),
        Some(LabelElement::BarcodeQrConfig(bc)) => {
            Ok(Some(LabelElement::BarcodeQr(BarcodeQrWithData {
                reverse_print: field.reverse_print.clone(),
                barcode: bc.clone(),
                height: field.height,
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

    let font = FontInfo {
        name: "GS".to_string(),
        width: gs.width,
        height: gs.height,
        orientation: gs.orientation,
    }
    .with_adjusted_sizes();

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
    let unicode_text = encodings::to_unicode_text(&text, field.current_charset)
        .map_err(|e| format!("failed to convert to unicode text: {}", e))?;

    Ok(Some(LabelElement::Text(TextField {
        font: field.font.with_adjusted_sizes(),
        position: field.position.clone(),
        alignment: field.alignment,
        text: unicode_text,
        block: fb.cloned(),
        reverse_print: field.reverse_print.clone(),
    })))
}
