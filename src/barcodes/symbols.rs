//! Encoders for additional linear, stacked and postal symbols.
use crate::elements::barcode_symbol::{BarcodeKind, BarcodeSymbol, PostalCode};
use anyd::codes::{
    codabar::CodabarEncoder,
    code11::Code11Encoder,
    code39::Code39Encoder,
    code93::Code93Encoder,
    databar::DataBarEncoder,
    ean::EanEncoder,
    msi::{MsiCheck, MsiEncoder},
    pdf417::{EcLevel, MicroPdf417Encoder, Pdf417Meta, Pdf417Variant},
    postal::PostalEncoder,
    twoof5::TwoOf5Encoder,
};
use anyd::{output::Encoding, traits::Encode, Segment, Symbol, SymbolMeta, Symbology};
use image::{Rgba, RgbaImage};

pub(super) fn modules(
    encoder: impl Encode,
    symbol: anyd::Result<Symbol>,
) -> Result<Encoding, String> {
    symbol
        .and_then(|s| encoder.encode(&s))
        .map_err(|e| e.to_string())
}

pub(super) fn micro(data: &[u8], variant: Option<u8>) -> Result<Encoding, String> {
    let encoder = MicroPdf417Encoder::new();
    let segment = if data.len() >= 13 && data.iter().all(u8::is_ascii_digit) {
        Segment::numeric(data.to_vec())
    } else if data.iter().all(|b| matches!(b, 32..=126 | 9 | 10 | 13)) {
        Segment::alphanumeric(data.to_vec())
    } else {
        Segment::byte(data.to_vec())
    };
    let selected = if let Some(mode) = variant {
        // Zebra numbers the 4 x 4 symbol last. ISO lists it before the other
        // four-column variants.
        let variant = match mode {
            0..=22 => mode,
            23..=32 => mode + 1,
            33 => 23,
            _ => return Err("Invalid MicroPDF417 size".into()),
        };
        const ROWS: [usize; 34] = [
            11, 14, 17, 20, 24, 28, 8, 11, 14, 17, 20, 23, 26, 6, 8, 10, 12, 15, 20, 26, 32, 38,
            44, 4, 6, 8, 10, 12, 15, 20, 26, 32, 38, 44,
        ];
        let columns = match variant {
            0..=5 => 1,
            6..=12 => 2,
            13..=22 => 3,
            _ => 4,
        };
        let meta = Pdf417Meta {
            rows: ROWS[variant as usize],
            columns,
            ec_level: EcLevel::new(0).unwrap(),
            variant: Pdf417Variant::Micro(variant),
        };
        Ok(Symbol::new(
            Symbology::MicroPdf417,
            vec![segment],
            SymbolMeta::Pdf417(meta),
        ))
    } else {
        encoder.build(vec![segment])
    };
    let Encoding::Matrix(original) = modules(encoder, selected)? else {
        unreachable!()
    };
    // The encoder repeats each MicroPDF417 row at its nominal 2:1 aspect.
    // Restore physical rows before applying the caller's height in dots.
    let mut logical = anyd::output::BitMatrix::new(original.width(), original.height() / 2, 0);
    for y in 0..logical.height() {
        for x in 0..logical.width() {
            logical.set(x, y, original.get(x, y * 2));
        }
    }
    Ok(Encoding::Matrix(logical))
}

