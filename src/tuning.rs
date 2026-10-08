//! Calibration constants for matching Zebra/Labelary text output.
//!
//! Zebra's resident font 0 is CG Triumvirate Bold Condensed, which cannot be
//! bundled, so the renderer substitutes an Apache-2.0 Roboto Condensed Bold
//! subset (see `assets`). The substitute agrees on neither glyph width nor
//! per-character advance, and the two errors are independent: [`FONT0_RATIO`]
//! sets glyph *shape* width, [`FONT0_ADVANCE_DELTAS`] corrects *spacing*. Both
//! were fitted against Labelary renders.

/// Width-to-height ratio for the scalable font 0.
///
/// This is the glyph shape width only; per-character spacing lives in
/// [`FONT0_ADVANCE_DELTAS`]. Fitted on Labelary *ink* widths of 14 probe
/// glyphs (H A O S a o e x n s t 1 M W at a 100-dot cell): the rendered
/// outline widths then match Labelary's font-0 substitute to ~3px RMS, and
/// the residual spacing error is absorbed per-character by the delta table.
pub(crate) const FONT0_RATIO: f64 = 1.2968;

/// Vertical scale applied to the font 0 substitute's em, on top of the cell
/// height. ab_glyph normalizes scalable fonts by `hhea ascent - descent`
/// (Roboto Condensed: 2701 units for a 2048-unit em), which renders caps at
/// only 0.539 of the cell; multiplying the em by this factor puts cap height
/// at the 0.75-of-cell that Labelary measures. The same factor divides the
/// font-0 constants below, which are expressed in *scaled-em* units because
/// they are multiplied by `scale.y` at their use sites.
pub(crate) const FONT0_CAP_SCALE: f64 = 1.3913;

/// ^FB line pitch factor for font 0. Labelary's font-0 block pitch equals the
/// cell height exactly (probed: 60px tops at h=60), while the cap-scaled em
/// inflates `scale.y`, so the pitch factor undoes [`FONT0_CAP_SCALE`].
pub(crate) const FONT0_LINE_HEIGHT: f64 = 0.7188;

/// Constant vertical offset applied to the font 0 text pen, in pixels.
///
/// Glyph bounds are rounded to integers when rasterised, so a sub-pixel
/// disagreement with Zebra's own dot-grid snapping shows up as a one-pixel offset
/// on a large share of glyphs.
pub(crate) const TEXT_Y_OFFSET: f64 = 0.2;

/// Vertical font 0 text offset expressed as a fraction of the font cell height.
///
/// The substitute face's ascent metric differs from Zebra's cell metrics, and that
/// error scales with the font size, so this part of the correction is
/// size-proportional rather than a constant pixel shift. Added to the pen as
/// `em * scale.y`, on top of [`TEXT_Y_OFFSET`]. A pure constant of -1.3 px scores
/// the same on the calibration corpus; the split is preferred because the
/// size-proportional part is the physically motivated one and so generalises to
/// font sizes the corpus does not cover.
pub(crate) const TEXT_Y_OFFSET_EM: f64 = -0.2516;

/// Advance-axis correction for rotated font 0 text, in pixels.
///
/// Rotated fields (I = 180°, B = 270°) are drawn into a buffer whose length is
/// `ceil(text_width) + 2`, and after rotation the buffer's far end — not the pen
/// origin — sits on the field origin. The 2px padding therefore pushes the whole
/// string 2px along the reading direction relative to Labelary, which anchors the
/// pen origin at the field origin. Probes across h=12..113 and the golden corpus
/// agree on 2.0 as the joint optimum; R and N rotations anchor at the pen already
/// and are unaffected.
pub(crate) const ROTATED_ADVANCE_OFFSET: f64 = 2.0;

/// Advance for characters the font-0 substitute has no glyph for (CJK text
/// decoded via ^CI28 and similar), in *scaled-em* units (0.2976 em of the
/// cell height, divided by [`FONT0_CAP_SCALE`]).
///
/// Neither the substitute face nor Labelary's own font-0 substitution covers
/// these characters, and Labelary renders them as blank space — no .notdef
/// box, no ink — while still advancing the pen. Measured with the probe-run
/// method (`^FD123` vs `^FD中123` vs `^FD中文123`: the x-shift of the
/// trailing "123" isolates the missing-glyph advance, and n vs 2n divides the
/// 1 px quantisation error): 12/13 px per character at a 42 dot cell across
/// hanzi, katakana and hangul, i.e. 12.5/42 ≈ 0.2976 em, constant per
/// character. Rendering nothing and advancing by this amount reproduces
/// Labelary's output; drawing anything (box or fallback glyph) diverges.
pub(crate) const FONT0_MISSING_GLYPH_ADVANCE_EM: f64 = 0.21390;

