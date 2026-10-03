//! PDF417 encoding pipeline: ISO/IEC 15438 high-level compaction with
//! block-smoothing Text/Byte/Numeric segmentation, plus low-level symbol
//! assembly (row indicators, clusters, start/stop patterns).
//!
//! The segmentation follows the classic block-smoothing approach of
//! ISO/IEC 15438 implementations (class-then-smooth, as used by zint up to
//! 2.9): classify each character as text/numeric/byte, merge short numeric
//! and text blocks into their neighbors, then emit each block. This matches
//! the segment choices of the reference renderer used for the golden tests,
//! which differ from ZXing's heuristic that rxing ports (notably: numeric
//! compaction for medium digit runs, text for leading punctuation runs).
//! Reed-Solomon error correction and the symbol bit patterns are reused
//! from rxing's public tables.

use std::sync::OnceLock;

use rxing::pdf417::encoder::pdf_417_error_correction;
use rxing::pdf417::pdf_417_common::{CODEWORD_TABLE, SYMBOL_TABLE};

// ---------------------------------------------------------------------------
// ISO/IEC 15438 text sub-mode classification
// ---------------------------------------------------------------------------

/// OR-able sub-mode membership masks (ISO/IEC 15438:2015 Table 3).
const T_ALPHA: u8 = 1;
const T_LOWER: u8 = 2;
const T_MIXED: u8 = 4;
const T_PUNCT: u8 = 8;
const T_ALWMX: u8 = T_ALPHA | T_LOWER | T_MIXED;
const T_MXPNC: u8 = T_MIXED | T_PUNCT;

/// (sub-mode membership mask, value in those sub-modes) for a byte, per
/// ISO/IEC 15438:2015 Tables 3-6. Mask 0 = byte compaction only.
fn text_class(c: u8) -> (u8, u8) {
    match c {
        0x09 => (T_MXPNC, 12), // HT
        0x0A => (T_PUNCT, 15), // LF
        0x0D => (T_MXPNC, 11), // CR
        0x20 => (T_ALWMX, 26), // space
        0x21 => (T_PUNCT, 10),
        0x22 => (T_PUNCT, 20),
        0x23 => (T_MIXED, 15), // #
        0x24 => (T_MXPNC, 18), // $
        0x25 => (T_MIXED, 21), // %
        0x26 => (T_MIXED, 10),
        0x27 => (T_PUNCT, 28),
        0x28 => (T_PUNCT, 23),
        0x29 => (T_PUNCT, 24),
        0x2A => (T_MXPNC, 22),
        0x2B => (T_MIXED, 20), // +
        0x2C => (T_MXPNC, 13),
        0x2D => (T_MXPNC, 16),
        0x2E => (T_MXPNC, 17),
        0x2F => (T_MXPNC, 19),
        0x30..=0x39 => (T_MIXED, c - b'0'),
        0x3A => (T_MXPNC, 14),
        0x3B => (T_PUNCT, 0),
        0x3C => (T_PUNCT, 1),
        0x3D => (T_MIXED, 23),
        0x3E => (T_PUNCT, 2),
        0x3F => (T_PUNCT, 25),
        0x40 => (T_PUNCT, 3),
        0x41..=0x5A => (T_ALPHA, c - b'A'),
        0x5B => (T_PUNCT, 4),
        0x5C => (T_PUNCT, 5),
        0x5D => (T_PUNCT, 6),
        0x5E => (T_MIXED, 24),
        0x5F => (T_PUNCT, 7),
        0x60 => (T_PUNCT, 8),
        0x61..=0x7A => (T_LOWER, c - b'a'),
        0x7B => (T_PUNCT, 26),
        0x7C => (T_PUNCT, 21),
        0x7D => (T_PUNCT, 27),
        0x7E => (T_PUNCT, 9),
        _ => (0, 0),
    }
}

// ---------------------------------------------------------------------------
// Block segmentation (classify, then smooth short blocks into neighbors)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockKind {
    Text,
    Byte,
    Numeric,
}

struct Block {
    kind: BlockKind,
    start: usize,
    len: usize,
}

