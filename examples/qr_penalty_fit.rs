//! Fit Labelary's QR mask-selection rule from measured (payload, ec, ref_mask)
//! datapoints. Rebuilds the masked candidate matrix per mask, scores it under
//! several N1–N4 penalty variants, and reports which variant's argmin matches
//! the mask Labelary actually encoded.
//!
//! Usage: cargo run --example qr_penalty_fit -- /tmp/qrprobe/fit.tsv

use qrcode::bits::Bits;
use qrcode::canvas::{Canvas, MaskPattern};
use qrcode::ec::construct_codewords;
use qrcode::types::EcLevel;

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

#[derive(Clone, Copy)]
struct Scores {
    n1: u32,
    n2: u32,
    n3: u32,
    n4_crate: u32,
    n4_iso: u32,
    n4_zx: u32,
}

fn score_components(colors: &[qrcode::Color], size: usize) -> Scores {
    let dark = |x: usize, y: usize| colors[y * size + x] == qrcode::Color::Dark;

    // N1: runs of 5+ identical modules (light terminator outside the matrix).
    let mut n1 = 0u32;
    for transpose in [false, true] {
        for row in 0..size {
            let get = |col: usize| {
                if transpose {
                    dark(row, col)
                } else {
                    dark(col, row)
                }
            };
            let mut run = 1usize;
            for col in 0..=size {
                let cur = if col < size { Some(get(col)) } else { None };
                let prev = if col == 0 { None } else { Some(get(col - 1)) };
                let same = match (cur, prev) {
                    (Some(c), Some(p)) => c == p,
                    (None, _) | (_, None) => false,
                };
                if same {
                    run += 1;
                } else {
                    if run >= 5 {
                        n1 += run as u32 - 2;
                    }
                    run = 1;
                }
            }
        }
    }

    // N2: 3 points per same-colour 2×2 block.
    let mut n2 = 0u32;
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let v = dark(x, y);
            if v == dark(x + 1, y) && v == dark(x, y + 1) && v == dark(x + 1, y + 1) {
                n2 += 3;
            }
        }
    }

    // N3: 40 points per 1:1:3:1:1 core with four light modules before OR after.
    let mut n3 = 0u32;
    for transpose in [false, true] {
        for row in 0..size {
            let get = |col: usize| {
                if transpose {
                    dark(row, col)
                } else {
                    dark(col, row)
                }
            };
            for j in 0..size.saturating_sub(6) {
                if [true, false, true, true, true, false, true]
                    .iter()
                    .enumerate()
                    .all(|(i, &v)| get(j + i) == v)
                {
                    let before = (j.saturating_sub(4)..j).all(|i| !get(i));
                    let after = (j + 7..(j + 11).min(size)).all(|i| !get(i));
                    if before || after {
                        n3 += 40;
                    }
                }
            }
        }
    }

    let total = size * size;
    let dark_count = (0..total).filter(|&i| colors[i] == qrcode::Color::Dark).count();

    // Crate: linear |200·dark/total − 100| (integer floors).
    let ratio = (dark_count * 200 / total) as u32;
    let n4_crate = if ratio >= 100 { ratio - 100 } else { 100 - ratio };
    // ISO: 10 points per complete 5% deviation from 50%
    // (= floor(|2·dark − total|·10/total) × 10, same as src/barcodes/qr_mask.rs).
    let n4_iso = ((dark_count * 2).abs_diff(total) * 10 / total * 10) as u32;
    // ZXing: |(int)(ratio·100 − 50)| / 5 × 10.
    let zx = ((dark_count as f64 / total as f64) * 100.0 - 50.0) as i32;
    let n4_zx = (zx.abs() / 5 * 10) as u32;


    Scores {
        n1,
        n2,
        n3,
        n4_crate,
        n4_iso,
        n4_zx,
    }
}

fn pick(per_mask: &[Scores], f: impl Fn(&Scores) -> u32) -> usize {
    let mut best = 0usize;
    let mut best_score = u32::MAX;
    for (i, s) in per_mask.iter().enumerate() {
        let v = f(s);
        if v < best_score {
            best_score = v;
            best = i;
        }
    }
    best
}

