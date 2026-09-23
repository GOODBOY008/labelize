// SPDX-FileCopyrightText: 2026 David Block
// SPDX-License-Identifier: MIT
//
// Ported from Streetblock/QR-Atelier (MIT option of MIT OR Apache-2.0):
// libs/DMlegacy.js and libs/DMlegacyPlacementGenerator.js, commit
// 432aa76de2116adc055062f91ef622f4dc9568cb. See licenses/QR-Atelier-MIT.txt and docs/DATAMATRIX_LEGACY.md.
// Rust adaptation: byte input, bounded lazy cache, BitMatrix output. The
// random-stream byte at index 211 uses the norm-confirmed BC value.

//! Historical Data Matrix ECC 000, 050, 080, 100 and 140.

use super::BitMatrix;
use std::sync::OnceLock;

static PLACEMENTS: [OnceLock<Vec<usize>>; 21] = [const { OnceLock::new() }; 21];

/// Return the immutable placement for an odd complete symbol size 9..=49.
/// Each requested size is computed once; no normative grid ships at runtime.
pub fn placement_for_size(symbol_side: usize) -> Result<&'static [usize], String> {
    if !(9..=49).contains(&symbol_side) || symbol_side.is_multiple_of(2) {
        return Err("Legacy DataMatrix: symbol size must be odd, from 9 through 49".into());
    }
    Ok(PLACEMENTS[(symbol_side - 9) / 2].get_or_init(|| generate_placement(symbol_side)))
}

// Empirically reconstructed from historical grids, not a quoted normative
// algorithm. All 21 supported sizes are checked against fixed PDF references.
fn generate_placement(symbol_side: usize) -> Vec<usize> {
    let n = symbol_side - 2;
    let width = usize::BITS - (n - 1).leading_zeros();
    let order: Vec<usize> = (0..1usize << width)
        .map(|i| i.reverse_bits() >> (usize::BITS - width))
        .filter(|&i| i < n)
        .collect();
    let mut inverse = vec![0; n];
    for (index, &value) in order.iter().enumerate() {
        inverse[value] = index;
    }
    let mut placement = vec![0; n * n];
    for row in 0..n {
        let k = n - 1 - row;
        for col in 0..n {
            let shifted = (col + 2 * n - 2 * k) % n;
            placement[row * n + col] = n * order[shifted] + inverse[k];
        }
    }
    // Swap instead of overwriting: each displaced value remains represented.
    for (value, corner) in [(0, n * (n - 1)), (1, n - 1), (2, 0), (3, n * n - 1)] {
        let from = placement.iter().position(|&v| v == value).unwrap();
        placement.swap(from, corner);
    }
    placement
}

fn push_lsb(bits: &mut Vec<bool>, value: u32, width: usize) {
    bits.extend((0..width).map(|bit| value & (1 << bit) != 0));
}

fn crc_register(format: u8, data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in [format, 0].iter().chain(data) {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x8408
            } else {
                crc >> 1
            };
        }
    }
    crc
}

fn encode_data(data: &[u8], format: u8) -> Result<Vec<bool>, String> {
    let mut bits = Vec::new();
    let (alphabet, group_size, widths): (&[u8], usize, &[usize]) = match format {
        1 => (b" 0123456789", 6, &[0, 4, 7, 11, 14, 18, 21]),
        2 => (b" ABCDEFGHIJKLMNOPQRSTUVWXYZ", 5, &[0, 5, 10, 15, 20, 24]),
        3 => (
            b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,-/",
            4,
            &[0, 6, 11, 17, 22],
        ),
        4 => (
            b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            4,
            &[0, 6, 11, 16, 21],
        ),
        5 | 6 => {
            for &byte in data {
                if format == 5 && byte > 127 {
                    return Err("Legacy DataMatrix: format 5 requires 7-bit ASCII".into());
                }
                push_lsb(&mut bits, u32::from(byte), if format == 5 { 7 } else { 8 });
            }
            return Ok(bits);
        }
        _ => return Err("Legacy DataMatrix: format must be 1 through 6".into()),
    };
    for group in data.chunks(group_size) {
        let mut value = 0u32;
        let mut weight = 1u32;
        for &byte in group {
            let digit = alphabet.iter().position(|&v| v == byte).ok_or_else(|| {
                format!("Legacy DataMatrix: byte 0x{byte:02X} is not allowed in format {format}")
            })?;
            value += digit as u32 * weight;
            weight *= alphabet.len() as u32;
        }
        push_lsb(&mut bits, value, widths[group.len()]);
    }
    Ok(bits)
}

