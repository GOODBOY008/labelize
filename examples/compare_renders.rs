//! Render all testdata ZPL/EPL files and produce side-by-side comparison PNGs
//! with the original reference image on the left and the new render on the right.
//!
//! Output goes to testdata/diffs/
//!
//! Usage: cargo run --example compare_renders

use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
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

/// Draw a label text centered on a colored banner at the top of an image strip.
fn draw_header(img: &mut RgbaImage, x_offset: u32, width: u32, label: &str) {
    let banner_height = 32u32;
    let bg = Rgba([40, 40, 40, 255]);
    for y in 0..banner_height {
        for x in x_offset..x_offset + width {
            img.put_pixel(x, y, bg);
        }
    }
    // Simple text: write the label character by character using a tiny 1px font
    // (We skip fancy font rendering — the filenames in the output path are enough context)
    let fg = Rgba([255, 255, 255, 255]);
    let start_x = x_offset + 8;
    let start_y = 10u32;
    for (i, ch) in label.chars().enumerate() {
        let cx = start_x + (i as u32) * 7;
        if cx + 5 >= x_offset + width {
            break;
        }
        // Draw a small dot per character as a minimal marker
        for dy in 0..3u32 {
            for dx in 0..5u32 {
                if ch != ' ' {
                    img.put_pixel(cx + dx, start_y + dy, fg);
                }
            }
        }
    }
}

/// Combine two images side-by-side with a separator and headers.
fn combine_side_by_side(
    reference: &DynamicImage,
    rendered: &DynamicImage,
) -> RgbaImage {
    let (rw, rh) = reference.dimensions();
    let (nw, nh) = rendered.dimensions();

    let header_h = 32u32;
    let separator = 4u32;
    let max_h = rh.max(nh);
    let total_w = rw + separator + nw;
    let total_h = max_h + header_h;

    let mut combined: RgbaImage = ImageBuffer::from_pixel(total_w, total_h, Rgba([200, 200, 200, 255]));

    // Draw headers
    draw_header(&mut combined, 0, rw, "REFERENCE");
    draw_header(&mut combined, rw + separator, nw, "RENDERED");

    // Draw red separator line
    let sep_color = Rgba([220, 50, 50, 255]);
    for y in 0..total_h {
        for dx in 0..separator {
            combined.put_pixel(rw + dx, y, sep_color);
        }
    }

    // Copy reference image
    let ref_rgba = reference.to_rgba8();
    for y in 0..rh {
        for x in 0..rw {
            combined.put_pixel(x, y + header_h, *ref_rgba.get_pixel(x, y));
        }
    }

    // Copy rendered image
    let new_rgba = rendered.to_rgba8();
    for y in 0..nh {
        for x in 0..nw {
            combined.put_pixel(rw + separator + x, y + header_h, *new_rgba.get_pixel(x, y));
        }
    }

    combined
}

fn main() {
    let testdata = Path::new("testdata");
    let outdir = testdata.join("diffs");
    std::fs::create_dir_all(&outdir).unwrap();

    let renderer = Renderer::new();
    let opts = default_options();
    let mut ok = 0usize;
    let mut skip = 0usize;
    let mut err = 0usize;

    // Collect all label files (.zpl and .epl)
    let mut label_files: Vec<_> = std::fs::read_dir(testdata)
        .unwrap()
        .flatten()
        .filter(|e| {
            let ext = e.path().extension().map(|x| x.to_string_lossy().to_string());
            matches!(ext.as_deref(), Some("zpl") | Some("epl"))
        })
        .map(|e| e.path())
        .collect();
    label_files.sort();

    for path in &label_files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let ext = path.extension().unwrap().to_string_lossy().to_string();

        // Check for a matching reference PNG
        let ref_png = testdata.join(format!("{}.png", name));
        if !ref_png.exists() {
            println!("SKIP {}.{} (no reference PNG)", name, ext);
            skip += 1;
            continue;
        }

        // Parse the label
        let content = std::fs::read(path).unwrap();
        let labels = match ext.as_str() {
            "epl" => EplParser::new().parse(&content),
            _ => {
                let mut p = ZplParser::new();
                p.parse(&content)
            }
        };

        let labels = match labels {
            Ok(l) if !l.is_empty() => l,
            _ => {
                println!("ERR  {}.{} (parse failed)", name, ext);
                err += 1;
                continue;
            }
        };

        // Render
        let mut buf = Cursor::new(Vec::new());
        if renderer
            .draw_label_as_png(&labels[0], &mut buf, opts.clone())
            .is_err()
        {
            println!("ERR  {}.{} (render failed)", name, ext);
            err += 1;
            continue;
        }

        let rendered_bytes = buf.into_inner();
        let rendered_img = match image::load_from_memory(&rendered_bytes) {
            Ok(img) => img,
            Err(_) => {
                println!("ERR  {}.{} (decode rendered)", name, ext);
                err += 1;
                continue;
            }
        };

        // Load reference
        let ref_img = match image::open(&ref_png) {
            Ok(img) => img,
            Err(_) => {
                println!("ERR  {}.{} (load reference PNG)", name, ext);
                err += 1;
                continue;
            }
        };

        // Combine and save
        let combined = combine_side_by_side(&ref_img, &rendered_img);
        let out_path = outdir.join(format!("{}.png", name));
        combined.save(&out_path).unwrap();
        println!("OK   {}.{} -> {}", name, ext, out_path.display());
        ok += 1;
    }

    println!(
        "\nDone: {} compared, {} skipped, {} errors → {}",
        ok,
        skip,
        err,
        outdir.display()
    );
}
