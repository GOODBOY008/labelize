use image::{Rgba, RgbaImage};

use super::pdf417_encoding::build_symbol;

/// Resolve the effective row height for PDF417 rendering.
fn resolve_row_height(b7_row_height: i32, by_height: i32, num_rows: usize) -> u32 {
    if b7_row_height > 0 {
        b7_row_height as u32
    } else {
        (by_height as u32 / num_rows.max(1) as u32).max(1)
    }
}

/// Generate a PDF417 barcode image (ISO/IEC 15438) with cost-optimal
/// Text/Byte/Numeric compaction, matching the reference renderer's segment
/// choices. Returns an image at 1-pixel module width; the renderer scales by
/// module_width.
pub fn encode(
    content: &str,
    row_height: i32,
    security_level: i32,
    column_count: i32,
    row_count: i32,
    truncated: bool,
    by_height: i32,
) -> Result<RgbaImage, String> {
    let symbol = build_symbol(
        content.as_bytes(),
        security_level,
        column_count,
        row_count,
        truncated,
    )?;

    let num_rows = symbol.rows.len();
    let row_h = resolve_row_height(row_height, by_height, num_rows) as usize;
    let width = symbol.rows.first().map(|r| r.len()).unwrap_or(0);
    let height = num_rows * row_h;

    let mut img = RgbaImage::from_pixel(width as u32, height as u32, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);
    for (r, modules) in symbol.rows.iter().enumerate() {
        let dst_y = r * row_h;
        for (x, &on) in modules.iter().enumerate() {
            if on {
                for dy in 0..row_h {
                    img.put_pixel(x as u32, (dst_y + dy) as u32, black);
                }
            }
        }
    }

    Ok(img)
}
