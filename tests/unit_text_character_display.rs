//! Comprehensive character-display sweep: which characters can each ZPL font
//! actually render, and does rotating the field (^A…R/I/B) preserve them?
//!
//! This generalises the PR #64 regression tests (`unit_rotated_diacritics.rs`,
//! which only covers "Ä" in font 0) to every character class the embedded
//! substitute fonts cover, across all four field orientations and every font
//! calibration class:
//!
//! | letters    | substitute face            | calibration class            |
//! |------------|----------------------------|------------------------------|
//! | `0`        | Roboto Condensed Bold subset| scalable, FONT0_* tuning     |
//! | `1`        | DejaVu Sans Mono           | FONT1_* tuning               |
//! | `A`, `C`   | DejaVu Sans Mono           | bitmap 7/6 cap correction    |
//! | `B`        | DejaVu Sans Mono Bold      | bitmap 1.59 cap correction   |
//! | `D`        | DejaVu Sans Mono Bold      | bitmap 7/6 cap correction    |
//! | `K`        | DejaVu Sans Mono           | plain (no bitmap correction) |
//! | `P`–`V`    | DejaVu Sans Mono Bold      | resident-matrix 0.85 model   |
//!
//! Three invariants are asserted per character:
//!
//! 1. **Displayability** — a character the font has a glyph for must produce
//!    ink at ^A…N (whitespace excepted). A glyph that silently renders blank
//!    is "cannot display".
//! 2. **Rotation preserves ink** — rotated fields are rasterised upright into
//!    a buffer and rotated by exact pixel transposes, so R/I/B must carry
//!    exactly the ink of N. Every character class swept here is held to this
//!    invariant with pixel equality (this was not true before the fix — see
//!    the history note below).
//! 3. **Missing-glyph policy** — characters outside a substitute's coverage
//!    render as blank with the pen still advancing (Labelary parity, see
//!    issues #51/#56) for every font, not just font 0.
//!
//! History (2026-10-05): before the renderer fix, 178 (font, character)
//! combinations lost ink in rotated fields — catastrophically for the font-0
//! characters whose advance deltas undercut their glyph ink (¤, and the
//! since-blanked Ā 267→12 px, Ŝ, ƀ, ℀), by 5–12% for the DejaVu Greek tonos
//! capitals and Vietnamese
//! horn glyphs (marks overhang the pen origin), and by 1–3 px on the DejaVu
//! mono trailing edge. The rotated buffer is now sized from the laid-out ink
//! extents (trailing growth + left margin + descent pad) with per-orientation
//! anchor rebasing, so the sweep pins every character to exact preservation.
//!
//! Note: font `B` uppercases its field data (matching Zebra bitmap font B,
//! which has no lowercase glyphs), so lowercase sweeps there exercise the
//! uppercased glyph — still a valid display/rotation check.

mod common;

use std::collections::HashMap;
use std::sync::OnceLock;

use ab_glyph::{Font as _, FontArc, GlyphId, PxScale};
use labelize::assets;

use common::render_helpers;

// ---------------------------------------------------------------------------
// Font model — mirrors `get_ttf_font_data` in src/drawers/renderer.rs.
// ---------------------------------------------------------------------------

fn font_data(letter: &str) -> &'static [u8] {
    match letter {
        "0" => assets::FONT_ZERO_SUBSTITUTE,
        "B" | "D" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" => assets::FONT_DEJAVU_SANS_MONO_BOLD,
        _ => assets::FONT_DEJAVU_SANS_MONO,
    }
}

fn font(letter: &str) -> &'static FontArc {
    static FONTS: OnceLock<HashMap<&'static str, FontArc>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            [
                "0", "1", "A", "B", "C", "D", "K", "P", "Q", "R", "S", "T", "U", "V",
            ]
            .iter()
            .map(|&l| {
                (
                    l,
                    FontArc::try_from_slice(font_data(l)).expect("embedded font parses"),
                )
            })
            .collect()
        })
        .get(letter)
        .expect("font letter in model table")
}

fn has_glyph(letter: &str, c: char) -> bool {
    let f = font(letter);
    let gid = f.glyph_id(c);
    if gid == GlyphId(0) {
        return false;
    }
    // Some cmap entries map to outline-less glyphs (e.g. U+019B ƛ in DejaVu
    // Mono Bold): the font itself draws nothing for them, in any renderer —
    // displayability must key on a real outline, not cmap presence.
    let probe = PxScale { x: 36.0, y: 36.0 };
    f.outline_glyph(gid.with_scale_and_position(probe, ab_glyph::point(0.0, 0.0)))
        .is_some()
}

