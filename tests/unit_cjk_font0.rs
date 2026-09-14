//! Regression tests for issue #51: under `^CI28` (UTF-8), scalable font 0
//! fields must render CJK characters exactly like the Labelary reference:
//! as **blank space** — no .notdef box, no fallback glyph — while the pen
//! still advances by the calibrated missing-glyph width
//! (`tuning::FONT0_MISSING_GLYPH_ADVANCE_EM`), so trailing text lands where
//! Labelary puts it.
//!
//! Labelary probes (`^FD123` vs `^FD中123` vs `^FD中文123`, hanzi/katakana/
//! hangul) measured a constant 12.5 px per missing character at a 42 dot cell
//! (≈0.2976 em). The assertions below check ink pixels, not just signatures.

mod common;

use common::render_helpers;
use labelize::DrawerOptions;

/// The exact reproduction ZPL from issue #51.
const ISSUE_ZPL: &str = "^XA
^CI28
^PW816
^LL1216
^FO48,48^A0N,42,42^FDABC中文测试123^FS
^FO48,140^BCN,120,Y,N,N^FDDEMO123^FS
^XZ
";

/// A single missing glyph followed by digits: the digits' ink start isolates
/// the missing-glyph advance against the baseline without one.
const ONE_HANZI_ZPL: &str = "^XA
^CI28
^FO48,48^A0N,42,42^FD中123^FS
^XZ
";

const BASELINE_ZPL: &str = "^XA
^CI28
^FO48,48^A0N,42,42^FD123^FS
^XZ
";

fn render(zpl: &str) -> image::RgbaImage {
    let png = render_helpers::render_zpl_to_png(zpl, options());
    image::load_from_memory(&png)
        .expect("decode png")
        .to_rgba8()
}

fn options() -> DrawerOptions {
    render_helpers::default_options()
}

/// Count dark pixels (ink) in a rectangle.
fn ink(img: &image::RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if img.get_pixel(x, y)[0] < 128 {
                n += 1;
            }
        }
    }
    n
}

/// Leftmost ink column in the text-line band, like the Labelary probe runs.
fn first_ink_column(img: &image::RgbaImage) -> i32 {
    for x in 30..500 {
        for y in 40..100 {
            if img.get_pixel(x, y)[0] < 128 {
                return x as i32;
            }
        }
    }
    panic!("no ink found in text-line band");
}

#[test]
fn font0_missing_glyph_renders_blank_with_calibrated_advance() {
    let base = render(BASELINE_ZPL);
    let one = render(ONE_HANZI_ZPL);
    let x_base = first_ink_column(&base);
    let x_one = first_ink_column(&one);

    // The blank hanzi still advances the pen by ~0.2976 em = ~12.5 px at a
    // 42 dot cell (Labelary probes: 12/13 px at n/2n).
    let shift = x_one - x_base;
    assert!(
        (10..=15).contains(&shift),
        "missing-glyph advance shift was {shift}px, expected ~12-13px like Labelary"
    );

    // Nothing is drawn for the hanzi itself: the region between the field
    // origin and the shifted digits carries no ink.
    let hanzi_zone = ink(&one, 48, 40, x_one as u32 - 2, 100);
    assert_eq!(
        hanzi_zone, 0,
        "missing glyph rendered {hanzi_zone} ink pixels — must be blank like Labelary"
    );
}

#[test]
fn issue51_reproduction_matches_labelary_layout() {
    let img = render(ISSUE_ZPL);

    // Latin "ABC" prefix still renders (font 0 path intact).
    let latin = ink(&img, 48, 44, 116, 95);
    assert!(latin > 300, "latin ABC region has only {latin} ink pixels");

    // The CJK run is blank — no .notdef boxes, no fallback glyphs.
    let hanzi_zone = ink(&img, 118, 44, 166, 95);
    assert_eq!(
        hanzi_zone, 0,
        "CJK zone rendered {hanzi_zone} ink pixels — must be blank like Labelary"
    );

    // The trailing "123" lands at Labelary's position (ink starts at x≈169
    // in the reference) because the missing glyphs still advance the pen.
    let digits = ink(&img, 166, 44, 235, 95);
    assert!(
        digits > 300,
        "trailing digits region has only {digits} ink pixels"
    );
    let first_col = (166..235)
        .find(|&x| (44..95).any(|y| img.get_pixel(x, y)[0] < 128))
        .expect("no digits found after the CJK zone");
    assert!(
        (167..=172).contains(&first_col),
        "trailing digits start at x={first_col}, Labelary puts them at ~169"
    );

    // The Code 128 field with interpretation line still renders.
    let barcode = ink(&img, 48, 140, 700, 300);
    assert!(
        barcode > 1000,
        "barcode region has only {barcode} ink pixels"
    );
}

#[test]
fn pdf_pipeline_succeeds_for_cjk_label() {
    // The PDF encoder embeds the rasterised label, so the pixel assertions
    // above cover its content; here we verify the pipeline runs and produces
    // a well-formed document for the CJK label.
    let png = render_helpers::render_zpl_to_png(ISSUE_ZPL, options());
    let img = image::load_from_memory(&png)
        .expect("decode png")
        .to_rgba8();
    let opts = options();
    let mut buf = Vec::new();
    labelize::encode_pdf(&img, &opts, &mut buf).expect("encode_pdf failed");
    assert_eq!(&buf[..5], b"%PDF-", "PDF should start with %PDF- header");
}
