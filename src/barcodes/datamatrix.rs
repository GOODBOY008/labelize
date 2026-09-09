use datamatrix::{DataMatrix, SymbolList};
use image::{Rgba, RgbaImage};

use crate::elements::barcode_datamatrix::DatamatrixRatio;

/// Generate a Data Matrix barcode image using a proper ECC 200 encoder.
///
/// Non-zero `rows` and `columns` constrain the size in modules. When both
/// are specified, they determine the shape; otherwise selection is square.
/// Use [`encode_with_ratio`] to explicitly select square or rectangular symbols.
pub fn encode(
    content: &str,
    magnification: i32,
    rows: i32,
    columns: i32,
) -> Result<RgbaImage, String> {
    encode_with_ratio(content, magnification, rows, columns, None)
}

/// Generate an ECC 200 symbol constrained by the ^BX dimensions and aspect ratio.
///
/// Zero dimensions are automatic; each positive dimension is an exact constraint.
/// An explicit ratio constrains the shape even when dimensions are supplied.
/// With no ratio, two fixed dimensions determine the shape; otherwise the default
/// is square. Only standard ECC 200 sizes are used (no DMRE extensions).
/// Negative dimensions, incompatible size/shape requests, and content that does
/// not fit return an error. No larger or differently shaped symbol is substituted.
/// The renderer propagates these errors to its caller.
///
/// See the [Zebra ^BX reference](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html).
pub fn encode_with_ratio(
    content: &str,
    magnification: i32,
    rows: i32,
    columns: i32,
    ratio: Option<DatamatrixRatio>,
) -> Result<RgbaImage, String> {
    if content.is_empty() {
        return Err("DataMatrix: empty content".to_string());
    }
    if rows < 0 || columns < 0 {
        return Err("DataMatrix: rows and columns must be non-negative".to_string());
    }

    let mag = magnification.max(1) as u32;

    let mut symbol_list = match ratio {
        Some(DatamatrixRatio::Square) => SymbolList::default().enforce_square(),
        Some(DatamatrixRatio::Rectangular) => SymbolList::default().enforce_rectangular(),
        None if rows > 0 && columns > 0 => SymbolList::default(),
        None => SymbolList::default().enforce_square(),
    };
    if rows > 0 {
        symbol_list = symbol_list.enforce_height_in(rows as usize..=rows as usize);
    }
    if columns > 0 {
        symbol_list = symbol_list.enforce_width_in(columns as usize..=columns as usize);
    }
    if symbol_list.is_empty() {
        return Err(format!(
            "DataMatrix: unsupported dimensions/ratio (rows={rows}, columns={columns}, ratio={ratio:?})"
        ));
    }

    let code = DataMatrix::encode(content.as_bytes(), symbol_list.clone())
        .or_else(|error| {
            // The encoder's multi-size planner can reject compressible content
            // when none of the candidates can hold its uncompressed bytes.
            // Retry individual allowed sizes; never relax the ^BX constraints.
            symbol_list
                .iter()
                .find_map(|size| DataMatrix::encode(content.as_bytes(), size).ok())
                .ok_or(error)
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
