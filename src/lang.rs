//! The single front door for label data.
//!
//! Every entry point (CLI, HTTP service, wasm bindings) goes through
//! [`Language`] / [`parse_labels`] instead of picking parser types itself, so
//! adding a language means adding one enum variant and one match arm here —
//! the parsers, elements, and renderer stay untouched.

use crate::elements::label_info::LabelInfo;
use crate::error::LabelizeError;
use crate::parsers::epl::parser::EplParser;
use crate::parsers::zpl::parser::ZplParser;

/// A label printer language the engine understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    /// Zebra Programming Language (`^XA … ^XZ`).
    Zpl,
    /// Eltron Programming Language (line commands `A`, `B`, `P`, …).
    Epl,
}

impl Language {
    /// Parse `bytes` in this language into one [`LabelInfo`] per label.
    ///
    /// `dpmm` feeds `^MU` unit conversion in ZPL; EPL is dot-native and
    /// ignores it.
    pub fn parse(self, bytes: &[u8], dpmm: i32) -> Result<Vec<LabelInfo>, LabelizeError> {
        match self {
            Language::Zpl => ZplParser::with_dpmm(dpmm)
                .parse(bytes)
                .map_err(LabelizeError::Parse),
            Language::Epl => EplParser::new().parse(bytes).map_err(LabelizeError::Parse),
        }
    }
}

/// Parse `bytes` in the given [`Language`] into one [`LabelInfo`] per label.
pub fn parse_labels(
    language: Language,
    bytes: &[u8],
    dpmm: i32,
) -> Result<Vec<LabelInfo>, LabelizeError> {
    language.parse(bytes, dpmm)
}

/// Guess the language of `bytes` from its shape.
///
/// ZPL formats are caret/tilde command streams; EPL is one command letter
/// per line with comma-separated parameters. When the input matches neither
/// (or is empty) the function falls back to ZPL, matching the historical
/// CLI behavior.
///
/// Multi-letter uppercase command words (`SIZE`, `TEXT`, `PRINT`, …) are the
/// TSPL signature — reserved for a future `Language::Tspl` variant.
pub fn detect_language(bytes: &[u8]) -> Language {
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut chars = line.chars();
        let first = chars.next().unwrap_or(' ');
        if first == '^' || first == '~' {
            return Language::Zpl;
        }
        if looks_like_epl_line(line) {
            return Language::Epl;
        }
    }
    Language::Zpl
}

/// EPL lines are a command letter plus comma-separated parameters: `q812`,
/// `A50,50,0,1,1,1,N,"Hi"`, a bare `N`, or one of the two-letter graphic
/// commands (`GW`, `LO`, `LW`, `LS`). Longer uppercase runs (`TEXT x,y`,
/// `SIZE 4,6`) are the TSPL signature, not EPL.
fn looks_like_epl_line(line: &str) -> bool {
    let command: String = line
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let rest = &line[command.len()..];
    match command.len() {
        0 => false,
        1 => rest.is_empty() || starts_parameter(rest),
        2 => matches!(command.as_str(), "GW" | "LO" | "LW" | "LS") && starts_parameter(rest),
        _ => false,
    }
}

fn starts_parameter(rest: &str) -> bool {
    match rest.chars().next() {
        Some(c) => c.is_ascii_digit() || c == ',' || c == '"' || c.is_whitespace(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zpl_from_caret_commands() {
        assert_eq!(detect_language(b"^XA\n^FO50,50\n^FDHi\n^FS\n^XZ"), Language::Zpl);
        assert_eq!(detect_language(b"\n\n~DG..."), Language::Zpl);
    }

    #[test]
    fn detects_epl_from_line_commands() {
        assert_eq!(detect_language(b"N\nq812\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n"), Language::Epl);
        assert_eq!(detect_language(b"B50,10,0,3,2,7,200,B,\"123\""), Language::Epl);
        assert_eq!(detect_language(b"GW50,10,2,10,"), Language::Epl);
        assert_eq!(detect_language(b"LO10,20,100,5"), Language::Epl);
    }

    #[test]
    fn multi_letter_words_are_not_epl() {
        // TSPL-shaped input must not be misread as EPL; falls back to Zpl
        // until Language::Tspl exists.
        assert_eq!(detect_language(b"SIZE 100 mm, 60 mm\nCLS\nTEXT 10,10,\"3\",0,1,1,\"Hi\"\nPRINT 1\n"), Language::Zpl);
    }

    #[test]
    fn empty_or_ambiguous_input_defaults_to_zpl() {
        assert_eq!(detect_language(b""), Language::Zpl);
        assert_eq!(detect_language(b"<<<binary>>>\x00\x01"), Language::Zpl);
    }

    #[test]
    fn parse_labels_dispatches_both_languages() {
        let zpl = parse_labels(Language::Zpl, b"^XA^FO50,50^A0N,30,30^FDHi^FS^XZ", 8).unwrap();
        assert_eq!(zpl.len(), 1);
        let epl = parse_labels(Language::Epl, b"N\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n", 8).unwrap();
        assert_eq!(epl.len(), 1);
    }
}
