use image::{Rgba, RgbaImage};
use pdf417::{PDF417Encoder, PDF417};

/// Calculate the number of codewords needed for byte encoding.
fn byte_encoding_codewords(data_len: usize) -> usize {
    let packed_groups = data_len / 6;
    let remaining = data_len % 6;
    1 + 1 + packed_groups * 5 + remaining
}

/// Compute default (cols, rows) enforcing ~1:2 aspect ratio when both are unspecified.
fn compute_default_dimensions(total_cws_needed: usize) -> (u8, u8) {
    for cols in 1u8..=30 {
        let min_rows = total_cws_needed.div_ceil(cols as usize).max(3);
        let target_rows = (cols as usize) * 2;
        let rows = min_rows.max(target_rows).min(90);
        if (cols as usize) * rows <= 928 && rows >= min_rows {
            return (cols, rows as u8);
        }
    }
    (30, (total_cws_needed.div_ceil(30).max(3).min(90)) as u8)
}

/// Resolve row height: ^B7 h overrides, otherwise ^BY height / num_rows.
fn resolve_row_height(b7_row_height: i32, by_height: i32, num_rows: u8) -> u32 {
    if b7_row_height > 0 {
        b7_row_height as u32
    } else {
        (by_height as u32 / num_rows as u32).max(1)
    }
}

/// Generate a PDF417 barcode image at 1-pixel module width.
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

    let data_bytes = content.as_bytes();
    let data_cws = byte_encoding_codewords(data_bytes.len());

    // Estimate security level for dimension calculation
    let sec_level_estimate = if security_level > 0 && security_level <= 8 {
        security_level as u8
    } else {
        2
    };
    let ecc_cws = pdf417::ecc::ecc_count(sec_level_estimate);
    let total_cws_needed = data_cws + ecc_cws;

    // Determine columns and rows
    let (cols, rows) = if column_count <= 0 && row_count <= 0 {
        // Both unspecified: use 1:2 aspect ratio
        compute_default_dimensions(total_cws_needed)
    } else {
        let cols = if column_count > 0 {
            column_count.clamp(1, 30) as u8
        } else {
            // Auto columns from rows
            let r = row_count.clamp(3, 90) as usize;
            total_cws_needed.div_ceil(r).clamp(1, 30) as u8
        };
        let min_rows = total_cws_needed.div_ceil(cols as usize).max(3) as u8;
        let rows = if row_count > 0 {
            (row_count as u8).max(min_rows).min(90)
        } else {
            min_rows.min(90)
        };
        (cols, rows)
    };

    // Validate 928-codeword cap
    let capacity = (cols as usize) * (rows as usize);
    if capacity > 928 {
        return Err(format!(
            "PDF417: cols({}) × rows({}) = {} exceeds 928 codeword limit",
            cols, rows, capacity
        ));
    }

    let mut codewords = vec![0u16; capacity];
    let encoder = PDF417Encoder::new(&mut codewords, false).append_bytes(data_bytes);

    let (level, sealed) = if security_level > 0 && security_level <= 8 {
        let level = security_level as u8;
        let s = encoder.seal(level);
        (level, s)
    } else {
        encoder
            .fit_seal()
            .ok_or_else(|| "PDF417: data too large for configuration".to_string())?
    };

    let scale_y = resolve_row_height(row_height, by_height, rows);

    encode_to_image(sealed, rows, cols, level, truncated, scale_y)
}

fn encode_to_image(
    codewords: &[u16],
    rows: u8,
    cols: u8,
    level: u8,
    truncated: bool,
    scale_y: u32,
) -> Result<RgbaImage, String> {
    let img_width = if truncated {
        (pdf417::START_PATTERN_LEN as usize + 17 + cols as usize * 17 + 1) as u32
    } else {
        (pdf417::START_PATTERN_LEN as usize
            + 17
            + cols as usize * 17
            + 17
            + pdf417::END_PATTERN_LEN as usize) as u32
    };
    let img_height = rows as u32 * scale_y;

    let buf_size = img_width as usize * img_height as usize;
    let mut storage = vec![false; buf_size];
    let pdf = PDF417::new(codewords, rows, cols, level)
        .truncated(truncated)
        .scaled((1, scale_y));
    pdf.render(&mut storage[..]);

    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);

    for (idx, &is_dark) in storage.iter().enumerate() {
        if is_dark {
            let x = (idx as u32) % img_width;
            let y = (idx as u32) / img_width;
            if x < img_width && y < img_height {
                img.put_pixel(x, y, black);
            }
        }
    }

    Ok(img)
}
