//! Reverse rendering: raster images → printer-language source.
//!
//! The rest of this crate runs forward — label source in, pixels out. This
//! module is the inverse: it takes an already-rendered raster (a logo, a
//! scanned label, or a PNG produced by the forward pipeline) and emits the
//! printer commands that draw it. The image is binarized — a pixel whose
//! luminance composited over white is below [`ReverseOptions::threshold`]
//! becomes ink — and embedded 1:1, one dot per pixel at
//! [`ReverseOptions::dpmm`]:
//!
//! - ZPL: `^FO0,0^GFA,…` — hex-packed graphic field, set bits print black,
//!   one output line per bitmap row (line breaks inside `^GF` data are
//!   ignored by printers and by this crate's ZPL parser alike).
//! - TSPL: `SIZE` + `BITMAP` — raw row-packed payload in TSC polarity
//!   (cleared bits print black), so the payload is the bitwise complement
//!   of the ZPL packing. `SIZE` carries the image's physical size, making
//!   TSPL the only one of the two that declares its own canvas.
//!
//! Both encodings round-trip through the forward pipeline pixel-identically;
//! `tests/e2e_reverse.rs` pins that against real label fixtures.

use crate::error::LabelizeError;
use image::{ImageDecoder, ImageReader, RgbaImage};
use std::io::Cursor;

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";
// Zebra Programming Guide, ^GF: b/c/d are each limited to 1..=99999.
const MAX_GRAPHIC_BYTES: usize = 99_999;
// Bound decoded rasters before allocating RGBA or printer payloads. The
// dimension cap also keeps every tiled ^FO coordinate within its 32000 limit.
const MAX_DIMENSION: u32 = 32_000;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = 128 * 1024 * 1024;

/// Printer languages supported by reverse rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseLanguage {
    Zpl,
    Tspl,
}

impl ReverseLanguage {
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Zpl => "zpl",
            Self::Tspl => "tspl",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Zpl => "text/plain; charset=utf-8",
            Self::Tspl => "application/octet-stream",
        }
    }
}

impl std::str::FromStr for ReverseLanguage {
    type Err = LabelizeError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.to_ascii_lowercase().as_str() {
            "zpl" => Ok(Self::Zpl),
            "tspl" => Ok(Self::Tspl),
            _ => Err(LabelizeError::Encode(format!(
                "unknown language {name:?} — expected zpl or tspl"
            ))),
        }
    }
}

/// Settings for image → printer-code conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReverseOptions {
    /// Dots per millimetre the emitted code assumes; one image pixel = one
    /// dot. 8 matches the most common thermal printhead and the render
    /// pipeline's default.
    pub dpmm: i32,
    /// Luminance threshold (0–255): pixels whose composite-over-white
    /// luminance is strictly below this print as ink. 128 puts the boundary
    /// between grey 127 (ink) and grey 128 (paper).
    pub threshold: u8,
}

impl Default for ReverseOptions {
    fn default() -> Self {
        ReverseOptions {
            dpmm: 8,
            threshold: 128,
        }
    }
}

impl ReverseOptions {
    fn validate(&self) -> Result<(), LabelizeError> {
        if self.dpmm < 1 {
            return Err(LabelizeError::Encode(format!(
                "dpmm must be positive, got {}",
                self.dpmm
            )));
        }
        Ok(())
    }
}

/// Decode a raster and encode it for the selected printer language.
///
/// Inputs are limited to 16 Mi pixels, 32000 dots per dimension and 128 MiB
/// of decoded data. Dimensions are checked before decoding pixels; invalid input
/// return [`LabelizeError::Encode`].
pub fn encode_image_bytes(
    bytes: &[u8],
    options: &ReverseOptions,
    language: ReverseLanguage,
) -> Result<Vec<u8>, LabelizeError> {
    options.validate()?;
    let image = decode_image(bytes)?;
    match language {
        ReverseLanguage::Zpl => image_to_zpl(&image, options).map(String::into_bytes),
        ReverseLanguage::Tspl => image_to_tspl(&image, options),
    }
}