// ---------------------------------------------------------------------------
// Rendering helpers.
// ---------------------------------------------------------------------------

/// Characters that legitimately render no ink: the quads (U+2000/U+2001 are
/// in several cmaps) and — defensively — the classic whitespace slots.
const WHITESPACE: &[char] = &[' ', '\u{00A0}', '\u{00AD}', '\u{2000}', '\u{2001}'];

fn small_options() -> labelize::DrawerOptions {
    // 240×160 px — large enough that a 36-dot field never touches an edge in
    // any orientation from FO (120,70), small enough to keep the sweep fast.
    labelize::DrawerOptions {
        label_width_mm: 30.0,
        label_height_mm: 20.0,
        dpmm: 8,
        ..Default::default()
    }
}

fn render_field(letter: &str, orientation: char, ch: char, h: u32, w: u32) -> usize {
    ink_count(&render_field_img(letter, orientation, ch, h, w))
}

fn render_field_img(letter: &str, orientation: char, ch: char, h: u32, w: u32) -> image::RgbaImage {
    let zpl = format!("^XA\n^CI28\n^FO120,70^A{letter}{orientation},{h},{w}^FD{ch}^FS\n^XZ\n");
    let png = render_helpers::render_zpl_to_png(&zpl, small_options());
    image::load_from_memory(&png)
        .expect("decode png")
        .to_rgba8()
}

fn ink_count(img: &image::RgbaImage) -> usize {
    img.pixels().filter(|p| p[0] < 128).count()
}