/// Labelary's substitute for Zebra scalable font 1 is a monospace face (DejaVu
/// Sans Mono class), unlike font 0's condensed-proportional substitute. Probed
/// against Labelary at h=10..40, w=10..40:
///
/// - `^A1N,h,w` renders cap ≈ 0.73·h with advance ≈ 1.17·w.
/// - `^A1,,…,w` (height slot empty) feeds the width value to BOTH axes with a
///   doubled em: cap ≈ 0.73·2w, advance ≈ 1.17·w — a trailing extra parameter
///   is ignored (`^A1,,10,40` renders identical to `^A1,,10,10`).
///
/// [`FONT1_RATIO`] is the scale.x-per-width-dot for `get_scale_x`: ab_glyph
/// advances DejaVu Sans Mono at h_advance(≈1235)/height(2384) ≈ 0.518 of
/// scale.x, and scale.x = ratio × w, so ratio ≈ 1.17 / 0.518.
pub(crate) const FONT1_RATIO: f64 = 2.32;

/// Cap-height correction for font 1: DejaVu Sans Mono caps land at
/// 1493/2384 ≈ 0.626 of the ab_glyph scale; multiplying scale.y by this factor
/// puts caps at 0.73 of the height parameter, matching Labelary.
pub(crate) const FONT1_CAP_SCALE: f64 = 1.164;
/// ^FB line pitch factor for font 1. Labelary's block line pitch measures
/// 18.75px (five justified lines at tops 1429/1447.75/1466.5/1485.25/1504 in
/// the packliste reference at a 23.3px scaled em), i.e. (18.75 − 1px spacing)
/// / em ≈ 0.7625 of the scaled em, versus the 1.0 factor used elsewhere.
pub(crate) const FONT1_LINE_HEIGHT: f64 = 0.7625;

