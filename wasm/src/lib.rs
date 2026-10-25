//! labelize compiled to wasm32-unknown-unknown, exposed through wasm-bindgen.

use labelize::Language;
use wasm_bindgen::prelude::*;

/// Failure stage, which maps to HTTP status in the JS worker:
/// Parse → 400 (bad input), Render → 500 (internal).
#[derive(Debug, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Render,
}

fn render_payload_lang(
    bytes: &[u8],
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: bool,
    want_pdf: bool,
    language: Language,
) -> Result<Vec<u8>, (Stage, String)> {
    // The crate's one-call render path; JS always passes explicit dimensions
    // (the browser playground knows its canvas size), so width/height are
    // Some(...) and the 102×152 fallback never engages here.
    let options = labelize::RenderOptions {
        language: Some(language),
        width_mm: if width_mm > 0.0 { Some(width_mm) } else { None },
        height_mm: if height_mm > 0.0 { Some(height_mm) } else { None },
        dpmm,
        antialias,
    };
    let stage = |err: labelize::LabelizeError| match err {
        labelize::LabelizeError::Parse(msg) => (Stage::Parse, msg),
        e => (Stage::Render, e.to_string()),
    };
    let rendered = if want_pdf {
        labelize::render_pdf(bytes, options)
    } else {
        labelize::render_png(bytes, options)
    };
    rendered.map(|out| out.bytes).map_err(stage)
}

/// Error surfaced to JS as a thrown string prefixed with the stage code:
/// `1:<msg>` → HTTP 400, `2:<msg>` → HTTP 500. A plain String keeps the
/// wasm-bindgen surface minimal (exported structs must be Copy).
fn render_error(stage: &Stage, msg: String) -> String {
    match stage {
        Stage::Parse => format!("1:{msg}"),
        Stage::Render => format!("2:{msg}"),
    }
}

/// Parse ZPL/EPL and render to PNG (or PDF when `want_pdf`). Copies all data
/// across the JS boundary automatically via wasm-bindgen.
///
/// Legacy boolean-language variant kept for backwards compatibility — prefer
/// `lz_render_lang`, which names all three languages.
#[wasm_bindgen]
pub fn lz_render(
    src: &[u8],
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: bool,
    want_pdf: bool,
    is_epl: bool,
) -> Result<Vec<u8>, String> {
    let language = if is_epl {
        Language::Epl
    } else {
        Language::Zpl
    };
    render_payload_lang(
        src,
        width_mm,
        height_mm,
        dpmm,
        antialias,
        want_pdf,
        language,
    )
    .map_err(|(stage, msg)| render_error(&stage, msg))
}

/// Language-explicit variant: `lang` is `"zpl"`, `"epl"`, `"tspl"`,
/// `"cpcl"` or `"dpl"`. Unknown values fall back to ZPL, matching the
/// CLI's sniff fallback.
#[wasm_bindgen]
pub fn lz_render_lang(
    src: &[u8],
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: bool,
    want_pdf: bool,
    lang: &str,
) -> Result<Vec<u8>, String> {
    let language = match lang.to_ascii_lowercase().as_str() {
        "epl" => Language::Epl,
        "tspl" => Language::Tspl,
        "cpcl" => Language::Cpcl,
        "dpl" => Language::Dpl,
        _ => Language::Zpl,
    };
    render_payload_lang(src, width_mm, height_mm, dpmm, antialias, want_pdf, language)
        .map_err(|(stage, msg)| render_error(&stage, msg))
}

/// The playground HTML page — single source of truth in labelize's
/// `src/playground.rs`, exported so the axum server and the worker serve
/// identical markup.
#[wasm_bindgen]
pub fn lz_playground_html() -> String {
    labelize::playground::PLAYGROUND_HTML.to_string()
}

/// Validate label source without rendering (the worker's POST /validate):
/// detect the language unless `lang` names one, parse, and return a JSON
/// report string `{"language":"zpl","ok":true,"labels":1}` or
/// `{"language":"epl","ok":false,"labels":0,"error":"..."}`. Never throws —
/// parse failures travel inside the report.
#[wasm_bindgen]
pub fn lz_validate(src: &[u8], dpmm: i32, lang: &str) -> String {
    let report = labelize::validate(src, dpmm, labelize::Language::from_name(lang));
    let mut out = String::with_capacity(160);
    out.push_str("{\"language\":\"");
    out.push_str(report.language.name());
    out.push_str("\",\"ok\":");
    out.push_str(if report.ok { "true" } else { "false" });
    out.push_str(",\"labels\":");
    out.push_str(&report.labels.to_string());
    if let Some(err) = report.error {
        out.push_str(",\"error\":\"");
        out.push_str(&json_escape(&err));
        out.push('"');
    }
    out.push('}');
    out
}