/// Character class for segmentation: digits are numeric candidates,
/// printable ASCII (plus tab/LF/CR) is text, everything else is byte-only.
fn char_kind(c: u8) -> BlockKind {
    if c.is_ascii_digit() {
        BlockKind::Numeric
    } else if c == b'\t' || c == b'\n' || c == b'\r' || (0x20..=0x7E).contains(&c) {
        BlockKind::Text
    } else {
        BlockKind::Byte
    }
}

fn initial_blocks(data: &[u8]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let kind = char_kind(data[i]);
        let start = i;
        while i < data.len() && char_kind(data[i]) == kind {
            i += 1;
        }
        blocks.push(Block {
            kind,
            start,
            len: i - start,
        });
    }
    blocks
}

fn regroupe(blocks: &mut Vec<Block>) {
    let mut i = 1;
    while i < blocks.len() {
        if blocks[i - 1].kind == blocks[i].kind {
            blocks[i - 1].len += blocks[i].len;
            blocks.remove(i);
        } else {
            i += 1;
        }
    }
}

/// Two smoothing passes: first fold short numeric blocks into their
/// neighbors, then fold short text blocks surrounded by byte blocks. The
/// first block of the label is never demoted from text.
fn smooth_blocks(data: &[u8]) -> Vec<Block> {
    let mut b = initial_blocks(data);
    let n = b.len();

    // pass 1: numeric blocks
    for i in 0..n {
        if b[i].kind != BlockKind::Numeric {
            continue;
        }
        let len = b[i].len;
        let last = if i > 0 { Some(b[i - 1].kind) } else { None };
        let next = if i + 1 < n { Some(b[i + 1].kind) } else { None };
        let target = if i == 0 {
            match next {
                Some(BlockKind::Text) if n > 1 && len < 8 => Some(BlockKind::Text),
                Some(BlockKind::Byte) if n > 1 && len == 1 => Some(BlockKind::Byte),
                _ => None,
            }
        } else if i == n - 1 {
            match last {
                Some(BlockKind::Text) if len < 7 => Some(BlockKind::Text),
                Some(BlockKind::Byte) if len == 1 => Some(BlockKind::Byte),
                _ => None,
            }
        } else {
            match (last, next) {
                (Some(BlockKind::Byte), Some(BlockKind::Byte)) if len < 4 => Some(BlockKind::Byte),
                (Some(BlockKind::Byte), Some(BlockKind::Text)) if len < 4 => Some(BlockKind::Text),
                (Some(BlockKind::Text), Some(BlockKind::Byte)) if len < 5 => Some(BlockKind::Text),
                (Some(BlockKind::Text), Some(BlockKind::Text)) if len < 8 => Some(BlockKind::Text),
                _ => None,
            }
        };
        if let Some(t) = target {
            b[i].kind = t;
        }
    }
    regroupe(&mut b);

    // pass 2: text blocks (the first block is never demoted)
    let n = b.len();
    for i in 1..n {
        if b[i].kind != BlockKind::Text {
            continue;
        }
        let len = b[i].len;
        let last = b[i - 1].kind;
        let next = if i + 1 < n { Some(b[i + 1].kind) } else { None };
        let target = if i == n - 1 {
            if last == BlockKind::Byte && len == 1 {
                Some(BlockKind::Byte)
            } else {
                None
            }
        } else {
            match next {
                Some(BlockKind::Byte) if last == BlockKind::Byte && len < 5 => {
                    Some(BlockKind::Byte)
                }
                Some(_)
                    if len < 3 && (last == BlockKind::Byte || next == Some(BlockKind::Byte)) =>
                {
                    Some(BlockKind::Byte)
                }
                _ => None,
            }
        };
        if let Some(t) = target {
            b[i].kind = t;
        }
    }
    regroupe(&mut b);
    b
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

/// Sub-mode latch sequences (ISO/IEC 15438:2015 5.4.2.4).
fn emit_table_switch(cur: u8, new: u8, out: &mut Vec<u8>) {
    match cur {
        x if x == T_ALPHA => match new {
            t if t == T_LOWER => out.extend_from_slice(&[27]), // LL
            t if t == T_MIXED => out.extend_from_slice(&[28]), // ML
            t if t == T_PUNCT => out.extend_from_slice(&[28, 25]), // ML + PL
            _ => {}
        },
        x if x == T_LOWER => match new {
            t if t == T_ALPHA => out.extend_from_slice(&[28, 28]), // ML + AL
            t if t == T_MIXED => out.extend_from_slice(&[28]),     // ML
            t if t == T_PUNCT => out.extend_from_slice(&[28, 25]), // ML + PL
            _ => {}
        },
        x if x == T_MIXED => match new {
            t if t == T_ALPHA => out.extend_from_slice(&[28]), // AL
            t if t == T_LOWER => out.extend_from_slice(&[27]), // LL
            t if t == T_PUNCT => out.extend_from_slice(&[25]), // PL
            _ => {}
        },
        x if x == T_PUNCT => match new {
            t if t == T_ALPHA => out.extend_from_slice(&[29]), // AL
            t if t == T_LOWER => out.extend_from_slice(&[29, 27]), // AL + LL
            t if t == T_MIXED => out.extend_from_slice(&[29, 28]), // AL + ML
            _ => {}
        },
        _ => {}
    }
}

/// Normalize a combined sub-mode mask to the preferred single sub-mode.
fn normalize_table(mask: u8) -> u8 {
    if mask & T_ALPHA != 0 && (mask & (T_LOWER | T_MIXED | T_PUNCT)) != 0 {
        T_ALPHA
    } else if mask & T_MIXED != 0 && mask & T_PUNCT != 0 {
        T_MIXED
    } else if mask & T_LOWER != 0 && mask & T_PUNCT != 0 {
        T_LOWER
    } else {
        mask
    }
}

/// Text compaction of one block (ISO/IEC 15438:2015 5.4.2). Emits the 900
/// latch unless this is the very start of the payload; pads odd value
/// counts with PS (29).
fn emit_text_block(data: &[u8], start: usize, len: usize, first_block: bool, out: &mut Vec<u16>) {
    let mut values: Vec<u8> = Vec::with_capacity(len + 2);
    let mut curtable = T_ALPHA;

    for j in 0..len {
        let (mask, value) = text_class(data[start + j]);
        if mask & curtable != 0 {
            values.push(value);
            continue;
        }
        // must change sub-mode; a lone off-table char may use a shift
        let lone = j + 1 == len || {
            let (next_mask, _) = text_class(data[start + j + 1]);
            mask & next_mask == 0
        };
        if lone {
            if mask & T_ALPHA != 0 && curtable == T_LOWER {
                values.extend_from_slice(&[27, value]); // AS + char
                continue;
            }
            if mask & T_PUNCT != 0 {
                values.extend_from_slice(&[29, value]); // PS + char
                continue;
            }
        }
        let newtable = if lone {
            mask
        } else {
            let (next_mask, _) = text_class(data[start + j + 1]);
            mask & next_mask
        };
        let newtable = normalize_table(newtable);
        emit_table_switch(curtable, newtable, &mut values);
        curtable = newtable;
        values.push(value);
    }

    if values.len() % 2 == 1 {
        values.push(29); // PS pad
    }
    if !first_block {
        out.push(900);
    }
    for pair in values.chunks(2) {
        out.push(30 * pair[0] as u16 + pair[1] as u16);
    }
}

/// Byte compaction (ISO/IEC 15438:2015 5.4.3): a single byte uses the 913
/// shift when it continues a text run, otherwise the 901 latch; longer runs
/// latch with 901 (or 924 when the run is a multiple of six) and pack
/// six-byte groups into five base-900 codewords.
fn emit_byte_block(data: &[u8], start: usize, len: usize, prev_was_text: bool, out: &mut Vec<u16>) {
    if len == 1 {
        out.push(if prev_was_text { 913 } else { 901 });
        out.push(data[start] as u16);
        return;
    }
    out.push(if len.is_multiple_of(6) { 924 } else { 901 });
    let mut i = 0;
    while i < len {
        let remain = len - i;
        if remain >= 6 {
            let mut total: u64 = 0;
            for j in 0..6 {
                total |= (data[start + i + j] as u64) << ((5 - j) * 8);
            }
            let mut group = [0u16; 5];
            let mut t = total;
            for slot in group.iter_mut().rev() {
                *slot = (t % 900) as u16;
                t /= 900;
            }
            out.extend_from_slice(&group);
            i += 6;
        } else {
            out.push(data[start + i] as u16);
            i += 1;
        }
    }
}

/// Numeric compaction (ISO/IEC 15438:2015 5.4.4): groups of up to 44 digits
/// (prefixed with a leading 1) converted to base-900 codewords, MSD first.
fn emit_numeric_block(data: &[u8], start: usize, len: usize, out: &mut Vec<u16>) {
    out.push(902);
    let mut i = 0;
    while i < len {
        let group = (len - i).min(44);
        let mut digits: Vec<u8> = Vec::with_capacity(group + 1);
        digits.push(1);
        digits.extend((0..group).map(|j| data[start + i + j] - b'0'));

        // repeated division by 900, collecting remainders (LSD first)
        let mut cws: Vec<u16> = Vec::new();
        loop {
            let mut rem = 0u32;
            let mut next: Vec<u8> = Vec::with_capacity(digits.len());
            let mut leading = true;
            for &d in &digits {
                let v = rem * 10 + d as u32;
                let q = v / 900;
                rem = v % 900;
                if q != 0 || !leading {
                    next.push(q as u8);
                    leading = false;
                }
            }
            cws.push(rem as u16);
            if next.is_empty() {
                break;
            }
            digits = next;
        }
        cws.reverse();
        out.extend_from_slice(&cws);
        i += group;
    }
}

/// High-level encode `data` into payload codewords (without the length
/// descriptor). `data` must be non-empty.
pub fn encode_high_level(data: &[u8]) -> Vec<u16> {
    let blocks = smooth_blocks(data);
    let mut out: Vec<u16> = Vec::new();
    let mut prev_was_text = false;
    for (i, b) in blocks.iter().enumerate() {
        match b.kind {
            BlockKind::Text => emit_text_block(data, b.start, b.len, i == 0, &mut out),
            BlockKind::Byte => emit_byte_block(data, b.start, b.len, prev_was_text, &mut out),
            BlockKind::Numeric => emit_numeric_block(data, b.start, b.len, &mut out),
        }
        prev_was_text = b.kind == BlockKind::Text;
    }
    out
}

// ---------------------------------------------------------------------------
// Symbol assembly
// ---------------------------------------------------------------------------

/// ISO/IEC 15438:2015 row start pattern (17 modules).
const START_PATTERN: u32 = 0x1FEA8;
/// ISO/IEC 15438:2015 row stop pattern (18 modules).
const STOP_PATTERN: u32 = 0x3FA29;

/// codeword -> 17-bit bar/space pattern, per cluster (0, 3, 6).
fn cluster_patterns() -> &'static [[u32; 929]; 3] {
    static TABLES: OnceLock<[[u32; 929]; 3]> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut tables = [[0u32; 929]; 3];
        for i in 0..SYMBOL_TABLE.len() {
            let v = CODEWORD_TABLE[i] as usize - 1;
            let cluster = v / 929;
            let codeword = v % 929;
            tables[cluster][codeword] = SYMBOL_TABLE[i];
        }
        tables
    })
}

