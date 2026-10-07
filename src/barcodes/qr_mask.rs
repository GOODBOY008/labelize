//! Mask selection for standard QR (Model 2), using ISO/IEC 18004:2015 7.8.3.1.
//! The dependency's N4 approximation can choose a non-minimal mask. Keep its
//! codeword construction and mask application, but evaluate the final matrices.
use qrcode::canvas::{Canvas, MaskPattern};
use qrcode::Color;

const MASKS: [MaskPattern; 8] = [
    MaskPattern::Checkerboard,
    MaskPattern::HorizontalLines,
    MaskPattern::VerticalLines,
    MaskPattern::DiagonalLines,
    MaskPattern::LargeCheckerboard,
    MaskPattern::Fields,
    MaskPattern::Diamonds,
    MaskPattern::Meadow,
];

pub(super) fn select(canvas: &Canvas, size: usize) -> Vec<Color> {
    MASKS
        .iter()
        .map(|&mask| {
            let mut candidate = canvas.clone();
            candidate.apply_mask(mask); // Includes the mask-specific format bits.
            candidate.into_colors()
        })
        .min_by_key(|modules| penalty(modules, size))
        .expect("eight QR masks")
}

fn balance_penalty(dark: usize, total: usize) -> u32 {
    // Ten points for each complete 5% deviation from 50%, using integers.
    ((dark * 2).abs_diff(total) * 10 / total * 10) as u32
}

fn penalty(modules: &[Color], size: usize) -> u32 {
    let dark = |x: usize, y: usize| modules[y * size + x] == Color::Dark;
    let mut score = 0u32;
    for transpose in [false, true] {
        for row in 0..size {
            let get = |col| {
                if transpose {
                    dark(row, col)
                } else {
                    dark(col, row)
                }
            };
            let mut run = 1;
            for col in 1..size {
                if get(col) == get(col - 1) {
                    run += 1;
                    if run == 5 {
                        score += 3;
                    } else if run > 5 {
                        score += 1;
                    }
                } else {
                    run = 1;
                }
            }
            // N3: one penalty per 1:1:3:1:1 core with four light modules
            // before OR after it. Outside the matrix is the light quiet zone.
            for col in 0..size.saturating_sub(6) {
                if [true, false, true, true, true, false, true]
                    .iter()
                    .enumerate()
                    .all(|(i, &value)| get(col + i) == value)
                {
                    let before = (col.saturating_sub(4)..col).all(|i| !get(i));
                    let after = (col + 7..(col + 11).min(size)).all(|i| !get(i));
                    if before || after {
                        score += 40;
                    }
                }
            }
        }
    }
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let value = dark(x, y);
            if value == dark(x + 1, y) && value == dark(x, y + 1) && value == dark(x + 1, y + 1) {
                score += 3;
            }
        }
    }
    score
        + balance_penalty(
            modules.iter().filter(|&&c| c == Color::Dark).count(),
            modules.len(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_uses_complete_five_percent_steps_symmetrically() {
        for (dark, expected) in [
            (5000, 0),
            (5499, 0),
            (5501, 10),
            (5999, 10),
            (6001, 20),
            (10000, 100),
        ] {
            assert_eq!(balance_penalty(dark, 10000), expected);
            assert_eq!(balance_penalty(10000 - dark, 10000), expected);
        }
        assert_eq!(balance_penalty(220, 441), 0);
        assert_eq!(balance_penalty(221, 441), 0);
    }

    #[test]
    fn score_does_not_overflow_for_version_forty() {
        assert_eq!(penalty(&vec![Color::Dark; 177 * 177], 177), 154978);
    }

    #[test]
    fn every_candidate_matches_independently_recorded_scores() {
        use qrcode::{bits::Bits, ec::construct_codewords, EcLevel, Version};
        for fixture in [
            include_str!("../../testdata/qr-mask/mask-24.txt"),
            include_str!("../../testdata/qr-mask/mask-37.txt"),
            include_str!("../../testdata/qr-mask/mask-46.txt"),
            include_str!("../../testdata/qr-mask/mask-64.txt"),
            include_str!("../../testdata/qr-mask/mask-907.txt"),
        ] {
            let mut lines = fixture.lines();
            let data = lines.next().unwrap();
            let ec = match lines.next().unwrap() {
                "L" => EcLevel::L,
                "M" => EcLevel::M,
                "Q" => EcLevel::Q,
                "H" => EcLevel::H,
                _ => unreachable!(),
            };
            let _mask = lines.next().unwrap();
            let expected: Vec<u32> = lines
                .next()
                .unwrap()
                .split(',')
                .map(|n| n.parse().unwrap())
                .collect();
            let size = lines.count();
            let version = Version::Normal(((size - 17) / 4) as i16);
            let mut bits = Bits::new(version);
            bits.push_byte_data(data.as_bytes()).unwrap();
            bits.push_terminator(ec).unwrap();
            let (data, ecc) = construct_codewords(&bits.into_bytes(), version, ec).unwrap();
            let mut canvas = Canvas::new(version, ec);
            canvas.draw_all_functional_patterns();
            canvas.draw_data(&data, &ecc);
            for (i, &mask) in MASKS.iter().enumerate() {
                let mut candidate = canvas.clone();
                candidate.apply_mask(mask);
                assert_eq!(
                    penalty(&candidate.into_colors(), size),
                    expected[i],
                    "{ec:?} mask {i}"
                );
            }
        }
    }
}
