//! Volatile downloads in module D. The native 7-bit format is specified in
//! "Image Loading" in the DPL Programmer's Manual (Figures 7-3 and 7-4).

use std::collections::HashMap;
use std::io::Cursor;

use image::{codecs::bmp::BmpDecoder, DynamicImage, ImageDecoder};

use crate::elements::graphic_field::{GraphicField, GraphicFieldFormat};
use crate::elements::label_position::LabelPosition;

const MAX_PACKED_BYTES: usize = 2 * 1024 * 1024; // 16 Mi dots of monochrome artwork
const MAX_DIMENSION: usize = 32_000;

const MAX_JOB_BYTES: usize = 128 * 1024 * 1024;

#[derive(Default)]
pub(super) struct ImageBank {
    images: HashMap<String, StoredImage>,
    stored_bytes: usize,
    recalled_bytes: usize,
}

impl ImageBank {
    pub(super) fn store(&mut self, name: &str, image: StoredImage) -> Result<(), String> {
        let previous = self.images.get(name).map_or(0, |img| img.data.len());
        let size = self.stored_bytes - previous + image.data.len();
        if size + self.recalled_bytes > MAX_JOB_BYTES {
            return Err("DPL images exceed job size limit".into());
        }
        self.images.insert(name.to_owned(), image);
        self.stored_bytes = size;
        Ok(())
    }

    pub(super) fn clear(&mut self) {
        self.images.clear();
        self.stored_bytes = 0;
    }

    pub(super) fn remove(&mut self, name: &str) {
        if let Some(image) = self.images.remove(name) {
            self.stored_bytes -= image.data.len();
        }
    }

    pub(super) fn recall(
        &mut self,
        name: &str,
        position: LabelPosition,
        mx: u8,
        my: u8,
    ) -> Result<GraphicField, String> {
        let image = self
            .images
            .get(name)
            .ok_or_else(|| format!("unresolved DPL image {name:?}"))?;
        let size = self.recalled_bytes + image.data.len();
        if self.stored_bytes + size > MAX_JOB_BYTES {
            return Err("DPL images exceed job size limit".into());
        }
        let field = image.field(position, mx, my)?;
        self.recalled_bytes = size;
        Ok(field)
    }
}

pub(super) struct StoredImage {
    row_bytes: usize,
    data: Vec<u8>,
}

impl StoredImage {
    fn field(&self, position: LabelPosition, mx: u8, my: u8) -> Result<GraphicField, String> {
        fn multiplier(digit: u8) -> Result<i32, String> {
            match digit {
                b'1'..=b'9' => Ok(i32::from(digit - b'0')),
                b'A'..=b'Z' => Ok(i32::from(digit - b'A') + 10),
                b'a'..=b'z' => Ok(i32::from(digit - b'a') + 36),
                _ => Err("invalid DPL image magnification".into()),
            }
        }
        Ok(GraphicField {
            position,
            magnification_x: multiplier(mx)?,
            magnification_y: multiplier(my)?,
            format: GraphicFieldFormat::Raw,
            data_bytes: self.data.len() as i32,
            total_bytes: self.data.len() as i32,
            row_bytes: self.row_bytes as i32,
            data: self.data.clone(),
            mode: Default::default(),
            reverse_print: Default::default(),
            zero_bits_black: false,
        })
    }
}

pub(super) fn image_name(bytes: &[u8]) -> Result<&str, String> {
    if !(1..=16).contains(&bytes.len()) || !bytes.iter().all(|b| (b' '..=b'~').contains(b)) {
        return Err("DPL image name must be 1 to 16 printable ASCII characters".into());
    }
    Ok(std::str::from_utf8(bytes).unwrap())
}

// The record scanner and formatter must advance by the whole download even
// when bitmap bytes are themselves record delimiters or system commands.
pub(super) fn download(input: &[u8]) -> Result<(&str, StoredImage, usize), String> {
    let (header, payload) = line(input)?;
    let Some((&bank, args)) = header[2..].split_first() else {
        return Err("missing DPL image module".into());
    };
    if bank != b'D' {
        return Err(format!(
            "unsupported DPL image module {} (expected RAM module D)",
            bank as char
        ));
    }
    let ascii = args.starts_with(b"A");
    let args = if ascii { &args[1..] } else { args };
    let (&kind, name) = args.split_first().ok_or("missing DPL image format")?;
    let name = image_name(name)?;
    let (image, consumed) = match kind {
        b'F' => datamax_rows(payload)?,
        b'b' if !ascii => bitmap(payload)?,
        _ => return Err(format!("unsupported DPL image format {}", kind as char)),
    };
    Ok((name, image, input.len() - payload.len() + consumed))
}

