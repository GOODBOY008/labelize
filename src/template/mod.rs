//! Fill a template exported by Labelize Studio with one record of string values.
//! The same filled bytes can be sent to a printer or passed to [`crate::render_png`].
use crate::{LabelizeError, Language};
use std::{collections::BTreeMap, fmt, ops::Range};

mod barcode;
mod expression;
mod native;
#[cfg(feature = "template-json")]
mod request;
mod zpl;
use barcode::*;
use expression::Expression;
#[cfg(feature = "template-json")]
pub use request::{
    fill_json_request, TemplateRequestError, MAX_TEMPLATE_OUTPUT_BYTES, MAX_TEMPLATE_REQUEST_BYTES,
};

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

enum FillError {
    Fields(Vec<TemplateProblem>),
    TooLarge,
}

enum FillMode {
    CollectProblems,
    #[cfg(feature = "template-json")]
    FailFast {
        max_bytes: usize,
    },
}

impl fmt::Display for FillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => f.write_str("Filled printer job exceeds the output limit"),
            Self::Fields(problems) => {
                for (i, problem) in problems.iter().enumerate() {
                    if i != 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{problem}")?;
                }
                Ok(())
            }
        }
    }
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
        self.slots
            .iter()
            .filter_map(|slot| slot.encode(values).err())
            .collect()
    }

    /// Return a new native printer job for one record. All fields must validate.
    pub fn fill(&self, values: &TemplateValues) -> Result<Vec<u8>, LabelizeError> {
        self.fill_job(values, FillMode::CollectProblems)
            .map_err(|err| LabelizeError::Encode(err.to_string()))
    }

    fn fill_job(&self, values: &TemplateValues, mode: FillMode) -> Result<Vec<u8>, FillError> {
        let limit = match mode {
            FillMode::CollectProblems => usize::MAX,
            #[cfg(feature = "template-json")]
            FillMode::FailFast { max_bytes } => max_bytes,
        };
        let mut job = Vec::with_capacity(self.body.len().min(limit));
        let mut append = |chunk: &[u8]| {
            if chunk.len() > limit - job.len() {
                return Err(FillError::TooLarge);
            }
            let end = job.len() + chunk.len();
            if end > job.capacity() {
                // Geometric growth for many fields, bounded even on the final allocation.
                let next = job.capacity().saturating_mul(2).max(end).min(limit);
                job.reserve_exact(next - job.len());
            }
            job.extend_from_slice(chunk);
            Ok(())
        };
        let mut problems = Vec::new();
        let mut cursor = 0;
        for slot in &self.slots {
            match slot.encode(values) {
                Ok(data) if problems.is_empty() => {
                    append(&self.body[cursor..slot.range.start])?;
                    append(&data)?;
                }
                Ok(_) => {}
                Err(problem) => match mode {
                    FillMode::CollectProblems => problems.push(problem),
                    #[cfg(feature = "template-json")]
                    FillMode::FailFast { .. } => return Err(FillError::Fields(vec![problem])),
                },
            }
            cursor = slot.range.end;
        }
        if !problems.is_empty() {
            return Err(FillError::Fields(problems));
        }
        append(&self.body[cursor..])?;
        Ok(job)
    }
}
