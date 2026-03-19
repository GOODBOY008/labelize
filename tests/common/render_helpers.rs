use std::io::Cursor;

use labelize::{DrawerOptions, EplParser, LabelInfo, Renderer, ZplParser};

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

/// Render a `LabelInfo` to PNG bytes using the given options.
pub fn render_label_to_png(label: &LabelInfo, options: DrawerOptions) -> Vec<u8> {
    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(label, &mut buf, options)
        .expect("render failed");
    buf.into_inner()
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
