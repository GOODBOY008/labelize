//! Encoders for additional linear, stacked and postal symbols.
use crate::elements::barcode_symbol::{BarcodeKind, PostalCode};
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

/// Logical modules. Linked symbols keep their independently sized components.
pub(crate) enum EncodedBars {
    Single(Encoding),
    Linked39 {
        linear: anyd::output::LinearPattern,
        secondary: anyd::output::BitMatrix,
        flag: [u8; 9],
    },
    Composite {
        modules: bwipp::BitMatrix,
        linear_start: usize,
    },
}

pub(super) fn code39_modules(data: &[u8]) -> Result<anyd::output::LinearPattern, String> {
    let e = Code39Encoder::new();
    let Encoding::Linear(row) = modules(e, e.build(data, false, false))? else {
        unreachable!()
    };
    Ok(row)
}

pub(crate) fn encode(
    kind: &BarcodeKind,
    data: &[u8],
    caption_checks: bool,
) -> Result<(EncodedBars, String), String> {
    use BarcodeKind::*;
    // Even the widest stacked format cannot hold more than 44 rows of
    // 62 codewords. Reject oversized input before entering an encoder.
    if data.len() > 8192 {
        return Err("Barcode data exceeds the supported symbol size".into());
    }
    let text = String::from_utf8_lossy(data);
    let mut caption = text.to_string();
    let encoding = match *kind {
        Aztec { size, inverted } => Encoding::Matrix(super::aztec::modules(data, size, inverted)?),
        Code11 { checks } => {
            let e = Code11Encoder::new();
            modules(e, e.build(data, checks))?
        }
        Code93 => {
            let e = Code93Encoder::new();
            modules(e, e.build(data, true))?
        }
        Code49 { mode } => super::code49::encode(data, mode)?,
        Codablock {
            mode,
            columns,
            rows,
            checksum,
            ..
        } => super::codablock::encode(mode, columns, rows, checksum, data)?,
        MicroPdf417 { variant } => micro(data, Some(variant))?,
        Industrial2of5 | Standard2of5 => {
            let kind = if matches!(kind, Industrial2of5) {
                Symbology::Std2of5
            } else {
                Symbology::Iata2of5
            };
            let e = TwoOf5Encoder::new();
            modules(e, e.build(kind, data))?
        }
        Codabar { start, stop } => {
            let e = CodabarEncoder::new();
            modules(e, e.build(start, data, stop))?
        }
        Logmars => {
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
        Tlc39 { .. } => return Ok((super::tlc39::encode(data)?, String::new())),
        DataBar {
            variant, segments, ..
        } => {
            if let Some((primary, secondary)) = text.split_once('|') {
                return Ok((
                    super::gs1_composite::encode(variant, segments, primary, secondary)?,
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
                modules(e, built)?
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
    match *kind {
        Code93 if caption_checks => caption.push_str(&super::check_text::code93(data)),
        Msi { check } if caption_checks => caption.push_str(&super::check_text::msi(data, check)),
        Plessey if caption_checks => caption.push_str(&super::check_text::plessey(data)),
        Logmars => caption.push(super::check_text::mod43(data)),
        _ => {}
    }
    Ok((EncodedBars::Single(encoding), caption))
}
