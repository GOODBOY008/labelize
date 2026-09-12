use super::datamatrix_field::{self, Token};
use datamatrix::placement::{Bitmap, MatrixMap};
use datamatrix::{DataMatrix, DataMatrixBuilder, EncodationType, SymbolList};
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
    encode_bytes(content.as_bytes(), magnification, rows, columns)
}

fn encode_bytes(
    content: &[u8],
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

    let code = DataMatrix::encode(content, symbol_list)
        .or_else(|_| {
            // If the specific size didn't work, fall back to square-only
            DataMatrix::encode(content, SymbolList::default().enforce_square())
        })
        .map_err(|e| format!("DataMatrix encoding failed: {:?}", e))?;

    Ok(render_bitmap(&code.bitmap(), mag))
}

fn render_bitmap(bitmap: &Bitmap<bool>, mag: u32) -> RgbaImage {
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

    img
}

/// Encode ZPL ECC 200 escapes after field-hex processing. The raw `encode`
/// entry point remains available for EPL and callers supplying literal text.
pub fn encode_zpl(
    content: &[u8],
    magnification: i32,
    rows: i32,
    columns: i32,
    escape: u8,
) -> Result<RgbaImage, String> {
    let tokens = datamatrix_field::parse(content, escape)?;
    if tokens.iter().all(|token| matches!(token, Token::Byte(_))) {
        let bytes: Vec<u8> = tokens
            .iter()
            .filter_map(|token| match token {
                Token::Byte(byte) => Some(*byte),
                _ => None,
            })
            .collect();
        return encode_bytes(&bytes, magnification, rows, columns);
    }
    let mut words = datamatrix_field::ascii_codewords(&tokens);
    // Ask the existing encoder to select a size and expose its padded capacity.
    // ASCII 'A' consumes exactly one codeword; no private capacity table is copied.
    let placeholders = vec![b'A'; words.len()];
    let select = |symbols| {
        DataMatrixBuilder::new()
            .with_encodation_types(EncodationType::Ascii)
            .with_macros(false)
            .with_symbol_list(symbols)
            .encode(&placeholders)
    };
    // Match the existing encoder's sizing policy; dimension handling is separate.
    let symbols = if rows > 0 && columns > 0 {
        SymbolList::default().enforce_height_in(rows as usize..=rows as usize)
    } else {
        SymbolList::default().enforce_square()
    };
    let selected = select(symbols)
        .or_else(|_| select(SymbolList::default().enforce_square()))
        .map_err(|error| format!("DataMatrix encoding failed: {error:?}"))?;
    let capacity = selected.data_codewords().len();
    // An explicit terminal PAD already starts the padding sequence. Inspect
    // its token, not just the last codeword (ECI/append parameters can be 129).
    let terminal_pad = matches!(
        tokens.last(),
        Some(Token::Codewords(codewords)) if codewords.as_slice() == [129]
    );
    if words.len() < capacity && !terminal_pad {
        words.push(129);
    }
    while words.len() < capacity {
        let randomized = 129 + (149 * (words.len() + 1) % 253) + 1;
        words.push(if randomized > 254 {
            randomized - 254
        } else {
            randomized
        } as u8);
    }
    let ecc = datamatrix::errorcode::encode_error(&words, selected.size);
    words.extend_from_slice(&ecc);
    let bitmap = MatrixMap::new_with_codewords(&words, selected.size).bitmap();
    Ok(render_bitmap(&bitmap, magnification.max(1) as u32))
}
