//! Symbol-level QR probe: compare the QR module grid between our renderer and
//! a Labelary reference render of the same minimal ZPL, and read the mask id +
//! EC level each side encoded in its format information (BCH-validated).
//!
//! Usage:
//!   cargo run --example qr_mask_probe -- <case.zpl> <labelary.png> <mag>
//!
//! The ZPL must contain exactly one dark element (the QR field). The reference
//! PNG is the Labelary render of the same ZPL at 8dpmm / 4.005x8.01in.

use labelize::{DrawerOptions, Renderer};

const FORMAT_XOR: u32 = 0b101010000010010;
const BCH_GEN: u32 = 0b10100110111;

/// The 32 valid raw format-info 15-bit words (already XORed with 0x5412),
/// indexed by (ecl, mask).
fn format_words() -> Vec<(u8, u8, u32)> {
    let ecl_bits: [(u8, u32); 4] = [(1, 0b01), (0, 0b00), (3, 0b11), (2, 0b10)]; // L M Q H
    let mut out = Vec::new();
    for &(id, bits) in &ecl_bits {
        for mask in 0u32..8 {
            let data = (bits << 3) | mask;
            // Append 10 BCH bits: remainder of data << 10 modulo the generator.
            let mut rem = data << 10;
            for i in (10..=14).rev() {
                if rem & (1 << i) != 0 {
                    rem ^= BCH_GEN << (i - 10);
                }
            }
            out.push((id, mask as u8, ((data << 10) | (rem & 0x3FF)) ^ FORMAT_XOR));
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (zpl_path, ref_path, mag) = match args.as_slice() {
        [a, b, c] => (a.clone(), b.clone(), c.parse::<u32>().unwrap()),
        _ => panic!("usage: qr_mask_probe <case.zpl> <labelary.png> <mag>"),
    };

    let zpl = std::fs::read(&zpl_path).unwrap();
    let labels = labelize::ZplParser::new().parse(&zpl).unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(&labels[0], &mut png, DrawerOptions::default())
        .unwrap();
    let local = image::load_from_memory(&png.into_inner())
        .unwrap()
        .to_luma8();
    let reference = image::open(&ref_path).unwrap().to_luma8();

    let local_grid = extract(&local, mag);
    let ref_grid = extract(&reference, mag);

    let ecl_name = |e: u8| match e {
        1 => "L",
        0 => "M",
        3 => "Q",
        2 => "H",
        _ => "?",
    };
    println!(
        "local   : side={} modules (module_px {:.2}), format: mask={} ec={}{}",
        local_grid.n,
        local_grid.module_px,
        local_grid.mask,
        local_grid.ecl,
        if local_grid.format_ok {
            ""
        } else {
            " [BCH MISMATCH]"
        }
    );
    println!(
        "labelary: side={} modules (module_px {:.2}), format: mask={} ec={}{}",
        ref_grid.n,
        ref_grid.module_px,
        ref_grid.mask,
        ref_grid.ecl,
        if ref_grid.format_ok {
            ""
        } else {
            " [BCH MISMATCH]"
        }
    );

    if local_grid.n != ref_grid.n {
        println!(
            "RESULT: version mismatch (side {} vs {})",
            local_grid.n, ref_grid.n
        );
        return;
    }
    let n = local_grid.n;
    let mut diffs = 0usize;
    let mut fn_pattern_diffs = 0usize;
    let mut data_diffs = 0usize;
    for y in 0..n {
        for x in 0..n {
            if local_grid.dark[y][x] != ref_grid.dark[y][x] {
                diffs += 1;
                if is_function_pattern(x, y, n) {
                    fn_pattern_diffs += 1;
                } else {
                    data_diffs += 1;
                }
            }
        }
    }
    let total = n * n;
    println!(
        "RESULT: grid diff {}/{} modules ({:.2}%), function-pattern diffs={}, data diffs={}",
        diffs,
        total,
        100.0 * diffs as f64 / total as f64,
        fn_pattern_diffs,
        data_diffs
    );
    match (
        rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&local_grid.dark),
        rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&ref_grid.dark),
    ) {
        (Ok(a), Ok(b)) => {
            println!(
                "RESULT: local decodes {:?}, labelary decodes {:?}",
                a.getText(),
                b.getText()
            );
            println!(
                "RESULT: payloads {}",
                if a.getText() == b.getText() {
                    "IDENTICAL"
                } else {
                    "DIFFER"
                }
            );
        }
        (e1, e2) => println!(
            "RESULT: decode failed local={} labelary={}",
            matches!(e1, Err(_)),
            matches!(e2, Err(_))
        ),
    }
    if local_grid.mask == ref_grid.mask && local_grid.ecl == ref_grid.ecl {
        println!(
            "RESULT: format agrees (mask {}, ec {})",
            local_grid.mask,
            ecl_name(local_grid.ecl)
        );
    } else {
        println!(
            "RESULT: FORMAT MISMATCH — local mask {}/ec {}, labelary mask {}/ec {}",
            local_grid.mask,
            ecl_name(local_grid.ecl),
            ref_grid.mask,
            ecl_name(ref_grid.ecl)
        );
    }
}

