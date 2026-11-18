use super::virtual_printer::VirtualPrinter;
use crate::elements::{
    font::{FontInfo, FontResources, TextProperties},
    stored_graphics::StoredGraphics,
};
use base64::Engine;
use std::{collections::HashMap, io::Read, sync::Arc};

pub(super) const MAX_BYTES: usize = 16 * 1024 * 1024;

struct StoredFont {
    bytes: Arc<[u8]>,
    cell: Option<(u16, u16)>,
}

#[derive(Default)]
pub(super) struct Fonts {
    files: HashMap<String, StoredFont>,
    legacy: HashMap<String, Vec<u8>>,
    links: Vec<(String, String)>,
    pub properties: Option<TextProperties>,
}

fn key(name: &str, ext: &str) -> String {
    let name = name.trim().to_ascii_uppercase();
    let mut name = if name.contains(':') {
        name
    } else {
        format!("R:{name}")
    };
    if name.ends_with(':') {
        name.push_str("UNKNOWN")
    }
    if !name.contains('.') {
        name.push('.');
        name.push_str(ext)
    }
    name
}

fn wildcard(pattern: &str, value: &str) -> bool {
    let (p, s) = (pattern.as_bytes(), value.as_bytes());
    let (mut i, mut j, mut star, mut back) = (0, 0, None, 0);
    while j < s.len() {
        if i < p.len() && (p[i] == b'?' || p[i] == s[j]) {
            i += 1;
            j += 1
        } else if p.get(i) == Some(&b'*') {
            star = Some(i);
            i += 1;
            back = j
        } else if let Some(k) = star {
            back += 1;
            j = back;
            i = k + 1
        } else {
            return false;
        }
    }
    p[i..].iter().all(|b| *b == b'*')
}

impl Fonts {
    pub fn find(&self, name: &str) -> Option<String> {
        let k = key(name, "FNT");
        (self.files.contains_key(&k) || self.legacy.contains_key(&k)).then_some(k)
    }
    pub fn remove(&mut self, name: &str) {
        if !name.contains('.') {
            return;
        }
        let key = key(name, "FNT");
        self.files.remove(&key);
        self.legacy.remove(&key);
    }
    pub fn needs_intellifont(&self, name: &str) -> bool {
        self.legacy.contains_key(&key(name, "FNT"))
    }
    fn store(
        &mut self,
        name: String,
        bytes: Vec<u8>,
        cell: Option<(u16, u16)>,
    ) -> Result<(), String> {
        if ab_glyph::FontRef::try_from_slice(&bytes).is_err() {
            return Err(format!("Invalid or unsupported downloadable font {name}"));
        }
        self.legacy.remove(&name);
        self.files.insert(
            name,
            StoredFont {
                bytes: bytes.into(),
                cell,
            },
        );
        Ok(())
    }
    pub fn link(&mut self, p: &[&str]) {
        if p.len() < 3 {
            return;
        }
        let ext = key(p[0], "TTF");
        let base = if p[1].contains(':') {
            key(p[1], "FNT")
        } else if p[1].starts_with('*') {
            format!("*:{}", p[1].trim().to_ascii_uppercase())
        } else {
            key(p[1], "FNT")
        };
        match p[2].trim() {
            "1" if !self.links.contains(&(base.clone(), ext.clone())) => {
                self.links.push((base, ext))
            }
            "0" => self.links.retain(|(b, e)| b != &base || e != &ext),
            _ => {}
        }
    }
    pub fn bind(&self, mut font: FontInfo) -> FontInfo {
        if self.files.is_empty() && self.links.is_empty() && self.properties.is_none() {
            return font;
        }
        let name = if font.name.len() == 1 {
            format!("Z:{}.FNT", font.name)
        } else {
            key(&font.name, "FNT")
        };
        let primary = self.files.get(&name);
        let fallbacks: Vec<_> = self
            .links
            .iter()
            .filter(|(base, _)| wildcard(base, &name))
            .filter_map(|(_, ext)| self.files.get(ext).map(|f| f.bytes.clone()))
            .take(5)
            .collect();
        if primary.is_some() || !fallbacks.is_empty() || self.properties.is_some() {
            font.resources = Some(Arc::new(FontResources {
                primary: primary.map(|f| f.bytes.clone()),
                native_cell: primary.and_then(|f| f.cell),
                fallbacks,
                properties: self.properties.clone().unwrap_or_default(),
            }));
        }
        font
    }
}