/// Encode a raster image as ZPL `^GFA` graphic fields.
///
/// The graphic starts at `^FO0,0`. The generated ZPL leaves the label size
/// unset, so the canvas comes from render options or printer configuration —
/// to round-trip pixel-identically, render at
/// `image.dimensions() / dpmm` millimetres.
/// Images larger than 99999 packed bytes are split into fields on row
/// boundaries, each within the printer's `^GF` byte-count limit.
///
/// ```
/// use labelize::{image_to_zpl, ReverseOptions};
/// use image::RgbaImage;
///
/// let img = RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
/// let zpl = image_to_zpl(&img, &ReverseOptions::default()).unwrap();
/// assert!(zpl.starts_with("^XA\n^FO0,0^GFA,8,8,1,\nFF\n"));
/// ```
pub fn image_to_zpl(image: &RgbaImage, options: &ReverseOptions) -> Result<String, LabelizeError> {
    let bitmap = ink_bitmap(image, options)?;
    let rows_per_field = MAX_GRAPHIC_BYTES / bitmap.row_bytes;
    let mut out = String::with_capacity(bitmap.bits.len() * 2 + bitmap.height as usize + 128);
    out.push_str("^XA\n");
    for (index, field) in bitmap
        .bits
        .chunks(rows_per_field * bitmap.row_bytes)
        .enumerate()
    {
        let total = field.len();
        let y = index * rows_per_field;
        out.push_str(&format!(
            "^FO0,{y}^GFA,{total},{total},{},\n",
            bitmap.row_bytes
        ));
        for row in field.chunks(bitmap.row_bytes) {
            for &byte in row {
                out.push(HEX_DIGITS[(byte >> 4) as usize] as char);
                out.push(HEX_DIGITS[(byte & 0x0F) as usize] as char);
            }
            out.push('\n');
        }
        out.push_str("^FS\n");
    }
    out.push_str("^XZ\n");
    Ok(out)
}

/// Encode a raster image as TSPL/TSC `SIZE` + `BITMAP` commands.
///
/// `SIZE` declares the image's physical size (`pixels / dpmm` mm) so the
/// label canvas matches the image; `BITMAP` carries the raw payload in TSC
/// polarity (cleared bit = ink). The result is raw bytes, not text: the
/// BITMAP payload is binary data that may contain line breaks and other
/// control bytes.
pub fn image_to_tspl(
    image: &RgbaImage,
    options: &ReverseOptions,
) -> Result<Vec<u8>, LabelizeError> {
    let bitmap = ink_bitmap(image, options)?;
    let mut out = Vec::with_capacity(bitmap.bits.len() + 96);
    // The payload must follow the fifth comma inline — that is where the
    // raw-data cursor for BITMAP starts, so no line break may precede it.
    out.extend_from_slice(
        format!(
            "SIZE {} mm,{} mm\r\nCLS\r\nBITMAP 0,0,{},{},0,",
            mm_string(bitmap.width, options.dpmm),
            mm_string(bitmap.height, options.dpmm),
            bitmap.row_bytes,
            bitmap.height
        )
        .as_bytes(),
    );
    // TSC BITMAP prints cleared bits, so invert the set-bit-is-ink packing;
    // the row padding becomes 1-bits (paper), matching the ZPL zero padding.
    out.extend(bitmap.bits.iter().map(|byte| !byte));
    out.extend_from_slice(b"\r\nPRINT 1,1\r\n");
    Ok(out)
}

/// Decode an encoded image (PNG, JPEG, GIF, BMP, …) and emit ZPL.
pub fn zpl_from_image_bytes(
    bytes: &[u8],
    options: &ReverseOptions,
) -> Result<String, LabelizeError> {
    options.validate()?;
    image_to_zpl(&decode_image(bytes)?, options)
}

/// Decode an encoded image (PNG, JPEG, GIF, BMP, …) and emit TSPL bytes.
pub fn tspl_from_image_bytes(
    bytes: &[u8],
    options: &ReverseOptions,
) -> Result<Vec<u8>, LabelizeError> {
    encode_image_bytes(bytes, options, ReverseLanguage::Tspl)
}

fn decode_image(bytes: &[u8]) -> Result<RgbaImage, LabelizeError> {
    let decode_error = |e| LabelizeError::Encode(format!("image decode error: {e}"));
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| LabelizeError::Encode(format!("image decode error: {e}")))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    let (width, height) = decoder.dimensions();
    validate_dimensions(width, height)?;
    if decoder.total_bytes() > MAX_DECODE_BYTES {
        return Err(LabelizeError::Encode(
            "decoded image exceeds 128 MiB".to_string(),
        ));
    }
    // Match the displayed raster (including phone-photo EXIF rotation), so
    // browser dimensions and the encoded graphic describe the same image.
    let orientation = decoder.orientation().map_err(decode_error)?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    image.apply_orientation(orientation);
    Ok(image.into_rgba8())
}

