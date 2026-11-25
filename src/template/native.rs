//! Studio's LABELIZE-TPL/1 directory describes byte ranges in an opaque printer job.
use super::{
    check_aztec_capacity, check_matrix_capacity, check_pdf_capacity, check_qr_capacity,
    normalize_linear, Encoder, Expression, Slot, TemplateUse,
};
use crate::Language;
use std::borrow::Cow;

#[derive(Debug)]
pub(super) struct Field {
    ascii_only: bool,
    content: Content,
    codec: Codec,
}

#[derive(Debug)]
enum Content {
    Text,
    Linear,
    Qr(u8),
    DataMatrix { rectangle: bool },
    Pdf417 { columns: usize, security: u8 },
    Aztec,
}

impl Content {
    fn parse(kind: &str, params: &str) -> Result<Self, String> {
        match (kind.strip_suffix("-caption").unwrap_or(kind), params) {
            ("code128" | "code39" | "ean13" | "ean8" | "upca" | "itf", "-") => Ok(Self::Linear),
            _ if kind.ends_with("-caption") => Err(format!("Invalid caption {kind}")),
            ("text", "-") => Ok(Self::Text),
            ("aztec", "-") => Ok(Self::Aztec),
            ("qr", "L" | "M" | "Q" | "H") => Ok(Self::Qr(params.as_bytes()[0])),
            ("datamatrix", "square" | "rectangle") => Ok(Self::DataMatrix {
                rectangle: params == "rectangle",
            }),
            ("pdf417", _) => {
                if let Some((columns, level)) = params.split_once(',') {
                    let valid_columns =
                        !columns.starts_with('0') && columns.bytes().all(|b| b.is_ascii_digit());
                    if let (true, Ok(columns @ 1..=30), [security @ b'0'..=b'8']) =
                        (valid_columns, columns.parse::<usize>(), level.as_bytes())
                    {
                        return Ok(Self::Pdf417 {
                            columns,
                            security: security - b'0',
                        });
                    }
                }
                Err(format!(
                    "Invalid PDF417 columns or correction level: {params}"
                ))
            }
            _ => Err(format!("Invalid field kind or parameters: {kind} {params}")),
        }
    }
}

