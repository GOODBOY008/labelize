use super::{FillError, FillMode, PrintTemplate, TemplateValues};
use crate::Language;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::borrow::Cow;

/// Maximum JSON body accepted by the template fill HTTP endpoint.
pub const MAX_TEMPLATE_REQUEST_BYTES: usize = 2 * 1024 * 1024;
/// Maximum native job produced by one request, matching the native `/convert` body limit.
pub const MAX_TEMPLATE_OUTPUT_BYTES: usize = MAX_TEMPLATE_REQUEST_BYTES;

/// Errors shared by the native HTTP and WebAssembly template adapters.
#[derive(Debug, thiserror::Error)]
pub enum TemplateRequestError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    TooLarge(&'static str),
}

/// A filled native printer job with its download metadata.
pub struct FilledTemplateJob {
    /// Original printer data, including any binary artwork.
    pub bytes: Vec<u8>,
    language: Language,
}

impl FilledTemplateJob {
    /// HTTP attachment header with a filename for the validated printer language.
    pub fn content_disposition(&self) -> &'static str {
        match self.language {
            Language::Zpl => r#"attachment; filename="job.zpl""#,
            Language::Epl => r#"attachment; filename="job.epl""#,
            Language::Tspl => r#"attachment; filename="job.tspl""#,
            Language::Cpcl => r#"attachment; filename="job.cpcl""#,
            Language::Dpl => r#"attachment; filename="job.dpl""#,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request<'a> {
    language: String,
    #[serde(borrow)]
    template_base64: Cow<'a, str>,
    values: TemplateValues,
}

/// Fill a JSON request containing `language`, `template_base64`, and a string-valued
/// `values` object. Both HTTP runtimes return these bytes without text conversion.
pub fn fill_json_request(body: &[u8]) -> Result<Vec<u8>, TemplateRequestError> {
    fill_json_job(body).map(|job| job.bytes)
}

/// Fill the same JSON request and retain the printer language for HTTP downloads.
pub fn fill_json_job(body: &[u8]) -> Result<FilledTemplateJob, TemplateRequestError> {
    use TemplateRequestError::{Invalid, TooLarge};
    if body.len() > MAX_TEMPLATE_REQUEST_BYTES {
        return Err(TooLarge("Template fill request exceeds 2 MiB"));
    }
    // Serde structs also accept positional arrays. The wire format requires named keys.
    if !body.trim_ascii_start().starts_with(b"{") {
        return Err(Invalid("Template fill requires a JSON object".into()));
    }
    let request: Request<'_> = serde_json::from_slice(body)
        .map_err(|e| Invalid(format!("Invalid template fill JSON: {e}")))?;
    let language = match request.language.to_ascii_lowercase().as_str() {
        "tsc" => Language::Tspl,
        "epl2" => Language::Epl,
        name => Language::from_name(name)
            .ok_or_else(|| Invalid(format!("Unknown template language {}", request.language)))?,
    };
    let bytes = STANDARD
        .decode(request.template_base64.as_bytes())
        .map_err(|e| Invalid(format!("Invalid template_base64: {e}")))?;
    let template = PrintTemplate::load(language, &bytes).map_err(|e| Invalid(e.to_string()))?;
    template
        .fill_job(
            &request.values,
            FillMode::FailFast {
                max_bytes: MAX_TEMPLATE_OUTPUT_BYTES,
            },
        )
        .map(|bytes| FilledTemplateJob { bytes, language })
        .map_err(|err| match err {
            FillError::TooLarge => TooLarge("Filled printer job exceeds 2 MiB"),
            FillError::Fields(_) => Invalid(err.to_string()),
        })
}