struct EccConfig {
    header: &'static [u8],
    minimum_side: usize,
    input_width: usize,
    flush_cycles: usize,
    taps: &'static [&'static [usize]],
}

fn ecc_config(quality: u16) -> Result<EccConfig, String> {
    // Flattened window: current input group, then successively older groups.
    // Historical Annex K state machines, ported from the attributed JS source.
    let (header, minimum_side, input_width, flush_cycles, taps): (
        &[u8],
        usize,
        usize,
        usize,
        &[&[usize]],
    ) = match quality {
        0 => (b"0111111", 9, 1, 0, &[]),
        50 => (
            b"0111000000000111000",
            11,
            3,
            3,
            &[
                &[0, 5, 8, 10, 11],
                &[1, 4, 6, 9, 10],
                &[2, 3, 4, 5, 6, 9],
                &[0, 1, 2, 3, 4, 5, 7, 11],
            ],
        ),
        80 => (
            b"0111000000111000111",
            13,
            2,
            11,
            &[
                &[0, 2, 6, 7, 10, 12, 14, 15, 17, 20, 23],
                &[1, 2, 7, 8, 10, 13, 16, 17, 18, 19, 20],
                &[0, 1, 3, 5, 9, 10, 12, 14, 15, 19, 23],
            ],
        ),
        100 => (
            b"0111000000111111111",
            13,
            1,
            15,
            &[
                &[0, 2, 5, 6, 7, 8, 9, 10, 15],
                // No tap at delay 12: confirmed in the historical diagrams.
                &[0, 1, 3, 4, 6, 11, 13, 14, 15],
            ],
        ),
        140 => (
            b"0111000111000111111",
            17,
            1,
            13,
            &[
                &[0, 4, 7, 10, 12, 13],
                &[0, 3, 4, 7, 8, 9, 10, 11, 13],
                &[0, 1, 2, 4, 5, 7, 9, 11, 12, 13],
                &[0, 1, 2, 4, 5, 7, 9, 10, 11, 12, 13],
            ],
        ),
        _ => return Err("Legacy DataMatrix: ECC must be 0, 50, 80, 100, or 140".into()),
    };
    Ok(EccConfig {
        header,
        minimum_side,
        input_width,
        flush_cycles,
        taps,
    })
}

fn protect_bits(input: &[bool], config: &EccConfig) -> Vec<bool> {
    if config.taps.is_empty() {
        return input.to_vec();
    }
    let width = config.input_width;
    let groups = input.len().div_ceil(width);
    let mut window = vec![false; (config.flush_cycles + 1) * width];
    let mut output = Vec::with_capacity((groups + config.flush_cycles) * config.taps.len());
    // Pad the final partial group, then flush the entire state with zero groups.
    // The ECC header is added later and is never fed into this state machine.
    for cycle in 0..groups + config.flush_cycles {
        let history_end = window.len() - width;
        window.copy_within(..history_end, width);
        for (offset, slot) in window[..width].iter_mut().enumerate() {
            *slot = input.get(cycle * width + offset).copied().unwrap_or(false);
        }
        output.extend(config.taps.iter().map(|taps| {
            taps.iter()
                .fold(false, |parity, &index| parity ^ window[index])
        }));
    }
    output
}

/// Encode ECC 000 literal bytes; no ZPL escape interpretation or UTF-8 conversion.
/// `symbol_size` is the complete square size including the finder border.
/// Oversized input fails without truncation or fallback to a different ECC.
pub fn encode(data: &[u8], format: u8, symbol_size: Option<usize>) -> Result<BitMatrix, String> {
    encode_with_ecc(data, format, 0, symbol_size)
}

