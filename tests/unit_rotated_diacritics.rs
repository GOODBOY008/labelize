//! Regression tests: diacritics that rise above the font ascent (the dots on
//! Ä/Ö/Ü) must survive in rotated fields (^A0R, ^A0I, ^A0B) just as they do in
//! ^A0N. Rotated text is rasterised into an off-screen buffer first, and that
//! buffer must leave headroom above the ascent or the marks are clipped.

mod common;

use common::render_helpers;

fn render(zpl: &str) -> image::RgbaImage {
    let png = render_helpers::render_zpl_to_png(zpl, render_helpers::default_options());
    image::load_from_memory(&png)
        .expect("decode png")
        .to_rgba8()
}

fn ink(img: &image::RgbaImage) -> usize {
    img.pixels().filter(|p| p[0] < 128).count()
}

fn field(orientation: char, text: &str) -> String {
    format!("^XA\n^CI28\n^FO200,200^A0{orientation},36,32^FD{text}^FS\n^XZ\n")
}

/// Rotated fields are rasterised upright and then rotated by whole pixels, so
/// a rotated "Ä" must carry exactly the ink of the unrotated one — dots included.
fn assert_diacritic_kept(orientation: char) {
    let reference = ink(&render(&field('N', "Ä")));
    let rotated = ink(&render(&field(orientation, "Ä")));
    assert_eq!(
        rotated, reference,
        "^A0{orientation}: rotated Ä has {rotated} ink pixels, unrotated has {reference} — the diacritic was clipped"
    );
}

#[test]
fn reference_field_renders_diacritics() {
    let plain = ink(&render(&field('N', "A")));
    let umlaut = ink(&render(&field('N', "Ä")));
    assert!(plain > 50, "A rendered only {plain} ink pixels");
    assert!(
        umlaut >= plain + 8,
        "Ä adds only {} ink pixels over A",
        umlaut as i64 - plain as i64
    );
}

#[test]
fn rotated_90_field_keeps_diacritics() {
    assert_diacritic_kept('R');
}

#[test]
fn rotated_180_field_keeps_diacritics() {
    assert_diacritic_kept('I');
}

#[test]
fn rotated_270_field_keeps_diacritics() {
    assert_diacritic_kept('B');
}
