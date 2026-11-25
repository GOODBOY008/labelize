//! TSPL/TSC frontend. Maps TSC command syntax onto the shared IR; TSPL font
//! cells and SIZE dimensions are resolved here (never via the Zebra tables).

mod aztec;
pub mod fonts;
pub mod format;
pub mod parser;
mod quoted;

pub use parser::TsplParser;

/// Command words that unambiguously identify TSPL. Deliberately narrow:
/// these are full-line command names no other supported language uses.
pub(crate) const DETECTION_WORDS: &[&str] = &[
    "SIZE",
    "DIRECTION",
    "REFERENCE",
    "SHIFT",
    "CLS",
    "PRINT",
    "TEXT",
    "BARCODE",
    "QRCODE",
    "AZTEC",
    "PDF417",
    "BOX",
    "BAR",
    "CIRCLE",
    "ELLIPSE",
    "ERASE",
    "REVERSE",
    "BITMAP",
    "GAP",
    "DENSITY",
    "SPEED",
    "OFFSET",
    "CODEPAGE",
];

/// These plain commands parse comma-separated arguments and can carry a
/// quoted value across physical lines. Device commands and comments are opaque.
fn parses_arguments(word: &[u8]) -> bool {
    [
        "SIZE",
        "DIRECTION",
        "REFERENCE",
        "SHIFT",
        "TEXT",
        "BAR",
        "BOX",
        "CIRCLE",
        "ELLIPSE",
        "ERASE",
        "REVERSE",
        "BARCODE",
        "QRCODE",
        "PDF417",
        "DMATRIX",
    ]
    .iter()
    .any(|name| word.eq_ignore_ascii_case(name.as_bytes()))
}
