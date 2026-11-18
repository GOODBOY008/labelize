use super::symbols::modules;
use anyd::{
    codes::code49::{Code49Encoder, Code49Meta},
    output::Encoding,
    Segment, Symbol, SymbolMeta, Symbology,
};
use std::sync::LazyLock;

const BASE: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
// Code 49 symbol-check weights. Y and Z use the same sequence starting
// one and two positions later than X.
const WEIGHTS: [usize; 34] = [
    1, 9, 31, 26, 2, 12, 17, 23, 37, 18, 22, 6, 27, 44, 15, 43, 39, 11, 13, 5, 41, 33, 36, 8, 4,
    32, 3, 19, 40, 25, 29, 10, 24, 30,
];

/// Ask the encoder for its standard ASCII expansion; a starting shift is
/// represented in the mode character rather than in the data region.
static ASCII: LazyLock<[[u8; 2]; 128]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        let s = Code49Encoder::new()
            .build(&[i as u8])
            .expect("ASCII has a Code 49 representation");
        let SymbolMeta::Code49(m) = s.meta else {
            unreachable!()
        };
        match m.grid[(m.rows - 1) * 8 + 6] % 7 {
            4 => [43, m.grid[0]],
            5 => [44, m.grid[0]],
            _ => [m.grid[0], 255],
        }
    })
});

fn numeric(mut digits: &[u8], out: &mut Vec<u8>) {
    fn push(out: &mut Vec<u8>, digits: &[u8], bias: usize, words: usize) {
        let n = digits.iter().fold(0usize, |v, &d| v * 10 + d as usize) + bias;
        for i in (0..words).rev() {
            out.push((n / 48usize.pow(i as u32) % 48) as u8);
        }
    }
    while digits.len() >= 5 {
        if digits.len() == 7 {
            push(out, &digits[..4], 100000, 3);
            push(out, &digits[4..], 0, 2);
            return;
        }
        push(out, &digits[..5], 0, 3);
        digits = &digits[5..];
    }
    match digits.len() {
        1 => out.push(digits[0]),
        2 => {
            out.push(48);
            out.extend(digits)
        }
        3 => push(out, digits, 0, 2),
        4 => push(out, digits, 100000, 3),
        _ => {}
    }
}

pub(super) fn encode(data: &[u8], selected: Option<u8>) -> Result<Encoding, String> {
    if data.is_empty() || data.len() > 81 || !data.is_ascii() {
        return Err("Code 49 requires 1 to 81 ASCII characters".into());
    }
    let manual = selected.is_some();
    let mut letters = Vec::new();
    for &b in data {
        if manual {
            if let Some(i) = BASE.iter().position(|x| *x == b) {
                letters.push(i as u8)
            } else if let Some(i) = b"<>:;?=".iter().position(|x| *x == b) {
                letters.push(i as u8 + 43)
            } else {
                break;
            }
        } else {
            letters.extend(ASCII[b as usize].into_iter().filter(|v| *v != 255));
        }
    }
    let mut words = Vec::new();
    let mut i = 0;
    let mut in_numeric = selected == Some(2);
    while i < letters.len() {
        let n = letters[i..].iter().take_while(|d| **d < 10).count();
        if n > 0 && (in_numeric || !manual && n >= 5) {
            if !in_numeric {
                words.push(48)
            }
            numeric(&letters[i..i + n], &mut words);
            i += n;
            if !manual && i < letters.len() {
                words.push(48)
            }
        } else if manual && letters[i] == 48 {
            words.push(48);
            in_numeric = !in_numeric;
            i += 1;
        } else if in_numeric {
            break;
        } else {
            words.push(letters[i]);
            i += 1;
        }
    }
    let mode = if let Some(m) = selected {
        m
    } else {
        match words.first() {
            Some(43) => 4,
            Some(44) => 5,
            Some(48) => 2,
            _ => 0,
        }
    };
    if !manual && mode != 0 {
        words.remove(0);
    }
    if mode > 5 || words.len() > 49 || words.is_empty() {
        return Err("Invalid Code 49 starting mode or field length".into());
    }
    let data_rows = words.len().div_ceil(7);
    let rows =
        data_rows + usize::from(data_rows == 1 || data_rows > 6 || data_rows * 7 - words.len() < 5);
    let mut grid = vec![48u8; rows * 8];
    for (i, v) in words.into_iter().enumerate() {
        grid[i / 7 * 8 + i % 7] = v
    }
    for row in grid.as_chunks_mut::<8>().0.iter_mut().take(rows - 1) {
        row[7] = (row[..7].iter().map(|&v| v as usize).sum::<usize>() % 49) as u8;
    }
    let m = 7 * (rows - 2) + mode as usize;
    let (mut x, mut y, mut z) = (m * 20, m * 16, m * 38);
    let last = (rows - 1) * 8;
    for (i, pair) in grid[..last].as_chunks::<2>().0.iter().enumerate() {
        let n = 49 * pair[0] as usize + pair[1] as usize;
        x += WEIGHTS[i] * n;
        y += WEIGHTS[i + 1] * n;
        z += WEIGHTS[i + 2] * n;
    }
    let set = |p: &mut [u8], n: usize| {
        p[0] = (n % 2401 / 49) as u8;
        p[1] = (n % 49) as u8;
    };
    let lastrow = &mut grid[last..];
    if rows > 6 {
        set(lastrow, z);
    }
    let pos = last / 2;
    let pair = lastrow[0] as usize * 49 + lastrow[1] as usize;
    y = (y + WEIGHTS[pos + 1] * pair) % 2401;
    set(&mut lastrow[2..], y);
    x += WEIGHTS[pos] * pair + WEIGHTS[pos + 1] * y;
    set(&mut lastrow[4..], x);
    lastrow[6] = m as u8;
    lastrow[7] = (lastrow[..7].iter().map(|&c| c as usize).sum::<usize>() % 49) as u8;
    modules(
        Code49Encoder::new(),
        Ok(Symbol::new(
            Symbology::Code49,
            vec![Segment::byte(data.to_vec())],
            SymbolMeta::Code49(Code49Meta { rows, grid }),
        )),
    )
}
