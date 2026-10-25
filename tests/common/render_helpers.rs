use std::io::Cursor;

use labelize::{
    CpclParser, DplParser, DrawerOptions, EplParser, LabelInfo, Renderer, TsplParser, ZplParser,
};

/// Label size (in inches) for every Labelary API request, yielding a native
/// 813×1626 px canvas — exactly the `default_options()` canvas.
///
/// Do NOT derive this from `default_options()` mm values (`mm / 25.4`): Labelary
/// floor-rounds the resulting 4.00197×8.00197 in request to a 812×1624 px canvas,
/// and padding that response up to 813×1626 shifts all content by (−1, −2) px
/// relative to a native render — every `^POI` (inverted) label then misaligns
/// against our renderer by that offset.
pub const LABELARY_LABEL_SIZE_IN: (f64, f64) = (4.005, 8.01);

/// Default DrawerOptions matching Labelary reference images (101.625mm × 203.25mm, 8 dpmm → 813×1626 px).
pub fn default_options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    }
}

/// Parse ZPL bytes and return the vector of `LabelInfo`.
pub fn parse_zpl(zpl: &[u8]) -> Vec<LabelInfo> {
    let mut parser = ZplParser::new();
    parser.parse(zpl).expect("ZPL parse failed")
}

/// Parse EPL bytes and return the vector of `LabelInfo`.
pub fn parse_epl(epl: &[u8]) -> Vec<LabelInfo> {
    let parser = EplParser::new();
    parser.parse(epl).expect("EPL parse failed")
}

/// Parse TSPL bytes and return the vector of `LabelInfo`.
pub fn parse_tspl(tspl: &[u8]) -> Vec<LabelInfo> {
    TsplParser::new().parse(tspl).expect("TSPL parse failed")
}

/// Parse CPCL bytes and return the vector of `LabelInfo`.
pub fn parse_cpcl(cpcl: &[u8]) -> Vec<LabelInfo> {
    CpclParser::new().parse(cpcl).expect("CPCL parse failed")
}

/// Parse DPL bytes and return the vector of `LabelInfo`.
pub fn parse_dpl(dpl: &[u8]) -> Vec<LabelInfo> {
    DplParser::new().parse(dpl).expect("DPL parse failed")
}

/// Parse ZPL and render the first label to PNG bytes.
pub fn render_zpl_to_png(zpl: &str, options: DrawerOptions) -> Vec<u8> {
    let mut parser = ZplParser::new();
    let labels = parser.parse(zpl.as_bytes()).expect("ZPL parse failed");
    assert!(!labels.is_empty(), "no labels produced from ZPL");
    render_label_to_png(&labels[0], options)
}

/// Parse EPL and render the first label to PNG bytes.
pub fn render_epl_to_png(epl: &str, options: DrawerOptions) -> Vec<u8> {
    let parser = EplParser::new();
    let labels = parser.parse(epl.as_bytes()).expect("EPL parse failed");
    assert!(!labels.is_empty(), "no labels produced from EPL");
    render_label_to_png(&labels[0], options)
}

/// Parse TSPL and render the first label to PNG bytes. The options' dims
/// come from the fixture's own `SIZE` when it declares one, falling back to
/// the given `options` — pass `unit_options()`/`default_options()` as the
/// fallback like the ZPL/EPL helpers do.
pub fn render_tspl_to_png(tspl: &str, options: DrawerOptions) -> Vec<u8> {
    let parser = TsplParser::new();
    let labels = parser.parse(tspl.as_bytes()).expect("TSPL parse failed");
    assert!(!labels.is_empty(), "no labels produced from TSPL");
    let label = &labels[0];
    let resolved = DrawerOptions {
        label_width_mm: label.label_width_mm.unwrap_or(options.label_width_mm),
        label_height_mm: label.label_height_mm.unwrap_or(options.label_height_mm),
        ..options
    };
    render_label_to_png(label, resolved)
}

/// Parse CPCL and render the first label to PNG bytes. The options' dims
/// come from the label's own header height / PAGE-WIDTH when declared,
/// falling back to the given `options`.
pub fn render_cpcl_to_png(cpcl: &str, options: DrawerOptions) -> Vec<u8> {
    let parser = CpclParser::new();
    let labels = parser.parse(cpcl.as_bytes()).expect("CPCL parse failed");
    assert!(!labels.is_empty(), "no labels produced from CPCL");
    let label = &labels[0];
    let resolved = DrawerOptions {
        label_width_mm: label.label_width_mm.unwrap_or(options.label_width_mm),
        label_height_mm: label.label_height_mm.unwrap_or(options.label_height_mm),
        ..options
    };
    render_label_to_png(label, resolved)
}

/// Parse DPL and render the first label to PNG bytes. DPL declares no
/// in-band label size (printer config does), so the canvas always comes
/// from the given `options`.
pub fn render_dpl_to_png(dpl: &[u8], options: DrawerOptions) -> Vec<u8> {
    let parser = DplParser::new();
    let labels = parser.parse(dpl).expect("DPL parse failed");
    assert!(!labels.is_empty(), "no labels produced from DPL");
    render_label_to_png(&labels[0], options)
}

/// Render a `LabelInfo` to PNG bytes using the given options.
pub fn render_label_to_png(label: &LabelInfo, options: DrawerOptions) -> Vec<u8> {
    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(label, &mut buf, options)
        .expect("render failed");
    buf.into_inner()
}

/// DrawerOptions matching Labelary unit test reference images (812×1624 px).
/// Labelary returns 812×1624 for these mm parameters due to floating-point rounding.
pub fn unit_options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 101.5,
        label_height_mm: 203.0,
        dpmm: 8,
        ..Default::default()
    }
}

/// Path to the testdata directory.
pub fn testdata_dir() -> std::path::PathBuf {
    let local = std::path::Path::new("testdata");
    if local.exists() {
        return local.to_path_buf();
    }
    let parent = std::path::Path::new("../testdata");
    if parent.exists() {
        return parent.to_path_buf();
    }
    panic!("testdata directory not found (tried ./testdata and ../testdata)");
}

/// Returns the path to the `testdata/labels/` directory (real carrier labels).
pub fn labels_dir() -> std::path::PathBuf {
    testdata_dir().join("labels")
}

/// Returns the path to the `testdata/unit/` directory (synthetic test fixtures).
pub fn unit_dir() -> std::path::PathBuf {
    testdata_dir().join("unit")
}
