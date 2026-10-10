//! End-to-end payload gate for ^BQ boundary parsing. Each fixture renders a
//! full label whose QR payload exercises one Labelary-verified boundary rule;
//! the rendered symbol is decoded with the independent rxing decoder and must
//! round-trip the exact payload. The pre-fix parser mangled all three
//! payloads (it dropped three characters unconditionally and stripped pipes),
//! so this gate fails without the fix.
mod common;

use common::render_helpers;

/// Extract the QR module grid from a rendered label: dark bounding box,
/// majority-vote per module centre (mirrors examples/qr_mask_probe.rs).
fn extract_modules(path: &str, mag: u32) -> Vec<Vec<bool>> {
    let img = image::open(path).expect("decode rendered label").to_luma8();
    let (w, h) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w as i64, h as i64, -1i64, -1i64);
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
    assert!(max_x >= 0, "no dark pixels in {path}");
    let bw = (max_x - min_x + 1) as f64;
    let n = (bw / mag as f64).round() as usize;
    assert!(
        (n as f64 * mag as f64 - bw).abs() < mag as f64,
        "QR bounding box {bw} does not fit {n} modules at mag {mag}"
    );
    let module_px = bw / n as f64;
    let mut grid = vec![vec![false; n]; n];
    for (y, row) in grid.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let cx = min_x as f64 + (x as f64 + 0.5) * module_px;
            let cy = min_y as f64 + (y as f64 + 0.5) * module_px;
            let r = (module_px * 0.4).ceil() as i64;
            let mut dark = 0usize;
            let mut total = 0usize;
            for dy in -r..=r {
                for dx in -r..=r {
                    let (px, py) = (cx as i64 + dx, cy as i64 + dy);
                    if px < 0 || py < 0 || px >= w as i64 || py >= h as i64 {
                        continue;
                    }
                    total += 1;
                    if img.get_pixel(px as u32, py as u32).0[0] < 128 {
                        dark += 1;
                    }
                }
            }
            *cell = dark * 2 > total;
        }
    }
    grid
}

#[test]
fn qr_payload_boundaries_round_trip_through_the_rendered_symbol() {
    let cases = [
        // (fixture, mag, expected decoded payload)
        // Digit designator: the payload starts after the two format chars —
        // the old unconditional 3-char skip encoded "6543210".
        ("qr_digit_payload", 6, "76543210"),
        // Unrecognized non-letter designator ({ then quote): both chars are
        // consumed and the rest kept verbatim — Labelary's symbol decodes
        // identically (the leading { is eaten as the ECC char).
        ("qr_json_payload", 6, "order\":\"123456\",\"code\":\"A9\"}"),
        // Unrecognized letter designator: one separator char consumed, the
        // pipes kept (field separators only for recognized modes).
        ("qr_pipe_kept", 6, "OP0003610428|DEP:8412|PIC:7236"),
    ];
    for (fixture, mag, expected) in cases {
        let zpl = std::fs::read_to_string(format!("testdata/unit/{fixture}.zpl")).expect("fixture");
        let labels = labelize::ZplParser::new().parse(zpl.as_bytes()).unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        labelize::Renderer::new()
            .draw_label_as_png(&labels[0], &mut png, render_helpers::default_options())
            .unwrap();
        let path = format!("/tmp/labelize_qr_gate_{fixture}.png");
        std::fs::write(&path, png.into_inner()).expect("write render");
        let grid = extract_modules(&path, mag);
        let decoded = rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&grid)
            .unwrap_or_else(|e| panic!("{fixture}: QR decode failed: {e:?}"));
        assert_eq!(
            decoded.getText(),
            expected,
            "{fixture}: rendered symbol must decode to the exact payload"
        );
    }
}