/// The binarized image: rows packed MSB-first with a set bit = ink, each row
/// zero-padded (paper) to a byte boundary.
struct InkBitmap {
    width: u32,
    height: u32,
    row_bytes: usize,
    bits: Vec<u8>,
}

fn ink_bitmap(image: &RgbaImage, options: &ReverseOptions) -> Result<InkBitmap, LabelizeError> {
    options.validate()?;
    let (width, height) = image.dimensions();
    validate_dimensions(width, height)?;
    let row_bytes = width.div_ceil(8) as usize;
    let mut bits = vec![0u8; row_bytes * height as usize];
    for (pixels, row) in image.rows().zip(bits.chunks_exact_mut(row_bytes)) {
        for (x, pixel) in pixels.enumerate() {
            if composite_over_white_luma(pixel) < options.threshold as u32 {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    Ok(InkBitmap {
        width,
        height,
        row_bytes,
        bits,
    })
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), LabelizeError> {
    if width == 0 || height == 0 {
        return Err(LabelizeError::Encode(
            "cannot encode an image with zero width or height".to_string(),
        ));
    }
    if width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(LabelizeError::Encode(
            "image exceeds 16 Mi pixels or 32000 dots per dimension".to_string(),
        ));
    }
    Ok(())
}

/// Luminance (Rec. 601) of `pixel` composited onto an opaque white page, so
/// transparent areas never become ink.
fn composite_over_white_luma(pixel: &image::Rgba<u8>) -> u32 {
    let [r, g, b, a] = pixel.0;
    let over_white = |c: u8| (c as u32 * a as u32 + 255 * (255 - a as u32) + 127) / 255;
    (299 * over_white(r) + 587 * over_white(g) + 114 * over_white(b)) / 1000
}

/// Keep enough precision for every positive dpmm, including sub-micron dots.
fn mm_string(px: u32, dpmm: i32) -> String {
    (px as f64 / dpmm as f64).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(color))
    }

    #[test]
    fn black_8x2_produces_expected_zpl_text() {
        let img = solid(8, 2, [0, 0, 0, 255]);
        let zpl = image_to_zpl(&img, &ReverseOptions::default()).unwrap();
        assert_eq!(zpl, "^XA\n^FO0,0^GFA,2,2,1,\nFF\nFF\n^FS\n^XZ\n");
    }

    #[test]
    fn black_8x2_produces_expected_tspl_bytes() {
        let img = solid(8, 2, [0, 0, 0, 255]);
        let tspl = image_to_tspl(&img, &ReverseOptions::default()).unwrap();
        let mut expected = b"SIZE 1 mm,0.25 mm\r\nCLS\r\nBITMAP 0,0,1,2,0,".to_vec();
        // TSC polarity: cleared bits print black.
        expected.extend_from_slice(&[0x00, 0x00]);
        expected.extend_from_slice(b"\r\nPRINT 1,1\r\n");
        assert_eq!(tspl, expected);
    }

    #[test]
    fn odd_width_pads_towards_paper_in_both_polarities() {
        // 13 ink columns: ZPL pads the row with 0-bits (paper), TSPL's
        // complement pads with 1-bits (also paper, since 0 = black there).
        let img = solid(13, 1, [0, 0, 0, 255]);
        let zpl = image_to_zpl(&img, &ReverseOptions::default()).unwrap();
        assert!(zpl.contains("^GFA,2,2,2,\nFFF8\n"), "{zpl}");
        let tspl = image_to_tspl(&img, &ReverseOptions::default()).unwrap();
        let expected_payload = [0x00, 0x07];
        assert!(tspl.windows(2).any(|w| w == expected_payload));
    }

    #[test]
    fn threshold_boundary_is_strictly_below() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([127, 127, 127, 255])); // luma 127 → ink
        img.put_pixel(1, 0, Rgba([128, 128, 128, 255])); // luma 128 → paper
        let zpl = image_to_zpl(&img, &ReverseOptions::default()).unwrap();
        assert!(zpl.contains("^GFA,1,1,1,\n80\n"), "{zpl}");
    }

    #[test]
    fn color_and_alpha_binarize_over_white() {
        let mut img = RgbaImage::new(4, 1);
        // Red: luma 76 → ink. Light green: luma 150 → paper.
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        // Transparent black composites onto white → paper. Half-transparent
        // black lands at luma 127 → ink.
        img.put_pixel(2, 0, Rgba([0, 0, 0, 0]));
        img.put_pixel(3, 0, Rgba([0, 0, 0, 128]));
        let zpl = image_to_zpl(&img, &ReverseOptions::default()).unwrap();
        // bit 0 = ink, bit 1 = paper, bit 2 = paper, bit 3 = ink → 1001 = 0x90.
        assert!(zpl.contains("^GFA,1,1,1,\n90\n"), "{zpl}");
    }

    #[test]
    fn mm_strings_trim_to_exact_values() {
        assert_eq!(mm_string(813, 8), "101.625");
        assert_eq!(mm_string(1626, 8), "203.25");
        assert_eq!(mm_string(60, 12), "5");
        assert_eq!(mm_string(8, 8), "1");
    }

    #[test]
    fn rejects_empty_images_and_bad_dpmm() {
        let empty = RgbaImage::new(0, 5);
        assert!(image_to_zpl(&empty, &ReverseOptions::default()).is_err());
        assert!(image_to_tspl(&empty, &ReverseOptions::default()).is_err());
        let opts = ReverseOptions {
            dpmm: 0,
            ..ReverseOptions::default()
        };
        assert!(image_to_zpl(&solid(8, 8, [0, 0, 0, 255]), &opts).is_err());
    }

    #[test]
    fn rejects_garbage_image_bytes() {
        assert!(zpl_from_image_bytes(b"not an image", &ReverseOptions::default()).is_err());
        assert!(tspl_from_image_bytes(b"", &ReverseOptions::default()).is_err());
    }

    #[test]
    fn shipping_labels_are_split_at_the_printer_field_limit() {
        // 101 bytes per row: 990 rows fit, 991 do not. This catches the
        // printer limit independently of our permissive forward parser.
        for (height, fields) in [(990, 1), (991, 2)] {
            let zpl = image_to_zpl(
                &solid(808, height, [0, 0, 0, 255]),
                &ReverseOptions::default(),
            )
            .unwrap();
            let mut total = 0;
            let mut count = 0;
            for field in zpl.split("^GFA,").skip(1) {
                let mut parts = field.splitn(4, ',');
                let bytes: usize = parts.next().unwrap().parse().unwrap();
                assert_eq!(parts.next().unwrap().parse::<usize>().unwrap(), bytes);
                assert_eq!(parts.next().unwrap(), "101");
                let hex = parts.next().unwrap().split("^FS").next().unwrap();
                assert_eq!(
                    hex.chars().filter(|c| !c.is_whitespace()).count(),
                    bytes * 2
                );
                assert!(bytes <= 99_999);
                total += bytes;
                count += 1;
            }
            assert_eq!(count, fields);
            assert_eq!(total, 101 * height as usize);
            if fields == 2 {
                assert!(zpl.contains("^FO0,990^GFA,101,101,101,"));
            }
        }
    }

    #[test]
    fn encoded_input_is_bounded_before_decoding_pixels() {
        // Header-only PPM: an allocation/read would fail with EOF. A size
        // error instead proves dimensions were rejected before pixel decode.
        let error =
            zpl_from_image_bytes(b"P6\n4097 4096\n255\n", &ReverseOptions::default()).unwrap_err();
        assert!(error.to_string().contains("16 Mi pixels"), "{error}");
        let error = zpl_from_image_bytes(
            b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 4096 +X 4096\n",
            &ReverseOptions::default(),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("decoded image exceeds 128 MiB"),
            "{error}"
        );
        let options = ReverseOptions {
            dpmm: 0,
            ..Default::default()
        };
        let error = zpl_from_image_bytes(b"not an image", &options).unwrap_err();
        assert!(
            error.to_string().contains("dpmm must be positive"),
            "{error}"
        );
    }

    #[test]
    fn millimetres_keep_sub_micron_precision() {
        for dpmm in [6, 7, 12, 24, 24000, i32::MAX] {
            for dots in [1, 13, 813] {
                let mm: f64 = mm_string(dots, dpmm).parse().unwrap();
                assert_eq!((mm * dpmm as f64).round() as u32, dots);
            }
        }
    }
}
