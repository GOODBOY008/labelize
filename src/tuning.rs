//! Calibration constants for matching Zebra/Labelary text output.
//!
//! Zebra's resident font 0 is CG Triumvirate Bold Condensed, which cannot be
//! bundled, so the renderer substitutes Helvetica Bold Condensed. The substitute
//! agrees on neither glyph width nor per-character advance, and the two errors are
//! independent: [`FONT0_RATIO`] sets glyph *shape* width, [`FONT0_ADVANCE_DELTAS`]
//! corrects *spacing*. Both were fitted against Labelary renders.

/// Width-to-height ratio for the scalable font 0.
///
/// This is the glyph shape width only; per-character spacing lives in
/// [`FONT0_ADVANCE_DELTAS`]. 0.95 is the joint optimum of the two. Without the
/// advance table the best single ratio is lower, because it then has to absorb the
/// spacing error too by squeezing every glyph.
pub(crate) const FONT0_RATIO: f64 = 0.95;

/// Constant vertical offset applied to the font 0 text pen, in pixels.
///
/// Glyph bounds are rounded to integers when rasterised, so a sub-pixel
/// disagreement with Zebra's own dot-grid snapping shows up as a one-pixel offset
/// on a large share of glyphs.
pub(crate) const TEXT_Y_OFFSET: f64 = -0.8;

/// Vertical font 0 text offset expressed as a fraction of the font cell height.
///
/// The substitute face's ascent metric differs from Zebra's cell metrics, and that
/// error scales with the font size, so this part of the correction is
/// size-proportional rather than a constant pixel shift. Added to the pen as
/// `em * scale.y`, on top of [`TEXT_Y_OFFSET`]. A pure constant of -1.3 px scores
/// the same on the calibration corpus; the split is preferred because the
/// size-proportional part is the physically motivated one and so generalises to
/// font sizes the corpus does not cover.
pub(crate) const TEXT_Y_OFFSET_EM: f64 = -0.015;
pub(crate) const DIGIT_GAMMA: f64 = 2.2;
pub(crate) const DIGIT_SCALE_X: f64 = 1.0;
pub(crate) const DIGIT_Y_OFFSET: f64 = 0.0;
pub(crate) const DIGIT_ADVANCE_ADJUST: f64 = 0.0;

/// Per-character advance correction for font 0, in em units (multiplied by the
/// font cell height at use). Characters absent from the table need no correction.
pub(crate) fn font0_advance_delta(ch: char) -> f64 {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    static TABLE: OnceLock<HashMap<char, f64>> = OnceLock::new();
    TABLE
        .get_or_init(|| FONT0_ADVANCE_DELTAS.iter().copied().collect())
        .get(&ch)
        .copied()
        .unwrap_or(0.0)
}

/// Calibrated per-character advance deltas for font 0, in em units.
///
/// The substitute face's advances differ per character — some drastically: `<`, `>`,
/// `+` and `=` are less than half the reference width, and `|`, `\\`, `{`, `}` and
/// `"` around half. A single global ratio cannot express that.
///
/// Measured from a probe suite that renders each character as runs of n and 2n
/// copies: subtracting the two run extents cancels the glyph's ink width and divides
/// the 1 px measurement error by n, giving ~0.003 em resolution — ten times finer
/// than whole-label comparison can resolve, and enough to separate a real advance
/// error from pixel quantisation.
const FONT0_ADVANCE_DELTAS: &[(char, f64)] = &[
    ('<', 0.48810),
    ('>', 0.48810),
    ('+', 0.39731),
    ('=', 0.39731),
    ('|', 0.24702),
    ('\\', 0.22619),
    ('{', 0.22254),
    ('}', 0.22240),
    ('"', 0.14198),
    ('-', 0.09374),
    ('µ', 0.08333),
    ('°', 0.07508),
    ('&', -0.06693),
    ('?', -0.06250),
    ('%', 0.06026),
    ('@', 0.06026),
    ('Q', -0.04598),
    ('O', -0.04596),
    ('a', -0.04464),
    ('à', -0.04464),
    ('ä', -0.04464),
    (' ', 0.04322),
    (',', -0.04314),
    ('.', -0.04238),
    ('!', -0.04226),
    ('(', -0.04226),
    (']', -0.04226),
    ('`', -0.04226),
    ('\'', 0.04167),
    (')', -0.04167),
    ('[', -0.04167),
    ('W', -0.03233),
    ('m', -0.03126),
    ('M', -0.03057),
    ('G', -0.02976),
    ('R', -0.02961),
    ('D', -0.02938),
    ('C', -0.02879),
    ('S', -0.02879),
    ('V', -0.02879),
    ('2', -0.02694),
    ('#', -0.02679),
    ('$', -0.02679),
    ('*', -0.02679),
    ('3', -0.02679),
    ('e', -0.02679),
    ('£', -0.02679),
    ('§', -0.02679),
    ('è', -0.02679),
    ('é', -0.02679),
    ('ö', -0.02679),
    ('€', -0.02679),
    ('6', -0.02665),
    ('1', -0.02646),
    ('0', -0.02631),
    ('5', -0.02631),
    ('7', -0.02631),
    ('L', -0.02631),
    ('o', -0.02631),
    ('s', -0.02622),
    ('4', -0.02618),
    ('8', -0.02618),
    ('9', -0.02618),
    ('j', -0.02386),
    ('i', -0.02247),
    ('l', -0.02247),
    (';', 0.01414),
    ('/', 0.01374),
    (':', 0.01374),
    ('w', -0.01105),
    ('H', -0.01025),
    ('N', -0.01025),
    ('U', -0.01025),
    ('K', -0.00955),
    ('ß', 0.00930),
    ('A', -0.00915),
    ('B', -0.00915),
    ('P', -0.00915),
    ('X', -0.00915),
    ('Y', -0.00915),
    ('t', -0.00633),
    ('y', -0.00625),
    ('r', -0.00597),
    ('J', -0.00595),
    ('c', -0.00595),
    ('k', -0.00595),
    ('v', -0.00595),
    ('x', -0.00595),
    ('ç', -0.00595),
    ('E', -0.00595),
    ('T', -0.00595),
    ('Z', -0.00595),
    ('_', -0.00595),
    ('b', -0.00595),
    ('d', -0.00595),
    ('g', -0.00595),
    ('p', -0.00595),
    ('q', -0.00595),
    ('z', -0.00595),
    ('ü', -0.00595),
    ('F', -0.00582),
    ('h', -0.00582),
    ('n', -0.00582),
    ('u', -0.00582),
    ('ñ', -0.00582),
    ('I', -0.00536),
    ('f', -0.00536),
];
