//! TSPL/TSPL2 Programming Manual, AZTEC (pp. 88–89). In the counted form
//! the byte following the tenth comma is already content, even if it is a
//! quote, a space or a command separator.
use super::parser::{counted_payload, is_escaped_quote_byte};
use crate::elements::{
    barcode_symbol::{BarcodeKind, BarcodeSymbol, BarcodeSymbolWithData},
    field_orientation::FieldOrientation,
    label_element::LabelElement,
    label_position::LabelPosition,
};

pub(super) struct Input<'a> {
    pub args: Vec<i32>,
    pub end: usize,
    data: &'a [u8],
    quoted: bool,
}

impl Input<'_> {
    pub fn payload(&self) -> Vec<u8> {
        if !self.quoted {
            return self.data.to_vec();
        }
        let mut result = Vec::with_capacity(self.data.len());
        let mut data = self.data;
        while let Some((&first, rest)) = data.split_first() {
            // Same quoted-literal escapes as TEXT, without decoding arbitrary bytes.
            if let Some(tail) = data
                .strip_prefix(br#"\["]"#)
                .or_else(|| data.strip_prefix(br#"\""#))
            {
                result.push(b'"');
                data = tail;
            } else {
                result.push(first);
                data = rest;
            }
        }
        result
    }
}

pub(super) fn split(data: &[u8], mut cursor: usize) -> Result<Input<'_>, String> {
    let mut args = Vec::<i32>::with_capacity(10);
    loop {
        // All ten numeric fields select counted data. Whitespace and quotes
        // in that payload are literal and PRINT/CLS inside it is not syntax.
        if args.len() == 10 {
            let len = usize::try_from(args.pop().unwrap())
                .map_err(|_| "TSPL AZTEC requires a nonnegative byte count")?;
            let end = cursor
                .checked_add(len)
                .filter(|&end| end <= data.len())
                .ok_or("TSPL AZTEC truncated counted payload")?;
            let (payload, next) = counted_payload(data, cursor, end);
            if data.get(next).is_some_and(|b| !matches!(b, b'\r' | b'\n')) {
                return Err("TSPL AZTEC unexpected bytes after counted payload".into());
            }
            return Ok(Input {
                args,
                end: next,
                data: &data[payload],
                quoted: false,
            });
        }

        while matches!(data.get(cursor), Some(b' ' | b'\t')) {
            cursor += 1;
        }
        if data.get(cursor) == Some(&b'"') {
            let close = (cursor + 1..data.len())
                .find(|&i| data[i] == b'"' && !is_escaped_quote_byte(data, i))
                .ok_or("TSPL AZTEC requires a closing quote")?;
            let content = &data[cursor + 1..close];
            let mut end = close + 1;
            while matches!(data.get(end), Some(b' ' | b'\t')) {
                end += 1;
            }
            if data.get(end).is_some_and(|b| !matches!(b, b'\r' | b'\n')) {
                return Err("TSPL AZTEC unexpected bytes after quoted payload".into());
            }
            if args.len() < 3 {
                return Err("TSPL AZTEC requires x, y and rotation before the data".into());
            }
            return Ok(Input {
                args,
                end,
                data: content,
                quoted: true,
            });
        }

        let start = cursor;
        while data
            .get(cursor)
            .is_some_and(|b| !matches!(b, b',' | b'\r' | b'\n'))
        {
            cursor += 1;
        }
        if data.get(cursor) != Some(&b',') {
            return Err("TSPL AZTEC requires quoted data or a byte count followed by data".into());
        }
        let value = std::str::from_utf8(data[start..cursor].trim_ascii())
            .ok()
            .and_then(|text| text.parse::<i32>().ok())
            .ok_or("TSPL AZTEC invalid numeric parameter")?;
        args.push(value);
        cursor += 1;
    }
}

pub(super) fn element(
    args: &[i32],
    mut data: Vec<u8>,
    position: LabelPosition,
) -> Result<LabelElement, String> {
    let orientation = match args[2] {
        0 => FieldOrientation::Normal,
        90 => FieldOrientation::Rotated90,
        180 => FieldOrientation::Rotated180,
        270 => FieldOrientation::Rotated270,
        _ => return Err("TSPL AZTEC rotation must be 0, 90, 180 or 270".into()),
    };
    let module = *args.get(3).unwrap_or(&6);
    if !(1..=20).contains(&module) {
        return Err("TSPL AZTEC module size must be 1 through 20".into());
    }
    let size = *args.get(4).unwrap_or(&0);
    if !matches!(size, 0..=99 | 101..=104 | 201..=232 | 300) {
        return Err("TSPL AZTEC invalid error correction or symbol size".into());
    }
    match args.get(5).copied().unwrap_or(0) {
        0 => {}
        1 => {
            let mut literal = Vec::with_capacity(data.len());
            let mut bytes = data.into_iter();
            while let Some(byte) = bytes.next() {
                if byte == 0x1b && bytes.next() != Some(0x1b) {
                    return Err("TSPL AZTEC FLG control sequences are unsupported".into());
                }
                literal.push(byte);
            }
            data = literal;
        }
        _ => return Err("TSPL AZTEC invalid flag mode".into()),
    }
    if args.get(6).copied().unwrap_or(0) != 0 {
        return Err("TSPL AZTEC menu symbols are unsupported".into());
    }
    // Studio exports zero for a single symbol. The manual examples use one.
    if !matches!(args.get(7), None | Some(0 | 1)) {
        return Err("TSPL AZTEC multi-symbol output is unsupported".into());
    }
    let inverted = match args.get(8).copied().unwrap_or(0) {
        0 => false,
        1 => true,
        _ => return Err("TSPL AZTEC reverse must be 0 or 1".into()),
    };
    if data.is_empty() {
        return Err("TSPL AZTEC requires nonempty data".into());
    }

    Ok(LabelElement::BarcodeSymbol(BarcodeSymbolWithData {
        symbol: BarcodeSymbol {
            kind: BarcodeKind::Aztec { size, inverted },
            module: module as u32,
            orientation,
            height: module as u32,
            ratio: 1.0,
            caption: false,
            caption_above: false,
            caption_checks: false,
        },
        data,
        position,
        reverse_print: Default::default(),
    }))
}