/// Encode a selected historical ECC quality (0, 50, 80, 100 or 140).
/// Dimensions include the finder border; invalid or insufficient sizes fail.
pub fn encode_with_ecc(
    data: &[u8],
    format: u8,
    quality: u16,
    symbol_size: Option<usize>,
) -> Result<BitMatrix, String> {
    let config = ecc_config(quality)?;
    if data.is_empty() {
        return Err("Legacy DataMatrix: input must not be empty".into());
    }
    if data.len() > 511 {
        return Err("Legacy DataMatrix: lengths above 511 are not yet supported; extended record-length semantics are unresolved".into());
    }
    let content = encode_data(data, format)?;
    let mut record: Vec<bool> = (0..5)
        .rev()
        .map(|bit| (format - 1) & (1 << bit) != 0)
        .collect();
    push_lsb(&mut record, u32::from(crc_register(format, data)), 16);
    push_lsb(&mut record, data.len() as u32, 9);
    record.extend(content);
    let mut bits: Vec<bool> = config.header.iter().map(|&bit| bit == b'1').collect();
    bits.extend(protect_bits(&record, &config));
    let size = if let Some(size) = symbol_size {
        placement_for_size(size)?;
        if size < config.minimum_side {
            return Err(format!(
                "Legacy DataMatrix: ECC {quality:03} requires a symbol size of at least {}",
                config.minimum_side
            ));
        }
        size
    } else {
        (config.minimum_side..=49)
            .step_by(2)
            .find(|size| (size - 2) * (size - 2) >= bits.len())
            .ok_or("Legacy DataMatrix: content exceeds maximum symbol capacity")?
    };
    let n = size - 2;
    if bits.len() > n * n {
        return Err("Legacy DataMatrix: content does not fit the requested symbol size".into());
    }
    bits.resize(n * n, false);
    for (index, bit) in bits.iter_mut().enumerate() {
        *bit ^= (MASTER_RANDOM[index / 8] >> (7 - index % 8)) & 1 != 0;
    }
    let placement = placement_for_size(size)?;
    let mut matrix = BitMatrix::new(size, size);
    for col in 0..size {
        matrix.set(col, 0, col % 2 == 0);
        matrix.set(col, size - 1, true);
    }
    for row in 0..size {
        matrix.set(0, row, true);
        matrix.set(size - 1, row, row % 2 == 0);
    }
    for row in 0..n {
        for col in 0..n {
            matrix.set(col + 1, row + 1, bits[placement[row * n + col]]);
        }
    }
    Ok(matrix)
}

/// ZPL Legacy dimensions: entries above 49 become automatic; the larger valid
/// requested dimension wins. Unsupported small/even dimensions are errors.
pub(crate) fn zpl_symbol_size(rows: i32, columns: i32) -> Result<Option<usize>, String> {
    let mut size = 0;
    for value in [rows, columns] {
        if value == 0 || value > 49 {
            continue;
        }
        if !(9..=49).contains(&value) || value % 2 == 0 {
            return Err(
                "Legacy DataMatrix: ZPL dimensions must be zero or odd values 9 through 49".into(),
            );
        }
        size = size.max(value);
    }
    Ok((size != 0).then_some(size as usize))
}

// Annex L, MSB first, with the last unused storage bits zero. Byte 211 is
// BC, as confirmed in the norm PDFs and corrected in the source commit.

