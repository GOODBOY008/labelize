pub mod aztec;
pub mod bit_matrix;
pub mod code128;
pub mod code39;
pub mod datamatrix;
mod datamatrix_field;
pub mod datamatrix_legacy;
pub mod ean13;
pub mod ean8;
pub mod maxicode;
pub mod pdf417;
pub mod pdf417_encoding;
mod qr_mask;
pub mod qrcode;
pub mod twooffive;
pub mod upca;
pub mod upce;

pub use bit_matrix::BitMatrix;

mod codablock;
pub(crate) mod symbols;

mod code49;

mod gs1_composite;

mod tlc39;

mod composite_field;

pub(crate) mod check_text;

use image::RgbaImage;

pub(crate) fn checked_image(width: u32, height: u32) -> Result<RgbaImage, String> {
    if width == 0
        || height == 0
        || width > 32000
        || height > 32000
        || u64::from(width) * u64::from(height) > 16 * 1024 * 1024
    {
        return Err(format!(
            "Barcode dimensions {width} x {height} exceed the supported image size"
        ));
    }
    Ok(RgbaImage::new(width, height))
}
