use image::{Rgba, RgbaImage};
use rxing::aztec::AztecWriter;
use rxing::{BarcodeFormat, EncodeHintValue, EncodeHints, Writer};

/// Generate an Aztec barcode image using rxing's proper encoder.
///
/// The `ec_symbol_size` parameter follows ZPL ^BO spec:
///   0       = default error correction
///   1-99    = error correction percentage
///   101-104 = compact Aztec with 1-4 layers
///   201-232 = full-range Aztec with 1-32 layers
///   300     = Aztec Rune (not yet supported, falls back to default)
pub fn encode(content: &str, magnification: i32, ec_symbol_size: i32) -> Result<RgbaImage, String> {
    if content.is_empty() {
        return Ok(RgbaImage::from_pixel(0, 0, Rgba([0, 0, 0, 0])));
    }

    let mag = magnification.max(1) as u32;

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
