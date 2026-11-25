use anyd::output::BitMatrix;
use image::{Rgba, RgbaImage};
use rxing::aztec::encoder::aztec_encoder;

/// Module map of the symbol Labelary draws for an empty ^BO field: the bare
/// 11×11 Aztec bullseye core (`#` = dark). Measured from Labelary renders
/// (8 dpmm, 4.005×8.01) — the pattern is fixed: it does not vary with the
/// magnification or the `d` error-correction parameter (checked at d = 0,
/// 101 and 201, mag 2 and 5).
const EMPTY_CORE: [&str; 11] = [
    "##..#####.#",
    "###########",
    "##.......##",
    ".#.#####.#.",
    ".#.#...#.##",
    ".#.#.#.#.##",
    "##.#...#.##",
    ".#.#####.##",
    ".#.......##",
    ".##########",
    "........#..",
];

/// Labelary places the 11×11 empty-data core centered on a 15×15 canvas,
/// i.e. behind a 2-module transparent margin on every side.
const EMPTY_MARGIN_MODULES: usize = 2;

/// Render the empty-data symbol: a 15×15 module canvas (transparent) with
/// the fixed 11×11 core at offset (2, 2), scaled by `mag` pixels per module.
fn empty_core_image(mag: u32) -> RgbaImage {
    let module = |v: usize| v as u32 * mag;
    let size = module(EMPTY_MARGIN_MODULES * 2 + EMPTY_CORE.len());
    let mut img = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);
    for (row, line) in EMPTY_CORE.iter().enumerate() {
        for (col, cell) in line.bytes().enumerate() {
            if cell == b'#' {
                let (px, py) = (
                    module(EMPTY_MARGIN_MODULES + col),
                    module(EMPTY_MARGIN_MODULES + row),
                );
                for dy in 0..mag {
                    for dx in 0..mag {
                        img.put_pixel(px + dx, py + dy, black);
                    }
                }
            }
        }
    }
    img
}

// Layer selectors use negative numbers for compact symbols in rxing.
// Error correction is a percentage of all words in the command, and a
// percentage of data bits in rxing.
fn parameters(size: i32) -> (u32, i32) {
    let ec = match size {
        1..=99 => (100 * size as u32).div_ceil(100 - size as u32),
        _ => aztec_encoder::DEFAULT_EC_PERCENT,
    };
    let layers = match size {
        101..=104 => 100 - size,
        201..=232 => size - 200,
        _ => 0,
    };
    (ec, layers)
}

/// Logical modules for byte-oriented Aztec symbols, including Runes.
pub(super) fn modules(data: &[u8], size: i32, inverted: bool) -> Result<BitMatrix, String> {
    fn copy(side: usize, dot: impl Fn(usize, usize) -> bool, inverted: bool) -> BitMatrix {
        let mut matrix = BitMatrix::new(side, side, 0);
        for y in 0..side {
            for x in 0..side {
                matrix.set(x, y, dot(x, y) != inverted);
            }
        }
        matrix
    }

    if size == 300 {
        let value = std::str::from_utf8(data).map_err(|_| "Aztec Rune requires decimal digits")?;
        let bwipp::Encoded::Matrix(matrix) = bwipp::Symbology::AztecRune
            .encode(value, &bwipp::Options::default())
            .map_err(|e| format!("Aztec Rune encoding failed: {e}"))?
        else {
            unreachable!("Aztec Rune is a matrix barcode")
        };
        return Ok(copy(matrix.width(), |x, y| matrix.get(x, y), inverted));
    }

    let (ec, layers) = parameters(size);
    let code = aztec_encoder::encode_bytes(data, ec, layers)
        .map_err(|e| format!("Aztec encoding failed: {e}"))?;
    let matrix = code.getMatrix();
    Ok(copy(
        matrix.getWidth() as usize,
        |x, y| matrix.get(x as u32, y as u32),
        inverted,
    ))
}

/// Generate an Aztec barcode image using rxing's proper encoder.
///
/// The `ec_symbol_size` parameter follows ZPL ^BO spec:
///   0       = default error correction
///   1-99    = error correction percentage
///   101-104 = compact Aztec with 1-4 layers
///   201-232 = full-range Aztec with 1-32 layers
///   300     = Aztec Rune (not yet supported, falls back to default)
///
/// Empty content renders the same fixed 11×11-module core as Labelary.
pub fn encode(content: &str, magnification: i32, ec_symbol_size: i32) -> Result<RgbaImage, String> {
    let mag = magnification.max(1) as u32;

    if content.is_empty() {
        return Ok(empty_core_image(mag));
    }

    let (ec, layers) = parameters(ec_symbol_size);
    let code = aztec_encoder::encode(content, ec, layers)
        .map_err(|e| format!("Aztec encoding failed: {e}"))?;
    let bit_matrix = code.getMatrix();
    let bm_width = bit_matrix.getWidth();
    let bm_height = bit_matrix.getHeight();

    // Render at magnification
    let img_width = bm_width * mag;
    let img_height = bm_height * mag;

    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([0, 0, 0, 0]));
    let black = Rgba([0, 0, 0, 255]);

    for y in 0..bm_height {
        for x in 0..bm_width {
            if bit_matrix.get(x, y) {
                let px = x * mag;
                let py = y * mag;
                for dy in 0..mag {
                    for dx in 0..mag {
                        img.put_pixel(px + dx, py + dy, black);
                    }
                }
            }
        }
    }

    Ok(img)
}
