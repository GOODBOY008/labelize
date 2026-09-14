use image::{Rgba, RgbaImage};
use rxing::aztec::AztecWriter;
use rxing::{BarcodeFormat, EncodeHintValue, EncodeHints, Writer};

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

/// Generate an Aztec barcode image using rxing's proper encoder.
///
/// The `ec_symbol_size` parameter follows ZPL ^BO spec:
///   0       = default error correction
///   1-99    = error correction percentage
///   101-104 = compact Aztec with 1-4 layers
///   201-232 = full-range Aztec with 1-32 layers
///   300     = Aztec Rune (not yet supported, falls back to default)
///
/// Empty content has no rxing representation; it gets the fixed minimal
/// symbol Labelary draws (see [`EMPTY_CORE`]) instead of an error.
pub fn encode(content: &str, magnification: i32, ec_symbol_size: i32) -> Result<RgbaImage, String> {
    let mag = magnification.max(1) as u32;

    if content.is_empty() {
        return Ok(empty_core_image(mag));
    }

    // Use rxing Aztec writer to get proper bit matrix
    let writer = AztecWriter;
    let mut hints = EncodeHints::default();
    hints = hints.with(EncodeHintValue::Margin("0".to_string()));

    match ec_symbol_size {
        // Compact Aztec: 101-104 -> AztecLayers(-1 to -4)
        101..=104 => {
            let layers = -(ec_symbol_size - 100);
            hints = hints.with(EncodeHintValue::AztecLayers(layers));
        }
        // Full-range Aztec: 201-232 -> AztecLayers(1 to 32)
        201..=232 => {
            let layers = ec_symbol_size - 200;
            hints = hints.with(EncodeHintValue::AztecLayers(layers));
        }
        // EC percentage: 1-99
        // ZPL ^BO d parameter defines EC% as a percentage of TOTAL codewords,
        // i.e. ecc_codewords / total_codewords = ec_percent / 100.
        // rxing's ErrorCorrection hint instead uses EC% of DATA bits:
        //   ecc_bits = data_bits * ec_percent / 100 + 11
        // Conversion: adjusted = ceil(ec_percent * 100 / (100 - ec_percent))
        // This ensures rxing picks the same minimum symbol layer as Zebra firmware.
        1..=99 => {
            let adjusted_ec =
                (ec_symbol_size * 100 + (100 - ec_symbol_size) - 1) / (100 - ec_symbol_size);
            hints = hints.with(EncodeHintValue::ErrorCorrection(adjusted_ec.to_string()));
        }
        // 0, 300, or other: use rxing defaults
        _ => {}
    }

    // Encode with minimum size, we'll apply magnification ourselves
    let bit_matrix = writer
        .encode_with_hints(content, &BarcodeFormat::AZTEC, 0, 0, &hints)
        .map_err(|e| format!("Aztec encoding failed: {}", e))?;

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
