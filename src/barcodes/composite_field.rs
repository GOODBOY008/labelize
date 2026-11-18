//! Unbracketed supplemental fields are allowed in printer streams, including
//! ordinary text with no GS1 application identifier. Encode such bytes in
//! the ISO/IEC 24723 general-purpose field. The composite encoder supplies
//! the linked primary, its alignment, row address patterns and separators.
use super::pdf417_encoding::cluster_patterns;
use anyd::codes::pdf417::ec;
use bwipp::{BitMatrix, Encoded, Options, Symbology};

// Rows, EC codewords, initial symbol cluster, and usable data bits for each
// ISO/IEC 24723 CC-A / CC-B variant. Indexed by primary component width.
type Size = (usize, usize, usize, usize);
const A: [&[Size]; 3] = [
    &[
        (5, 4, 2, 59),
        (6, 4, 0, 78),
        (7, 5, 1, 88),
        (8, 5, 1, 108),
        (9, 6, 1, 118),
        (10, 6, 0, 138),
        (12, 7, 1, 167),
    ],
    &[
        (4, 4, 1, 78),
        (5, 5, 0, 98),
        (6, 6, 1, 118),
        (7, 7, 2, 138),
        (8, 7, 2, 167),
    ],
    &[
        (3, 4, 0, 78),
        (4, 5, 0, 108),
        (5, 6, 0, 138),
        (6, 7, 0, 167),
        (7, 8, 1, 197),
    ],
];
const B: [&[Size]; 3] = [
    &[
        (8, 8, 0, 56),
        (11, 9, 0, 104),
        (14, 9, 1, 160),
        (17, 10, 2, 208),
        (20, 11, 0, 256),
        (23, 13, 2, 296),
        (26, 15, 2, 336),
    ],
    &[
        (6, 12, 0, 32),
        (8, 14, 0, 72),
        (10, 16, 2, 112),
        (12, 18, 0, 152),
        (15, 21, 0, 208),
        (20, 26, 0, 304),
        (26, 32, 0, 416),
        (32, 38, 2, 536),
        (38, 44, 2, 648),
        (44, 50, 0, 768),
    ],
    &[
        (4, 8, 1, 56),
        (6, 12, 0, 96),
        (8, 14, 0, 152),
        (10, 16, 2, 208),
        (12, 18, 0, 264),
        (15, 21, 0, 352),
        (20, 26, 0, 496),
        (26, 32, 0, 672),
        (32, 38, 2, 840),
        (38, 44, 2, 1016),
        (44, 50, 0, 1184),
    ],
];

fn bits_for(raw: &str) -> Result<Vec<bool>, String> {
    // Generic method (0). The initial numeric mode latches to alphanumeric
    // and then ISO/IEC 646. Numeric pairs stay compact in all-digit runs.
    let mut bits = vec![false];
    let mut numeric = true;
    let mut bytes = raw.bytes().peekable();
    let push = |bits: &mut Vec<bool>, v: u16, n: usize| {
        bits.extend((0..n).rev().map(|i| v & (1 << i) != 0))
    };
    while let Some(b) = bytes.next() {
        if numeric && b.is_ascii_digit() && bytes.peek().is_some_and(u8::is_ascii_digit) {
            let second = bytes.next().unwrap() - b'0';
            push(
                &mut bits,
                8 + u16::from(b - b'0') * 11 + u16::from(second),
                7,
            );
            continue;
        }
        if numeric {
            push(&mut bits, 0, 4);
            push(&mut bits, 4, 5);
            numeric = false
        }
        let (v, n) = match b {
            b'0'..=b'9' => (u16::from(b) - 43, 5),
            b'A'..=b'Z' => (u16::from(b) - 1, 7),
            b'a'..=b'z' => (u16::from(b) - 7, 7),
            29 => {
                numeric = true;
                (15, 5)
            }
            b'!' => (232, 8),
            b'"' => (233, 8),
            b'%'..=b'/' => (u16::from(b) + 197, 8),
            b':'..=b'?' => (u16::from(b) + 187, 8),
            b'_' => (251, 8),
            b' ' => (252, 8),
            _ => return Err("Invalid character in composite secondary field".into()),
        };
        push(&mut bits, v, n);
    }
    if numeric {
        push(&mut bits, 0, 4);
    }
    Ok(bits)
}
fn numeric_words(mut value: u128, base: u128, width: usize) -> Vec<u32> {
    let mut words = vec![0; width];
    for word in words.iter_mut().rev() {
        *word = (value % base) as u32;
        value /= base;
    }
    words
}
fn as_number(bits: &[bool]) -> u128 {
    bits.iter().fold(0, |v, &b| (v << 1) | u128::from(b))
}