#[derive(Debug)]
enum Codec {
    TsplQuoted,
    EplQuoted,
    Line,
    TsplPdfLength,
    TsplAztecCount,
    CpclMatrix,
    CpclTerminated(&'static str),
    DplCount(String),
    DplAztec,
}

impl Codec {
    fn parse(language: Language, kind: &str, codec: &str) -> Result<Self, String> {
        let invalid = || format!("Invalid {} field codec {codec} for {kind}", language.name());
        let (name, encoding) = match (language, kind) {
            (Language::Tspl, "pdf417") => ("tspl-length", Self::TsplPdfLength),
            (Language::Tspl, "aztec") => ("aztec-count", Self::TsplAztecCount),
            (Language::Tspl, _) => ("tspl", Self::TsplQuoted),
            (Language::Epl, _) => ("epl", Self::EplQuoted),
            (Language::Cpcl, "datamatrix") => ("dm-line", Self::CpclMatrix),
            (Language::Cpcl, "pdf417") => ("pdf-line", Self::CpclTerminated("ENDPDF")),
            (Language::Cpcl, "aztec") => ("aztec-line", Self::CpclTerminated("ENDAZTEC")),
            (Language::Cpcl, _) => ("line", Self::Line),
            // Earlier Studio exports labelled W1f fields with the generic line codec.
            (Language::Dpl, "aztec") if codec == "line" => return Ok(Self::DplAztec),
            (Language::Dpl, "aztec") => ("dpl-aztec", Self::DplAztec),
            (Language::Dpl, "datamatrix" | "pdf417") => {
                let options = codec.strip_prefix("count4:").ok_or_else(invalid)?;
                let valid = match (kind, options.as_bytes()) {
                    ("datamatrix", [b'2', b'0', b'0', digits @ ..]) => {
                        digits.len() == 7 && digits.iter().all(u8::is_ascii_digit)
                    }
                    ("pdf417", [b'F', b'0'..=b'8', digits @ ..]) => {
                        digits.len() == 6 && digits.iter().all(u8::is_ascii_digit)
                    }
                    _ => false,
                };
                return if valid {
                    Ok(Self::DplCount(options.to_owned()))
                } else {
                    Err(invalid())
                };
            }
            (Language::Dpl, _) => ("line", Self::Line),
            (Language::Zpl, _) => return Err(invalid()),
        };
        if codec == name {
            Ok(encoding)
        } else {
            Err(invalid())
        }
    }
}

pub(super) fn load(language: Language, bytes: &[u8]) -> Result<(Vec<u8>, Vec<Slot>), String> {
    let split = bytes
        .windows(2)
        .position(|w| w == b"\n\n")
        .ok_or("Missing LABELIZE-TPL/1 field directory")?;
    let (directory, rest) = bytes.split_at(split);
    let body = &rest[2..];
    if !directory.is_ascii() {
        return Err("The field directory must be ASCII".into());
    }
    let mut lines = std::str::from_utf8(directory)
        .expect("ASCII directory")
        .split('\n');
    let (target, charset) = lines
        .next()
        .and_then(|header| header.strip_prefix("LABELIZE-TPL/1 "))
        .and_then(|data| data.split_once(' '))
        .ok_or("Invalid LABELIZE-TPL/1 header")?;
    if target != language.name() || language == Language::Zpl {
        return Err(format!(
            "Template language {target} does not match {}",
            language.name()
        ));
    }
    let ascii_only = match charset {
        "US-ASCII" => true,
        "UTF-8" if matches!(language, Language::Tspl | Language::Cpcl) => false,
        _ => return Err(format!("Unsupported {target} template encoding: {charset}")),
    };
    let mut slots = Vec::new();
    let mut previous_end = 0;
    for (i, line) in lines.enumerate() {
        let line_number = i + 2;
        let columns: Vec<_> = line.splitn(7, ' ').collect();
        let &[offset, length, object, kind, codec, params] = columns.as_slice() else {
            return Err(format!("Invalid field directory at line {line_number}"));
        };
        let number = |s: &str| {
            s.parse::<i32>()
                .ok()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| {
                    format!("Invalid field offset, length or object at line {line_number}")
                })
        };
        let (start, length, object) = (number(offset)?, number(length)?, number(object)?);
        // Directory numbers are nonnegative i32, so the sum fits even on 32-bit targets.
        let end = start + length;
        if object == 0 || length == 0 || start < previous_end || end > body.len() {
            return Err(format!(
                "Invalid or overlapping field range at line {line_number}"
            ));
        }
        let content = Content::parse(kind, params)?;
        let codec = Codec::parse(language, kind, codec)?;
        let expr = Expression::parse(&body[start..end])
            .map_err(|error| format!("Invalid field expression at line {line_number}: {error}"))?;
        slots.push(Slot {
            range: start..end,
            usage: TemplateUse {
                object,
                kind: kind.to_owned(),
            },
            expr,
            encoder: Encoder::Native(Field {
                ascii_only,
                content,
                codec,
            }),
        });
        previous_end = end;
    }
    Ok((body.to_vec(), slots))
}

