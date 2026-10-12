/// Convert text from a ZPL charset to Unicode.
/// Charsets 0-13 are variants of CP850, charset 27 is Windows-1252.
pub fn to_unicode_text(text: &str, charset: i32) -> Result<String, String> {
    match charset {
        0..=13 => {
            // For simplicity in MVP, treat as Latin-1 passthrough
            // A full CP850 decoder would be needed for 100% fidelity
            let result = cp850_to_unicode(text);
            if charset < 13 {
                Ok(apply_charset_substitutions(&result, charset as usize))
            } else {
                Ok(result)
            }
        }
        27 => {
            // Windows-1252 - treat bytes as Windows-1252
            Ok(windows1252_to_unicode(text))
        }
        _ => Ok(text.to_string()),
    }
}

fn cp850_to_unicode(text: &str) -> String {
    // CP850 maps bytes 0x80-0xFF to specific Unicode characters
    // For ASCII range (0x00-0x7F) this is identity
    // For simplicity, do byte-level passthrough treating input as Latin-1
    text.chars()
        .map(|c| {
            let b = c as u32;
            if b < 128 {
                c
            } else {
                // CP850 to Unicode mapping for common chars
                cp850_char(b as u8)
            }
        })
        .collect()
}

fn cp850_char(b: u8) -> char {
    // Subset of CP850 mappings for commonly used characters
    match b {
        0x80 => '\u{00C7}', // Ç
        0x81 => '\u{00FC}', // ü
        0x82 => '\u{00E9}', // é
        0x83 => '\u{00E2}', // â
        0x84 => '\u{00E4}', // ä
        0x85 => '\u{00E0}', // à
        0x86 => '\u{00E5}', // å
        0x87 => '\u{00E7}', // ç
        0x88 => '\u{00EA}', // ê
        0x89 => '\u{00EB}', // ë
        0x8A => '\u{00E8}', // è
        0x8B => '\u{00EF}', // ï
        0x8C => '\u{00EE}', // î
        0x8D => '\u{00EC}', // ì
        0x8E => '\u{00C4}', // Ä
        0x8F => '\u{00C5}', // Å
        0x90 => '\u{00C9}', // É
        0x91 => '\u{00E6}', // æ
        0x92 => '\u{00C6}', // Æ
        0x93 => '\u{00F4}', // ô
        0x94 => '\u{00F6}', // ö
        0x95 => '\u{00F2}', // ò
        0x96 => '\u{00FB}', // û
        0x97 => '\u{00F9}', // ù
        0x98 => '\u{00FF}', // ÿ
        0x99 => '\u{00D6}', // Ö
        0x9A => '\u{00DC}', // Ü
        0x9B => '\u{00F8}', // ø
        0x9C => '\u{00A3}', // £
        0x9D => '\u{00D8}', // Ø
        0x9F => '\u{0192}', // ƒ
        0xA0 => '\u{00E1}', // á
        0xA1 => '\u{00ED}', // í
        0xA2 => '\u{00F3}', // ó
        0xA3 => '\u{00FA}', // ú
        0xA4 => '\u{00F1}', // ñ
        0xA5 => '\u{00D1}', // Ñ
        0xA6 => '\u{00AA}', // ª (feminine ordinal indicator)
        0xA7 => '\u{00BA}', // º (masculine ordinal indicator)
        0xA8 => '\u{00BF}', // ¿ (inverted question mark)
        0xA9 => '\u{00AE}', // ® (registered sign)
        0xAA => '\u{00AC}', // ¬ (not sign)
        0xAB => '\u{00BD}', // ½ (vulgar fraction one half)
        0xAC => '\u{00BC}', // ¼ (vulgar fraction one quarter)
        0xAD => '\u{00A1}', // ¡ (inverted exclamation mark)
        0xAE => '\u{00AB}', // « (left double angle quotation mark)
        0xAF => '\u{00BB}', // » (right double angle quotation mark)
        0xB5 => '\u{00C1}', // Á
        0xD6 => '\u{00CE}', // Î
        _ => b as char,     // fallback
    }
}

static CHARACTER_SETS_013: [[&str; 11]; 14] = [
    ["#", "0", "@", "[", "\u{00A2}", "]", "^", "`", "{", "|", "}"],
    [
        "#", "0", "@", "\u{2153}", "\u{00A2}", "\u{2154}", "^", "`", "\u{00BC}", "\u{00BD}",
        "\u{00BE}",
    ],
    [
        "\u{00A3}", "0", "@", "[", "\u{00A2}", "]", "^", "`", "{", "|", "}",
    ],
    [
        "\u{0192}", "0", "\u{00A7}", "[", "IJ", "]", "^", "`", "{", "ij", "}",
    ],
    [
        "#", "0", "@", "\u{00C6}", "\u{00D8}", "\u{00C5}", "^", "`", "\u{00E6}", "\u{00F8}",
        "\u{00E5}",
    ],
    [
        "\u{00DC}", "0", "\u{00C9}", "\u{00C4}", "\u{00D6}", "\u{00C5}", "\u{00DC}", "\u{00E9}",
        "\u{00E4}", "\u{00F6}", "\u{00E5}",
    ],
    [
        "#", "0", "\u{00A7}", "\u{00C4}", "\u{00D6}", "\u{00DC}", "^", "`", "\u{00E4}", "\u{00F6}",
        "\u{00FC}",
    ],
    [
        "\u{00A3}", "0", "\u{00E0}", "[", "\u{00E7}", "]", "^", "`", "\u{00E9}", "|", "\u{00F9}",
    ],
    [
        "#", "0", "\u{00E0}", "\u{00E2}", "\u{00E7}", "\u{00EA}", "\u{00EE}", "\u{00F4}",
        "\u{00E9}", "\u{00F9}", "\u{00E8}",
    ],
    [
        "\u{00A3}", "0", "\u{00A7}", "[", "\u{00E7}", "\u{00E9}", "^", "\u{00F9}", "\u{00E0}",
        "\u{00F2}", "\u{00E8}",
    ],
    [
        "#", "0", "\u{00A7}", "\u{00A1}", "\u{00D1}", "\u{00BF}", "^", "`", "{", "\u{00F1}",
        "\u{00E7}",
    ],
    [
        "\u{00A3}", "0", "\u{00C9}", "\u{00C4}", "\u{00D6}", "\u{00DC}", "^", "\u{00E4}",
        "\u{00EB}", "\u{00EF}", "\u{00F6}",
    ],
    ["#", "0", "@", "[", "\u{00A5}", "]", "^", "`", "{", "|", "}"],
    ["#", "0", "@", "[", "\\", "]", "^", "`", "{", "|", "}"],
];

fn apply_charset_substitutions(text: &str, charset: usize) -> String {
    let search = &CHARACTER_SETS_013[13];
    let replace = &CHARACTER_SETS_013[charset];
    let mut result = text.to_string();
    for (i, s) in search.iter().enumerate() {
        result = result.replace(s, replace[i]);
    }
    result
}

fn windows1252_to_unicode(text: &str) -> String {
    // For ASCII, this is identity. For extended bytes, map Windows-1252.
    text.to_string()
}
