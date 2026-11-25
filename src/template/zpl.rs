//! Labelize Studio's line-oriented ZPL marker format. A slot spans the marker
//! and its following field, leaving the field's native line ending untouched.
use std::borrow::Cow;

use super::{
    check_aztec_capacity, check_matrix_capacity, check_pdf_capacity, check_qr_capacity,
    normalize_linear, Encoder, Expression, Slot, TemplateUse,
};

const MARKER: &str = "^FXLabelize field ";
const DATA: &str = "^FH^FD";
const MAX_DATA: usize = 3072;

#[derive(Debug)]
pub(super) struct Field {
    commands: String,
    format: Format,
}

#[derive(Debug)]
enum Format {
    Text { block: bool, named_font: bool },
    Linear,
    Qr(u8),
    Pdf417 { columns: usize, security: u8 },
    DataMatrix { rectangle: bool },
    Aztec,
}

pub(super) fn load(bytes: &[u8]) -> Result<(Vec<u8>, Vec<Slot>), String> {
    let source = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in ZPL template")?;
    let mut slots = Vec::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let start = cursor;
        let (line, next) = read_line(source, cursor);
        cursor = next;
        let Some(marker) = line.strip_prefix(MARKER) else {
            continue;
        };
        let kind = marker
            .strip_suffix("^FS")
            .ok_or("Incomplete ZPL field marker")?;
        let (field_line, next) = read_line(source, cursor);
        let (field, expr) = Field::parse(kind, field_line)
            .map_err(|err| format!("ZPL field {} ({kind}): {err}", slots.len() + 1))?;
        slots.push(Slot {
            range: start..cursor + field_line.len(),
            usage: TemplateUse {
                object: slots.len() + 1,
                kind: kind.into(),
            },
            encoder: Encoder::Zpl(field),
            expr,
        });
        cursor = next;
    }
    Ok((bytes.to_vec(), slots))
}

// Accept CR, LF, and CRLF. Returning byte offsets preserves the original
// line endings and fixed bytes around each replaced field.
fn read_line(source: &str, start: usize) -> (&str, usize) {
    let tail = &source[start..];
    let end = tail.find(['\r', '\n']).unwrap_or(tail.len());
    let rest = &tail[end..];
    let ending = if rest.starts_with("\r\n") {
        2
    } else {
        usize::from(!rest.is_empty())
    };
    (&tail[..end], start + end + ending)
}

impl Field {
    fn parse(kind: &str, line: &str) -> Result<(Self, Expression), String> {
        let line = line
            .strip_suffix("^FS")
            .ok_or("Missing field terminator ^FS")?;
        let end = line.find(DATA).ok_or("Missing field data command ^FH^FD")? + DATA.len();
        let (commands, mut expr) = line.split_at(end);
        let format = match kind {
            "text" | "block" => Format::Text {
                block: kind == "block",
                named_font: commands.contains("^A@"),
            },
            "qr" => {
                let [ecc @ (b'L' | b'M' | b'Q' | b'H'), b'A', b',', ..] = expr.as_bytes() else {
                    return Err("Invalid QR correction or input-mode prefix".into());
                };
                expr = &expr[3..];
                Format::Qr(*ecc)
            }
            "pdf417" => {
                let (columns, security) =
                    pdf417_params(commands).ok_or("Invalid PDF417 parameters")?;
                Format::Pdf417 { columns, security }
            }
            "datamatrix" => Format::DataMatrix {
                rectangle: commands.ends_with(",|,2^FH^FD"),
            },
            "aztec" => Format::Aztec,
            kind if matches!(
                kind.strip_suffix("-caption").unwrap_or(kind),
                "code128" | "code39" | "ean8" | "ean13" | "upca" | "itf"
            ) =>
            {
                Format::Linear
            }
            _ => return Err(format!("Unknown ZPL template field type {kind}")),
        };
        let field = Self {
            commands: commands.to_owned(),
            format,
        };
        Ok((field, Expression::parse(expr.as_bytes())?))
    }