impl Field {
    pub(super) fn encode(&self, kind: &str, value: &str) -> Result<Vec<u8>, String> {
        if value.len() > 4096 {
            return Err("Filled content exceeds 4096 bytes".into());
        }
        if self.ascii_only && !value.is_ascii() {
            return Err("Value cannot be encoded as US-ASCII".into());
        }
        if !matches!(
            self.codec,
            Codec::TsplPdfLength | Codec::TsplAztecCount | Codec::CpclMatrix
        ) && value
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
        {
            return Err("Control or line separator in a printer field".into());
        }
        if !matches!(self.content, Content::Text) && value.is_empty() {
            return Err("Empty barcode value".into());
        }
        let mut data = Cow::Borrowed(value);
        match self.content {
            Content::Text => {}
            Content::Linear => {
                let base = kind.strip_suffix("-caption");
                data = Cow::Owned(normalize_linear(
                    base.unwrap_or(kind),
                    value,
                    base.is_some(),
                )?);
            }
            Content::Qr(correction) => check_qr_capacity(value, correction)?,
            Content::DataMatrix { rectangle } => check_matrix_capacity(value, rectangle)?,
            Content::Pdf417 { columns, security } => check_pdf_capacity(value, columns, security)?,
            Content::Aztec => check_aztec_capacity(value)?,
        }
        let n = data.len();
        let encoded = match &self.codec {
            Codec::TsplQuoted => data.replace('"', r#"\["]"#).into_bytes(),
            Codec::EplQuoted => data.replace('\\', "\\\\").replace('"', "\\\"").into_bytes(),
            Codec::Line => data.as_bytes().to_vec(),
            Codec::TsplPdfLength => format!("L{n},{data}").into_bytes(),
            Codec::TsplAztecCount => format!("{n},{data}").into_bytes(),
            Codec::DplAztec => format!("0000{data}").into_bytes(),
            Codec::DplCount(options) => {
                // DPL counts its options too. Options + the 4096-byte value fit four digits.
                format!("{:04}{options}{data}", n + options.len()).into_bytes()
            }
            Codec::CpclTerminated(tag) => {
                if is_cpcl_terminator(&data, tag) {
                    return Err(format!("Value is the CPCL {tag} terminator"));
                }
                data.as_bytes().to_vec()
            }
            Codec::CpclMatrix => cpcl_matrix_data(&data),
        };
        Ok(encoded)
    }
}

// Match Studio's whitespace and case rules for end records. Non-breaking
// spaces remain data. The I in ENDDATAMATRIX also matches ı and İ.
fn is_cpcl_terminator(value: &str, tag: &str) -> bool {
    let value = value.trim_matches(|c: char| {
        matches!(c, '\u{1c}'..='\u{1f}')
            || (c.is_whitespace() && !matches!(c, '\u{85}' | '\u{a0}' | '\u{2007}' | '\u{202f}'))
    });
    value
        .chars()
        .map(|c| match c {
            'ı' | 'İ' => 'I',
            c => c.to_ascii_uppercase(),
        })
        .eq(tag.chars())
}

fn cpcl_matrix_data(value: &str) -> Vec<u8> {
    let end_tag = is_cpcl_terminator(value, "ENDDATAMATRIX");
    let mut bytes = Vec::with_capacity(value.len());
    for (i, b) in value.bytes().enumerate() {
        if b.is_ascii_control() || (end_tag && i == 0) {
            bytes.extend_from_slice(&[
                b'_',
                b'd',
                b'0' + b / 100,
                b'0' + (b / 10) % 10,
                b'0' + b % 10,
            ]);
        } else {
            if b == b'_' {
                bytes.push(b'_');
            }
            bytes.push(b);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        language: Language,
        kind: &str,
        codec: &str,
        params: &str,
        ascii_only: bool,
        value: &str,
    ) -> Result<Vec<u8>, String> {
        let field = Field {
            ascii_only,
            content: Content::parse(kind, params)?,
            codec: Codec::parse(language, kind, codec)?,
        };
        field.encode(kind, value)
    }

    #[test]
    fn printer_encodings_handle_delimiters_and_payload_lengths() {
        use Language::*;
        let cases = [
            (Tspl, "text", "tspl", "-", r#"a"b\c"#, r#"a\["]b\c"#),
            (Epl, "text", "epl", "-", r#"a"b\c"#, r#"a\"b\\c"#),
            (
                Tspl,
                "pdf417",
                "tspl-length",
                "3,2",
                ",A\r\nB",
                "L5,,A\r\nB",
            ),
            (Tspl, "aztec", "aztec-count", "-", ",杯\n", "5,,杯\n"),
            (
                Cpcl,
                "datamatrix",
                "dm-line",
                "square",
                "A_\n\x7f",
                "A___d010_d127",
            ),
            (
                Cpcl,
                "datamatrix",
                "dm-line",
                "rectangle",
                "eNdDaTaMaTrIx",
                "_d101NdDaTaMaTrIx",
            ),
            (
                Dpl,
                "datamatrix",
                "count4:2000000000",
                "square",
                "00123",
                "0015200000000000123",
            ),
            (
                Dpl,
                "pdf417",
                "count4:F2000201",
                "3,2",
                "00123",
                "0013F200020100123",
            ),
            (Dpl, "aztec", "line", "-", "000123", "0000000123"),
            (Dpl, "aztec", "dpl-aztec", "-", "000123", "0000000123"),
        ];
        for (language, kind, codec, params, value, expected) in cases {
            let actual = encode(language, kind, codec, params, false, value).unwrap();
            assert_eq!(actual, expected.as_bytes(), "{language:?} {codec}");
        }
    }

    #[test]
    fn unicode_end_record_detection_matches_studio() {
        assert_eq!(
            cpcl_matrix_data("ENDDATAMATRİX"),
            "_d069NDDATAMATRİX".as_bytes()
        );
        let padded = "\u{a0}ENDDATAMATRIX\u{a0}";
        assert_eq!(cpcl_matrix_data(padded), padded.as_bytes());
        assert_eq!(
            encode(
                Language::Cpcl,
                "aztec",
                "aztec-line",
                "-",
                false,
                "\u{a0}ENDAZTEC"
            )
            .unwrap(),
            "\u{a0}ENDAZTEC".as_bytes(),
        );
    }

    #[test]
    fn printer_fields_report_invalid_records() {
        use Language::*;
        for (language, kind, codec, params, ascii_only, value) in [
            (Dpl, "text", "line", "-", true, "保温杯"),
            (Tspl, "text", "tspl", "-", false, "line\nPRINT 2"),
            (Cpcl, "aztec", "aztec-line", "-", true, " endaztec "),
            (Cpcl, "pdf417", "pdf-line", "2,3", false, " EndPdf"),
            (Tspl, "pdf417", "tspl-length", "2,3", false, "杯"),
            (Epl, "ean13-caption", "epl", "-", true, "4006381333930"),
            (Epl, "qr", "epl", "M", true, ""),
        ] {
            assert!(
                encode(language, kind, codec, params, ascii_only, value).is_err(),
                "{language:?} {kind}: {value:?}"
            );
        }
    }

    #[test]
    fn directory_rejects_invalid_ranges_kinds_and_codec_metadata() {
        for row in [
            "-1 5 1 text line -",
            "0 0 1 text line -",
            "0 5 0 text line -",
            "5 5 1 text line -",
            "2147483647 2147483647 1 text line -",
            "0 5 1 text line -\n3 2 2 text line -",
            "0 5 1 text line - unexpected",
            "0 5 1 qr-caption line M",
            "0 5 1 another line -",
            "0 5 1 qr line X",
            "0 5 1 text tspl -",
            "0 5 1 datamatrix count4:200000000X rectangle",
            "0 5 1 pdf417 count4:F9000000 30,8",
            "0 5 1 pdf417 count4:F8000000 31,8",
            "0 5 1 pdf417 count4:F8000000 01,8",
            "0 5 1 pdf417 count4:F8000000 1,9",
        ] {
            let template = format!("LABELIZE-TPL/1 dpl US-ASCII\n{row}\n\n{{{{v}}}}");
            assert!(load(Language::Dpl, template.as_bytes()).is_err(), "{row}");
        }
        for header in [
            "LABELIZE-TPL/2 dpl US-ASCII",
            "LABELIZE-TPL/1 cpcl US-ASCII",
            "LABELIZE-TPL/1 dpl UTF-8",
        ] {
            let template = format!("{header}\n0 5 1 text line -\n\n{{{{v}}}}");
            assert!(
                load(Language::Dpl, template.as_bytes()).is_err(),
                "{header}"
            );
        }
    }

    #[test]
    fn binary_body_is_opaque_outside_declared_ranges() {
        let directory = b"LABELIZE-TPL/1 epl US-ASCII\n7 5 19 text epl -\n\n";
        let body = b"\x00\xff{{x}}{{v}}\r\n\xff\0";
        let (fixed, slots) = load(Language::Epl, &[directory.as_slice(), body].concat()).unwrap();
        assert_eq!(fixed, body);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].range, 7..12);
        assert_eq!(slots[0].usage.object, 19);
    }
}
