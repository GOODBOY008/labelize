//! Stored-format IR types (^DF/^XF machinery).
//!
//! Only the data structures live here so `LabelElement` can reference them;
//! the ZPL-specific resolution (merging ^FN data, ^CI charset conversion,
//! Zebra font-size adjustment) is `crate::parsers::zpl::stored_formats`.

use std::collections::HashMap;

use super::field_info::FieldInfo;
use super::font::FontInfo;
use super::label_element::LabelElement;
use super::reverse_print::ReversePrint;

#[derive(Clone, Debug)]
pub struct StoredFormat {
    pub inverted: bool,
    pub mirrored: Option<bool>,
    pub elements: Vec<LabelElement>,
}

impl StoredFormat {
    pub fn to_recalled_format(&self) -> RecalledFormat {
        let mut rf = RecalledFormat {
            inverted: self.inverted,
            mirrored: self.mirrored,
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
    pub sequence: Option<super::field_info::FieldSequence>,
    pub reference_delimiter: Option<char>,
    pub clock_indicators: [Option<char>; 3],
    pub number: i32,
    pub data: String,
    pub data_bytes: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct RecalledField {
    pub stored: StoredField,
    pub data: String,
    pub data_bytes: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct RecalledFormat {
    pub inverted: bool,
    pub mirrored: Option<bool>,
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
                        data_bytes: None,
                    }));
                self.field_refs.entry(sf.number).or_default().push(idx);
                true
            }
            LabelElement::RecalledField(field) => {
                if field.stored.number >= 0 {
                    self.field_refs
                        .entry(field.stored.number)
                        .or_default()
                        .push(self.elements.len());
                }
                self.elements.push(LabelElement::RecalledField(field));
                true
            }
            LabelElement::RecalledFieldData(rfd) => {
                if let Some(indices) = self.field_refs.remove(&rfd.number) {
                    for idx in indices {
                        if let LabelElement::RecalledField(ref mut rf) = self.elements[idx] {
                            if rfd.sequence.is_some() {
                                rf.stored.field.sequence = rfd.sequence.clone();
                            }
                            if rfd.reference_delimiter.is_some() {
                                rf.stored.field.reference_delimiter = rfd.reference_delimiter;
                            }
                            if rfd.clock_indicators[0].is_some() {
                                rf.stored.field.clock_indicators = rfd.clock_indicators;
                            }
                            rf.data = rfd.data.clone();
                            rf.data_bytes = rfd.data_bytes.clone();
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
                                    character_map: None,
                                    sequence: rfd.sequence.clone(),
                                    reference_delimiter: rfd.reference_delimiter,
                                    variable: true,
                                    clock_indicators: rfd.clock_indicators,
                                    text_flow: Default::default(),
                                },
                            },
                            data: rfd.data,
                            data_bytes: rfd.data_bytes,
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
}
