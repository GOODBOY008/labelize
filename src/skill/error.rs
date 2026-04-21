/// Error types for the ZPL diff auto-fix skill.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("diff report not found at {path}")]
    ReportNotFound { path: String },

    #[error("failed to parse diff report: {reason}")]
    ParseError { reason: String },

    #[error(
        "label '{name}' not found. Available labels: {available:?}. Did you mean '{suggestion}'?"
    )]
    LabelNotFound {
        name: String,
        available: Vec<String>,
        suggestion: String,
    },
}

#[derive(Error, Debug)]
pub enum AnalyzeError {
    #[error("failed to load diff image: {path}")]
    DiffImageNotFound { path: String },

    #[error("failed to parse ZPL: {reason}")]
    ZplParseError { reason: String },

    #[error("no drawable elements found in label")]
    NoElements,

    #[error("render failed: {0}")]
    RenderFailed(String),
}

#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("failed to parse ZPL: {reason}")]
    ZplParseError { reason: String },

    #[error("element index {index} out of range (label has {total} elements)")]
    ElementOutOfRange { index: usize, total: usize },

    #[error("failed to write snippet file: {reason}")]
    WriteError { reason: String },
}

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("scan error: {0}")]
    Scan(#[from] ScanError),

    #[error("analyze error: {0}")]
    Analyze(#[from] AnalyzeError),

    #[error("extract error: {0}")]
    Extract(#[from] ExtractError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