pub(super) fn checked_image(width: u32, height: u32) -> Result<RgbaImage, String> {
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

pub(super) fn raster(
    encoding: Encoding,
    module: u32,
    height: u32,
    ratio: Option<f64>,
    postal: Option<PostalCode>,
    row_height: bool,
) -> Result<RgbaImage, String> {
    let ink = Rgba([0, 0, 0, 255]);
    match encoding {
        Encoding::Linear(row) => {
            let mut runs = Vec::new();
            for bit in row.modules {
                match runs.last_mut() {
                    Some((dark, n)) if *dark == bit => *n += 1u32,
                    _ => runs.push((bit, 1u32)),
                }
            }
            let mut width = 0u32;
            for (_, n) in &mut runs {
                *n = if *n > 1 && ratio.is_some() {
                    (module as f64 * ratio.unwrap_or(1.0)).round() as u32
                } else {
                    n.saturating_mul(module)
                };
                width = width.checked_add(*n).ok_or("Barcode width overflow")?;
            }
            let mut img = checked_image(width, height)?;
            let mut x = 0;
            for (dark, n) in runs {
                if dark {
                    for y in 0..height {
                        for xx in x..x + n {
                            img.put_pixel(xx, y, ink)
                        }
                    }
                }
                x += n;
            }
            Ok(img)
        }
        Encoding::Matrix(m) => {
            let bar_pitch = if postal.is_some() {
                (module as f64 * 2.5).round() as u32
            } else {
                0
            };
            let w = if postal.is_some() {
                (m.width() as u32 / 2).saturating_mul(bar_pitch) + module
            } else {
                (m.width() as u32).saturating_mul(module)
            };
            let h = if row_height {
                height.saturating_mul(m.height() as u32)
            } else {
                height
            };
            let mut img = checked_image(w, h)?;
            for y in 0..h {
                let my = if matches!(postal, Some(PostalCode::Planet | PostalCode::Postnet)) {
                    if y * 5 < h * 3 {
                        0
                    } else {
                        1
                    }
                } else {
                    ((u64::from(y) * m.height() as u64) / u64::from(h)) as usize
                };
                for x in 0..w {
                    let mx = if postal.is_some() {
                        if x % bar_pitch >= module {
                            continue;
                        }
                        (x / bar_pitch * 2) as usize
                    } else {
                        (x / module) as usize
                    };
                    if m.get(mx, my) {
                        img.put_pixel(x, y, ink)
                    }
                }
            }
            Ok(img)
        }
    }
}

/// Paint continuous start/stop patterns through the interior separator bars.
fn stacked_bars(spec: &BarcodeSymbol, grid: &anyd::output::BitMatrix) -> Result<RgbaImage, String> {
    let module = spec.module;
    let (pitch, left_margin, right_margin, start, stop) = match spec.kind {
        // Code 49's outer binding bars include the 10X leading and 1X trailing
        // quiet zones. Interior separators span the 70 symbol modules.
        BarcodeKind::Code49 { .. } => (module, 10 * module, module, 0, 0),
        BarcodeKind::Codablock { mode: b'A', .. } => {
            let wide = (f64::from(module) * spec.ratio).round() as u32;
            let start = 7 * module + 3 * wide;
            (1, 0, 0, start, start)
        }
        BarcodeKind::Codablock { .. } => (module, 0, 0, 11 * module, 13 * module),
        _ => unreachable!(),
    };
    let rows = grid.height() as u32;
    let bars = grid.width() as u32 * pitch;
    let separators = (rows + 1) * module;
    let row_area = if matches!(
        spec.kind,
        BarcodeKind::Codablock {
            height_is_total: true,
            ..
        }
    ) {
        spec.height.saturating_sub(separators).max(rows)
    } else {
        spec.height.saturating_mul(rows)
    };
    let mut image = checked_image(left_margin + bars + right_margin, row_area + separators)?;
    let ink = Rgba([0, 0, 0, 255]);
    let width = image.width();
    for row in 0..=rows {
        let boundary = (u64::from(row) * u64::from(row_area) / u64::from(rows)) as u32;
        let y = boundary + row * module;
        let next = (u64::from((row + 1).min(rows)) * u64::from(row_area) / u64::from(rows)) as u32
            + (row + 1) * module;
        let outer = row == 0 || row == rows;
        for x in 0..width {
            let inner = x >= left_margin && x < left_margin + bars;
            let dark = inner
                && grid.get(
                    ((x - left_margin) / pitch) as usize,
                    row.min(rows - 1) as usize,
                );
            if outer
                || inner && (dark || (left_margin + start..left_margin + bars - stop).contains(&x))
            {
                for yy in y..y + module {
                    image.put_pixel(x, yy, ink);
                }
            }
            if row < rows && dark {
                for yy in y + module..next {
                    image.put_pixel(x, yy, ink);
                }
            }
        }
    }
    Ok(image)
}

pub(crate) fn encode(spec: &BarcodeSymbol, data: &[u8]) -> Result<(RgbaImage, String), String> {
    use BarcodeKind::*;
    // Even the widest stacked format cannot hold more than 44 rows of
    // 62 codewords. Reject oversized input before entering an encoder.
    if data.len() > 8192 {
        return Err("Barcode data exceeds the supported symbol size".into());
    }
    let text = String::from_utf8_lossy(data);
    let mut caption = text.to_string();
    let mut ratio = None;
    let mut postal = None;
    let mut row_height = false;
    let mut height = spec.height;
    let encoding = match spec.kind {
        Code11 { checks } => {
            ratio = Some(spec.ratio);
            let e = Code11Encoder::new();
            modules(e, e.build(data, checks))?
        }
        Code93 => {
            let e = Code93Encoder::new();
            modules(e, e.build(data, true))?
        }
        Code49 { mode } => super::code49::encode(data, mode)?,
        Codablock { .. } => super::codablock::encode(spec, data)?,
        MicroPdf417 { variant } => {
            row_height = true;
            micro(data, Some(variant))?
        }
        Industrial2of5 | Standard2of5 => {
            ratio = Some(spec.ratio);
            let kind = if matches!(spec.kind, Industrial2of5) {
                Symbology::Std2of5
            } else {
                Symbology::Iata2of5
            };
            let e = TwoOf5Encoder::new();
            modules(e, e.build(kind, data))?
        }
        Codabar { start, stop } => {
            ratio = Some(spec.ratio);
            let e = CodabarEncoder::new();
            modules(e, e.build(start, data, stop))?
        }
        Logmars => {
            ratio = Some(spec.ratio);
            let e = Code39Encoder::new();
            modules(e, e.build(data, false, true))?
        }
        Msi { check } => {
            let check = match check {
                b'A' => MsiCheck::None,
                b'C' => MsiCheck::Mod1010,
                b'D' => MsiCheck::Mod1110,
                _ => MsiCheck::Mod10,
            };
            let e = MsiEncoder::new();
            modules(e, e.build_msi(data, check))?
        }
        Plessey => {
            let e = MsiEncoder::new();
            modules(e, e.build_plessey(data))?
        }
        Postal(code) => {
            postal = Some(code);
            let e = PostalEncoder::new();
            let s = match code {
                PostalCode::Postnet => e.build_postnet(&text),
                PostalCode::Planet => e.build_planet(&text),
                PostalCode::IntelligentMail => {
                    if !data.iter().all(u8::is_ascii_digit) || data.len() < 20 {
                        return Err("Invalid Intelligent Mail tracking number".into());
                    }
                    e.build_imb(&text[..20], &text[20..])
                }
            };
            modules(e, s)?
        }
        EanSupplement => {
            let e = EanEncoder::new();
            modules(
                e,
                if text.len() == 2 {
                    e.build_ean2(&text)
                } else {
                    e.build_ean5(&text)
                },
            )?
        }
        Tlc39 {
            micro_width,
            micro_height,
        } => {
            return Ok((
                super::tlc39::encode(spec, data, micro_width, micro_height)?,
                String::new(),
            ))
        }
        DataBar {
            variant,
            segments,
            separator,
        } => {
            if let Some((primary, secondary)) = text.split_once('|') {
                return Ok((
                    super::gs1_composite::encode(
                        spec, variant, segments, separator, primary, secondary,
                    )?,
                    String::new(),
                ));
            }
            let e = DataBarEncoder::new();
            if variant <= 6 {
                let gtin = data.strip_prefix(b"(01)").unwrap_or(data);
                let gtin = if gtin.len() == 16 && gtin.starts_with(b"01") {
                    &gtin[2..]
                } else {
                    gtin
                };
                let padded;
                let gtin = if variant <= 5 && gtin.len() < 13 && gtin.iter().all(u8::is_ascii_digit)
                {
                    padded = format!("{:0>13}", String::from_utf8_lossy(gtin));
                    padded.as_bytes()
                } else {
                    gtin
                };
                let built = match variant {
                    1 | 2 => e.build_omni(gtin),
                    3 => e.build_stacked(gtin),
                    4 => e.build_stacked_omni(gtin),
                    5 => e.build_limited(gtin),
                    _ => {
                        let reduced = super::gs1_composite::reduced(data)?;
                        if segments < 22 {
                            e.build_expanded_stacked(&reduced, segments / 2)
                        } else {
                            e.build_expanded(&reduced)
                        }
                    }
                };
                let result = modules(e, built)?;
                if matches!(&result, Encoding::Matrix(_)) {
                    row_height = true;
                    height = spec.module
                } else {
                    height = spec.module
                        * match variant {
                            1 => 33,
                            2 => 13,
                            5 => 10,
                            _ => 34,
                        };
                }
                result
            } else if variant >= 11 {
                use anyd::codes::code128::{Code128Encoder, Code128Input};
                let reduced = super::gs1_composite::reduced(data)?;
                let input: Vec<_> = reduced
                    .iter()
                    .map(|&b| {
                        if b == 29 {
                            Code128Input::Fnc1
                        } else {
                            Code128Input::Data(b)
                        }
                    })
                    .collect();
                let e = Code128Encoder::new();
                modules(e, e.build_gs1(&input))?
            } else {
                let e = EanEncoder::new();
                let built = match variant {
                    7 => e.build_upca(&text),
                    8 => e.build_upce(&text),
                    9 => e.build_ean13(&text),
                    10 => e.build_ean8(&text),
                    _ => {
                        return Err(
                            "GS1 composite components require a linked secondary field".into()
                        )
                    }
                };
                modules(e, built)?
            }
        }
    };
    caption.retain(|c| !c.is_control());
    match spec.kind {
        Code93 if spec.caption_checks => caption.push_str(&super::check_text::code93(data)),
        Msi { check } if spec.caption_checks => {
            caption.push_str(&super::check_text::msi(data, check))
        }
        Plessey if spec.caption_checks => caption.push_str(&super::check_text::plessey(data)),
        Logmars => caption.push(super::check_text::mod43(data)),
        _ => {}
    }
    let image = match (&spec.kind, &encoding) {
        (Code49 { .. } | Codablock { .. }, Encoding::Matrix(grid)) => stacked_bars(spec, grid)?,
        _ => raster(encoding, spec.module, height, ratio, postal, row_height)?,
    };
    Ok((image, caption))
}
