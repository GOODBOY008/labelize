//! Fill a template exported by Labelize Studio with one record of string values.
//! The same filled bytes can be sent to a printer or passed to [`crate::render_png`].
use crate::{LabelizeError, Language};
use std::{collections::BTreeMap, fmt, ops::Range};

mod barcode;
mod expression;
mod native;
mod zpl;
use barcode::*;
use expression::Expression;

/// One input record. Identifiers such as order numbers retain their leading zeros.
pub type TemplateValues = BTreeMap<String, String>;

/// A native field using a variable, including human-readable barcode captions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateUse {
    /// One-based Studio design object number, or marker order for ZPL.
    pub object: usize,
    /// Studio field kind, for example `text`, `code128`, or `ean13-caption`.
    pub kind: String,
}

/// A named variable and all the native fields that use its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateField {
    pub name: String,
    pub uses: Vec<TemplateUse>,
}

/// A missing or invalid value for one occurrence in the printer template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateProblem {
    pub usage: TemplateUse,
    pub fields: Vec<String>,
    pub message: String,
}

impl fmt::Display for TemplateProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (object {}, {}): {}",
            self.fields.join(", "),
            self.usage.object,
            self.usage.kind,
            self.message
        )
    }
}

#[derive(Debug)]
enum Encoder {
    Zpl(zpl::Field),
    Native(native::Field),
}

#[derive(Debug)]
struct Slot {
    range: Range<usize>,
    usage: TemplateUse,
    expr: Expression,
    encoder: Encoder,
}

impl Slot {
    fn encode(&self, values: &TemplateValues) -> Result<Vec<u8>, TemplateProblem> {
        let missing: Vec<_> = self
            .expr
            .names
            .iter()
            .filter(|name| !values.contains_key(*name))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(TemplateProblem {
                usage: self.usage.clone(),
                fields: missing,
                message: "Missing required value".into(),
            });
        }
        self.expr
            .expand(values)
            .and_then(|value| match &self.encoder {
                Encoder::Zpl(field) => field.encode(&self.usage.kind, &value),
                Encoder::Native(field) => field.encode(&self.usage.kind, &value),
            })
            .map_err(|message| TemplateProblem {
                usage: self.usage.clone(),
                fields: self.expr.names.clone(),
                message,
            })
    }
}

/// An immutable, reusable printer template. Parsing and field discovery happen once.
#[derive(Debug)]
pub struct PrintTemplate {
    body: Vec<u8>,
    slots: Vec<Slot>,
    fields: Vec<TemplateField>,
}

impl PrintTemplate {
    /// Load ZPL field markers or a Studio `LABELIZE-TPL/1` directory.
    pub fn load(language: Language, bytes: &[u8]) -> Result<Self, LabelizeError> {
        let (body, slots) = match language {
            Language::Zpl => zpl::load(bytes),
            _ => native::load(language, bytes),
        }
        .map_err(LabelizeError::Parse)?;
        if slots.is_empty() {
            return Err(LabelizeError::Parse(
                "No variable fields in this template".into(),
            ));
        }
        let mut fields = Vec::new();
        let mut indices = BTreeMap::new();
        for slot in &slots {
            for name in &slot.expr.names {
                let index = *indices.entry(name.as_str()).or_insert_with(|| {
                    fields.push(TemplateField {
                        name: name.clone(),
                        uses: Vec::new(),
                    });
                    fields.len() - 1
                });
                fields[index].uses.push(slot.usage.clone());
            }
        }
        Ok(Self {
            body,
            slots,
            fields,
        })
    }

    /// Variables in first-use order. Repetitions in a single expression count as one use.
    pub fn fields(&self) -> &[TemplateField] {
        &self.fields
    }

    /// Report all invalid occurrences in printer order. Extra record keys are ignored.
    pub fn validate(&self, values: &TemplateValues) -> Vec<TemplateProblem> {
        self.process(values, None)
    }

    /// Return a new native printer job for one record. All fields must validate.
    pub fn fill(&self, values: &TemplateValues) -> Result<Vec<u8>, LabelizeError> {
        let mut job = Vec::with_capacity(self.body.len());
        let problems = self.process(values, Some(&mut job));
        if problems.is_empty() {
            Ok(job)
        } else {
            Err(LabelizeError::Encode(
                problems
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ))
        }
    }

    fn process(
        &self,
        values: &TemplateValues,
        mut output: Option<&mut Vec<u8>>,
    ) -> Vec<TemplateProblem> {
        let mut problems = Vec::new();
        let mut cursor = 0;
        for slot in &self.slots {
            match slot.encode(values) {
                Ok(data) => {
                    if let Some(job) = &mut output {
                        job.extend_from_slice(&self.body[cursor..slot.range.start]);
                        job.extend(data);
                    }
                }
                Err(problem) => {
                    problems.push(problem);
                    output = None;
                }
            }
            cursor = slot.range.end;
        }
        if let Some(job) = output {
            job.extend_from_slice(&self.body[cursor..]);
        }
        problems
    }
}