fn line(input: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let end = input
        .iter()
        .position(|b| matches!(b, b'\r' | b'\n'))
        .ok_or("unterminated DPL image record")?;
    let skip = if input[end..].starts_with(b"\r\n") {
        2
    } else {
        1
    };
    Ok((&input[..end], &input[end + skip..]))
}

fn hex_pair(bytes: &[u8]) -> Result<u8, String> {
    let [hi, lo] = bytes else {
        return Err("invalid DPL image hex byte".into());
    };
    let digit = |b: u8| {
        char::from(b)
            .to_digit(16)
            .ok_or("invalid DPL image hex digit")
    };
    Ok(((digit(*hi)? << 4) | digit(*lo)?) as u8)
}

fn datamax_rows(input: &[u8]) -> Result<(StoredImage, usize), String> {
    let mut rest = input;
    let mut data = Vec::new();
    let mut row_bytes = 0;
    loop {
        let row;
        (row, rest) = line(rest)?;
        if row.eq_ignore_ascii_case(b"FFFF") {
            if row_bytes == 0 {
                return Err("empty DPL image".into());
            }
            break;
        }
        let copies = if row.starts_with(b"80") && row.len() >= 4 {
            let width = usize::from(hex_pair(&row[2..4])?);
            if width == 0 || row.len() != 4 + 2 * width || (row_bytes != 0 && row_bytes != width) {
                return Err("invalid DPL image row width".into());
            }
            row_bytes = width;
            // Rows start at the bottom. Reverse bytes within each row now,
            // then reverse the complete buffer to produce top-down rows.
            for pair in row[4..].rchunks_exact(2) {
                data.push(!hex_pair(pair)?);
            }
            0
        } else if row.len() == 8 && row[..6].eq_ignore_ascii_case(b"0000FF") && row_bytes > 0 {
            usize::from(hex_pair(&row[6..])?)
        } else {
            return Err("invalid DPL image row or repeat record".into());
        };
        let size = data.len() + copies * row_bytes;
        if size > MAX_PACKED_BYTES || size / row_bytes > MAX_DIMENSION {
            return Err("DPL image exceeds size limit".into());
        }
        for _ in 0..copies {
            data.extend_from_within(data.len() - row_bytes..);
        }
    }
    data.reverse();
    Ok((StoredImage { row_bytes, data }, input.len() - rest.len()))
}

fn bitmap(input: &[u8]) -> Result<(StoredImage, usize), String> {
    if !input.starts_with(b"BM") || input.len() < 6 {
        return Err("invalid DPL BMP header".into());
    }
    let size = u32::from_le_bytes(input[2..6].try_into().unwrap()) as usize;
    let payload = input.get(..size).ok_or("truncated DPL BMP download")?;
    let decoder =
        BmpDecoder::new(Cursor::new(payload)).map_err(|e| format!("invalid DPL BMP: {e}"))?;
    let (w, h) = decoder.dimensions();
    let (width, height) = (w as usize, h as usize);
    let row_bytes = width.div_ceil(8);
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || row_bytes * height > MAX_PACKED_BYTES
    {
        return Err("DPL image exceeds size limit".into());
    }
    let pixels = DynamicImage::from_decoder(decoder)
        .map_err(|e| format!("invalid DPL BMP: {e}"))?
        .into_luma8();
    let mut data = vec![0; row_bytes * height];
    for (x, y, pixel) in pixels.enumerate_pixels() {
        if pixel[0] < 128 {
            data[y as usize * row_bytes + x as usize / 8] |= 0x80 >> (x % 8);
        }
    }
    Ok((StoredImage { row_bytes, data }, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_budget_tracks_live_downloads_and_retained_recalls() {
        let mut bank = ImageBank {
            recalled_bytes: MAX_JOB_BYTES - 3,
            ..Default::default()
        };
        let image = |n| StoredImage {
            row_bytes: 1,
            data: vec![0; n],
        };
        bank.store("A", image(2)).unwrap();
        bank.store("A", image(1)).unwrap();
        bank.store("B", image(2)).unwrap();
        assert!(bank.store("C", image(1)).is_err());
        assert!(bank
            .recall("A", LabelPosition::default(), b'1', b'1')
            .is_err());
        bank.remove("B");
        bank.recall("A", LabelPosition::default(), b'1', b'1')
            .unwrap();
        bank.clear();
        assert!(bank.store("C", image(3)).is_err());
        bank.store("C", image(2)).unwrap();
    }
}