/// Constant pen-x offset for font 1, in pixels. The mono substitute's
/// left side bearing rounds one pixel wider than Labelary's face — every
/// probed font-1 line (numbers row and justified block) starts 1px right of
/// the reference, so the whole pen is shifted back.
pub(crate) const FONT1_X_OFFSET: f64 = -1.0;

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
    // Regenerated for the Roboto Condensed Bold subset: each entry is
    // (Labelary advance − rendered advance) in scaled-em units, derived from
    // the validated per-character Labelary model of the previous calibration
    // plus direct probes of the six Latin Extended-A chars from issue #65.
    (' ', 0.04998),
    ('!', 0.02784),
    ('"', 0.11782),
    ('#', -0.02836),
    ('$', -0.01318),
    ('%', 0.19767),
    ('&', 0.02765),
    ('\'', 0.06791),
    ('(', -0.01667),
    (')', -0.01625),
    ('*', 0.02340),
    ('+', 0.31338),
    (',', 0.03756),
    ('-', 0.39237),
    ('.', 0.00602),
    ('/', -0.02194),
    ('0', -0.01283),
    ('1', -0.01294),
    ('2', -0.01329),
    ('3', -0.01318),
    ('4', -0.01274),
    ('5', -0.01283),
    ('6', -0.01308),
    ('7', -0.01283),
    ('8', -0.01274),
    ('9', -0.01274),
    (':', 0.01809),
    (';', 0.03218),
    ('<', 0.39866),
    ('=', 0.29371),
    ('>', 0.39348),
    ('?', 0.00187),
    ('@', 0.10761),
    ('A', -0.02431),
    ('B', 0.00123),
    ('C', -0.01945),
    ('D', 0.02392),
    ('E', 0.01250),
    ('F', 0.02329),
    ('G', 0.00950),
    ('H', 0.00316),
    ('I', 0.00712),
    ('J', -0.02788),
    ('K', 0.00646),
    ('L', 0.00649),
    ('M', 0.00785),
    ('N', 0.00420),
    ('O', -0.01526),
    ('P', -0.00429),
    ('Q', -0.01527),
    ('R', 0.03169),
    ('S', 0.00195),
    ('T', -0.02443),
    ('U', 0.03318),
    ('V', -0.02497),
    ('W', 0.05313),
    ('X', 0.00226),
    ('Y', 0.01434),
    ('Z', -0.01545),
    ('[', 0.02585),
    ('\\', 0.07486),
    (']', 0.02543),
    ('^', 0.08752),
    ('_', 0.07668),
    ('`', -0.02219),
    ('a', -0.00116),
    ('b', 0.00939),
    ('c', -0.00648),
    ('d', 0.00905),
    ('e', 0.00615),
    ('f', -0.03325),
    ('g', 0.00387),
    ('h', 0.01156),
    ('i', 0.00794),
    ('j', 0.01143),
    ('k', -0.01960),
    ('l', 0.00794),
    ('m', 0.01909),
    ('n', 0.01156),
    ('o', -0.00697),
    ('p', 0.00939),
    ('q', 0.00801),
    ('r', 0.00354),
    ('s', -0.01691),
    ('t', -0.01946),
    ('u', 0.01156),
    ('v', 0.00111),
    ('w', 0.02985),
    ('x', -0.00131),
    ('y', 0.00296),
    ('z', -0.04130),
    ('{', 0.14869),
    ('|', 0.18024),
    ('}', 0.14859),
    ('~', -0.03016),
    (' ', -0.14834),
    ('¡', 0.05063),
    ('¢', 0.00401),
    ('£', -0.02629),
    ('¤', -0.12540),
    ('¥', 0.02816),
    ('¦', 0.00338),
    ('§', -0.04596),
    ('¨', -0.08879),
    ('©', 0.04934),
    ('ª', -0.05964),
    ('«', 0.04611),
    ('¬', 0.02092),
    ('®', 0.04934),
    ('¯', -0.08154),
    ('°', 0.07014),
    ('±', 0.02954),
    ('²', -0.01892),
    ('³', -0.01892),
    ('´', 0.00749),
    ('µ', 0.03526),
    ('¶', 0.09213),
    ('·', 0.04096),
    ('¸', 0.06581),
    ('¹', -0.01892),
    ('º', -0.06862),
    ('»', 0.04576),
    ('¼', 0.10539),
    ('½', 0.07710),
    ('¾', 0.04742),
    ('¿', 0.04783),
    ('À', -0.01773),
    ('Á', -0.01773),
    ('Â', -0.01773),
    ('Ã', -0.01773),
    ('Ä', -0.01773),
    ('Å', -0.01773),
    ('Æ', -0.01193),
    ('Ç', 0.00125),
    ('È', 0.01677),
    ('É', 0.01677),
    ('Ê', 0.01677),
    ('Ë', 0.01677),
    ('Ì', 0.01098),
    ('Í', 0.01098),
    ('Î', 0.01098),
    ('Ï', 0.01098),
    ('Ð', 0.03468),
    ('Ñ', 0.01156),
    ('Ò', 0.01778),
    ('Ó', 0.01778),
    ('Ô', 0.01778),
    ('Õ', 0.01778),
    ('Ö', 0.01778),
    ('×', 0.03230),
    ('Ø', 0.01881),
    ('Ù', 0.04055),
    ('Ú', 0.04055),
    ('Û', 0.04055),
    ('Ü', 0.04055),
    ('Ý', 0.02092),
    ('Þ', 0.02747),
    ('ß', -0.02485),
    ('à', -0.00116),
    ('á', 0.03092),
    ('â', 0.03092),
    ('ã', 0.03092),
    ('ä', -0.00116),
    ('å', 0.03092),
    ('æ', 0.01049),
    ('ç', -0.00648),
    ('è', 0.00615),
    ('é', 0.00615),
    ('ê', 0.02540),
    ('ë', 0.02540),
    ('ì', 0.01684),
    ('í', 0.01684),
    ('î', 0.01684),
    ('ï', 0.01684),
    ('ð', 0.00849),
    ('ñ', 0.01156),
    ('ò', 0.01194),
    ('ó', 0.01194),
    ('ô', 0.01194),
    ('õ', 0.01194),
    ('ö', -0.00731),
    ('÷', 0.01056),
    ('ø', 0.01263),
    ('ù', 0.01574),
    ('ú', 0.01574),
    ('û', 0.01574),
    ('ü', 0.01146),
    ('ý', 0.00746),
    ('þ', 0.01091),
    ('ÿ', 0.00746),
    ('Ā', -0.40750),
    ('Ă', -0.02433),
    ('ă', -0.00204),
    ('Ą', 0.00481),
    ('ą', 0.03092),
    ('Ć', 0.00125),
    ('ć', 0.00652),
    ('Č', 0.00125),
    ('č', -0.00221),
    ('Ď', 0.04504),
    ('ď', -0.03844),
    ('Đ', 0.01445),
    ('đ', -0.04090),
    ('Ę', 0.01532),
    ('ę', 0.01304),
    ('Ě', 0.01677),
    ('ě', 0.02540),
    ('Ĝ', 0.03089),
    ('ĝ', 0.00815),
    ('İ', 0.01098),
    ('ı', 0.01684),
    ('Ł', 0.01401),
    ('Ń', 0.01156),
    ('ń', 0.01574),
    ('Ň', 0.01156),
    ('ň', 0.01574),
    ('Ő', 0.01778),
    ('ő', 0.01194),
    ('Œ', 0.01115),
    ('œ', -0.01987),
    ('Ř', 0.05297),
    ('ř', 0.00783),
    ('Ś', 0.01682),
    ('ś', 0.00193),
    ('Ŝ', -0.36712),
    ('Ş', 0.02264),
    ('ş', 0.00193),
    ('Š', 0.02264),
    ('š', 0.00193),
    ('Ţ', -0.02676),
    ('ţ', -0.01820),
    ('Ť', -0.02015),
    ('ť', 0.01201),
    ('Ů', 0.04055),
    ('ů', 0.01574),
    ('Ÿ', 0.02092),
    ('Ź', -0.00536),
    ('ź', -0.02394),
    ('Ż', -0.01118),
    ('ż', -0.02394),
    ('Ž', -0.01118),
    ('ž', -0.03702),
    ('ƀ', -0.36436),
    ('ƒ', 0.12996),
    ('ˆ', -0.07844),
    ('ˇ', -0.06256),
    ('ˈ', 0.06550),
    ('˘', -0.07188),
    ('˙', 0.05442),
    ('˚', 0.00680),
    ('˛', 0.04096),
    ('˜', -0.06670),
    ('˝', -0.01943),
    (' ', -0.30190),
    ('–', -0.02326),
    ('—', 0.26062),
    ('‘', 0.04031),
    ('’', 0.04341),
    ('‚', 0.02892),
    ('“', 0.08993),
    ('”', 0.08752),
    ('„', 0.09097),
    ('†', 0.03161),
    ('‡', 0.00366),
    ('•', 0.05108),
    ('…', 0.25855),
    ('‰', 0.22780),
    ('‹', 0.00131),
    ('›', 0.00822),
    ('⁄', -0.15739),
    ('€', -0.01318),
    ('℀', -0.47651),
    ('™', 0.23404),
    ('⅓', 0.01843),
    ('⅔', -0.04472),
    ('−', 0.02195),
    ('ﬁ', -0.02222),
    ('ﬂ', -0.02498),
];