/// Pretty-print label source (the worker's POST /format): one command per
/// line, `GW`/`BITMAP` binary payloads byte-exact. `lang` optionally forces
/// the language ("" = auto-detect). Returns raw bytes; non-payload source
/// is ASCII text, so JS consumers can TextDecoder it.
#[wasm_bindgen]
pub fn lz_format(src: &[u8], lang: &str) -> Vec<u8> {
    labelize::format_source(src, labelize::Language::from_name(lang)).1
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ZPL: &str = "^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ";
    const SAMPLE_EPL: &str = include_str!("../../testdata/labels/dpduk.epl");
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn png(bytes: &[u8]) -> bool {
        bytes.len() >= 8 && bytes[..8] == PNG_MAGIC
    }

    fn render_payload(
        bytes: &[u8],
        width_mm: f64,
        height_mm: f64,
        dpmm: i32,
        antialias: bool,
        want_pdf: bool,
        is_epl: bool,
    ) -> Result<Vec<u8>, (Stage, String)> {
        render_payload_lang(
            bytes,
            width_mm,
            height_mm,
            dpmm,
            antialias,
            want_pdf,
            if is_epl { Language::Epl } else { Language::Zpl },
        )
    }

    #[test]
    fn renders_zpl_to_png() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, false, false)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_epl_to_png() {
        let out = render_payload(SAMPLE_EPL.as_bytes(), 102.0, 152.0, 8, false, false, true)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_to_pdf() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, true, false)
            .expect("render ok");
        assert!(out.starts_with(b"%PDF"), "expected PDF header");
    }

    #[test]
    fn parse_error_is_reported() {
        let err = render_payload(b"this is not a label", 102.0, 152.0, 8, false, false, false)
            .expect_err("garbage input must fail");
        assert_eq!(err.0, Stage::Parse);
    }

    #[test]
    fn empty_input_yields_no_labels_error() {
        let err = render_payload(b"", 102.0, 152.0, 8, false, false, false)
            .expect_err("empty input must fail");
        assert_eq!(err.1, "No labels found".to_string());
    }

    #[test]
    fn exports_roundtrip_zpl() {
        let out = lz_render(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, false, false)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn exports_report_parse_error() {
        let err = lz_render(b"NOPE", 102.0, 152.0, 8, false, false, false)
            .expect_err("parse must fail");
        assert!(err.starts_with("1:"), "parse error carries code 1: {err}");
    }

    #[test]
    fn exports_playground_html() {
        let html = lz_playground_html();
        assert!(html.contains("<textarea"), "playground page has editor");
        assert!(html.contains("compare-btn"), "playground page has compare tool");
    }

    #[test]
    fn lz_validate_reports_ok_json() {
        let json = lz_validate(SAMPLE_ZPL.as_bytes(), 8, "");
        assert_eq!(
            json,
            r#"{"language":"zpl","ok":true,"labels":1}"#,
            "got: {json}"
        );
    }

    #[test]
    fn lz_validate_detects_and_forces_language() {
        // EPL sample detected from content; `lang` left empty = auto-detect.
        let json = lz_validate(b"N\nq812\nB10,10,0,9,2,4,80,B,\"oops\"\nP1\n", 8, "");
        assert!(
            json.starts_with(r#"{"language":"epl","ok":false,"labels":0,"error":"#),
            "got: {json}"
        );
        assert!(json.ends_with('}'));

        // Forced language beats detection.
        let json = lz_validate(b"A50,50,0,1,1,1,N,\"Hi\"\nP1\n", 8, "epl");
        assert_eq!(
            json,
            r#"{"language":"epl","ok":true,"labels":1}"#,
            "got: {json}"
        );
    }

    #[test]
    fn lz_format_reflows_each_language() {
        let zpl = lz_format(b"^XA^FDHi^FS^XZ", "");
        assert_eq!(zpl, b"^XA\n  ^FDHi\n^FS\n^XZ\n".to_vec());

        let tspl = lz_format(b"SIZE 60 mm,30 mm\r\nCLS\r\nPRINT 1\r\n", "");
        assert_eq!(tspl, b"SIZE 60 mm,30 mm\nCLS\nPRINT 1\n".to_vec());

        // Forced language beats detection.
        let epl = lz_format(b"A50,50,0,1,1,1,N,\"Hi\"\nP1\n", "epl");
        assert_eq!(epl, b"A50,50,0,1,1,1,N,\"Hi\"\nP1\n".to_vec());
    }

    #[test]
    fn json_escape_handles_specials() {
        assert_eq!(json_escape("a\"b\\c\nd\te"), "a\\\"b\\\\c\\nd\\te");
        assert_eq!(json_escape("\u{1}"), "\\u0001");
        assert_eq!(json_escape("plain text"), "plain text");
    }
}