pub(super) fn unhex(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut high = None;
    for &b in data {
        if b.is_ascii_whitespace() {
            continue;
        }
        let nibble = char::from(b)
            .to_digit(16)
            .ok_or("Invalid hexadecimal object")? as u8;
        if let Some(first) = high.take() {
            if bytes.len() == MAX_BYTES {
                return Err("Downloaded object exceeds 16 MiB".into());
            }
            bytes.push(first << 4 | nibble)
        } else {
            high = Some(nibble);
        }
    }
    if high.is_some() {
        return Err("Incomplete hexadecimal byte in object".into());
    }
    Ok(bytes)
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub(super) fn ascii_bytes(data: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    if !data.starts_with(b":Z64:") && !data.starts_with(b":B64:") {
        return unhex(data);
    }
    let tail = &data[5..];
    let end = tail
        .iter()
        .position(|&b| b == b':')
        .ok_or("Missing object CRC")?;
    let body = &tail[..end];
    if body.len() > MAX_BYTES * 2 {
        return Err("Encoded object exceeds 16 MiB".into());
    }
    let checksum = &tail[end + 1..];
    let checksum = std::str::from_utf8(checksum)
        .ok()
        .filter(|s| s.len() == 4 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .and_then(|s| u16::from_str_radix(s, 16).ok())
        .ok_or("Invalid object CRC")?;
    if crc16(body) != checksum {
        return Err("Downloaded object CRC mismatch".into());
    }
    let packed = base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|e| e.to_string())?;
    if data[1] == b'B' {
        return Ok(packed);
    }
    // Zebra's original font download example uses a gzip header. Modern
    // graphics generators commonly use the zlib wrapper around DEFLATE.
    let mut reader: Box<dyn Read> = if packed.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::GzDecoder::new(packed.as_slice()))
    } else {
        Box::new(flate2::read::ZlibDecoder::new(packed.as_slice()))
    };
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(expected as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn count(s: &[u8]) -> Result<usize, String> {
    std::str::from_utf8(s)
        .ok()
        .and_then(|p| p.trim().parse().ok())
        .filter(|&n| n <= MAX_BYTES)
        .ok_or_else(|| "Invalid downloaded object size".into())
}

pub(super) fn download(
    printer: &mut VirtualPrinter,
    command: &[u8],
    delim: u8,
) -> Result<(), String> {
    let cmd = command[2].to_ascii_uppercase();
    if cmd == b'B' {
        let bitmap = super::bitmap_font::decode(&command[3..], delim)?;
        let name = key(&bitmap.name, "FNT");
        super::fs::validate_device(&name)?;
        return printer.fonts.store(name, bitmap.font, Some(bitmap.cell));
    }
    let mut fields = command[3..].splitn(if cmd == b'Y' { 6 } else { 3 }, |&b| b == delim);
    let path = std::str::from_utf8(fields.next().unwrap_or_default())
        .map_err(|_| "Invalid object filename")?;
    let (encoding, extension, length, row) = if cmd == b'Y' {
        let code = |s: Option<&[u8]>| {
            s.and_then(|s| s.first())
                .map(u8::to_ascii_uppercase)
                .unwrap_or_default()
        };
        let transport = code(fields.next());
        let kind = match fields
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase()
            .as_slice()
        {
            b"PAC" | b"NRD" | b"C" | b"F" | b"H" => 0, // Non-renderable device objects
            [c @ (b'B' | b'E' | b'G' | b'P' | b'T' | b'X')] => *c,
            [] if transport == b'P' => b'P',
            _ => b'G',
        };
        let length = count(fields.next().ok_or("Missing object size")?)?;
        let row = fields.next().unwrap_or_default();
        (
            transport,
            kind,
            length,
            if row.is_empty() { 0 } else { count(row)? },
        )
    } else {
        (
            b'A',
            b'T',
            count(fields.next().ok_or("Missing font size")?)?,
            0,
        )
    };
    let input = fields.next().ok_or("Missing downloaded object data")?;
    let bytes = match encoding {
        b'A' | b'P' => ascii_bytes(input, length)?,
        b'B' => input.to_vec(),
        b'C' => return Err("Unsupported AR-compressed object".into()),
        _ => return Err("Unknown object transport encoding".into()),
    };
    if bytes.len() != length {
        return Err("Downloaded object length differs from declared size".into());
    }
    let ext = match (cmd, extension) {
        (b'T', _) => "DAT",
        (b'U' | b'S', _) => "FNT",
        (_, b'P') => "PNG",
        (_, b'B') => "BMP",
        (_, b'X') => "PCX",
        (_, b'T') => "TTF",
        (_, b'E') => "TTE",
        _ => "GRF",
    };
    let name = key(path, ext);
    super::fs::validate_device(&name)?;
    if cmd == b'S' {
        printer.fonts.files.remove(&name);
        printer.fonts.legacy.insert(name, bytes);
        return Ok(());
    }
    if matches!(extension, b'T' | b'E') {
        return printer.fonts.store(name, bytes, None);
    }
    let (row_bytes, data) = match extension {
        b'G' => {
            if row == 0 || length == 0 || !length.is_multiple_of(row) {
                return Err("Invalid downloaded graphic row width".into());
            }
            (row, bytes)
        }
        b'B' | b'P' => crate::reverse::packed_image(&bytes).map_err(|e| e.to_string())?,
        b'X' => super::objects::pcx(&bytes)?,
        _ => return Ok(()),
    };
    printer.saved_images.remove(&name);
    printer.stored_graphics.insert(
        name,
        StoredGraphics {
            total_bytes: data.len() as i32,
            row_bytes: row_bytes as i32,
            data,
        },
    );
    Ok(())
}

/// ZSoft PCX scanlines are padded per plane. Graphic storage uses MSB-first
/// ink bits with the image width rounded up to complete bytes.
fn pcx(bytes: &[u8]) -> Result<(usize, Vec<u8>), String> {
    if bytes.len() < 128 || bytes[0] != 0x0a || bytes[2] > 1 {
        return Err("Invalid PCX header".into());
    }
    let word = |i| u16::from_le_bytes([bytes[i], bytes[i + 1]]) as usize;
    let w = word(8).checked_sub(word(4)).ok_or("Invalid PCX width")? + 1;
    let h = word(10).checked_sub(word(6)).ok_or("Invalid PCX height")? + 1;
    let bits = bytes[3] as usize;
    let planes = bytes[65] as usize;
    let stride = word(66);
    if !matches!((bits, planes), (1, 1..=4) | (8, 1 | 3))
        || w > 32000
        || h > 32000
        || w * h > 16 * 1024 * 1024
        || stride < (w * bits).div_ceil(8)
    {
        return Err("Unsupported PCX layout".into());
    }
    let row_bytes = w.div_ceil(8);
    let mut ink = vec![0; row_bytes * h];
    let mut pos = 128;
    let palette = if bits == 8 && planes == 1 {
        Some(
            bytes
                .get(bytes.len().saturating_sub(769)..)
                .filter(|p| p.len() == 769 && p[0] == 12)
                .ok_or("Missing PCX palette")?,
        )
    } else {
        None
    };
    let mut line = Vec::with_capacity(planes * stride);
    for y in 0..h {
        line.clear();
        while line.len() < planes * stride {
            let b = *bytes.get(pos).ok_or("Truncated PCX")?;
            pos += 1;
            let (n, byte) = if bytes[2] == 1 && b & 192 == 192 {
                let v = *bytes.get(pos).ok_or("Truncated PCX run")?;
                pos += 1;
                ((b & 63) as usize, v)
            } else {
                (1, b)
            };
            if n == 0 || line.len() + n > planes * stride {
                return Err("Invalid PCX run".into());
            }
            line.resize(line.len() + n, byte);
        }
        for x in 0..w {
            let rgb = if planes == 3 && bits == 8 {
                [line[x], line[stride + x], line[2 * stride + x]]
            } else {
                let (pal, idx) = if let Some(p) = palette {
                    (&p[1..], line[x] as usize)
                } else {
                    let idx = (0..planes).fold(0, |v, p| {
                        v | ((((line[p * stride + x / 8] >> (7 - x % 8)) & 1) as usize) << p)
                    });
                    (&bytes[16..64], idx)
                };
                [pal[idx * 3], pal[idx * 3 + 1], pal[idx * 3 + 2]]
            };
            if 299 * u32::from(rgb[0]) + 587 * u32::from(rgb[1]) + 114 * u32::from(rgb[2]) < 128000
            {
                ink[y * row_bytes + x / 8] |= 128 >> (x % 8)
            }
        }
    }
    Ok((row_bytes, ink))
}
