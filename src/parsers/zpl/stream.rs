//! Command boundaries shared by the parser and the byte-preserving formatter.
#[derive(Clone, Copy)]
pub(super) struct Syntax {
    pub format: u8,
    pub control: u8,
    pub delimiter: u8,
}

impl Default for Syntax {
    fn default() -> Self {
        Self {
            format: b'^',
            control: b'~',
            delimiter: b',',
        }
    }
}

impl Syntax {
    fn prefix(self, byte: u8) -> bool {
        byte == self.format || byte == self.control
    }
}

pub(super) struct Command<'a> {
    pub raw: &'a [u8],
    pub syntax: Syntax,
    pub payload: Option<usize>,
}

impl Command<'_> {
    pub fn is_format(&self, name: &[u8; 2]) -> bool {
        self.raw.first() == Some(&self.syntax.format)
            && self
                .raw
                .get(1..3)
                .is_some_and(|s| s.eq_ignore_ascii_case(name))
    }

    pub fn normalized(&self) -> Option<Vec<u8>> {
        if !self.syntax.prefix(*self.raw.first()?) {
            return None;
        }
        let (head, data) = self.raw.split_at(self.payload.unwrap_or(self.raw.len()));
        let mut bytes = Vec::with_capacity(self.raw.len());
        bytes.push(if head[0] == self.syntax.format {
            b'^'
        } else {
            b'~'
        });
        bytes.extend(
            head[1..]
                .iter()
                .copied()
                .filter(|&b| b == self.syntax.delimiter || !matches!(b, b'\n' | b'\r' | b'\t')),
        );
        bytes.extend_from_slice(data);
        Some(bytes)
    }
}

pub(super) struct Stream<'a> {
    rest: &'a [u8],
    pub syntax: Syntax,
}

impl<'a> Stream<'a> {
    pub fn new(data: &'a [u8], syntax: Syntax) -> Self {
        Self { rest: data, syntax }
    }

    pub fn next(&mut self) -> Result<Option<Command<'a>>, String> {
        let bytes = self.rest;
        let Some(&prefix) = bytes.first() else {
            return Ok(None);
        };
        let syntax = self.syntax;
        let mut payload = None;
        let end = if syntax.prefix(prefix)
            && bytes.get(1..3).is_some_and(|name| {
                [b"CC", b"CT", b"CD"]
                    .iter()
                    .any(|code| name.eq_ignore_ascii_case(*code))
            })
            && bytes.len() >= 4
        {
            let new = bytes[3];
            if new.is_ascii() {
                match bytes[2].to_ascii_uppercase() {
                    b'C' => self.syntax.format = new,
                    b'T' => self.syntax.control = new,
                    _ => self.syntax.delimiter = new,
                }
            }
            4
        } else if let Some((header, length)) = super::binary_graphics::payload(bytes, syntax)? {
            payload = Some(header);
            header
                .checked_add(length)
                .filter(|&end| end <= bytes.len())
                .ok_or("Truncated binary ^GF or object payload")?
        } else {
            1 + bytes[1..]
                .iter()
                .position(|&b| syntax.prefix(b))
                .unwrap_or(bytes.len() - 1)
        };
        let (raw, rest) = bytes.split_at(end);
        self.rest = rest;
        Ok(Some(Command {
            raw,
            syntax,
            payload,
        }))
    }
}