/// Ink bounding box (x0, y0, x1, y1) of the dark pixels, or None when blank.
fn ink_bbox(img: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for (x, y, p) in img.enumerate_pixels() {
        if p[0] < 128 {
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bbox
}

fn crop_ink(img: &image::RgbaImage) -> Option<image::RgbaImage> {
    let (x0, y0, x1, y1) = ink_bbox(img)?;
    Some(image::imageops::crop_imm(img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image())
}

/// Rotating a rotated render back must reproduce the unrotated ink mask:
/// same cropped-bbox dimensions, same pixels. This is strictly stronger than
/// comparing ink counts — a partially clipped or distorted glyph keeps a
/// count-equal but shape-different mask. (A pure translation of the whole
/// field is not caught here; absolute rotated positions stay pinned by the
/// golden corpus and the explicit anchor tests below.)
fn assert_rotation_mask_matches(normal: &image::RgbaImage, rotated: &image::RgbaImage, o: char) {
    let unrotated = match o {
        'R' => image::imageops::rotate270(rotated), // undo 90° cw
        'I' => image::imageops::rotate180(rotated),
        'B' => image::imageops::rotate90(rotated), // undo 270°
        _ => rotated.clone(),
    };
    let a = crop_ink(normal);
    let b = crop_ink(&unrotated);
    match (a, b) {
        (Some(a), Some(b)) => {
            assert_eq!(
                a.dimensions(),
                b.dimensions(),
                "^A{o}: inverse-rotated glyph mask is {a:?} vs normal {b:?}"
            );
            for (pa, pb) in a.pixels().zip(b.pixels()) {
                assert_eq!(
                    pa[0] < 128,
                    pb[0] < 128,
                    "^A{o}: inverse-rotated glyph mask differs from the unrotated one"
                );
            }
        }
        (None, None) => {}
        (a, b) => panic!("^A{o}: ink present in one render only ({a:?} vs {b:?})"),
    }
}

// ---------------------------------------------------------------------------
// Character universes.
// ---------------------------------------------------------------------------

fn push_range(v: &mut Vec<char>, lo: u32, hi: u32) {
    for cp in lo..=hi {
        let c = char::from_u32(cp).unwrap();
        if !v.contains(&c) {
            v.push(c);
        }
    }
}

/// Every character class the substitute fonts plausibly meet on a label:
/// ASCII, Latin-1, Latin Extended-A/B/Additional, spacing modifiers, Greek,
/// Cyrillic, punctuation and currency. Used for the calibration classes with
/// the richest profiles (`0`, `1`, `B`, `D`).
fn full_universe() -> Vec<char> {
    let mut v = Vec::new();
    // ASCII printable minus ^ and ~ (ZPL command prefixes in field data).
    for cp in 0x21..=0x7E {
        let c = char::from_u32(cp).unwrap();
        if c != '^' && c != '~' {
            v.push(c);
        }
    }
    push_range(&mut v, 0xA1, 0xFF); // Latin-1 supplement
    push_range(&mut v, 0x100, 0x17F); // Latin Extended-A
    push_range(&mut v, 0x180, 0x24F); // Latin Extended-B
    push_range(&mut v, 0x1E00, 0x1EFF); // Latin Extended Additional (Vietnamese…)
    push_range(&mut v, 0x2C6, 0x2DD); // spacing modifier letters (in font 0's subset)
    push_range(&mut v, 0x386, 0x3CE); // Greek
    push_range(&mut v, 0x410, 0x44F); // Cyrillic
    push_range(&mut v, 0x2000, 0x2001); // en/em quad (big negative font-0 delta)
    push_range(&mut v, 0x2010, 0x2027); // punctuation
    push_range(&mut v, 0x2039, 0x203A);
    push_range(&mut v, 0x2044, 0x2045); // fraction slash
    push_range(&mut v, 0x20AC, 0x20AD); // €
    push_range(&mut v, 0x2100, 0x2155); // ℀ ℅ ℃ … ⅓ ⅔
    push_range(&mut v, 0x2122, 0x2123); // ™
    push_range(&mut v, 0x2212, 0x2213); // −
    push_range(&mut v, 0xFB01, 0xFB02); // ﬁ ﬂ
    for c in ['中', '日', '文'] {
        v.push(c);
    }
    v
}

/// The scripts where real-world labels actually live, plus the specific
/// Extended-B characters known to trip the rotated buffer. Used for the
/// letters that share a calibration class with a fully-swept sibling
/// (`A`/`C`≡`K` on the mono face, `Q`–`U` alongside `P` and `V`).
fn reduced_universe() -> Vec<char> {
    let mut v = Vec::new();
    for cp in 0x21..=0x7E {
        let c = char::from_u32(cp).unwrap();
        if c != '^' && c != '~' {
            v.push(c);
        }
    }
    push_range(&mut v, 0xA1, 0xFF);
    push_range(&mut v, 0x100, 0x17F);
    push_range(&mut v, 0x386, 0x3CE);
    push_range(&mut v, 0x410, 0x44F);
    // Extended-B characters that appear in the KNOWN lists of reduced letters.
    for c in ['\u{0189}', '\u{018A}'] {
        v.push(c);
    }
    v
}

// ---------------------------------------------------------------------------
// Known-clipping debt — the machine-readable answer to "which characters
// cannot be displayed (rotated)". Entries were measured at ^A,36,32 on main;
// each comment shows normal-ink → rotated-ink. When a rendering fix lands,
// remove fixed entries: the ratchet assertion below fails otherwise.
// ---------------------------------------------------------------------------

const KNOWN_ROTATION_CLIPPED: &[(&str, char)] = &[
    // Empty since the renderer fix: rotated buffers are sized from laid-out
    // ink extents. Entries here pin characters whose rotated ink STILL differs
    // from normal (a debt ledger — remove entries as they are fixed).
];
/// ^FB block fields (3 lines of diacritic + descender text) must preserve
/// their ink in R/I/B exactly (empty since the renderer fix; entries here
/// would pin fonts whose blocks still lose ink).
const KNOWN_ROTATION_BLOCK_LOSS: &[&str] = &[];

fn is_known_clipped(letter: &str, ch: char) -> bool {
    KNOWN_ROTATION_CLIPPED
        .iter()
        .any(|&(l, c)| l == letter && c == ch)
}

// ---------------------------------------------------------------------------
// The sweep itself.
// ---------------------------------------------------------------------------

fn sweep_letter(letter: &str, universe: &[char], min_covered: usize) {
    let mut covered_count = 0usize;
    let mut blank_checked = 0usize;
    for &ch in universe {
        if ch == '^' || ch == '~' {
            continue;
        }
        // Font B uppercases its field data (the renderer does the same), so
        // displayability keys on the displayed characters: e.g. ƛ folds to
        // Ƛ (U+A7DC), which the DejaVu faces don't cover — blank is correct.
        let displayed: String = if letter == "B" {
            ch.to_uppercase().collect()
        } else {
            ch.to_string()
        };
        let covered = displayed.chars().any(|c| {
            has_glyph(letter, c)
                && !(letter == "0" && labelize::FONT0_LABELARY_BLANKED.contains(&c))
        });
        let normal_img = render_field_img(letter, 'N', ch, 36, 32);
        let normal = ink_count(&normal_img);

        if covered {
            covered_count += 1;
            if !WHITESPACE.contains(&ch) {
                assert!(
                    normal > 0,
                    "^A{letter}N {ch:?} (U+{:04X}) has a glyph in the substitute font \
                     but renders no ink — the character cannot be displayed",
                    ch as u32
                );
            }
            for &o in &['R', 'I', 'B'] {
                let rotated_img = render_field_img(letter, o, ch, 36, 32);
                let rotated = ink_count(&rotated_img);
                if is_known_clipped(letter, ch) {
                    // Ratchet: this character is pinned as broken; if the fix
                    // lands, delete its KNOWN_ROTATION_CLIPPED entry.
                    assert_ne!(
                        rotated, normal,
                        "^A{letter}{o} {ch:?} (U+{:04X}) now preserves its ink — \
                         remove it from KNOWN_ROTATION_CLIPPED",
                        ch as u32
                    );
                } else {
                    assert_eq!(
                        rotated, normal,
                        "^A{letter}{o} {ch:?} (U+{:04X}) lost ink vs ^A{letter}N \
                         (N={normal}, {o}={rotated}) — new rotated-field clipping",
                        ch as u32
                    );
                }
                assert_rotation_mask_matches(&normal_img, &rotated_img, o);
            }
        } else if letter == "0" {
            blank_checked += 1;
            // Labelary parity: font 0 renders uncovered glyphs as blank (issue
            // #51/#56), in every orientation.
            assert_eq!(
                normal, 0,
                "^A0N {ch:?} (U+{:04X}) is outside the font-0 substitute but produced ink",
                ch as u32
            );
            for &o in &['R', 'I', 'B'] {
                assert_eq!(
                    render_field(letter, o, ch, 36, 32),
                    0,
                    "^A0{o} {ch:?} (U+{:04X}) is outside the font-0 substitute but produced ink",
                    ch as u32
                );
            }
        }
    }
    // Guard the guard: if the font model or asset ever changes so the sweep
    // stops exercising real characters, fail loudly instead of vacuously
    // passing.
    assert!(
        covered_count >= min_covered,
        "^A{letter} sweep only covered {covered_count} glyphs (expected ≥{min_covered}) — \
         the font model or universe has rotted"
    );
    if letter == "0" {
        assert!(
            blank_checked >= 150,
            "^A0 blank-parity branch only saw {blank_checked} chars"
        );
    }
}

/// Split a universe in halves so each half is its own #[test] — keeps the
/// per-test wall time CI-friendly while the halves still run in parallel.
fn halves(universe: &[char]) -> (&[char], &[char]) {
    let mid = universe.len() / 2;
    (&universe[..mid], &universe[mid..])
}

// One pair of tests per font letter (per font class where letters share the
// substitute face) so cargo runs them in parallel.

#[test]
fn font0_character_display_sweep_first_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = b;
    sweep_letter("0", a, 100);
}

#[test]
fn font0_character_display_sweep_second_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = a;
    // Font 0's subset lives in ASCII/Latin-1/Ext-A (first half); the second
    // half only retains the modifiers/punct/currency slice (~33 glyphs).
    sweep_letter("0", b, 20);
}

#[test]
fn font1_character_display_sweep_first_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = b;
    sweep_letter("1", a, 250);
}

