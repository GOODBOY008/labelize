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

/// Include commands that can also occur in other printer languages or
/// have no rendered effect. These delimit a TSPL job once its language is known.
fn is_command_word(word: &[u8]) -> bool {
    const OTHER_WORDS: &[&str] = &[
        "DMATRIX",
        "MPDF417",
        "PUTPCX",
        "PUTBMP",
        "BLINE",
        "GAPDETECT",
        "BLINEDETECT",
        "AUTODETECT",
        "FEED",
        "BACKFEED",
        "FORMFEED",
        "HOME",
        "SELFTEST",
        "INITIALPRINTER",
        "DOWNLOAD",
        "EOP",
        "FILES",
        "KILL",
        "RUN",
        "OUT",
        "SET",
    ];
    DETECTION_WORDS
        .iter()
        .chain(OTHER_WORDS)
        .any(|name| word.eq_ignore_ascii_case(name.as_bytes()))
}