/// Per-character advance multiplier for the resident bitmap fonts A–H, applied
/// on top of the DejaVu Sans Mono substitute's cell advance (which equals the
/// `width` parameter). Zebra's real bitmap cells advance wider than the coded
/// cell width; measured start-to-start over 9-gap `HHHHHHHHHH` runs against
/// Labelary at 1x and 2x (2x confirms linear scaling):
///
/// | Font | cell advance (coded) | Labelary advance | multiplier |
/// |------|----------------------|------------------|------------|
/// | A    | 6                    | 6                | 1.0        |
/// | B    | 9                    | 9                | 1.0        |
/// | C    | 10                   | 12               | 1.2        |
/// | D    | 12 (ratio 2.317)     | 12               | 1.0        |
/// | E    | 15                   | 20               | 4/3        |
/// | F    | 13                   | 16               | 16/13      |
/// | G    | 40                   | 48               | 1.2        |
/// | H    | 13                   | 19               | 19/13      |
///
/// The multiplier decouples ONLY the pen advance: glyph ink is still laid out
/// with the substitute's own shape width (`scale.x` unchanged), so glyph
/// rasterization and the vertical model are untouched. Fonts 0, 1 and P–V have
/// their own calibrated advance models and return 1.0 here.
pub(crate) fn bitmap_advance_mult(name: &str) -> f64 {
    match name {
        "C" | "G" => 1.2,
        "E" => 4.0 / 3.0,
        "F" => 16.0 / 13.0,
        "H" => 19.0 / 13.0,
        _ => 1.0,
    }
}