struct Component {
    cols: usize,
    rows: usize,
    ecc: usize,
    cluster: usize,
    cap: usize,
    offset: usize,
    x: usize,
    stride: usize,
    kind: u8,
}
fn word(m: &BitMatrix, x: usize, y: usize, cluster: usize) -> Option<usize> {
    if y >= m.height() || x.checked_add(17)? > m.width() {
        return None;
    }
    let pat = (0..17).fold(0, |v, i| v * 2 + u32::from(m.get(x + i, y)));
    cluster_patterns()[cluster].iter().position(|&p| p == pat)
}
impl Component {
    fn geometry(m: &BitMatrix, variant: u8) -> Option<Self> {
        let stride = if variant == 12 { 3 } else { 2 };
        let same = |a, b| (0..m.width()).all(|x| m.get(x, a) == m.get(x, b));
        let rows = (0..m.height() / stride)
            .take_while(|r| {
                (0..m.width())
                    .filter(|&x| m.get(x, r * stride))
                    .take(8)
                    .count()
                    == 8
                    && (1..stride).all(|i| same(r * stride, r * stride + i))
            })
            .count();
        let x = (0..m.width()).find(|&x| m.get(x, 0))?;
        let w = (x..m.width()).rfind(|&x| m.get(x, 0))? + 1 - x;
        if variant == 12 {
            let cols = w.checked_sub(69)? / 17;
            if cols == 0 || rows < 3 {
                return None;
            }
            let level = word(m, x + 17, stride, 1)? % 30 / 3;
            let ecc = 2usize.checked_shl(level as u32)?;
            let count = (rows * cols).checked_sub(ecc + 3)?;
            let n = (0..=count * 2)
                .take_while(|n| n / 6 * 5 + n % 6 <= count)
                .last()?;
            return Some(Self {
                cols,
                rows,
                ecc,
                cluster: 0,
                cap: n * 8,
                offset: 34,
                x,
                stride,
                kind: b'C',
            });
        }
        let cols = match variant {
            3 | 4 | 8 => 2,
            5 | 10 => 3,
            _ => 4,
        };
        for (kind, table) in [(b'B', B[cols - 2]), (b'A', A[cols - 2])] {
            let Some(&(_, ecc, cluster, cap)) = table.iter().find(|s| s.0 == rows) else {
                continue;
            };
            let offset = if cols == 3 && kind == b'A' { 0 } else { 10 };
            if (kind == b'B' && word(m, x + offset, 0, cluster) != Some(920))
                || (kind == b'A' && word(m, x + offset, 0, cluster).is_none())
            {
                continue;
            }
            return Some(Self {
                cols,
                rows,
                ecc,
                cluster,
                cap,
                offset,
                x,
                stride,
                kind,
            });
        }
        None
    }
    fn replace(&self, m: &mut BitMatrix, src: &[bool]) {
        let mut bits = src.to_vec();
        let pad = [false, false, true, false, false];
        bits.extend(pad.into_iter().cycle().take(self.cap - src.len()));
        let mut words = Vec::new();
        if self.kind == b'A' {
            for chunk in bits.chunks(69) {
                words.extend(numeric_words(as_number(chunk), 928, chunk.len() / 10 + 1))
            }
        } else {
            if self.kind == b'C' {
                words.push((self.cols * self.rows - self.ecc) as u32);
            }
            let bytes: Vec<_> = bits
                .as_chunks::<8>()
                .0
                .iter()
                .map(|b| as_number(b) as u8)
                .collect();
            words.extend([
                920,
                if bytes.len().is_multiple_of(6) {
                    924
                } else {
                    901
                },
            ]);
            let (groups, tail) = bytes.as_chunks::<6>();
            for g in groups {
                words.extend(numeric_words(
                    g.iter().fold(0, |n, b| n * 256 + u128::from(*b)),
                    900,
                    5,
                ))
            }
            words.extend(tail.iter().copied().map(u32::from));
        }
        words.resize(self.rows * self.cols - self.ecc, 900);
        words.extend(ec::encode(&words, self.ecc));
        for y in 0..self.rows {
            for c in 0..self.cols {
                let centre = if self.kind != b'C'
                    && ((self.cols == 3 && c >= 1) || (self.cols == 4 && c >= 2))
                {
                    10
                } else {
                    0
                };
                let pos = self.x + self.offset + 17 * c + centre;
                let pattern =
                    cluster_patterns()[(self.cluster + y) % 3][words[y * self.cols + c] as usize];
                for dy in 0..self.stride {
                    for i in 0..17 {
                        m.set(
                            pos + i,
                            y * self.stride + dy,
                            pattern & (1 << (16 - i)) != 0,
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn encode(
    sym: Symbology,
    opts: &Options,
    variant: u8,
    primary: &str,
    secondary: &str,
) -> Result<BitMatrix, String> {
    let bits = bits_for(secondary)?;
    if bits.len() > 9000 {
        return Err("Composite secondary field exceeds symbol capacity".into());
    }
    // The public encoder chooses the geometry from an AI string. A variable
    // length AI supplies a carrier of sufficient capacity. Only its linear
    // component, row addressing and alignment are retained.
    let mut n = bits.len().saturating_sub(16).div_ceil(7).max(1);
    for _ in 0..12 {
        let mut dummy = String::new();
        // The GS1 encoder accepts at most 30 characters per internal AI.
        for i in (0..n).step_by(30) {
            dummy.push_str("(91)");
            dummy.extend(std::iter::repeat_n('a', (n - i).min(30)));
        }
        let Encoded::Matrix(mut template) = sym
            .encode(&format!("{primary}|{dummy}"), opts)
            .map_err(|e| e.to_string())?
        else {
            return Err("Missing composite matrix".into());
        };
        let component =
            Component::geometry(&template, variant).ok_or("Unrecognized composite row geometry")?;
        if component.cap >= bits.len() {
            component.replace(&mut template, &bits);
            return Ok(template);
        }
        n += (bits.len() - component.cap).div_ceil(7).max(1);
    }
    Err("Composite secondary field exceeds symbol capacity".into())
}
