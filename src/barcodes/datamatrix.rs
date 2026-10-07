use super::datamatrix_field::{self, Token};
use crate::elements::barcode_datamatrix::DatamatrixRatio;
use datamatrix::placement::{Bitmap, MatrixMap};
use datamatrix::{DataMatrix, DataMatrixBuilder, EncodationType, SymbolList};
use image::{Rgba, RgbaImage};

enum EncodeError {
    Invalid(String),
    Capacity(datamatrix::data::DataEncodingError),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Capacity(error) => write!(f, "DataMatrix encoding failed: {error:?}"),
        }
    }
}

/// Generate a Data Matrix barcode image using a proper ECC 200 encoder.
///
/// Positive dimensions constrain the symbol in modules. Two fixed dimensions
/// determine its shape; otherwise the default is square. No out-of-size fallback.
///
/// Empty content encodes naturally to the smallest square symbol (10×10,
/// 54 dark modules). The label renderer skips empty fields instead of drawing
/// this raw encoder result.
pub fn encode(
    content: &str,
    magnification: i32,
    rows: i32,
    columns: i32,
) -> Result<RgbaImage, String> {
    encode_with_ratio(content, magnification, rows, columns, None)
}

/// Encode with explicit square/rectangular shape and ^BX dimension constraints.
/// With fixed rows and automatic columns, select the smallest matching standard
/// symbol before encoding; do not widen it to accommodate longer input.
/// Invalid constraints or content that does not fit return an error.
pub fn encode_with_ratio(
    content: &str,
    magnification: i32,
    rows: i32,
    columns: i32,
    ratio: Option<DatamatrixRatio>,
) -> Result<RgbaImage, String> {
    encode_bytes(content.as_bytes(), magnification, rows, columns, ratio)
        .map_err(|error| error.to_string())
}

fn symbol_list(
    rows: i32,
    columns: i32,
    ratio: Option<DatamatrixRatio>,
) -> Result<SymbolList, EncodeError> {
    if rows < 0 || columns < 0 {
        return Err(EncodeError::Invalid(
            "DataMatrix: rows and columns must be non-negative".into(),
        ));
    }
    let mut symbols = match ratio {
        Some(DatamatrixRatio::Square) => SymbolList::default().enforce_square(),
        Some(DatamatrixRatio::Rectangular) => SymbolList::default().enforce_rectangular(),
        None if rows > 0 && columns > 0 => SymbolList::default(),
        None => SymbolList::default().enforce_square(),
    };
    if rows > 0 {
        symbols = symbols.enforce_height_in(rows as usize..=rows as usize);
    }
    if columns > 0 {
        symbols = symbols.enforce_width_in(columns as usize..=columns as usize);
    }
    if symbols.is_empty() {
        return Err(EncodeError::Invalid(format!("DataMatrix: unsupported dimensions/ratio (rows={rows}, columns={columns}, ratio={ratio:?})")));
    }
    if rows > 0 && columns == 0 {
        // ZD421: c0/r12/a2 with ABC matches c26/r12; the D04 payload is
        // suppressed, although explicit c36/r12 prints. Labelary agrees.
        // The automatic column choice must precede payload fitting.
        let first = symbols.iter().next().expect("nonempty symbol list");
        symbols = first.into();
    }
    Ok(symbols)
}

fn encode_bytes(
    content: &[u8],
    magnification: i32,
    rows: i32,
    columns: i32,
    ratio: Option<DatamatrixRatio>,
) -> Result<RgbaImage, EncodeError> {
    let mag = magnification.max(1) as u32;

    // Build a symbol list: if rows/columns are specified, try to match
    // a specific size. Otherwise default to square-only (ZPL standard).
    let symbols = symbol_list(rows, columns, ratio)?;
    let code = DataMatrix::encode(content, symbols.clone())
        .or_else(|error| {
            // Retry only allowed sizes when the multi-size planner cannot fit
            // compressible bytes. A row-only request already has one candidate.
            symbols
                .iter()
                .find_map(|size| DataMatrix::encode(content, size).ok())
                .ok_or(error)
        })
        .map_err(EncodeError::Capacity)?;

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
    encode_zpl_with_ratio(content, magnification, rows, columns, escape, None)
}

/// Encode ZPL escapes with the same size/shape constraints as literal bytes.
pub fn encode_zpl_with_ratio(
    content: &[u8],
    magnification: i32,
    rows: i32,
    columns: i32,
    escape: u8,
    ratio: Option<DatamatrixRatio>,
) -> Result<RgbaImage, String> {
    encode_zpl_inner(content, magnification, rows, columns, escape, ratio)
        .map_err(|error| error.to_string())
}

/// Renderer policy for the observed row-only rectangular ^BX rejection.
/// Keep malformed escapes and invalid constraints as errors; only a capacity
/// failure in this selection mode suppresses the barcode field. ^CV is separate.
pub(crate) fn render_zpl_field(
    content: &[u8],
    magnification: i32,
    rows: i32,
    columns: i32,
    escape: u8,
    ratio: Option<DatamatrixRatio>,
) -> Result<Option<RgbaImage>, String> {
    match encode_zpl_inner(content, magnification, rows, columns, escape, ratio) {
        Ok(image) => Ok(Some(image)),
        Err(EncodeError::Capacity(datamatrix::data::DataEncodingError::TooMuchOrIllegalData))
            if rows > 0 && columns == 0 && ratio == Some(DatamatrixRatio::Rectangular) =>
        {
            Ok(None)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn encode_zpl_inner(
    content: &[u8],
    magnification: i32,
    rows: i32,
    columns: i32,
    escape: u8,
    ratio: Option<DatamatrixRatio>,
) -> Result<RgbaImage, EncodeError> {
    if escape == 0 {
        return encode_bytes(content, magnification, rows, columns, ratio);
    }
    let tokens = datamatrix_field::parse(content, escape).map_err(EncodeError::Invalid)?;
    if tokens.iter().all(|token| matches!(token, Token::Byte(_))) {
        let bytes: Vec<u8> = tokens
            .iter()
            .filter_map(|token| match token {
                Token::Byte(byte) => Some(*byte),
                _ => None,
            })
            .collect();
        return encode_bytes(&bytes, magnification, rows, columns, ratio);
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
    let symbols = symbol_list(rows, columns, ratio)?;
    let selected = select(symbols).map_err(EncodeError::Capacity)?;
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
