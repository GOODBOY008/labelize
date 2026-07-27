use crate::error::LabelizeError;
use image::RgbaImage;
use std::io::Write;

pub fn encode_png(img: &RgbaImage, w: &mut impl Write) -> Result<(), LabelizeError> {
    let (width, height) = img.dimensions();
    let mut gray = image::GrayImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            gray.put_pixel(x, y, image::Luma([pixel[0]]));
        }
    }

    let encoder = image::codecs::png::PngEncoder::new(w);
    use image::ImageEncoder;
    encoder
        .write_image(gray.as_raw(), width, height, image::ExtendedColorType::L8)
        .map_err(|e| LabelizeError::Encode(format!("PNG encode error: {}", e)))
}
