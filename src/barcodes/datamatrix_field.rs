//! ZPL ECC 200 field-data escapes, evaluated after ^FH.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Token {
    Byte(u8),
    Codewords(Vec<u8>),
}

pub(super) fn parse(data: &[u8], escape: u8) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] != escape {
            tokens.push(Token::Byte(data[i]));
            i += 1;
            continue;
        }
        let command = *data
            .get(i + 1)
            .ok_or("DataMatrix: incomplete escape sequence")?;
        i += 2;
        if command == escape {
            tokens.push(Token::Byte(escape));
            continue;
        }
        match command {
            b'd' => {
                let value = decimal(data, &mut i)?;
                let byte =
                    u8::try_from(value).map_err(|_| "DataMatrix: decimal escape exceeds 255")?;
                tokens.push(Token::Byte(byte));
            }
            b'0' => tokens.push(Token::Codewords(vec![129])),
            b'1' => tokens.push(Token::Codewords(vec![232])),
            b'2' => {
                let mut words = vec![233];
                for _ in 0..3 {
                    let value = decimal(data, &mut i)?;
                    if !(1..=254).contains(&value) {
                        return Err("DataMatrix: FNC2 values must be 001 through 254".into());
                    }
                    words.push(value as u8);
                }
                tokens.push(Token::Codewords(words));
            }
            b'3' => tokens.push(Token::Codewords(vec![234])),
            b'5' => {
                let value = decimal(data, &mut i)?;
                let words = if value <= 126 {
                    vec![241, (value + 1) as u8]
                } else {
                    let adjusted = value - 127;
                    vec![
                        241,
                        (adjusted / 254 + 128) as u8,
                        (adjusted % 254 + 1) as u8,
                    ]
                };
                tokens.push(Token::Codewords(words));
            }
            b'@'..=b'_' => tokens.push(Token::Byte(command - b'@')),
            _ => {
                return Err(format!(
                    "DataMatrix: unsupported escape command 0x{command:02X}"
                ))
            }
        }
    }
    Ok(tokens)
}

fn decimal(data: &[u8], i: &mut usize) -> Result<u16, String> {
    let digits = data
        .get(*i..*i + 3)
        .ok_or("DataMatrix: escape requires three decimal digits")?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return Err("DataMatrix: escape requires three decimal digits".into());
    }
    *i += 3;
    Ok(digits
        .iter()
        .fold(0u16, |value, digit| value * 10 + u16::from(digit - b'0')))
}

/// ASCII encodation keeps function codewords at their exact data positions.
/// Numeric pairs are compacted only when both adjacent tokens are data bytes.
pub(super) fn ascii_codewords(tokens: &[Token]) -> Vec<u8> {
    let mut words = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Codewords(special) => words.extend_from_slice(special),
            Token::Byte(byte) => {
                if byte.is_ascii_digit() {
                    if let Some(Token::Byte(next)) = tokens.get(i + 1) {
                        if next.is_ascii_digit() {
                            words.push(130 + 10 * (byte - b'0') + (next - b'0'));
                            i += 2;
                            continue;
                        }
                    }
                }
                if *byte <= 127 {
                    words.push(byte + 1);
                } else {
                    words.extend_from_slice(&[235, byte - 127]);
                }
            }
        }
        i += 1;
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_preserve_bytes_and_function_tokens() {
        assert_eq!(
            parse(b"A__B_@_G_d029_d128_d255_1", b'_').unwrap(),
            vec![
                Token::Byte(b'A'),
                Token::Byte(b'_'),
                Token::Byte(b'B'),
                Token::Byte(0),
                Token::Byte(7),
                Token::Byte(29),
                Token::Byte(128),
                Token::Byte(255),
                Token::Codewords(vec![232]),
            ]
        );
        assert_eq!(
            parse(b"#1A##B#d029", b'#').unwrap(),
            parse(b"_1A#B_d029", b'_').unwrap()
        );
    }

    #[test]
    fn fnc1_is_not_an_ordinary_group_separator() {
        let tokens = parse(b"_112_134_d02956", b'_').unwrap();
        assert_eq!(ascii_codewords(&tokens), [232, 142, 232, 164, 30, 186]);
    }

    #[test]
    fn function_and_eci_codewords_are_explicit() {
        assert_eq!(
            ascii_codewords(&parse(b"_0_2214001001_3_5009_5127", b'_').unwrap()),
            [129, 233, 214, 1, 1, 234, 241, 10, 241, 128, 1]
        );
    }

    #[test]
    fn malformed_escapes_fail_without_literal_fallback() {
        for data in [
            "_",
            "_d12",
            "_d256",
            "_dX01",
            "_4",
            "_2",
            "_2000001001",
            "_2255001001",
            "_5x09",
        ] {
            assert!(parse(data.as_bytes(), b'_').is_err(), "{data}");
        }
    }

    #[test]
    fn every_decimal_byte_survives_parsing_and_ascii_encoding() {
        for byte in 0..=255u8 {
            let tokens = parse(format!("_d{byte:03}").as_bytes(), b'_').unwrap();
            assert_eq!(tokens, [Token::Byte(byte)]);
            let expected = if byte < 128 {
                vec![byte + 1]
            } else {
                vec![235, byte - 127]
            };
            assert_eq!(ascii_codewords(&tokens), expected);
        }
    }
}