#[test]
fn font1_character_display_sweep_second_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = a;
    sweep_letter("1", b, 250);
}

#[test]
fn fontB_character_display_sweep_first_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = b;
    sweep_letter("B", a, 250);
}

#[test]
fn fontB_character_display_sweep_second_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = a;
    sweep_letter("B", b, 250);
}

#[test]
fn fontD_character_display_sweep_first_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = b;
    sweep_letter("D", a, 250);
}

#[test]
fn fontD_character_display_sweep_second_half() {
    let u = full_universe();
    let (a, b) = halves(&u);
    let _ = a;
    sweep_letter("D", b, 250);
}

#[test]
fn fontA_character_display_sweep() {
    sweep_letter("A", &reduced_universe(), 150);
}

#[test]
fn fontC_character_display_sweep() {
    sweep_letter("C", &reduced_universe(), 150);
}

#[test]
fn fontK_character_display_sweep() {
    sweep_letter("K", &reduced_universe(), 150);
}

#[test]
fn fontP_character_display_sweep() {
    sweep_letter("P", &reduced_universe(), 150);
}

#[test]
fn fontQ_character_display_sweep() {
    sweep_letter("Q", &reduced_universe(), 150);
}

#[test]
fn fontR_character_display_sweep() {
    sweep_letter("R", &reduced_universe(), 150);
}

#[test]
fn fontS_character_display_sweep() {
    sweep_letter("S", &reduced_universe(), 150);
}

#[test]
fn fontT_character_display_sweep() {
    sweep_letter("T", &reduced_universe(), 150);
}

#[test]
fn fontU_character_display_sweep() {
    sweep_letter("U", &reduced_universe(), 150);
}

#[test]
fn fontV_character_display_sweep() {
    sweep_letter("V", &reduced_universe(), 150);
}