const MASTER_RANDOM: [u8; 277] = [
    0x05, 0xff, 0xc7, 0x31, 0x88, 0xa8, 0x83, 0x9c, 0x64, 0x87, 0x9f, 0x64, 0xb3, 0xe0, 0x4d, 0x9c,
    0x80, 0x29, 0x3a, 0x90, 0xb3, 0x8b, 0x9e, 0x90, 0x45, 0xbf, 0xf5, 0x68, 0x4b, 0x08, 0xcf, 0x44,
    0xb8, 0xd4, 0x4c, 0x5b, 0xa0, 0xab, 0x72, 0x52, 0x1c, 0xe4, 0xd2, 0x74, 0xa4, 0xda, 0x8a, 0x08,
    0xfa, 0xa7, 0xc7, 0xdd, 0x00, 0x30, 0xa9, 0xe6, 0x64, 0xab, 0xd5, 0x8b, 0xed, 0x9c, 0x79, 0xf8,
    0x08, 0xd1, 0x8b, 0xc6, 0x22, 0x64, 0x0b, 0x33, 0x43, 0xd0, 0x80, 0xd4, 0x44, 0x95, 0x2e, 0x6f,
    0x5e, 0x13, 0x8d, 0x47, 0x62, 0x06, 0xeb, 0x80, 0x82, 0xc9, 0x41, 0xd5, 0x73, 0x8a, 0x30, 0x23,
    0x24, 0xe3, 0x7f, 0xb2, 0xa8, 0x0b, 0xed, 0x38, 0x42, 0x4c, 0xd7, 0xb0, 0xce, 0x98, 0xbd, 0xe1,
    0xd5, 0xe4, 0xc3, 0x1d, 0x15, 0x4a, 0xcf, 0xd1, 0x1f, 0x39, 0x26, 0x18, 0x93, 0xfc, 0x19, 0xb2,
    0x2d, 0xab, 0xf2, 0x6e, 0xa1, 0x9f, 0xaf, 0xd0, 0x8a, 0x2b, 0xa0, 0x56, 0xb0, 0x41, 0x6d, 0x43,
    0xa4, 0x63, 0xf3, 0xaa, 0x7d, 0xaf, 0x35, 0x57, 0xc2, 0x94, 0x4a, 0x65, 0x0b, 0x41, 0xde, 0xb8,
    0xe2, 0x30, 0x12, 0x27, 0x9b, 0x66, 0x2b, 0x34, 0x5b, 0xb8, 0x99, 0xe8, 0x28, 0x71, 0xd0, 0x95,
    0x6b, 0x07, 0x4d, 0x3c, 0x7a, 0xb3, 0xe5, 0x29, 0xb3, 0xba, 0x8c, 0xcc, 0x2d, 0xe0, 0xc9, 0xc0,
    0x22, 0xec, 0x4c, 0xde, 0xf8, 0x58, 0x07, 0xfc, 0x19, 0xf2, 0x64, 0xe2, 0xc3, 0xe2, 0xd8, 0xb9,
    0xfd, 0x67, 0xa0, 0xbc, 0xf5, 0x2e, 0xc9, 0x49, 0x75, 0x62, 0x82, 0x27, 0x10, 0xf4, 0x19, 0x6f,
    0x49, 0xf7, 0xb3, 0x84, 0x14, 0xea, 0xeb, 0xe1, 0x2a, 0x31, 0xab, 0x47, 0x7d, 0x08, 0x29, 0xac,
    0xbb, 0x72, 0xfa, 0xfa, 0x62, 0xb8, 0xc8, 0xd3, 0x86, 0x89, 0x95, 0xfd, 0xdf, 0xcc, 0x9c, 0xad,
    0xf1, 0xd4, 0x6c, 0x64, 0x23, 0x24, 0x2a, 0x56, 0x1f, 0x36, 0xeb, 0xb7, 0xd6, 0xff, 0xda, 0x57,
    0xf4, 0x50, 0x79, 0x08, 0x00,
];
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecc050_annex_q_protected_bits() {
        let input = "000101001101010101110011000000001001011110110011111011111111110";
        let expected = "000010101011111110101010101000000100001101101000010100011000000011101010100110101001100001001010";
        let input: Vec<bool> = input.bytes().map(|b| b == b'1').collect();
        assert_eq!(
            bit_string(&protect_bits(&input, &ecc_config(50).unwrap())),
            expected
        );
    }

    #[test]
    fn convolution_stages_match_js_port_including_partial_groups_and_flush() {
        let fixtures = include_str!("../../testdata/legacy/convolution-js-stages.txt");
        let mut count = 0;
        for line in fixtures.lines() {
            let fields: Vec<_> = line.split('|').collect();
            let quality = fields[0].parse().unwrap();
            let input: Vec<bool> = fields[1].bytes().map(|b| b == b'1').collect();
            let config = ecc_config(quality).unwrap();
            let actual = protect_bits(&input, &config);
            assert_eq!(
                bit_string(&actual),
                fields[2],
                "ECC {quality}, input length {}",
                input.len()
            );
            assert_eq!(
                actual.len(),
                (input.len().div_ceil(config.input_width) + config.flush_cycles)
                    * config.taps.len()
            );
            count += 1;
        }
        assert_eq!(count, 200);
    }

    #[test]
    fn ecc100_has_no_delay_12_tap_and_flushes_all_15_delays() {
        // Single input impulse from the reviewed Annex K diagram: two outputs
        // per cycle, current cycle followed by all fifteen delayed cycles.
        let actual = protect_bits(&[true], &ecc_config(100).unwrap());
        assert_eq!(bit_string(&actual), "11011001011011101010100100010111");
        assert_eq!(&actual[24..26], &[false, false]);
        assert_eq!(&actual[30..32], &[true, true]);
    }

    fn bit_string(bits: &[bool]) -> String {
        bits.iter().map(|&v| if v { '1' } else { '0' }).collect()
    }

    #[test]
    fn norm_crc_and_partial_base_groups() {
        assert_eq!(crc_register(3, b"AB12-X"), 0x7559);
        let cases: &[(u8, &[u8], &[&str])] = &[
            (
                1,
                b"123456",
                &[
                    "0100",
                    "1100010",
                    "11100000010",
                    "01100000001110",
                    "001101001100111010",
                    "100101110110010101001",
                ],
            ),
            (
                2,
                b"ABCDE",
                &[
                    "10000",
                    "1110110000",
                    "010000110001000",
                    "01110010001111001000",
                    "110000000001001110010100",
                ],
            ),
            (
                3,
                b"ABC.",
                &[
                    "100000",
                    "11001010000",
                    "01100000001010000", // A + 41*B + 41^2*C = 5126, LSB first.
                    "1100001010111111011001",
                ],
            ),
            (
                4,
                b"ABC1",
                &[
                    "100000",
                    "11010010000",
                    "0110101000001000",
                    "010000010010110110101",
                ],
            ),
        ];
        for &(format, data, expected) in cases {
            for (index, &bits) in expected.iter().enumerate() {
                assert_eq!(
                    bit_string(&encode_data(&data[..index + 1], format).unwrap()),
                    bits,
                    "format {format}, length {}",
                    index + 1
                );
            }
        }
        assert_eq!(
            bit_string(&encode_data(b"AB12-X", 3).unwrap()),
            concat!("0010010111101100111110", "11111111110")
        );
    }

    #[test]
    fn literal_bytes_and_format_repertoires() {
        assert_eq!(bit_string(&encode_data(b"B", 5).unwrap()), "0100001");
        assert_eq!(bit_string(&encode_data(&[0x96], 6).unwrap()), "01101001");
        assert!(encode_data(&[128], 5).is_err());
        for format in 1..=4 {
            assert!(encode_data(b"a", format).is_err());
        }
        for format in [0, 7, 255] {
            assert!(encode(b"A", format, None).is_err());
        }
        assert!(encode(&(0..=255).collect::<Vec<u8>>(), 6, None).is_ok());
    }

    #[test]
    fn dimensions_and_capacity_never_truncate_or_fallback() {
        assert!(encode(&[], 6, None).is_err());
        assert!(encode(&vec![b'1'; 512], 1, None).is_err());
        assert!(encode(&vec![b'1'; 511], 1, None).is_ok());
        assert!(encode(&vec![0; 272], 6, None).is_err());
        assert!(encode(&vec![0; 271], 6, None).is_ok());
        assert!(encode(b"AB12", 6, Some(9)).is_err());
        for size in [0, 7, 8, 10, 50, 51, usize::MAX] {
            assert!(placement_for_size(size).is_err());
            assert!(encode(b"1", 1, Some(size)).is_err());
        }
        for (rows, cols, expected) in [
            (0, 0, None),
            (13, 17, Some(17)),
            (17, 13, Some(17)),
            (50, 0, None),
            (0, 51, None),
            (51, 17, Some(17)),
        ] {
            assert_eq!(zpl_symbol_size(rows, cols).unwrap(), expected);
        }
        for (rows, cols) in [(8, 13), (10, 13), (-1, 0), (0, -1)] {
            assert!(zpl_symbol_size(rows, cols).is_err());
        }
        assert_eq!(MASTER_RANDOM[211], 0xbc);
        assert_eq!(MASTER_RANDOM[276], 0);
    }
}
