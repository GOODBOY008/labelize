//! One-call rendering: bytes in, PNG/PDF bytes out.
//!
//! The layers below this module (`Language`/`parse_labels`, `Renderer`,
//! `encode_pdf`) are the composable building blocks; this module is the
//! convenience surface SDK callers should reach for first. It owns the glue
//! every entry point used to re-implement — language detection, first-label
//! selection, dimension fallback, and PNG/PDF encoding — and returns a
//! single [`LabelizeError`] so parse failures (bad input) and render
//! failures (engine-side) are distinguishable without matching on
//! heterogeneous error types.

use std::io::Cursor;

use crate::elements::drawer_options::DrawerOptions;
use crate::error::LabelizeError;
use crate::lang::{detect_language, parse_labels, Language};
use crate::Renderer;

/// Fallback canvas when neither the caller nor the label declares a size
/// (TSPL `SIZE` / label-declared dims are preferred when present).
/// 102 × 152 mm matches the historical CLI/HTTP/wasm defaults (4×6"
/// rounded); the low-level `DrawerOptions::default()` keeps its own
/// 101.6 × 203.2 for direct renderer users.
pub const DEFAULT_LABEL_WIDTH_MM: f64 = 102.0;
pub const DEFAULT_LABEL_HEIGHT_MM: f64 = 152.0;

/// High-level render settings; every field has a sensible default.
///
/// ```no_run
/// use labelize::{RenderOptions, Language};
///
/// let opts = RenderOptions {
///     language: Some(Language::Epl), // None = auto-detect from the data
///     width_mm: Some(100.0),         // None = label-declared, else 102×152
///     ..RenderOptions::default()     // dpmm: 8, antialias: false
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Parse in this language instead of auto-detecting.
    pub language: Option<Language>,
    /// Explicit canvas width in mm; `None` prefers the label's declared
    /// size (TSPL `SIZE`), falling back to [`DEFAULT_LABEL_WIDTH_MM`].
    pub width_mm: Option<f64>,
    /// Explicit canvas height in mm; see [`RenderOptions::width_mm`].
    pub height_mm: Option<f64>,
    /// Dots per millimetre (6 / 8 / 12 / 24). Feeds ZPL `^MU` unit
    /// conversion and the raster size.
    pub dpmm: i32,
    /// Keep renderer greys instead of thresholding to 1-bit.
    pub antialias: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            language: None,
            width_mm: None,
            height_mm: None,
            dpmm: 8,
            antialias: false,
        }
    }
}

/// The result of a one-call render.
#[derive(Debug)]
pub struct Rendered {
    /// Language the input was parsed as (detected unless forced).
    pub language: Language,
    /// Total labels in the input — a `^PQ` quantity or repeated `^XA` blocks
    /// produce more than one. This call renders the first; use
    /// [`parse_labels`] to iterate the rest.
    pub labels: usize,
    /// Encoded bytes: PNG by default, PDF via [`render_pdf`].
    pub bytes: Vec<u8>,
}

/// Detect, parse and render the first label to PNG bytes.
///
/// ```no_run
/// let out = labelize::render_png(b"^XA^FO50,50^A0N,40,40^FDHi^FS^XZ", Default::default()).unwrap();
/// std::fs::write("label.png", out.bytes).unwrap();
/// ```
pub fn render_png(bytes: &[u8], options: RenderOptions) -> Result<Rendered, LabelizeError> {
    render(bytes, options, false)
}

/// Detect, parse and render the first label to a single-page PDF.
pub fn render_pdf(bytes: &[u8], options: RenderOptions) -> Result<Rendered, LabelizeError> {
    render(bytes, options, true)
}

fn render(bytes: &[u8], options: RenderOptions, want_pdf: bool) -> Result<Rendered, LabelizeError> {
    let language = options.language.unwrap_or_else(|| detect_language(bytes));
    let labels = parse_labels(language, bytes, options.dpmm)?;
    let label = labels
        .first()
        .ok_or_else(|| LabelizeError::Parse("No labels found".to_string()))?;

    let drawer = DrawerOptions {
        label_width_mm: options
            .width_mm
            .or(label.label_width_mm)
            .unwrap_or(DEFAULT_LABEL_WIDTH_MM),
        label_height_mm: options
            .height_mm
            .or(label.label_height_mm)
            .unwrap_or(DEFAULT_LABEL_HEIGHT_MM),
        dpmm: options.dpmm,
        antialias: options.antialias,
        ..Default::default()
    };

    let renderer = Renderer::new();
    let mut png = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(label, &mut png, drawer.clone())
        .map_err(LabelizeError::Render)?;

    let bytes = if want_pdf {
        let img = image::load_from_memory(&png.into_inner())
            .map_err(|e| LabelizeError::Render(format!("image decode: {e}")))?
            .to_rgba8();
        let mut pdf = Cursor::new(Vec::new());
        crate::images::pdf::encode_pdf(&img, &drawer, &mut pdf)?;
        pdf.into_inner()
    } else {
        png.into_inner()
    };

    Ok(Rendered {
        language,
        labels: labels.len(),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn dimensions(png: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory(png).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn renders_first_label_with_detection() {
        let out = render_png(
            b"^XA^FO50,50^A0N,40,40^FDHi^FS^XZ",
            RenderOptions::default(),
        )
        .expect("render ok");
        assert_eq!(out.language, Language::Zpl);
        assert_eq!(out.labels, 1);
        assert!(out.bytes.starts_with(&PNG_MAGIC));
    }

    #[test]
    fn forced_language_and_declared_dims_win() {
        let tspl = b"SIZE 60 mm,30 mm\nCLS\nTEXT 10,10,\"3\",0,1,1,\"Hi\"\nPRINT 1\n";
        let out = render_png(tspl, RenderOptions::default()).expect("render ok");
        assert_eq!(out.language, Language::Tspl);
        assert_eq!(dimensions(&out.bytes), (480, 240)); // 60×30 mm @ 8 dpmm

        // An explicit user size overrides the declared one.
        let out = render_png(
            tspl,
            RenderOptions {
                width_mm: Some(100.0),
                height_mm: Some(62.0),
                ..RenderOptions::default()
            },
        )
        .expect("render ok");
        assert_eq!(dimensions(&out.bytes), (800, 496));
    }

    #[test]
    fn fallback_dims_match_http_defaults() {
        let out = render_png(b"^XA^FDHi^FS^XZ", RenderOptions::default()).expect("render ok");
        assert_eq!(dimensions(&out.bytes), (816, 1216)); // 102×152 mm @ 8 dpmm
    }

    #[test]
    fn pdf_prefix_and_labels_count() {
        let two = b"^XA^FO10,10^FDa^FS^XZ^XA^FO10,10^FDb^FS^XZ";
        let out = render_pdf(two, RenderOptions::default()).expect("render ok");
        assert!(out.bytes.starts_with(b"%PDF"));
        assert_eq!(out.labels, 2, "PQ/repeated formats report their count");
    }

    #[test]
    fn parse_errors_carry_the_parser_message() {
        let err = render_png(
            b"N\nq812\nB10,10,0,9,2,4,80,B,\"x\"\nP1\n",
            RenderOptions::default(),
        )
        .expect_err("invalid EPL must fail");
        assert!(matches!(err, LabelizeError::Parse(_)));
        assert!(err.to_string().contains("unsupported bar code type"));
    }

    #[test]
    fn empty_input_is_a_parse_error() {
        let err = render_png(b"", RenderOptions::default()).expect_err("empty must fail");
        assert!(err.to_string().contains("No labels found"));
    }
}