/// The font-0 family that used to lose ~95% of its ink (advance deltas
/// undercutting glyph ink) must stay whole at other cell sizes too. ¤ is the
/// remaining member whose substitute glyph outgrows its corrected advance.
/// Ā Ŝ ƀ ℀ ⁄ were in the same clipping family until the subset was aligned
/// with Labelary's font-0 coverage: Labelary renders all of them blank with
/// a uniform advance (probe-measured at ^CI28, ^A0N,36,32: 9 px, identical
/// to the CJK/missing-glyph advance), so they must render NO ink in any
/// orientation — a glyph re-added to the subset would regress this.
#[test]
fn font0_formerly_clipped_chars_preserve_at_small_size() {
    for ch in ['\u{00A4}'] {
        let normal = render_field("0", 'N', ch, 18, 16);
        assert!(
            normal > 10,
            "^A0N,18,16 {ch:?} rendered only {normal} ink px"
        );
        for &o in &['R', 'I', 'B'] {
            assert_eq!(
                render_field("0", o, ch, 18, 16),
                normal,
                "^A{o},18,16 {ch:?} lost ink vs N"
            );
        }
    }
    for ch in ['\u{0100}', '\u{015C}', '\u{0180}', '\u{2100}', '\u{2044}'] {
        let normal = render_field("0", 'N', ch, 18, 16);
        assert_eq!(
            normal, 0,
            "^A0N,18,16 {ch:?} rendered {normal} ink px — Labelary blanks it \
             (blank + uniform advance); it must stay in FONT0_LABELARY_BLANKED"
        );
        for &o in &['R', 'I', 'B'] {
            assert_eq!(
                render_field("0", o, ch, 18, 16),
                0,
                "^A{o},18,16 {ch:?} rendered ink vs N's blank"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ^FB block fields.
// ---------------------------------------------------------------------------

fn render_block(letter: &str, orientation: char) -> usize {
    let zpl = format!(
        "^XA\n^CI28\n^FO240,200^A{letter}{orientation},36,32^FB180,3,0,L^FDÄÖÜ ÉĞŤ ĥjpqy ÅÑÖ^FS\n^XZ\n"
    );
    let png = render_helpers::render_zpl_to_png(
        &zpl,
        labelize::DrawerOptions {
            label_width_mm: 60.0,
            label_height_mm: 60.0,
            dpmm: 8,
            ..Default::default()
        },
    );
    let img = image::load_from_memory(&png)
        .expect("decode png")
        .to_rgba8();
    img.pixels().filter(|p| p[0] < 128).count()
}

#[test]
fn fb_block_rotation_preserves_ink() {
    for letter in [
        "0", "1", "A", "C", "B", "D", "K", "P", "Q", "R", "S", "T", "U", "V",
    ] {
        let normal = render_block(letter, 'N');
        assert!(
            normal > 500,
            "^A{letter} block rendered only {normal} ink px"
        );
        for &o in &['R', 'I', 'B'] {
            let rotated = render_block(letter, o);
            if KNOWN_ROTATION_BLOCK_LOSS.contains(&letter) {
                assert_ne!(
                    rotated, normal,
                    "^A{letter}{o} ^FB block now preserves ink — remove {letter} \
                     from KNOWN_ROTATION_BLOCK_LOSS"
                );
            } else {
                assert_eq!(
                    rotated, normal,
                    "^A{letter}{o} ^FB block lost ink vs N (N={normal}, {o}={rotated})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Missing-glyph policy.
// ---------------------------------------------------------------------------

/// Font 0 renders characters outside its subset as blank — no .notdef box —
/// across scripts (extends unit_cjk_font0 beyond CJK).
#[test]
fn font0_renders_uncovered_chars_blank_across_scripts() {
    for ch in [
        'Ω', 'Ж', '中', '\u{01C4}', '\u{1E00}', '\u{2113}', '\u{0101}',
    ] {
        assert!(
            !has_glyph("0", ch),
            "fixture rot: {ch:?} is now covered by the font-0 subset — pick another"
        );
        for &o in &['N', 'R', 'I', 'B'] {
            assert_eq!(
                render_field("0", o, ch, 36, 32),
                0,
                "^A0{o} must render uncovered {ch:?} as blank (Labelary parity)"
            );
        }
    }
}

/// Every substitute renders uncovered characters as blank — no .notdef box —
/// while the pen still advances (Labelary parity, probed against font A and
/// font 1 with CJK field data; the DejaVu faces used to draw tofu boxes).
#[test]
fn dejavu_fonts_render_uncovered_chars_blank() {
    // U+1EC0 Ề is in the uncovered part of DejaVu's Latin Ext Additional block.
    for letter in ["1", "A", "B", "D", "P", "V"] {
        for ch in ['中', '\u{01C4}', '\u{1EC0}'] {
            assert!(
                !has_glyph(letter, ch),
                "fixture rot: {ch:?} is now covered by font {letter}"
            );
            for &o in &['N', 'R', 'I', 'B'] {
                assert_eq!(
                    render_field(letter, o, ch, 36, 32),
                    0,
                    "^A{letter}{o} must render uncovered {ch:?} as blank (Labelary parity)"
                );
            }
        }
    }
}

/// Sanity anchor for the whole model: the classic PR #64 case must stay green
/// in every letter class (Ä is covered everywhere and carries no overhangs).
#[test]
fn umlaut_survives_rotation_in_all_font_classes() {
    for letter in [
        "0", "1", "A", "C", "B", "D", "K", "P", "Q", "R", "S", "T", "U", "V",
    ] {
        let normal = render_field(letter, 'N', 'Ä', 36, 32);
        assert!(normal > 50, "^A{letter}N Ä rendered only {normal} ink px");
        for &o in &['R', 'I', 'B'] {
            assert_eq!(
                render_field(letter, o, 'Ä', 36, 32),
                normal,
                "^A{letter}{o} Ä lost ink vs N"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Anchor-sensitive regressions (review follow-ups): fields whose ink reaches
// left of the block origin or below the last line's em box.
// ---------------------------------------------------------------------------

/// Right-aligned ^FB lines wider than max_width get a NEGATIVE alignment
/// offset (word_wrap keeps unbreakable words whole) — the rotated buffer must
/// still contain them. Regression case: ^FB20,1,0,R "WWWW" used to render
/// 1725 px normally and 222 px rotated.
#[test]
fn fb_right_aligned_unbreakable_word_survives_rotation() {
    for letter in [
        "0", "1", "A", "C", "B", "D", "K", "P", "Q", "R", "S", "T", "U", "V",
    ] {
        let zpl = |o: char| {
            format!("^XA\n^CI28\n^FO240,200^A{letter}{o},36,32^FB20,1,0,R^FDWWWW^FS\n^XZ\n")
        };
        let opts = labelize::DrawerOptions {
            label_width_mm: 60.0,
            label_height_mm: 60.0,
            dpmm: 8,
            ..Default::default()
        };
        let render = |o: char| {
            let png = render_helpers::render_zpl_to_png(&zpl(o), opts.clone());
            image::load_from_memory(&png)
                .expect("decode png")
                .to_rgba8()
        };
        let normal = render('N');
        let n_ink = ink_count(&normal);
        // Small matrices (Q renders ~350 px) — the threshold only guards
        // against blankness, exactness is checked below.
        assert!(n_ink > 200, "^A{letter} WWWW rendered only {n_ink} ink px");
        for &o in &['R', 'I', 'B'] {
            let rotated = render(o);
            assert_eq!(
                ink_count(&rotated),
                n_ink,
                "^A{letter}{o} right-aligned unbreakable block lost ink vs N"
            );
            assert_rotation_mask_matches(&normal, &rotated, o);
        }
    }
}

/// Deep-descending glyphs on the LAST line of a block hang below that line's
/// em box by (num_lines−1)·pitch — the descent pad must account for the line
/// tops. Regression case: font V ^FB40,3,0,L "ˎ ˎ ˎ" used to lose 4 px.
#[test]
fn fb_deep_descent_on_last_line_survives_rotation() {
    // ˎ (U+02CE) lives in the DejaVu faces only — font 0 renders it blank.
    for letter in ["V", "D", "B", "1"] {
        let zpl = |o: char| {
            format!("^XA\n^CI28\n^FO240,200^A{letter}{o},36,32^FB40,3,0,L^FDˎ ˎ ˎ^FS\n^XZ\n")
        };
        let opts = labelize::DrawerOptions {
            label_width_mm: 60.0,
            label_height_mm: 60.0,
            dpmm: 8,
            ..Default::default()
        };
        let render = |o: char| {
            let png = render_helpers::render_zpl_to_png(&zpl(o), opts.clone());
            image::load_from_memory(&png)
                .expect("decode png")
                .to_rgba8()
        };
        let normal = render('N');
        let n_ink = ink_count(&normal);
        assert!(n_ink > 100, "^A{letter} ˎˎˎ rendered only {n_ink} ink px");
        for &o in &['R', 'I', 'B'] {
            let rotated = render(o);
            assert_eq!(
                ink_count(&rotated),
                n_ink,
                "^A{letter}{o} deep-descent block lost ink vs N"
            );
            assert_rotation_mask_matches(&normal, &rotated, o);
        }
    }
}

/// An uncovered character in a DejaVu font must advance the pen by one mono
/// cell (Labelary parity): inserting 中 shifts the trailing text by the same
/// amount as inserting one more covered character (DejaVu's .notdef shares
/// the mono cell advance). Mirrors unit_cjk_font0's font-0 probe for the
/// newly blanked DejaVu path.
#[test]
fn dejavu_missing_glyph_advances_one_mono_cell() {
    // The trailing text's position shows up on the RIGHT edge of the ink.
    let text_bbox_x1 = |text: &str| -> u32 {
        let zpl = format!("^XA\n^CI28\n^FO120,70^A1N,36,32^FD{text}^FS\n^XZ\n");
        let png = render_helpers::render_zpl_to_png(&zpl, small_options());
        let img = image::load_from_memory(&png)
            .expect("decode png")
            .to_rgba8();
        ink_bbox(&img)
            .unwrap_or_else(|| panic!("{text:?} rendered no ink"))
            .2
    };
    let base = text_bbox_x1("AB");
    let with_covered = text_bbox_x1("AAB"); // one more mono cell
    let with_missing = text_bbox_x1("A中B"); // one blank cell
    let cell = with_covered as i64 - base as i64;
    assert!(
        cell > 5,
        "mono cell advance measured as {cell} px — fixture rot"
    );
    let shift = with_missing as i64 - base as i64;
    assert!(
        (shift - cell).abs() <= 1,
        "uncovered 中 advanced {shift} px but a mono cell is {cell} px"
    );
}

// ---------------------------------------------------------------------------
// ^FB negative line spacing and padded-pen snapping (review round 2).
// ---------------------------------------------------------------------------

/// ^FB accepts a negative line-spacing parameter and the parser preserves it:
/// later lines then sit ABOVE the pen top, and the deepest line need not be
/// the last. The rotated buffer must grow a top margin for the raised lines
/// and size its bottom margin from the deepest line — regression case:
/// `^A1R,36,32^FB20,2,-80,L^FDj j` used to lose one whole line (414 → 207 px).
#[test]
fn fb_negative_line_spacing_survives_rotation() {
    let zpl = |o: char| format!("^XA\n^CI28\n^FO240,200^A1{o},36,32^FB20,2,-80,L^FDj j^FS\n^XZ\n");
    let opts = labelize::DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    };
    let render = |o: char| {
        let png = render_helpers::render_zpl_to_png(&zpl(o), opts.clone());
        image::load_from_memory(&png)
            .expect("decode png")
            .to_rgba8()
    };
    let normal = render('N');
    let n_ink = ink_count(&normal);
    assert!(
        n_ink > 200,
        "negative-spacing block rendered only {n_ink} ink px"
    );
    for &o in &['R', 'I', 'B'] {
        let rotated = render(o);
        assert_eq!(
            ink_count(&rotated),
            n_ink,
            "^A1{o} negative-spacing block lost ink vs N"
        );
        assert_rotation_mask_matches(&normal, &rotated, o);
    }
}

/// Right-aligned ^FB lines wider than max_width get negative alignment
/// offsets. The rotated buffer's left margin must be added AFTER the per-line
/// pen snapping, or `as i32` truncation of a pen that crosses zero inside the
/// padded value shifts that line 1 px relative to the others. Regression
/// case: two unbreakable i words (24 and 22 chars, sized to stay on the
/// label with no label-edge clipping). The guarded invariant is the two lines' RELATIVE position along
/// the stacking axis: project the ink onto that axis, split the two line
/// bands, and require both renders to show the same band-start offset (the
/// ^FB pitch) and the same per-band ink extents — a per-line pen-snapping
/// bug shifts one band by 1+ px, a clipped line loses a band.
#[test]
fn fb_right_aligned_relative_line_positions_survive_rotation() {
    let sh: char = 'i';
    let w1: String = std::iter::repeat(sh).take(24).collect();
    let w2: String = std::iter::repeat(sh).take(22).collect();
    let zpl =
        |o: char| format!("^XA\n^CI28\n^FO240,200^A0{o},36,32^FB20,2,0,R^FD{w1} {w2}^FS\n^XZ\n");
    let opts = labelize::DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    };
    let render = |o: char| {
        let png = render_helpers::render_zpl_to_png(&zpl(o), opts.clone());
        image::load_from_memory(&png)
            .expect("decode png")
            .to_rgba8()
    };
    let normal = render('N');
    let rotated = render('R');

    // The two lines touch (pitch == glyph height), so split the ink at the
    // known pitch: each half holds one word length. The rotated stacking axis
    // reverses line order (line 1 is the rightmost column), so the rotated
    // halves pair with the normal halves in reverse — a relative line shift
    // or clipped line spills tens of pixels across the split.
    fn half_inks(
        img: &image::RgbaImage,
        along_x: bool,
        lo: u32,
        hi: u32,
        fixed: (u32, u32),
    ) -> (u32, u32) {
        let first = (lo..hi)
            .find(|&i| {
                (lo..hi).any(|j| {
                    let p = if along_x {
                        img.get_pixel(i, j)
                    } else {
                        img.get_pixel(j, i)
                    };
                    p[0] < 128
                })
            })
            .expect("block ink present");
        let pitch = 36u32;
        let mut a = 0u32;
        let mut b = 0u32;
        for i in first..hi.min(first + 2 * pitch) {
            let mut c = 0u32;
            for j in fixed.0..fixed.1 {
                let p = if along_x {
                    img.get_pixel(i, j)
                } else {
                    img.get_pixel(j, i)
                };
                if p[0] < 128 {
                    c += 1;
                }
            }
            if i < first + pitch {
                a += c;
            } else {
                b += c;
            }
        }
        (a, b)
    }

    /// Project the ink onto the stacking axis and split it into the two
    /// line bands. Returns (band1_start, band2_start, band1_extent,
    /// band2_extent) — band starts are absolute axis coordinates, so the
    /// (band2 − band1) offset compares across orientations directly.
    fn line_bands(img: &image::RgbaImage, along_x: bool) -> (u32, u32, u32, u32) {
        let (w, h) = img.dimensions();
        let hits: Vec<u32> = (0..if along_x { w } else { h })
            .filter(|&i| {
                (0..if along_x { h } else { w }).any(|j| {
                    let p = if along_x {
                        img.get_pixel(i, j)
                    } else {
                        img.get_pixel(j, i)
                    };
                    p[0] < 128
                })
            })
            .collect();
        assert!(
            hits.len() >= 2,
            "block bands: expected two ink bands, got {hits:?}"
        );
        // Split at the first gap larger than 5 px (clears the i dot/stem
        // gap ~3 px; the inter-line pitch gap is ~8-10 px).
        let mut split = 1usize;
        while split < hits.len() && hits[split] - hits[split - 1] <= 5 {
            split += 1;
        }
        assert!(split < hits.len(), "block bands: no inter-line gap found");
        (
            hits[0],
            hits[split],
            hits[split - 1] - hits[0] + 1,
            *hits.last().unwrap() - hits[split] + 1,
        )
    }

    let (na, nb, na_ext, nb_ext) = line_bands(&normal, false);
    let (ra, rb, ra_ext, rb_ext) = line_bands(&rotated, true);
    let n_offset = nb as i64 - na as i64;
    let r_offset = rb as i64 - ra as i64;
    assert_eq!(
        n_offset, r_offset,
        "rotated inter-line offset {r_offset} != normal {n_offset} — per-line pen snapping"
    );
    // Both lines present and unclipped in both orientations (the words are
    // sized to stay inside the label, so no label-edge clipping pollutes the
    // extents).
    assert!(
        na_ext > 25 && nb_ext > 25,
        "normal band extents {na_ext}/{nb_ext}"
    );
    assert!(
        ra_ext > 25 && rb_ext > 25,
        "rotated band extents {ra_ext}/{rb_ext}"
    );
    for (label, rotated_ext, normal_ext) in [("line1", na_ext, ra_ext), ("line2", nb_ext, rb_ext)] {
        let diff = rotated_ext.abs_diff(normal_ext);
        assert!(
            diff * 20 <= normal_ext,
            "{label}: band ink extent {rotated_ext} vs normal {normal_ext} (off by {diff}) — clipped line ink"
        );
    }

    // And the full inverse-rotated mask matches within edge-noise tolerance
    // (sub-pixel glyph phases differ between orientations; a real defect —
    // clipped ink or a shifted line — costs hundreds of pixels).
    let unrotated = image::imageops::rotate270(&rotated);
    let a = crop_ink(&normal).expect("normal ink");
    let b = crop_ink(&unrotated).expect("rotated ink");
    assert_eq!(a.dimensions(), b.dimensions(), "inverse-rotated mask size");
    let diff = a
        .pixels()
        .zip(b.pixels())
        .filter(|(pa, pb)| (pa[0] < 128) != (pb[0] < 128))
        .count();
    let ink = ink_count(&normal);
    assert!(
        diff * 200 <= ink,
        "inverse-rotated right-aligned block mask differs in {diff} px \
         ({ink} ink) — clipped ink or a shifted line"
    );
}
