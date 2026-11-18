use super::symbols::modules;
use anyd::codes::{
    codablockf::{CodablockFEncoder, CodablockFMeta},
    code128::{Code128Encoder, Code128Input},
};
use anyd::{
    output::{BitMatrix, Encoding},
    Segment, Symbol, SymbolMeta, Symbology,
};

const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

pub(super) fn encode(
    mode: u8,
    columns: usize,
    rows: usize,
    checksum: bool,
    data: &[u8],
) -> Result<Encoding, String> {
    if data.is_empty() || data.len() > 2728 {
        return Err("Invalid Codablock data length".into());
    }
    if mode == b'A' {
        return encode_a(data, columns, rows, checksum);
    }
    if mode != b'F' && mode != b'E' {
        return Err("Unknown Codablock mode".into());
    }
    let gs1 = mode == b'E';
    let mut values = Vec::new();
    if gs1 {
        values.push((None, 102));
    }
    for &b in data {
        values.push(match b {
            0x80 => (None, 102),
            0x81 => (None, 97),
            0x82 => (None, 96),
            0x83 => (None, 100),
            0..=31 => (Some(98), b + 64),
            0x84 => (Some(98), 64),
            32..=127 => (None, b - 32),
            _ => return Err("Invalid Codablock character".into()),
        });
    }
    let total: usize = values
        .iter()
        .map(|(shift, _)| 1 + usize::from(shift.is_some()))
        .sum();
    if rows <= 1 && (columns == 0 || total <= columns) {
        let e = Code128Encoder::new();
        if data.iter().any(|v| matches!(v, 0x81..=0x83)) {
            let mut words = vec![104]; // Start B
            for (shift, ch) in values {
                words.extend(shift);
                words.push(ch);
            }
            let mut pattern = anyd::output::LinearPattern::new();
            e.encode_into(&words, &mut pattern)
                .map_err(|e| e.to_string())?;
            return Ok(Encoding::Linear(pattern));
        }
        let input: Vec<_> = data
            .iter()
            .map(|&v| match v {
                0x80 => Code128Input::Fnc1,
                0x84 => Code128Input::Data(0),
                _ => Code128Input::Data(v),
            })
            .collect();
        return modules(
            e,
            if gs1 {
                e.build_gs1(&input)
            } else {
                e.build(&input)
            },
        );
    }
    let width = if columns == 0 {
        (total + 2).div_ceil(rows.max(2)).max(4)
    } else {
        columns.max(4)
    };
    let mut lines = vec![Vec::new()];
    for (shift, ch) in values {
        if lines.last().unwrap().len() + 1 + usize::from(shift.is_some()) > width {
            lines.push(Vec::new());
        }
        let line = lines.last_mut().unwrap();
        line.extend(shift);
        line.push(ch);
    }
    if lines.last().unwrap().len() + 2 > width {
        lines.push(Vec::new())
    }
    let count = rows.max(lines.len()).max(2);
    if width > 62 || count > 44 {
        return Err("Codablock data exceeds the selected dimensions".into());
    }
    lines.resize_with(count, Vec::new);
    let mut k1 = 0usize;
    let mut k2 = 0usize;
    for (i, &b) in data.iter().enumerate() {
        k1 = (k1 + (i + 1) * b as usize) % 86;
        k2 = (k2 + i * b as usize) % 86
    }
    let check_value = |n: usize| match n {
        0..=31 => n + 64,
        32..=47 => n - 32,
        _ => n - 22,
    } as u8;
    let mut grid = Vec::with_capacity(count * (width + 5));
    for (r, data) in lines.into_iter().enumerate() {
        let base = grid.len();
        grid.extend([
            103,
            100,
            check_value(if r == 0 { count - 2 } else { r + 42 }),
        ]);
        let check_row = r + 1 == count;
        let n = width - if check_row { 2 } else { 0 };
        let mut set_c = false;
        let filled = data.len();
        grid.extend(data);
        for _ in filled..n {
            grid.push(if set_c { 100 } else { 99 });
            set_c = !set_c;
        }
        if check_row {
            let val = |k| if set_c { k as u8 } else { check_value(k) };
            grid.extend([val(k1), val(k2)]);
        }
        let checksum = grid[base..]
            .iter()
            .enumerate()
            .map(|(i, &v)| i.max(1) * v as usize)
            .sum::<usize>()
            % 103;
        grid.extend([checksum as u8, 106]);
    }
    let meta = CodablockFMeta {
        columns: width + 5,
        rows: count,
        codewords: grid,
    };
    modules(
        CodablockFEncoder::new(),
        Ok(Symbol::new(
            Symbology::CodablockF,
            vec![Segment::byte(data.to_vec())],
            SymbolMeta::CodablockF(meta),
        )),
    )
}

fn encode_a(data: &[u8], columns: usize, rows: usize, checks: bool) -> Result<Encoding, String> {
    let cells: Result<Vec<_>, _> = data
        .iter()
        .map(|b| {
            ALPHABET
                .iter()
                .position(|c| c == b)
                .ok_or("Invalid Codablock A character")
        })
        .collect();
    let mut cells = cells?;
    let single = rows <= 1 && (columns == 0 || data.len() <= columns);
    let ncheck = if !single && checks { 2 } else { 0 };
    let width = if columns == 0 {
        (cells.len() + ncheck).div_ceil(rows.max(1))
    } else {
        columns
    };
    let count = rows.max((cells.len() + ncheck).div_ceil(width));
    if count > 22 || width > 62 {
        return Err("Codablock A data exceeds the selected dimensions".into());
    }
    cells.resize(count * width - ncheck, 38);
    if ncheck != 0 {
        let k1 = cells
            .iter()
            .enumerate()
            .map(|(i, v)| (i + 1) * v)
            .sum::<usize>()
            % 43;
        let k2 = cells.iter().enumerate().map(|(i, v)| i * v).sum::<usize>() % 43;
        cells.extend([k1, k2]);
    }
    let mut matrix = None;
    for (r, data) in cells.chunks_exact(width).enumerate() {
        let index = ALPHABET[if r == 0 { count + 20 } else { r - 1 }];
        let mut row = vec![index];
        row.extend(data.iter().map(|&v| ALPHABET[v]));
        if count > 1 {
            row.push(index)
        }
        let bars = super::symbols::code39_modules(&row)?;
        let grid = matrix.get_or_insert_with(|| BitMatrix::new(bars.modules.len(), count, 0));
        for (x, dark) in bars.modules.into_iter().enumerate() {
            grid.set(x, r, dark);
        }
    }
    matrix
        .map(Encoding::Matrix)
        .ok_or_else(|| "Missing Codablock A data".into())
}
