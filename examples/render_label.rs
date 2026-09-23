//! Render a ZPL or EPL file to PNG (or PDF) through the public library API —
//! the same parse → render → encode pipeline the CLI uses, in ~60 lines.
//!
//! Usage:
//!   cargo run --example render_label -- shipping.zpl
//!   cargo run --example render_label -- shipping.epl
//!   cargo run --example render_label -- shipping.zpl out.pdf --pdf
//!   cargo run --example render_label -- shipping.zpl --width 101.6 --height 203.2 --dpmm 8

use std::io::Cursor;
use std::path::{Path, PathBuf};

use labelize::{DrawerOptions, Renderer};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut options = DrawerOptions::default();
    let mut pdf = false;
    let mut epl_override: Option<bool> = None;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--width" => options.label_width_mm = it.next().unwrap().parse().unwrap(),
            "--height" => options.label_height_mm = it.next().unwrap().parse().unwrap(),
            "--dpmm" => options.dpmm = it.next().unwrap().parse().unwrap(),
            "--antialias" => options.antialias = true,
            "--pdf" => pdf = true,
            "--zpl" => epl_override = Some(false),
            "--epl" => epl_override = Some(true),
            _ if input.is_none() => input = Some(PathBuf::from(arg)),
            _ if output.is_none() => output = Some(PathBuf::from(arg)),
            _ => panic!("unexpected argument: {arg}"),
        }
    }

    let input =
        input.unwrap_or_else(|| panic!("usage: render_label <input.zpl|input.epl> [output]"));
    let is_epl =
        epl_override.unwrap_or_else(|| input.extension().map(|e| e == "epl").unwrap_or(false));
    let output = output.unwrap_or_else(|| {
        let ext = if pdf { "pdf" } else { "png" };
        input.with_extension(ext)
    });

    let data = std::fs::read(&input).unwrap();
    let labels = if is_epl {
        labelize::EplParser::new().parse(&data).unwrap()
    } else {
        labelize::ZplParser::new().parse(&data).unwrap()
    };
    println!("parsed {} label(s) from {}", labels.len(), input.display());

    let renderer = &Renderer::new();
    for (i, label) in labels.iter().enumerate() {
        let bytes = render(renderer, label, &options, pdf);
        let path = output_path(&output, i, labels.len());
        std::fs::write(&path, bytes).unwrap();
        println!("wrote {}", path.display());
    }
}

fn render(
    renderer: &Renderer,
    label: &labelize::LabelInfo,
    options: &DrawerOptions,
    pdf: bool,
) -> Vec<u8> {
    // PNG output comes straight from the renderer.
    let mut png = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(label, &mut png, options.clone())
        .unwrap();
    if !pdf {
        return png.into_inner();
    }
    // PDF wraps the rendered pixels in a single-page document.
    let img = image::load_from_memory(&png.into_inner())
        .unwrap()
        .to_rgba8();
    let mut pdf = Cursor::new(Vec::new());
    labelize::encode_pdf(&img, options, &mut pdf).unwrap();
    pdf.into_inner()
}

fn output_path(base: &Path, index: usize, total: usize) -> PathBuf {
    if total == 1 {
        return base.to_path_buf();
    }
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("png");
    base.with_file_name(format!("{stem}_{}.{ext}", index + 1))
}
