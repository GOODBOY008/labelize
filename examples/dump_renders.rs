//! Dump rendered PNGs to testdata/rendered/ for visual comparison.

use std::io::Cursor;
use std::path::Path;

use labelize::{DrawerOptions, EplParser, Renderer, ZplParser};

/// Matches Labelary reference dimensions: 813×1626 at 8 dpmm
fn default_options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    }
}

fn main() {
    let testdata = Path::new("testdata");
    let outdir = testdata.join("rendered");
    std::fs::create_dir_all(&outdir).unwrap();

    let renderer = Renderer::new();
    let mut ok = 0usize;
    let mut err = 0usize;

    // Render all .zpl files found in testdata/
    let mut zpl_files: Vec<_> = std::fs::read_dir(testdata)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "zpl").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    zpl_files.sort();

    for path in &zpl_files {
        let name = path.file_stem().unwrap().to_string_lossy();
        let content = std::fs::read(path).unwrap();
        let mut parser = ZplParser::new();
        match parser.parse(&content) {
            Ok(labels) if !labels.is_empty() => {
                let mut buf = Cursor::new(Vec::new());
                if renderer.draw_label_as_png(&labels[0], &mut buf, default_options()).is_ok() {
                    let out = outdir.join(format!("{}.png", name));
                    std::fs::write(&out, buf.into_inner()).unwrap();
                    println!("OK  {}.zpl", name);
                    ok += 1;
                } else {
                    println!("ERR render {}.zpl", name);
                    err += 1;
                }
            }
            _ => {
                println!("ERR parse {}.zpl", name);
                err += 1;
            }
        }
    }

    // Render all .epl files found in testdata/
    let mut epl_files: Vec<_> = std::fs::read_dir(testdata)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "epl").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    epl_files.sort();

    for path in &epl_files {
        let name = path.file_stem().unwrap().to_string_lossy();
        let content = std::fs::read(path).unwrap();
        let parser = EplParser::new();
        match parser.parse(&content) {
            Ok(labels) if !labels.is_empty() => {
                let mut buf = Cursor::new(Vec::new());
                if renderer.draw_label_as_png(&labels[0], &mut buf, default_options()).is_ok() {
                    let out = outdir.join(format!("{}.png", name));
                    std::fs::write(&out, buf.into_inner()).unwrap();
                    println!("OK  {}.epl", name);
                    ok += 1;
                } else {
                    println!("ERR render {}.epl", name);
                    err += 1;
                }
            }
            _ => {
                println!("ERR parse {}.epl", name);
                err += 1;
            }
        }
    }

    println!("\nRendered {} OK, {} ERR → testdata/rendered/", ok, err);
}
