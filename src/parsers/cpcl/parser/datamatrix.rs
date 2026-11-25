//! CPCL for Link-OS, §6.2.3. Coordinates use session units, H uses printer dots,
//! and physical line breaks inside the data block contribute no bytes.

use super::{
    header_dpmm, orientation_for, parse_num, CpclLines, CpclSession, Flow, LabelElement,
    ReversePrint,
};
use crate::elements::barcode_datamatrix::{
    BarcodeDatamatrix, BarcodeDatamatrixWithData, DatamatrixRatio,
};

pub(super) fn parse(
    session: &mut CpclSession,
    header: &str,
    vertical: bool,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let mut fields = header.split_ascii_whitespace().skip(1);
    let x = parse_num(fields.next().ok_or("DATAMATRIX requires X and Y")?)?;
    let y = parse_num(fields.next().ok_or("DATAMATRIX requires X and Y")?)?;
    if !(0.0..=65535.0).contains(&x) || !(0.0..=65535.0).contains(&y) {
        return Err("CPCL DATAMATRIX coordinates must be in 0..=65535".into());
    }
    let mut scale = 6;
    let mut quality = 0;
    let mut rows = None;
    let mut columns = None;
    let mut escape = b'_';
    while let Some(key) = fields.next() {
        let value = fields
            .next()
            .ok_or_else(|| format!("CPCL DATAMATRIX missing value for {key}"))?;
        if key.eq_ignore_ascii_case("G") {
            let [byte] = *value.as_bytes() else {
                return Err("CPCL DATAMATRIX G requires a single byte".into());
            };
            if byte == 0 {
                return Err("CPCL DATAMATRIX G cannot be NUL".into());
            }
            escape = byte;
            continue;
        }
        let number: u16 = value
            .parse()
            .map_err(|_| format!("invalid CPCL DATAMATRIX {key} value {value:?}"))?;
        match key.to_ascii_uppercase().as_str() {
            "H" => scale = number.max(1),
            "S" => quality = number,
            "R" => rows = Some(number),
            "C" => columns = Some(number),
            "F" => {} // Only applies to legacy ECC.
            _ => return Err(format!("unsupported CPCL DATAMATRIX option {key}")),
        }
    }
    if quality != 200 {
        return Err(format!(
            "CPCL DATAMATRIX supports ECC 200 only (S {quality})"
        ));
    }
    if rows.zip(columns).is_some_and(|(r, c)| r != c) {
        return Err("CPCL DATAMATRIX requires a square symbol".into());
    }
    let size = i32::from(rows.or(columns).unwrap_or(0));
    let mut payload = Vec::new();
    loop {
        let line = lines
            .next_line_bytes()
            .ok_or("CPCL data block missing ENDDATAMATRIX")?;
        if line.eq_ignore_ascii_case(b"ENDDATAMATRIX") {
            break;
        }
        payload.extend_from_slice(line);
    }
    if !payload.is_empty() {
        let bytes = if payload.contains(&escape) {
            decode_payload(&payload, escape)?
        } else {
            payload
        };
        let height = (f64::from(scale) * session.dpmm / header_dpmm(session.vdpi))
            .round()
            .max(1.0) as i32;
        session
            .elements
            .push(LabelElement::BarcodeDatamatrix(BarcodeDatamatrixWithData {
                reverse_print: ReversePrint::default(),
                barcode: BarcodeDatamatrix {
                    orientation: orientation_for(vertical),
                    height,
                    quality: 200,
                    columns: size,
                    rows: size,
                    format: 6,
                    escape: 0,
                    ratio: Some(DatamatrixRatio::Square),
                },
                position: session.position(x, y),
                data: String::from_utf8_lossy(&bytes).into_owned(),
                data_bytes: Some(bytes),
            }));
    }
    Ok(Flow::Continue)
}

fn decode_payload(payload: &[u8], escape: u8) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(payload.len());
    let mut input = payload.iter().copied();
    while let Some(byte) = input.next() {
        if byte != escape {
            bytes.push(byte);
            continue;
        }
        let next = input.next().ok_or("incomplete CPCL DATAMATRIX escape")?;
        bytes.push(match next {
            literal if literal == escape => escape,
            b'@'..=b'_' => next - b'@',
            b'd' => {
                let mut value = 0u16;
                for _ in 0..3 {
                    let digit = input
                        .next()
                        .filter(u8::is_ascii_digit)
                        .ok_or("CPCL DATAMATRIX decimal escape requires three digits")?;
                    value = value * 10 + u16::from(digit - b'0');
                }
                u8::try_from(value).map_err(|_| "CPCL DATAMATRIX decimal escape exceeds 255")?
            }
            _ => return Err(format!("unsupported CPCL DATAMATRIX escape 0x{next:02x}")),
        });
    }
    Ok(bytes)
}