struct Grid {
    dark: Vec<Vec<bool>>,
    n: usize,
    module_px: f64,
    mask: u8,
    ecl: u8,
    format_ok: bool,
}

/// Binarize, find the dark bounding box, and sample the module grid.
fn extract(img: &image::GrayImage, mag: u32) -> Grid {
    let (w, h) = img.dimensions();
    let mut min_x = w as i64;
    let mut min_y = h as i64;
    let mut max_x = -1i64;
    let mut max_y = -1i64;
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y).0[0] < 128 {
                min_x = min_x.min(x as i64);
                min_y = min_y.min(y as i64);
                max_x = max_x.max(x as i64);
                max_y = max_y.max(y as i64);
            }
        }
    }
    assert!(max_x >= 0, "no dark pixels found");
    let bw = (max_x - min_x + 1) as f64;
    let bh = (max_y - min_y + 1) as f64;
    assert!(
        (bw - bh).abs() <= 1.0,
        "QR bounding box not square: {bw}x{bh}"
    );

    // Symbol side is 4k+17 modules; pick the size whose mag-scaled extent best
    // matches the bounding box (Labelary edges can be a pixel short).
    let mut n = 0usize;
    let mut best_err = f64::MAX;
    let mut k = 1usize;
    loop {
        let cand = 17 + 4 * k;
        if cand as f64 * mag as f64 > bw + mag as f64 {
            break;
        }
        let err = (cand as f64 * mag as f64 - bw).abs();
        if err < best_err {
            best_err = err;
            n = cand;
        }
        k += 1;
    }
    assert!(n > 0, "no QR size fits bbox {bw} at mag {mag}");
    let module_px = bw / n as f64;

    let mut grid = vec![vec![false; n]; n];
    for y in 0..n {
        for x in 0..n {
            let cx = min_x as f64 + (x as f64 + 0.5) * module_px;
            let cy = min_y as f64 + (y as f64 + 0.5) * module_px;
            // Majority vote over a box of up to ±40% of a module.
            let r = (module_px * 0.4).ceil() as i64;
            let mut dark_count = 0usize;
            let mut total = 0usize;
            for dy in -r..=r {
                for dx in -r..=r {
                    let px = cx as i64 + dx;
                    let py = cy as i64 + dy;
                    if px < 0 || py < 0 || px >= w as i64 || py >= h as i64 {
                        continue;
                    }
                    total += 1;
                    if img.get_pixel(px as u32, py as u32).0[0] < 128 {
                        dark_count += 1;
                    }
                }
            }
            grid[y][x] = dark_count * 2 > total;
        }
    }

    let (mask, ecl, format_ok) = read_format_info(&grid, n);

    Grid {
        dark: grid,
        n,
        module_px,
        mask,
        ecl,
        format_ok,
    }
}

/// Read both format-info copies (bit 0 at (8,0) … bit 14 at (0,8); second copy
/// mirrored at the bottom-right), majority-vote each bit, then BCH-validate
/// against the 32 valid format words.
fn read_format_info(grid: &Vec<Vec<bool>>, n: usize) -> (u8, u8, bool) {
    let first = [
        (8usize, 0usize),
        (8, 1),
        (8, 2),
        (8, 3),
        (8, 4),
        (8, 5),
        (8, 7),
        (8, 8),
        (7, 8),
        (5, 8),
        (4, 8),
        (3, 8),
        (2, 8),
        (1, 8),
        (0, 8),
    ];
    let mut raw = vec![0u32; 15];
    for (i, &(x, y)) in first.iter().enumerate() {
        raw[i] += grid[y][x] as u32;
    }
    // Second copy around the bottom-right corner.
    for i in 0..8usize {
        let (x, y) = (n - 1 - i, 8);
        raw[i] += grid[y][x] as u32;
    }
    for i in 8..15usize {
        // Crate side copy: bit14 at (8, n-1) … bit8 at (8, n-7) — the bottom
        // strip of column 8 (FORMAT_INFO_COORDS_QR_SIDE, MSB first).
        let (x, y) = (8, n - 15 + i);
        raw[i] += grid[y][x] as u32;
    }

    let table = format_words();
    // Find the valid word with the fewest flipped bits (distance ≤ 5 tolerates
    // one copy badly mis-sampled; BCH protects 3 bits per copy).
    let mut best = (u32::MAX, 0u8, 0u8);
    for &(ecl, mask, word) in &table {
        let dist: u32 = (0..15)
            .map(|i| ((word >> i) & 1) != (if raw[i] >= 2 { 1 } else { 0 }) as u32)
            .filter(|b| *b)
            .count() as u32;
        if dist < best.0 {
            best = (dist, mask, ecl);
        }
    }
    let (dist, mask, ecl) = best;
    let ok = dist <= 5;
    (mask, ecl, ok)
}

/// Finder patterns + separators + timing patterns are function patterns that
/// must not depend on the mask.
fn is_function_pattern(x: usize, y: usize, n: usize) -> bool {
    let in_finder = |cx: usize, cy: usize| x < cx + 8 && y < cy + 8;
    if in_finder(0, 0) || in_finder(n - 8, 0) || in_finder(0, n - 8) {
        return true;
    }
    if x == 6 || y == 6 {
        return true;
    }
    false
}
