use datamatrix::{DataMatrix, SymbolList};
use image::{Rgba, RgbaImage};

/// Generate a Data Matrix barcode image using a proper ECC 200 encoder.
///
/// `rows` and `columns` from ^BX are used to select the symbol size when
/// both are non-zero. Otherwise, the smallest square symbol that fits the
/// data is chosen (ZPL ^BX defaults to square symbols per the Zebra spec).
pub fn encode(
    content: &str,
    magnification: i32,
    rows: i32,
    columns: i32,
) -> Result<RgbaImage, String> {
    if content.is_empty() {
        return Err("DataMatrix: empty content".to_string());
    }

    let mag = magnification.max(1) as u32;

    // Build a symbol list: if rows/columns are specified, try to match
    // a specific size. Otherwise default to square-only (ZPL standard).
    let symbol_list = if rows > 0 && columns > 0 {
        // Try to find a matching symbol size — fall back to square-only
        SymbolList::default().enforce_height_in(rows as usize..=rows as usize)
    } else {
        SymbolList::default().enforce_square()
    };

    let code = DataMatrix::encode(content.as_bytes(), symbol_list)
        .or_else(|_| {
            // If the specific size didn't work, fall back to square-only
            DataMatrix::encode(content.as_bytes(), SymbolList::default().enforce_square())
        })
        .map_err(|e| format!("DataMatrix encoding failed: {:?}", e))?;

    let bitmap = code.bitmap();
    let bm_width = bitmap.width() as u32;
    let bm_height = bitmap.height() as u32;

    // Render to image (no quiet zone — Labelary omits it)
    let img_width = bm_width * mag;
    let img_height = bm_height * mag;
    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);

    // pixels() yields (x, y) for each dark module
    for (col, row) in bitmap.pixels() {
        let px = col as u32 * mag;
        let py = row as u32 * mag;
        for dy in 0..mag {
            for dx in 0..mag {
                if px + dx < img_width && py + dy < img_height {
                    img.put_pixel(px + dx, py + dy, black);
                }
            }
        }
    }

    Ok(img)
}