/// A fully assembled PDF417 symbol: black/white module rows.
pub struct Pdf417Symbol {
    pub rows: Vec<Vec<bool>>,
}

/// Build the complete symbol for `data`.
///
/// * `security` — ^B7 error-correction level, used verbatim including 0
///   (the reference renderer treats an unset level as 0, not as
///   "recommended minimum"). A negative value means "auto-select": the
///   codeword-count table from the EPL2 manual ("b" PDF417 options).
/// * `column_count` / `row_count` — ^B7 constraints, 0 = automatic.
/// * `truncated` — compact PDF417 stop pattern.
pub fn build_symbol(
    data: &[u8],
    security: i32,
    column_count: i32,
    row_count: i32,
    truncated: bool,
) -> Result<Pdf417Symbol, String> {
    if data.is_empty() {
        return Err("PDF417: empty content".to_string());
    }
    let payload = encode_high_level(data);
    let level = if security < 0 {
        match payload.len() {
            0..=31 => 1,
            32..=63 => 2,
            64..=127 => 3,
            128..=255 => 4,
            256..=511 => 5,
            _ => 6,
        }
    } else {
        security.clamp(0, 8)
    } as u32;
    let ecc_count = 2u32 << level;

    let payload_len = payload.len();
    let total = payload_len + 1 + ecc_count as usize;
    if total > 928 {
        return Err(format!(
            "PDF417: {} codewords exceed 928-codeword limit",
            total
        ));
    }

    // symbol dimensions (ISO/IEC 15438 symbol-size rules; columns clamp at 30)
    let mut cols = column_count.clamp(0, 30) as usize;
    let (cols, rows) = if row_count > 0 {
        let mut rows = row_count.clamp(3, 90) as usize;
        if cols > 0 {
            while rows <= 90 && rows * cols < total {
                rows += 1;
            }
        } else {
            cols = total.div_ceil(rows).clamp(1, 30);
            while cols > 30 && rows < 90 {
                rows += 1;
                cols = total.div_ceil(rows).clamp(1, 30);
            }
        }
        if rows > 90 || rows * cols > 928 {
            return Err(format!(
                "PDF417: {} columns x {} rows exceeds the 928-codeword limit",
                cols, rows
            ));
        }
        (cols, rows)
    } else {
        if cols == 0 {
            // classic near-cubic automatic shape; bump columns to keep rows
            // within the 90-row limit
            cols = (0.5 + ((total as f64) / 3.0).sqrt()) as usize;
            if cols == 0 {
                cols = 1;
            }
            while total.div_ceil(cols) > 90 && cols < 30 {
                cols += 1;
            }
        }
        let mut rows = total.div_ceil(cols).max(3);
        while rows * cols > 928 && cols < 30 {
            cols += 1;
            rows = total.div_ceil(cols);
        }
        if rows * cols > 928 {
            return Err(format!(
                "PDF417: {} codewords exceed 928-codeword limit",
                total
            ));
        }
        (cols, rows)
    };
    let padding = cols * rows - total;

    let mut codewords: Vec<u16> = Vec::with_capacity(cols * rows);
    codewords.push((payload.len() + padding + 1) as u16); // length descriptor
    codewords.extend_from_slice(&payload);
    codewords.extend(std::iter::repeat_n(900u16, padding));

    // Reed-Solomon over GF(929), reusing rxing's ISO 15438 Annex A implementation
    let data_str: String = codewords
        .iter()
        .map(|&w| char::from_u32(w as u32).unwrap())
        .collect();
    let ec = pdf_417_error_correction::generateErrorCorrection(&data_str, level)
        .map_err(|e| format!("PDF417: error correction failed: {}", e))?;
    let ec_words: Vec<u16> = ec.chars().map(|c| c as u16).collect();
    if ec_words.len() != ecc_count as usize {
        return Err("PDF417: unexpected error-correction codeword count".to_string());
    }
    codewords.extend_from_slice(&ec_words);

    let patterns = cluster_patterns();
    let mut symbol_rows = Vec::with_capacity(rows);

    let c1 = (rows - 1) / 3;
    let c2 = level as usize * 3 + (rows - 1) % 3;
    let c3 = cols - 1;

    for y in 0..rows {
        let cluster = y % 3;
        let k = (y / 3) * 30;
        let (left, right) = match cluster {
            0 => (k + c1, k + c3),
            1 => (k + c2, k + c1),
            _ => (k + c3, k + c2),
        };

        let mut row: Vec<bool> = Vec::with_capacity(17 * cols + 86);
        push_pattern(&mut row, START_PATTERN, 17);
        push_pattern(&mut row, patterns[cluster][left], 17);
        for x in 0..cols {
            push_pattern(
                &mut row,
                patterns[cluster][codewords[y * cols + x] as usize],
                17,
            );
        }
        if truncated {
            row.push(true);
        } else {
            push_pattern(&mut row, patterns[cluster][right], 17);
            push_pattern(&mut row, STOP_PATTERN, 18);
        }
        symbol_rows.push(row);
    }

    Ok(Pdf417Symbol { rows: symbol_rows })
}

