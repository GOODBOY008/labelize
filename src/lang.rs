//! The single front door for label data.
//!
//! Every entry point (CLI, HTTP service, wasm bindings) goes through
//! [`Language`] / [`parse_labels`] instead of picking parser types itself, so
//! adding a language means adding one enum variant and one match arm here —
//! the parsers, elements, and renderer stay untouched.

use crate::elements::label_info::LabelInfo;
use crate::error::LabelizeError;
use crate::parsers::epl::parser::EplParser;
use crate::parsers::tspl::parser::TsplParser;
use crate::parsers::zpl::parser::ZplParser;

/// A label printer language the engine understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    /// Zebra Programming Language (`^XA … ^XZ`).
    Zpl,
    /// Eltron Programming Language (line commands `A`, `B`, `P`, …).
    Epl,
    /// TSPL/TSC command language (`SIZE`, `TEXT`, `PRINT`, …).
    Tspl,
}

impl Language {
    /// Lowercase wire name: `"zpl"`, `"epl"`, `"tspl"`. Used by the HTTP
    /// `?language=` parameter, the wasm `lz_check` binding, and JSON reports.
    pub fn name(self) -> &'static str {
        match self {
            Language::Zpl => "zpl",
            Language::Epl => "epl",
            Language::Tspl => "tspl",
        }
    }

    /// Inverse of [`Language::name`]; case-insensitive, unknown → `None`.
    pub fn from_name(name: &str) -> Option<Language> {
        match name.to_ascii_lowercase().as_str() {
            "zpl" => Some(Language::Zpl),
            "epl" => Some(Language::Epl),
            "tspl" => Some(Language::Tspl),
            _ => None,
        }
    }

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
            Language::Tspl => TsplParser::with_dpmm(dpmm)
                .parse(bytes)
                .map_err(LabelizeError::Parse),
        }
    }
}

/// Outcome of a [`validate`] pass: which language the data was checked as,
/// whether it parsed, and the parse error when it did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// Language the data was checked as (auto-detected when not forced).
    pub language: Language,
    /// `true` when the data parsed without errors.
    pub ok: bool,
    /// Number of labels found (`0` when parsing failed).
    pub labels: usize,
    /// Parse error message when [`ValidationReport::ok`] is `false`.
    pub error: Option<String>,
}

/// Check the syntax of `bytes` without rendering: detect the language (or
/// use `forced` when given), parse, and report pass/fail plus the label
/// count. Empty input parses as zero labels, so it reports `ok`.
pub fn validate(bytes: &[u8], dpmm: i32, forced: Option<Language>) -> ValidationReport {
    let language = forced.unwrap_or_else(|| detect_language(bytes));
    match parse_labels(language, bytes, dpmm) {
        Ok(labels) => ValidationReport {
            language,
            ok: true,
            labels: labels.len(),
            error: None,
        },
        Err(e) => ValidationReport {
            language,
            ok: false,
            labels: 0,
            error: Some(e.to_string()),
        },
    }
}

/// Pretty-print label source without changing what it renders: detect the
/// language (or use `forced`), reflow it one command per line, and return
/// the language together with the formatted bytes. Formatters never fail and
/// never touch binary payloads (`^FD` data, `GW`/`BITMAP` graphics), so the
/// output parses to the same labels as the input.
pub fn format_source(bytes: &[u8], forced: Option<Language>) -> (Language, Vec<u8>) {
    let language = forced.unwrap_or_else(|| detect_language(bytes));
    let formatted = match language {
        Language::Zpl => crate::parsers::zpl::format::format_zpl(bytes),
        Language::Epl => crate::parsers::epl::format::format_epl(bytes),
        Language::Tspl => crate::parsers::tspl::format::format_tspl(bytes),
    };
    (language, formatted)
}

/// Parse `bytes` in the given [`Language`] into one [`LabelInfo`] per label.
pub fn parse_labels(
    language: Language,
    bytes: &[u8],
    dpmm: i32,
) -> Result<Vec<LabelInfo>, LabelizeError> {
    language.parse(bytes, dpmm)
}