    pub(super) fn encode(&self, kind: &str, value: &str) -> Result<Vec<u8>, String> {
        if value.len() > MAX_DATA {
            return Err("Exceeds the 3072-byte ZPL field-data limit".into());
        }
        let mut commands = Cow::Borrowed(self.commands.as_str());
        let mut data = Cow::Borrowed(value);
        if !matches!(self.format, Format::Text { .. }) && value.is_empty() {
            return Err("Empty barcode value".into());
        }
        match self.format {
            Format::Text { block, named_font } => {
                if !named_font && value.chars().any(|ch| u32::from(ch) > 255) {
                    return Err("Set a printer font path in the design for Unicode text".into());
                }
                if block {
                    data = Cow::Owned(block_text(value));
                } else if value.contains(['\n', '\r']) {
                    return Err("Enable text wrapping for multiline values".into());
                }
            }
            Format::Linear => {
                let caption = kind.strip_suffix("-caption");
                let normalized =
                    normalize_linear(caption.unwrap_or(kind), value, caption.is_some())?;
                data = if caption.is_some() {
                    Cow::Owned(block_text(&normalized))
                } else if kind == "code128" && normalized.contains('>') {
                    // '>' invokes Code 128 subset/function codes in ^BC.
                    // Studio switches to explicit subsets for a literal '>'.
                    commands = Cow::Owned(commands.replace(",N,N,N,A^FH^FD", ",N,N,N,N^FH^FD"));
                    Cow::Owned(literal_code128(&normalized))
                } else {
                    Cow::Owned(normalized)
                };
            }
            Format::Qr(ecc) => check_qr_capacity(value, ecc)?,
            Format::Pdf417 { columns, security } => check_pdf_capacity(value, columns, security)?,
            Format::DataMatrix { rectangle } => {
                check_matrix_capacity(value, rectangle)?;
                data = Cow::Owned(value.replace('|', "||"));
            }
            Format::Aztec => check_aztec_capacity(value)?,
        }
        let mut bytes = commands.as_bytes().to_vec();
        let data_start = bytes.len();
        if let Format::Qr(ecc) = self.format {
            bytes.extend_from_slice(&[ecc, b'A', b',']);
        }
        field_hex(data.as_bytes(), &mut bytes);
        if bytes.len() - data_start > MAX_DATA {
            return Err("Exceeds the 3072-byte ZPL field-data limit".into());
        }
        bytes.extend_from_slice(b"^FS");
        Ok(bytes)
    }
}

fn pdf417_params(commands: &str) -> Option<(usize, u8)> {
    let (_, tail) = commands.split_once("^B7")?;
    let params = tail.split('^').next()?;
    let mut parts = params.split(',');
    if !matches!(parts.next()?, "N" | "R" | "I" | "B") {
        return None;
    }
    let height = parts.next()?;
    if height.is_empty() || !height.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let security = match parts.next()?.as_bytes() {
        [digit @ b'0'..=b'8'] => digit - b'0',
        _ => return None,
    };
    let columns = match parts.next()? {
        "" => 30,
        s if s.bytes().all(|b| b.is_ascii_digit()) => s.parse().ok()?,
        _ => return None,
    };
    if !(1..=30).contains(&columns) || !parts.eq(["", "N"]) {
        return None;
    }
    Some((columns, security))
}

fn literal_code128(text: &str) -> String {
    let mut encoded = String::from(">9");
    let mut set_a = true;
    for byte in text.bytes() {
        let uses_a = byte < 96;
        if uses_a != set_a {
            encoded.push_str(if uses_a { ">7" } else { ">6" });
            set_a = uses_a;
        }
        if uses_a {
            let code = if byte < 32 { byte + 64 } else { byte - 32 };
            encoded.push(char::from(b'0' + code / 10));
            encoded.push(char::from(b'0' + code % 10));
        } else {
            encoded.push(char::from(byte));
        }
    }
    encoded
}