/// Append the `bits`-wide 1=black MSB-first bit pattern of `value`.
fn push_pattern(row: &mut Vec<bool>, value: u32, bits: u32) {
    for i in (0..bits).rev() {
        row.push((value >> i) & 1 == 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn high(s: &str) -> Vec<u16> {
        encode_high_level(s.as_bytes())
    }

    #[test]
    fn pure_uppercase_text() {
        // "HELLO WORLD": 6 text codewords, no latch (starts in alpha)
        assert_eq!(
            high("HELLO WORLD"),
            vec![214, 341, 446, 674, 521, 119] // HE LL O_ WO RL D+ps
        );
    }

    #[test]
    fn digits_use_numeric_compaction() {
        // "0123456789": 902 latch + base-900 of 1_0123456789
        // (10,123,456,789 = 13*900^3 + 798*900^2 + 85*900 + 289)
        assert_eq!(high("0123456789"), vec![902, 13, 798, 85, 289]);
    }

    #[test]
    fn fedex_prefix_uses_punct_text_then_bytes_then_numeric() {
        let data = "[)>\x1e01\x1d0294105";
        // "[)>" latches alpha->punct (ML+PL); RS,'0','1',GS ride a byte run
        // (the 2-digit block folds into its byte neighbors); the 7-digit
        // group stays numeric.
        assert_eq!(
            high(data),
            vec![865, 144, 89, 901, 30, 48, 49, 29, 902, 12, 637, 805]
        );
    }

    #[test]
    fn short_digit_runs_between_bytes_stay_bytes() {
        // "840" (3 digits) folds into the surrounding byte blocks
        let data = "\x01\x02840\x03\x04";
        let out = high(data);
        assert_eq!(out.first(), Some(&901)); // one byte run, not numeric
        assert_eq!(out.last(), Some(&4)); // trailing byte 0x04
    }

    #[test]
    fn text_between_bytes_folds_when_short() {
        // "FDE" (3 chars) between byte blocks folds into bytes
        let data = "\x01FDE\x02";
        assert_eq!(high(data), vec![901, 1, 70, 68, 69, 2]);
    }

    #[test]
    fn leading_text_block_is_never_demoted() {
        // same 3-char text at the START stays text (F,D + E,ps pad)
        let data = "FDE\x01\x02";
        assert_eq!(high(data), vec![153, 149, 901, 1, 2]);
    }

    #[test]
    fn six_byte_group_packs_to_five_codewords() {
        // a 6-byte run uses the 924 latch + 5 base-900 codewords
        let six = high("\x01\x02\x03\x04\x05\x06");
        assert_eq!(six.first(), Some(&924));
        assert_eq!(six.len(), 6);
    }

    #[test]
    fn symbol_geometry_matches_reference_conventions() {
        // 429 lowercase chars -> LL latch + 429 values + PS pad = 430 values
        // -> 215 payload codewords; at ECC 5 with 14 columns that is 280
        // total = 20 rows exactly, like the reference.
        let sym = build_symbol(&vec![b'x'; 429], 5, 14, 0, false).unwrap();
        assert_eq!(sym.rows.len(), 20);
        assert_eq!(sym.rows[0].len(), 17 * 14 + 69);
    }

    #[test]
    fn tiny_payload_gets_at_least_three_rows() {
        // "AB" -> 1 codeword + descriptor + 2 ECC = 4 slots, 1 column
        let sym = build_symbol(b"AB", 0, 0, 0, false).unwrap();
        assert_eq!(sym.rows.len(), 4);
    }
}
