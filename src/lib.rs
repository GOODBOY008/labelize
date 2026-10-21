/// Crate version, exposed for the wasm/Android bindings' diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod assets;
pub mod barcodes;
pub mod drawers;
pub mod elements;
// Compatibility alias for the pre-reorganization layout; the ^CI tables
// live at parsers::zpl::charsets now (ZPL-only).
pub mod encodings;
pub mod error;
pub mod hex;
pub mod images;
pub mod lang;
pub mod parsers;
#[cfg(feature = "playground")]
pub mod playground;
pub(crate) mod tuning;

/// Font-0 characters Labelary blanks even though the substitute subset has
/// outlines for them (see `tuning`). Exposed for the display-sweep ratchets.
pub use tuning::FONT0_LABELARY_BLANKED;

#[cfg(feature = "skill")]
pub mod skill;

pub use drawers::renderer::Renderer;
pub use elements::drawer_options::DrawerOptions;
pub use elements::label_info::LabelInfo;
pub use error::LabelizeError;
pub use images::monochrome::encode_png;
pub use images::pdf::encode_pdf;
pub use lang::{
    detect_language, format_source, parse_labels, validate, Language, ValidationReport,
};
pub use parsers::{EplParser, TsplParser, ZplParser};
