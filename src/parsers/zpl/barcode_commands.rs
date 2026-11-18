use super::command_utils::{parse_int_scaled, to_field_orientation};
use super::virtual_printer::VirtualPrinter;
use crate::elements::barcode_symbol::{BarcodeKind, BarcodeSymbol, PostalCode};
use crate::elements::field_orientation::FieldOrientation;

pub(super) fn configuration(
    prefix: &[u8],
    parts: &[&str],
    printer: &VirtualPrinter,
) -> Option<BarcodeSymbol> {
    let defaults = &printer.default_barcode_dimensions;
    let dpmm = printer.dpmm;
    let scale = printer.unit_scale();
    let dimension = |i: usize, default: u32, low: u32, high: u32| {
        parts
            .get(i)
            .and_then(|p| parse_int_scaled(p, scale))
            .and_then(|dots| u32::try_from(dots).ok())
            .filter(|n| (low..=high).contains(n))
            .unwrap_or(default)
    };
    let letter = |i: usize| {
        parts
            .get(i)
            .and_then(|s| s.trim().bytes().next())
            .map(|b| b.to_ascii_uppercase())
    };
    let num = |i: usize, default: u32, low: u32, high: u32| {
        parts
            .get(i)
            .and_then(|p| p.trim().parse::<u32>().ok())
            .filter(|n| (low..=high).contains(n))
            .unwrap_or(default)
    };
    let yes = |i: usize, default: bool| match letter(i) {
        Some(b'Y') => true,
        Some(b'N') => false,
        _ => default,
    };
    let inherited_height = defaults.height.clamp(1, 32000) as u32;
    let mut module = defaults.module_width.clamp(1, 10) as u32;
    let mut height = dimension(1, inherited_height, 1, 32000);
    let mut caption = yes(2, true);
    let mut above = yes(3, false);
    let mut checks = false;
    let mut ratio = defaults.width_ratio;
    let mut orientation = letter(0)
        .map(to_field_orientation)
        .unwrap_or(printer.default_orientation);
    let kind = match prefix {
        b"^B1" | b"^BK" | b"^BM" | b"^BP" => {
            height = dimension(2, inherited_height, 1, 32000);
            caption = yes(3, true);
            above = yes(4, false);
            match prefix {
                b"^B1" => BarcodeKind::Code11 {
                    checks: if yes(1, false) { 1 } else { 2 },
                },
                b"^BK" => BarcodeKind::Codabar {
                    start: letter(5)
                        .filter(|c| matches!(c, b'A'..=b'D'))
                        .unwrap_or(b'A'),
                    stop: letter(6)
                        .filter(|c| matches!(c, b'A'..=b'D'))
                        .unwrap_or(b'A'),
                },
                b"^BM" => {
                    checks = yes(5, false);
                    BarcodeKind::Msi {
                        check: letter(1).unwrap_or(b'B'),
                    }
                }
                _ => {
                    checks = yes(1, false);
                    BarcodeKind::Plessey
                }
            }
        }
        b"^B4" => {
            caption = matches!(letter(2), Some(b'A' | b'B'));
            above = letter(2) == Some(b'A');
            height = num(1, inherited_height, 1, 32000).saturating_mul(module);
            BarcodeKind::Code49 {
                mode: letter(3).filter(|b| b.is_ascii_digit()).map(|b| b - b'0'),
            }
        }
        b"^BA" => {
            checks = yes(4, false);
            BarcodeKind::Code93
        }
        b"^BB" => {
            caption = false;
            let height_is_total =
                printer.barcode_height_explicit && parts.get(1).is_none_or(|p| p.is_empty());
            height = if height_is_total {
                inherited_height
            } else {
                num(1, 8, 2, 32000).saturating_mul(module)
            };
            orientation = letter(0).map(to_field_orientation).unwrap_or_default();
            BarcodeKind::Codablock {
                mode: letter(5).unwrap_or(b'F'),
                columns: num(3, 0, 2, 62) as usize,
                rows: num(4, 0, 1, 44) as usize,
                checksum: yes(2, true),
                height_is_total,
            }
        }
        b"^BF" => {
            caption = false;
            BarcodeKind::MicroPdf417 {
                variant: num(2, 0, 0, 33) as u8,
            }
        }
        b"^BI" => BarcodeKind::Industrial2of5,
        b"^BJ" => BarcodeKind::Standard2of5,
        b"^BL" => {
            caption = true;
            above = yes(2, false);
            BarcodeKind::Logmars
        }
        b"^BR" => {
            caption = false;
            module = num(
                2,
                if dpmm >= 24 {
                    6
                } else if dpmm > 8 {
                    3
                } else {
                    2
                },
                1,
                10,
            );
            height = dimension(4, 25, 1, 32000);
            orientation = letter(0)
                .map(to_field_orientation)
                .unwrap_or(FieldOrientation::Rotated90);
            BarcodeKind::DataBar {
                variant: num(1, 1, 1, 12) as u8,
                segments: match num(5, 22, 2, 22) {
                    n if n % 2 == 0 => n as usize,
                    _ => 22,
                },
                separator: num(3, 1, 1, 2),
            }
        }
        b"^BS" => {
            above = yes(3, true);
            BarcodeKind::EanSupplement
        }
        b"^BT" => {
            caption = false;
            let hd = dpmm >= 24;
            module = dimension(1, if hd { 4 } else { 2 }, 1, 10);
            ratio = parts
                .get(2)
                .and_then(|s| s.parse().ok())
                .filter(|r| (2.0..=3.0).contains(r))
                .unwrap_or(2.0);
            height = dimension(
                3,
                if hd {
                    120
                } else if dpmm >= 12 {
                    60
                } else {
                    40
                },
                1,
                9999,
            );
            BarcodeKind::Tlc39 {
                micro_width: dimension(4, if hd { 4 } else { 2 }, 1, 10),
                micro_height: dimension(5, if hd { 8 } else { 4 }, 1, 255),
            }
        }
        b"^B5" | b"^BZ" => {
            caption = yes(2, false);
            BarcodeKind::Postal(if prefix == b"^B5" || num(4, 0, 0, 3) == 1 {
                PostalCode::Planet
            } else if num(4, 0, 0, 3) == 3 {
                PostalCode::IntelligentMail
            } else {
                PostalCode::Postnet
            })
        }
        _ => return None,
    };
    Some(BarcodeSymbol {
        kind,
        module,
        height,
        ratio,
        orientation,
        caption,
        caption_above: above,
        caption_checks: checks,
    })
}

/// Interpret the four printable shift markers used in a Code 93 field.
pub(super) fn field_bytes(kind: &BarcodeKind, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if !matches!(kind, BarcodeKind::Code93) {
        return Ok(bytes.to_vec());
    }
    let mut iter = bytes.iter().copied();
    let mut out = Vec::with_capacity(bytes.len());
    while let Some(b) = iter.next() {
        if let Some(shift) = b"&'()".iter().position(|&c| c == b) {
            let ch = iter
                .next()
                .and_then(|c| crate::barcodes::check_text::code93_pair(shift as u8, c))
                .ok_or("Invalid Code 93 shift pair")?;
            out.push(ch);
        } else {
            out.push(b);
        }
    }
    Ok(out)
}
