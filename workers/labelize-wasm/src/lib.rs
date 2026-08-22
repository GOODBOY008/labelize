//! labelize compiled to wasm32-unknown-unknown, exposed through wasm-bindgen.

use std::io::Cursor;

use labelize::drawers::renderer::Renderer;
use labelize::elements::drawer_options::DrawerOptions;
use labelize::parsers::epl_parser::EplParser;
use labelize::parsers::zpl_parser::ZplParser;
use wasm_bindgen::prelude::*;

/// Failure stage, which maps to HTTP status in the JS worker:
/// Parse → 400 (bad input), Render → 500 (internal).
#[derive(Debug, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Render,
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
    let labels = if is_epl {
        EplParser::new().parse(bytes)
    } else {
        ZplParser::with_dpmm(dpmm).parse(bytes)
    }
    .map_err(|e| (Stage::Parse, e.to_string()))?;

    let label = labels
        .into_iter()
        .next()
        .ok_or_else(|| (Stage::Parse, "No labels found".to_string()))?;

    let options = DrawerOptions {
        label_width_mm: width_mm,
        label_height_mm: height_mm,
        dpmm,
        antialias,
        ..Default::default()
    };

    let renderer = Renderer::new();
    let mut png_buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(&label, &mut png_buf, options.clone())
        .map_err(|e| (Stage::Render, e.to_string()))?;

    if want_pdf {
        let img = image::load_from_memory(png_buf.get_ref())
            .map_err(|e| (Stage::Render, format!("image decode: {e}")))?
            .to_rgba8();
        let mut pdf_buf = Cursor::new(Vec::new());
        labelize::encode_pdf(&img, &options, &mut pdf_buf)
            .map_err(|e| (Stage::Render, e.to_string()))?;
        Ok(pdf_buf.into_inner())
    } else {
        Ok(png_buf.into_inner())
    }
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
    render_payload(src, width_mm, height_mm, dpmm, antialias, want_pdf, is_epl)
        .map_err(|(stage, msg)| render_error(&stage, msg))
}

/// The playground HTML page — single source of truth in labelize's
/// `src/playground.rs`, exported so the axum server and the worker serve
/// identical markup.
#[wasm_bindgen]
pub fn lz_playground_html() -> String {
    labelize::playground::PLAYGROUND_HTML.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ZPL: &str = "^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ";
    const SAMPLE_EPL: &str = include_str!("../../../testdata/labels/dpduk.epl");
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn png(bytes: &[u8]) -> bool {
        bytes.len() >= 8 && bytes[..8] == PNG_MAGIC
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
    }
}