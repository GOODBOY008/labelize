use image::{Rgba, RgbaImage};
use rxing::pdf417::encoder::Dimensions;
use rxing::{BarcodeFormat, EncodeHintValue, EncodeHints, MultiFormatWriter, Writer};

/// Resolve the effective row height for PDF417 rendering.
fn resolve_row_height(b7_row_height: i32, by_height: i32, num_rows: u32) -> u32 {
    if b7_row_height > 0 {
        b7_row_height as u32
    } else {
        (by_height as u32 / num_rows.max(1)).max(1)
    }
}

/// Generate a PDF417 barcode image using rxing's MultiFormatWriter.
/// Returns an image at 1-pixel module width; the renderer scales by module_width.
pub fn encode(
    content: &str,
    row_height: i32,
    security_level: i32,
    column_count: i32,
    row_count: i32,
    truncated: bool,
    by_height: i32,
) -> Result<RgbaImage, String> {
    if content.is_empty() {
        return Err("PDF417: empty content".to_string());
    }

    let mut hints = EncodeHints::default();
    hints = hints.with(EncodeHintValue::Margin("0".to_string()));

    // Security level (0 = auto, 1-8 = explicit)
    if security_level > 0 && security_level <= 8 {
        hints = hints.with(EncodeHintValue::ErrorCorrection(security_level.to_string()));
    }

    // Truncated PDF417
    if truncated {
        hints = hints.with(EncodeHintValue::Pdf417Compact("true".to_string()));
    }

    // Column/row constraints via Pdf417Dimensions
    let min_cols = if column_count > 0 {
        column_count.clamp(1, 30) as usize
    } else {
        1
    };
    let max_cols = if column_count > 0 {
        column_count.clamp(1, 30) as usize
    } else {
        30
    };
    let min_rows = if row_count > 0 {
        row_count.clamp(3, 90) as usize
    } else {
        3
    };
    let max_rows = if row_count > 0 {
        row_count.clamp(3, 90) as usize
    } else {
        90
    };

    // Validate 928-codeword cap before calling rxing (it panics on overflow)
    if min_cols * min_rows > 928 {
        return Err(format!(
            "PDF417: cols({}) × rows({}) = {} exceeds 928 codeword limit",
            min_cols,
            min_rows,
            min_cols * min_rows
        ));
    }

    hints = hints.with(EncodeHintValue::Pdf417Dimensions(Dimensions::new(
        min_cols, max_cols, min_rows, max_rows,
    )));

    // Encode via rxing
    let writer = MultiFormatWriter::default();
    let bit_matrix = writer
        .encode_with_hints(content, &BarcodeFormat::PDF_417, 0, 0, &hints)
        .map_err(|e| format!("PDF417 encoding failed: {}", e))?;

    let bm_width = bit_matrix.getWidth();
    let bm_height = bit_matrix.getHeight();

    // rxing applies aspectRatio=4 internally, so bm_height = num_symbol_rows * 4
    let num_symbol_rows = (bm_height / 4).max(1);
    let scale_y = resolve_row_height(row_height, by_height, num_symbol_rows);

    let img_width = bm_width;
    let img_height = num_symbol_rows * scale_y;

    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);

    // Sample the first pixel of each 4-pixel row group from the BitMatrix
    for row in 0..num_symbol_rows {
        let src_y = row * 4; // first pixel of this symbol row in the BitMatrix
        let dst_y = row * scale_y;
        for x in 0..bm_width {
            if bit_matrix.get(x, src_y) {
                for dy in 0..scale_y {
                    img.put_pixel(x, dst_y + dy, black);
                }
            }
        }
    }

    Ok(img)
}
