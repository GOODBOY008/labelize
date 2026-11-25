//! Character decoding for declared printer text fields. Opaque payloads
//! are consumed separately by the individual language's byte cursor.
use std::borrow::Cow;

use encoding_rs::{GB18030, GBK};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum TextEncoding {
    #[default]
    Unspecified,
    Utf8,
    Gbk,
    Gb18030,
}

impl TextEncoding {
    pub(super) fn decode(self, bytes: &[u8]) -> Result<Cow<'_, str>, &'static str> {
        let (encoding, error) = match self {
            Self::Unspecified => return Ok(String::from_utf8_lossy(bytes)),
            Self::Utf8 => {
                return std::str::from_utf8(bytes)
                    .map(Cow::Borrowed)
                    .map_err(|_| "invalid UTF-8 text")
            }
            Self::Gbk => (GBK, "invalid GBK (CODEPAGE 936) text"),
            Self::Gb18030 => (GB18030, "invalid GB18030 text"),
        };

        // encoding_rs shares the WHATWG decoder for both encodings. Restrict
        // four-byte characters to GB18030 and the standalone 0x80 to CP936.
        let mut remaining = bytes;
        while !remaining.is_empty() {
            remaining = &remaining[self.gb_char_len(remaining).ok_or(error)?..];
        }
        encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or(error)
    }

    pub(super) fn gb_char_len(self, bytes: &[u8]) -> Option<usize> {
        match bytes {
            [0x00..=0x7f, ..] => Some(1),
            [0x80, ..] if self == Self::Gbk => Some(1),
            [0x81..=0xfe, 0x40..=0x7e | 0x80..=0xfe, ..] => Some(2),
            [0x81..=0xfe, b'0'..=b'9', 0x81..=0xfe, b'0'..=b'9', ..] if self == Self::Gb18030 => {
                Some(4)
            }
            _ => None,
        }
    }
}