fn block_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace("\r\n", "\n")
        .replace(['\r', '\n'], "\\&")
}

fn field_hex(data: &[u8], target: &mut Vec<u8>) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in data {
        if byte.is_ascii_graphic() && !b"^~_{}".contains(&byte) {
            target.push(byte);
        } else {
            target.extend_from_slice(&[b'_', HEX[(byte >> 4) as usize], HEX[(byte & 15) as usize]]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(kind: &str, commands: &str, value: &str) -> Result<String, String> {
        let (field, _) = Field::parse(kind, commands)?;
        Ok(String::from_utf8(field.encode(kind, value)?).unwrap())
    }

    #[test]
    fn byte_ranges_leave_fixed_graphics_and_line_endings_untouched() {
        let bytes = b"^XA\r\n^FO0,0^GFA,1,1,1,FF^FS\r\n^FXLabelize field text^FS\r\n^A0N,2,2^FH^FD{{v}}^FS\r\n^FD{{unmarked}}^FS\r\n^XZ";
        let (body, slots) = load(bytes).unwrap();
        assert_eq!(slots.len(), 1);
        let mut out = body;
        out.splice(
            slots[0].range.clone(),
            b"^A0N,2,2^FH^FDabc^FS".iter().copied(),
        );
        assert_eq!(
            &out,
            b"^XA\r\n^FO0,0^GFA,1,1,1,FF^FS\r\n^A0N,2,2^FH^FDabc^FS\r\n^FD{{unmarked}}^FS\r\n^XZ"
        );
    }

    #[test]
    fn block_and_command_specific_escaping() {
        assert_eq!(
            encode(
                "block",
                "^A@N,40,40,E:F.TTF^FB20,2^FH^FD{{v}}^FS",
                "\\q\r\n^~_{}茶"
            )
            .unwrap(),
            r"^A@N,40,40,E:F.TTF^FB20,2^FH^FD\\q\&_5E_7E_5F_7B_7D_E8_8C_B6^FS"
        );
        assert_eq!(
            encode("code128", "^BCN,40,N,N,N,A^FH^FD{{v}}^FS", "a>B").unwrap(),
            "^BCN,40,N,N,N,N^FH^FD>9>6a>73034^FS"
        );
        assert_eq!(
            encode("datamatrix", "^BXN,5,200,,,,|,2^FH^FD{{v}}^FS", "A|B").unwrap(),
            "^BXN,5,200,,,,|,2^FH^FDA||B^FS"
        );
        assert_eq!(
            encode("qr", "^BQN,2,4^FH^FDQA,{{v}}^FS", "0^").unwrap(),
            "^BQN,2,4^FH^FDQA,0_5E^FS"
        );
    }

    #[test]
    fn invalid_field_configuration_and_values() {
        assert!(load(b"^FXLabelize field text^FS\r").is_err());
        assert!(load(b"^FXLabelize field qr^FS\n^BQN,2,4^FH^FDX,{{v}}^FS").is_err());
        assert!(load(b"^FXLabelize field other^FS\n^FH^FD{{v}}^FS").is_err());
        assert!(encode("text", "^A0N,32,32^FH^FD{{v}}^FS", "茶").is_err());
        assert!(encode("text", "^A0N,32,32^FH^FD{{v}}^FS", "hello\nworld").is_err());
        assert!(encode("pdf417", "^B7N,12,8,1,,N^FH^FD{{v}}^FS", "12345").is_err());
        assert!(encode("pdf417", "^B7N,12,0,,,N^FH^FD{{v}}^FS", "ABC").is_ok());
        assert!(encode("pdf417", "^B7N,12,9,,,N^FH^FD{{v}}^FS", "12345").is_err());
        assert!(encode("text", "^A0N,32,32^FH^FD{{v}}^FS", &"^".repeat(1025)).is_err());
        assert!(encode("text", "^A0N,32,32^FH^FD{{v}}^FS", &"^".repeat(1024)).is_ok());
    }
}