fn main() {
    let path = std::env::args().nth(1).expect("fit.tsv path");
    let text = std::fs::read_to_string(&path).unwrap();

    let mut tally = [("crate", 0u32), ("iso", 0), ("zx", 0), ("no_n4", 0)];
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let payload = parts.next().unwrap();
        let ec = match parts.next().unwrap() {
            "L" => EcLevel::L,
            "M" => EcLevel::M,
            "Q" => EcLevel::Q,
            "H" => EcLevel::H,
            _ => unreachable!(),
        };
        let ref_mask: usize = parts.next().unwrap().parse().unwrap();
        let _pre_mask: usize = parts.next().unwrap().parse().unwrap();

        let bits: Bits = qrcode::bits::encode_auto(payload.as_bytes(), ec).unwrap();
        let version = bits.version();
        let size = version.width() as usize;
        let (data, ecc) = construct_codewords(&bits.into_bytes(), version, ec).unwrap();
        let mut canvas = Canvas::new(version, ec);
        canvas.draw_all_functional_patterns();
        canvas.draw_data(&data, &ecc);

        let per_mask: Vec<Scores> = MASKS
            .iter()
            .map(|&mask| {
                let mut candidate = canvas.clone();
                candidate.apply_mask(mask);
                let colors = candidate.into_colors();
                score_components(&colors, size)
            })
            .collect();

        let picks = [
            ("crate", pick(&per_mask, |s| s.n1 + s.n2 + s.n3 + s.n4_crate)),
            ("iso", pick(&per_mask, |s| s.n1 + s.n2 + s.n3 + s.n4_iso)),
            ("zx", pick(&per_mask, |s| s.n1 + s.n2 + s.n3 + s.n4_zx)),
            ("no_n4", pick(&per_mask, |s| s.n1 + s.n2 + s.n3)),
        ];
        for (name, got) in &picks {
            if got == &ref_mask {
                tally.iter_mut().find(|(n, _)| n == name).unwrap().1 += 1;
            }
        }
        rows.push((payload.to_string(), ref_mask, picks));
    }

    println!("=== match counts (of {})", rows.len());
    for (name, count) in &tally {
        println!("  {name}: {count}");
    }
    if std::env::args().nth(2).as_deref() == Some("--dump") {
        for line in text.lines() {
            let mut parts = line.split('\t');
            let payload = parts.next().unwrap();
            let ec = match parts.next().unwrap() {
                "L" => EcLevel::L,
                "M" => EcLevel::M,
                "Q" => EcLevel::Q,
                _ => EcLevel::H,
            };
            let ref_mask: usize = parts.next().unwrap().parse().unwrap();
            let bits = qrcode::bits::encode_auto(payload.as_bytes(), ec).unwrap();
            let version = bits.version();
            let size = version.width() as usize;
            let (data, ecc) = construct_codewords(&bits.into_bytes(), version, ec).unwrap();
            let mut canvas = Canvas::new(version, ec);
            canvas.draw_all_functional_patterns();
            canvas.draw_data(&data, &ecc);
            let comps: Vec<(u32, u32, u32, u32, u32, u32)> = MASKS
                .iter()
                .map(|&mask| {
                    let mut c = canvas.clone();
                    c.apply_mask(mask);
                    let colors = c.into_colors();
                    let s = score_components(&colors, size);
                    (s.n1, s.n2, s.n3, s.n4_crate, s.n4_iso, s.n4_zx)
                })
                .collect();
            println!("DUMP\t{payload}\t{ref_mask}\t{comps:?}");
        }
    }
    println!("\n=== divergent payloads (ref vs variants)");
    for (payload, ref_mask, picks) in &rows {
        if !picks.iter().all(|(_, m)| m == ref_mask) {
            let p = if payload.len() > 40 {
                format!("{}..", &payload[..40])
            } else {
                payload.clone()
            };
            println!(
                "{p:<42} ref={ref_mask} crate={} iso={} zx={} no_n4={}",
                picks[0].1, picks[1].1, picks[2].1, picks[3].1
            );
        }
    }
}

// {{{
#[cfg(test)]
mod probe_dump {
    // Calibration against the crate's own penalty test constants.
    #[test]
    fn components_match_crate_constants() {
        use super::*;
        use qrcode::EcLevel as E;
        use qrcode::types::Version;
        let mut canvas = Canvas::new(Version::Normal(1), E::Q);
        canvas.draw_all_functional_patterns();
        canvas.draw_data(
            b"\x20\x5b\x0b\x78\xd1\x72\xdc\x4d\x43\x40\xec\x11\x00",
            b"\xa8\x48\x16\x52\xd9\x36\x9c\x00\x2e\x0f\xb4\x7a\x10",
        );
        canvas.apply_mask(MaskPattern::Checkerboard);
        let colors = canvas.into_colors();
        let s = score_components(&colors, 21);
        // Crate expects: adjacent(h)=88, adjacent(v)=92 (sum 180), block=90,
        // finder(h)=0, finder(v)=40, balance=2.
        assert_eq!(s.n1, 180, "N1 h+v");
        assert_eq!(s.n2, 90, "N2");
        assert_eq!(s.n3, 760, "N3 raw h+v (crate subtracts 360 per orientation)");
        assert_eq!(s.n4_crate, 2, "N4 crate linear");
    }
}
// }}}