/// Command words that unambiguously identify TSPL. Deliberately narrow:
/// these are full-line command names no other supported language uses.
const TSPL_COMMAND_WORDS: &[&str] = &[
    "SIZE",
    "DIRECTION",
    "REFERENCE",
    "SHIFT",
    "CLS",
    "PRINT",
    "TEXT",
    "BARCODE",
    "QRCODE",
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

/// Guess the language of `bytes` from its shape.
///
/// ZPL formats are caret/tilde command streams; EPL is one command letter
/// per line with comma-separated parameters; TSPL lines start with a
/// multi-letter command word (`SIZE 100 mm, 60 mm`, `PRINT 1`). When the
/// input matches none (or is empty) the function falls back to ZPL,
/// matching the historical CLI behavior.
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
        let word: String = line
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        if TSPL_COMMAND_WORDS.contains(&word.to_ascii_uppercase().as_str()) {
            return Language::Tspl;
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
        assert_eq!(
            detect_language(b"^XA\n^FO50,50\n^FDHi\n^FS\n^XZ"),
            Language::Zpl
        );
        assert_eq!(detect_language(b"\n\n~DG..."), Language::Zpl);
    }

    #[test]
    fn detects_epl_from_line_commands() {
        assert_eq!(
            detect_language(b"N\nq812\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n"),
            Language::Epl
        );
        assert_eq!(
            detect_language(b"B50,10,0,3,2,7,200,B,\"123\""),
            Language::Epl
        );
        assert_eq!(detect_language(b"GW50,10,2,10,"), Language::Epl);
        assert_eq!(detect_language(b"LO10,20,100,5"), Language::Epl);
    }

    #[test]
    fn multi_letter_command_words_are_tspl() {
        assert_eq!(
            detect_language(b"SIZE 100 mm, 60 mm\nCLS\nTEXT 10,10,\"3\",0,1,1,\"Hi\"\nPRINT 1\n"),
            Language::Tspl
        );
        assert_eq!(detect_language(b"GAP 0,0\nPRINT 1\n"), Language::Tspl);
    }

    #[test]
    fn parse_labels_dispatches_tspl() {
        let tspl = parse_labels(
            Language::Tspl,
            b"SIZE 101.5 mm,203 mm\nCLS\nTEXT 50,50,\"1\",0,1,1,\"Hi\"\nPRINT 1\n",
            8,
        )
        .unwrap();
        assert_eq!(tspl.len(), 1);
        assert_eq!(tspl[0].label_width_mm, Some(101.5));
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

    #[test]
    fn language_names_roundtrip() {
        for lang in [Language::Zpl, Language::Epl, Language::Tspl] {
            assert_eq!(Language::from_name(lang.name()), Some(lang));
        }
        assert_eq!(Language::from_name("ZPL"), Some(Language::Zpl));
        assert_eq!(Language::from_name("pdf"), None);
    }

    #[test]
    fn validate_passes_valid_labels() {
        let report = validate(b"^XA^FO50,50^A0N,30,30^FDHi^FS^XZ", 8, None);
        assert_eq!(report.language, Language::Zpl);
        assert!(report.ok);
        assert_eq!(report.labels, 1);
        assert!(report.error.is_none());
    }

    #[test]
    fn validate_reports_parse_errors() {
        let report = validate(b"N\nq812\nB10,10,0,9,2,4,80,B,\"oops\"\nP1\n", 8, None);
        assert_eq!(report.language, Language::Epl);
        assert!(!report.ok);
        assert_eq!(report.labels, 0);
        let err = report.error.expect("error message present");
        assert!(err.contains("Parse error"), "got: {err}");
    }

    #[test]
    fn validate_forced_language_skips_detection() {
        let report = validate(b"A50,50,0,1,1,1,N,\"Hi\"\nP1\n", 8, Some(Language::Epl));
        assert_eq!(report.language, Language::Epl);
        assert!(report.ok);
    }

    #[test]
    fn validate_empty_input_is_ok_with_zero_labels() {
        let report = validate(b"", 8, None);
        assert!(report.ok);
        assert_eq!(report.labels, 0);
    }

    #[test]
    fn format_source_dispatches_all_languages() {
        let (lang, out) = format_source(b"^XA^FDHi^FS^XZ", None);
        assert_eq!(lang, Language::Zpl);
        assert_eq!(out, b"^XA\n  ^FDHi\n^FS\n^XZ\n".to_vec());

        let (lang, out) = format_source(b"N\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n", None);
        assert_eq!(lang, Language::Epl);
        assert_eq!(out, b"N\nA50,50,0,1,1,1,N,\"Hi\"\nP1\n".to_vec());

        let (lang, out) = format_source(b"SIZE 60 mm,30 mm\r\nCLS\r\nPRINT 1\r\n", None);
        assert_eq!(lang, Language::Tspl);
        assert_eq!(out, b"SIZE 60 mm,30 mm\nCLS\nPRINT 1\n".to_vec());
    }

    #[test]
    fn format_source_forced_language_skips_detection() {
        let (lang, _) = format_source(b"A50,50,0,1,1,1,N,\"Hi\"\nP1\n", Some(Language::Epl));
        assert_eq!(lang, Language::Epl);
    }

    #[test]
    fn format_source_round_trips_real_fixtures() {
        // Formatting must never change what parses: same label and element
        // counts on real corpus fixtures.
        let cases: [(&str, Language); 3] = [
            ("testdata/labels/dpdpl.zpl", Language::Zpl),
            ("testdata/labels/dpduk.epl", Language::Epl),
            ("testdata/unit/tspl_graphics.tspl", Language::Tspl),
        ];
        for (path, lang) in cases {
            let Ok(src) = std::fs::read(path) else {
                continue;
            };
            let before = parse_labels(lang, &src, 8).unwrap_or_else(|e| panic!("{path}: {e}"));
            let (_, formatted) = format_source(&src, Some(lang));
            let after = parse_labels(lang, &formatted, 8).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(before.len(), after.len(), "{path} label count");
            for (b, a) in before.iter().zip(after.iter()) {
                assert_eq!(b.elements.len(), a.elements.len(), "{path} elements");
            }
        }
    }
}
