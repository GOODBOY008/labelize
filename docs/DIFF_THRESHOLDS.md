# Rendering Diff Thresholds

Labelize renders ZPL/EPL labels to pixel images. This document tracks the
expected pixel-difference percentage for every test label compared against
reference images from the [Labelary ZPL viewer](https://labelary.com/viewer.html).

## Reference Setup

### Carrier Labels (`testdata/labels/`) and Unit Tests (`testdata/unit/`)

| Parameter | Value |
|-----------|-------|
| DPI | 8 dpmm (≈ 203 dpi) |
| Label size | 4.005 × 8.01 inches (101.625 × 203.25 mm) |
| Pixel dims | 813 × 1626 |
| Source | Labelary API `http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/` |

Both directories share the same canvas: `default_options()` renders at 813 × 1626 and
every Labelary request uses `LABELARY_LABEL_SIZE_IN` (4.005 × 8.01 in), which Labelary
serves natively at 813 × 1626 — the exact same size, so no padding is needed.

**Do not derive the request size from the mm values** (`101.625 / 25.4` in): Labelary
floor-rounds that 4.00197 × 8.00197 in request to a 812 × 1624 px canvas server-side.
Padding such a response up to 813 × 1626 shifts all content by (−1, −2) px relative to
a native render, which misaligned every `^POI` (inverted) label — e.g. the fedex family
frame lines and barcodes sat 2 px off, inflating `fedex_express` from 6.2 % to 11.3 %.
References fetched before this convention were re-fetched at the native size (all
`^POI` labels: bpost, brtit, canadapost, colissimo, fedex, fedex_express, fedex_ground,
mu_millimeters, purolator, ups, ups_import_control, ups_maxicode, ups_surepost,
usps_apo, usps_intl).

Non-inverted labels are insensitive to this (their content anchors to the top-left
origin), so a few of their references may still contain padded 812 × 1624 renders —
harmless, and they are refreshed opportunistically when a label's fixture changes.

The EPL label `dpduk.epl` uses a reference rendered by the Go-based labelize
predecessor because Labelary does not support EPL. The `epl2_showcase.epl`
snippet (Table 1 bar code types, `b` 2-D bar codes, `GW`/`X`/`LS`/`LW`)
uses the renderer baseline as its reference (0.00 % at rest), so its
tolerance only guards against regressions in the covered EPL paths.

## Diff Categories

### Reverse encoding (`testdata/reverse/`)

All eight `golden_reverse_*` tests require **0% difference and identical dimensions**,
and also compare generated ZPL/TSPL bytes to committed snapshots. References are
Labelary captures of the generated ZPL, independently checked against the intended
thresholded raster. TSPL uses that raster as its expected output, not a TSPL
printer capture. See [fixture provenance and regeneration](../testdata/reverse/README.md).

| Case | Languages | Dimensions | dpmm / threshold | Tolerance |
|---|---|---|---|---|
| shipping | ZPL, TSPL | 813×1626 | 8 / 128 | 0.00% |
| rgba | ZPL, TSPL | 205×137 | 8 / 128 | 0.00% |
| threshold | ZPL, TSPL | 205×137 | 12 / 200 | 0.00% |
| field_block | ZPL, TSPL | 813×1626 | 8 / 128 | 0.00% |

The report is `testdata/diffs/diff_report_reverse.txt`; side-by-side images and
pixel-difference masks are `testdata/diffs/reverse_*`.

### Categories for forward rendering

| Category | Range | Meaning |
|----------|-------|---------|
| PERFECT | 0 % | Pixel-identical |
| GOOD | < 1 % | Sub-pixel / anti-alias noise |
| MINOR | 1 – 5 % | Small font or position deltas |
| MODERATE | 5 – 15 % | Font engine, embedded graphics, or 2D barcode differences |
| HIGH | ≥ 15 % | Missing encoder or large structural mismatch |

## Per-Label Thresholds

Each label has a CI tolerance set slightly above the current diff to catch regressions.
If a future change raises the diff beyond this ceiling the golden test fails.
Labels marked **—** have no dedicated golden test; they appear only in the diff report
(`cargo test --test e2e_diff_report`), whose HIGH (≥ 15 %) classification is
informational.

| Label | Ext | Diff % | Tolerance | Primary diff source |
|-------|-----|--------|-----------|---------------------|
| amazonshipping | zpl | 2.18 | 4.0 | DataMatrix in 4 orientations + ^FR + font metrics |
| aztec_ec_1_ec23 | zpl | 0.36 | 7.5 | Aztec encoder pattern differences (rxing vs Labelary), EC 23% |
| aztec_ec_2_ec45 | zpl | 0.53 | 7.5 | Aztec encoder pattern differences, EC 45% |
| aztec_ec_3_ec70 | zpl | 0.87 | 7.5 | Aztec encoder pattern differences, EC 70% |
| aztec_ec_4_ec95 | zpl | 5.06 | 7.5 | Aztec encoder pattern differences, EC 95% |
| brtit | zpl | 1.21 | 2.0 | ^POI orientation + ~DG logo + font metrics |
| cf_font_designator | zpl | 0.13 | 5.0 | ^CF default font metrics |
| cf_font_no_orientation | zpl | 0.18 | 5.0 | ^CF default font metrics |
| code128_mode_d_fnc1 | zpl | 0.50 | 1.0 | Code128 mode D FNC1 display |
| dhlparcelit | zpl | 1.84 | 2.5 | ~DG/^XG stored graphics + font metrics |
| dhlparceluk_dhl_text | zpl | 0.12 | 5.5 | Font metrics |
| dhlparceluk_ver | zpl | 0.05 | 5.5 | Font metrics |
| empty_barcodes | zpl | 0.00 | 1.0 | Empty 2D barcode fields (QR/DM/MaxiCode skip, Aztec core) — pixel-identical |
| fb_single_line_overflow | zpl | 0.12 | 0.2 | Shipping header: long identifier wraps at character boundaries and overprints the last ^FB line; residual font metrics |
| fo_lenient_coord | zpl | 0.02 | 5.0 | ^FO coordinate parsing leniency |
| lowercase_orientation | zpl | 0.24 | 1.0 | Lowercase single-letter params (^FWr, ^BCr, ^A0r, ^FB c, y/n) — issue #104 |
| maxicode_default_mode2 | zpl | 0.59 | 1.0 | MaxiCode mode 2 module placement |
| maxicode_mode4 | zpl | 0.58 | 1.0 | MaxiCode mode 4 module placement |
| mu_dpi_conversion | zpl | 0.05 | 2.0 | ^MU dpi conversion + font metrics |
| mu_millimeters | zpl | 3.16 | 8.0 | ^MU millimeter units + font metrics |
| pdf417_basic | zpl | 0.00 | 0.5 | Byte-identical to Labelary after compaction match |
| posteit | zpl | 2.62 | 7.5 | ^GFA Z64 logo + DataMatrix + font metrics |
| postnl_qr | zpl | 0.00 | 5.0 | Perfect |
| qr_ft_600 | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| qr_ft_by100 | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| qr_ft_test | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| ups_maxicode | zpl | 0.62 | 5.0 | MaxiCode + font metrics |
| ean8_upca | zpl | 0.41 | 2.0 | EAN-8/UPC-A ^B8/^BU (bars pixel-perfect, module-centered interpretation font) |
| ge_ellipse | zpl | 0.22 | 2.0 | ^GE ellipse (exact ring; Labelary rasterizer cap differs slightly) |
| lt_ls | zpl | 0.73 | 2.0 | ^LT/^LS content shift (position-perfect, font substitution on text) |
| upce | zpl | 0.73 | 2.0 | UPC-E ^B9 (bars pixel-perfect, module-centered interpretation font) |
| amazon | zpl | 1.25 | 3.5 | Font metrics |
| aztec_ec | zpl | 6.82 | 7.5 | Aztec barcode encoding (correct symbol size, different internal patterns from rxing) |
| barcode128_default_width | zpl | 0.25 | 2.0 | Sub-pixel barcode bars |
| barcode128_line | zpl | 0.23 | 2.0 | Sub-pixel |
| barcode128_line_above | zpl | 0.28 | 2.0 | Sub-pixel |
| barcode128_mode_a | zpl | 0.25 | 2.0 | Sub-pixel |
| barcode128_mode_d | zpl | 0.25 | 2.0 | Sub-pixel barcode bars |
| barcode128_mode_n | zpl | 0.25 | 2.0 | Sub-pixel |
| barcode128_mode_n_cba_sets | zpl | 0.23 | 2.0 | Barcode set switching |
| barcode128_mode_u | zpl | 0.25 | 2.0 | Font metrics |
| barcode128_rotated | zpl | 0.24 | 2.0 | Sub-pixel |
| bstc | zpl | 0.00 | 1.0 | Perfect |
| dbs | zpl | 1.87 | 5.0 | Font metrics |
| dhlecommercetr | zpl | 1.88 | 2.5 | Font metrics |
| dhlpaket | zpl | 1.13 | 3.5 | Font metrics |
| dhlparceluk | zpl | 3.28 | 4.5 | Font metrics (rotated I/B pen-anchor offset fixed) |
| dpdpl | zpl | 3.76 | 5.5 | Font metrics (R-block ^FB anchor now reserves the max_lines box like Labelary) |
| dpduk | epl | 3.80 | 6.5 | EPL reference from Go renderer |
| epl2_showcase | epl | 0.01 | 1.5 | Renderer baseline reference (Labelary has no EPL); ECC auto table per EPL2 manual |
| tspl_graphics | tspl | 0.00 | 1.5 | Renderer baseline reference (no public TSPL reference renderer); SIZE-driven canvas 608x400 |
| tspl_shipping | tspl | 0.00 | 1.5 | Renderer baseline reference; SIZE-driven canvas 812x480 |
| tspl_text_styles | tspl | 0.00 | 1.5 | Renderer baseline reference; SIZE-driven canvas 813x407 |
| tspl_real_probe_text | tspl | 0.54 | 2.0 | labelixa reference (filament-db probe) |
| tspl_real_drybox | tspl | 6.75 | 8.5 | labelixa reference; REVERSE + magnified-text open questions |
| tspl_real_te210 | tspl | 7.12 | 9.0 | labelixa reference (TSC TE210 hardware capture); labelixa skips ITF14 |
| tspl_real_sklad | tspl | 7.29 | 9.0 | labelixa reference; ${placeholder} literal text |
| tspl_real_probe_full | tspl | 10.88 | 12.5 | labelixa reference; REVERSE band skipped by labelixa (~2.4pp) + ymul-width quirk (see TSPL_BASELINES.md) |
| cpcl_2d_graphics | cpcl | 0.00 | 0.01 | Renderer baseline reference (labelixa skips QR/PDF-417/Aztec/EG/ML); header-driven canvas 576x480 |
| cpcl_manual_examples | cpcl | 13.28 | 15.0 | labelixa CPCL reference; font substitute shapes + barcode geometry distance |
| cpcl_shipping | cpcl | 16.12 | 17.5 | labelixa CPCL reference; magnified font 5 + barcode interpretation-line distance |
| dpl_shipping | dpl | 0.00 | 0.01 | Renderer baseline reference (no public DPL reference renderer); unit canvas 812x1624 |
| dpl_2d_graphics | dpl | 0.00 | 0.01 | Renderer baseline reference; QR/DataMatrix/PDF-417/MaxiCode/Aztec coverage |
| dpl_text_styles | dpl | 0.00 | 0.01 | Renderer baseline reference; fonts 0-9, rotations, scalable sizes, C/R offsets |
| cpcl_real_asset_tag | cpcl | 17.76 | 18.5 | labelixa reference (Labelixa/thermal-printer-examples corpus, MIT) |
| cpcl_real_route_label | cpcl | 12.24 | 13.0 | labelixa reference; TEXT270 + BOX + Code 128 |
| cpcl_real_receipt | cpcl | 9.54 | 10.5 | labelixa reference; text block + Code 128 |
| dpl_real_manual_component | dpl | 14.49 | 16.0 | evolabel layout-view reference (manual Appendix B Fig B-1); font-substitute width delta |
| dpl_real_manual_order | dpl | 9.74 | 13.0 | evolabel layout-view reference (manual Appendix B VB sample, adapted) |
| ean13 | zpl | 0.71 | 2.0 | Module-centered interpretation line (bars pixel-perfect) |
| edi_triangle | zpl | 0.02 | 2.0 | Sub-pixel |
| encodings_013 | zpl | 1.37 | 2.5 | Character encoding |
| fedex | zpl | 2.06 | 4.0 | PDF417 compaction now matches reference; font floor |
| fedex_express | zpl | 3.30 | 5.0 | PDF417 compaction now matches reference; font floor |
| fedex_ground | zpl | 2.53 | 4.5 | PDF417 compaction now matches reference; font floor |
| font_p | zpl | 0.17 | 1.0 | Bitmap font P (20x18 base, DejaVu Mono Bold substitute) |
| font_q | zpl | 0.18 | 1.0 | Bitmap font Q (28x24 base, DejaVu Mono Bold substitute) |
| font_r | zpl | 0.44 | 1.0 | Bitmap font R (35x31 base, DejaVu Mono Bold substitute) |
| font_s | zpl | 0.36 | 1.0 | Bitmap font S (40x35 base, DejaVu Mono Bold substitute) |
| font_t | zpl | 0.73 | 1.0 | Bitmap font T (48x42 base, DejaVu Mono Bold substitute) |
| font_u | zpl | 1.22 | 1.5 | Bitmap font U (59x53 base, DejaVu Mono Bold substitute) |
| font_v | zpl | 2.14 | 2.5 | Bitmap font V (80x71 base, DejaVu Mono Bold substitute) |
| gb_0_height | zpl | 0.00 | 1.0 | Perfect |
| gb_0_width | zpl | 0.00 | 1.0 | Perfect |
| gb_normal | zpl | 0.00 | 1.0 | Perfect |
| gb_rounded | zpl | 0.07 | 1.0 | Rounding artefacts |
| gd_default_params | zpl | 0.12 | 1.0 | Sub-pixel diagonal |
| gd_thick | zpl | 0.08 | 1.0 | Diagonal rendering |
| gd_thin_l | zpl | 0.03 | 1.0 | Sub-pixel |
| gd_thin_r | zpl | 0.03 | 1.0 | Sub-pixel |
| glscz | zpl | 1.51 | 3.5 | Font metrics |
| glsdk_return | zpl | 2.51 | 5.5 | DataMatrix + font metrics |
| gs | zpl | 1.14 | 2.0 | Graphic symbol font |
| icapaket | zpl | 3.07 | 5.5 | Font metrics |
| jcpenney | zpl | 2.34 | 6.0 | Font metrics |
| kmart | zpl | 3.40 | 5.0 | Font metrics |
| labelary | zpl | 1.85 | 4.5 | Font metrics + Code128 |
| pnldpd | zpl | 7.06 | 11.5 | Aztec + font metrics |
| pocztex | zpl | 1.91 | 4.5 | Font metrics |
| porterbuddy | zpl | 5.65 | 7.0 | QR code + font metrics |
| posten | zpl | 0.80 | 3.0 | Font metrics |
| qr_code_ft_manual | zpl | 0.29 | 1.0 | Perfect |
| qr_code_offset | zpl | 0.00 | 1.0 | Perfect |
| return_qrcode | zpl | 2.00 | 4.0 | QR + font |
| reverse | zpl | 0.25 | 1.5 | Sub-pixel |
| reverse_qr | zpl | 0.12 | 1.5 | QR barcode |
| rotated_char_display | zpl | 1.57 | 3.0 | Rotated single/multi-char fields across font classes + f1 CJK blank parity + f0 superset line (ĀŜƀ℀⁄ blank on both sides — renderer routes the Labelary-blanked glyphs through the missing-glyph advance) + rotated ^FB blocks; ink-extent buffer fix removed the f1 CJK .notdef box, R-block anchor now reserves the ^FB max_lines box like Labelary |
| swisspost | zpl | 0.71 | 1.5 | Font metrics |
| templating | zpl | 1.17 | 2.5 | Font metrics |
| text_fallback_default | zpl | 2.04 | 2.5 | Font metrics (font-1 ^A1 mono substitute; was 2.84 before the font-1 model) |
| dein_ticket_packliste | zpl | 1.23 | 1.5 | Real German packing-list fragment: font-1 ^FB L/J text + font 0; residual is the 1-bit vs Labelary AA edge floor (~0.96% measured by binarizing the reference) |
| fb_bitmap_line_pitch | zpl | 2.11 | 2.4 | Multi-line ^FB in bitmap fonts A/C/D/E/F; line positions, per-font cell advances and per-font zero dots match Labelary, residual is substitute-face glyph shapes/ink width (pitch pinned by a unit test; was 2.73 before advance calibration + zero-dot policy) |
| bitmap_font_advance | zpl | 1.33 | 1.8 | A-H `HHHHHHHHHH 0000000000` rows at 1x: pins the per-font cell advance (C=12, E=20, F=16, G=48, H=19 dots) and the per-font zero-dot policy (E/H clean, the rest dotted); residual is substitute glyph shape |
| text_fo_b | zpl | 0.05 | 1.0 | Sub-pixel |
| text_fo_i | zpl | 0.05 | 1.0 | Sub-pixel |
| text_fo_n | zpl | 0.02 | 1.0 | Sub-pixel |
| text_fo_r | zpl | 0.02 | 1.0 | Sub-pixel |
| text_ft_auto_pos | zpl | 0.62 | 2.5 | Auto-position cursor |
| text_ft_b | zpl | 0.01 | 1.0 | Sub-pixel |
| text_ft_i | zpl | 0.01 | 1.0 | Sub-pixel |
| text_ft_n | zpl | 0.02 | 1.0 | Sub-pixel |
| text_ft_r | zpl | 0.02 | 1.0 | Sub-pixel |
| text_multiline | zpl | 0.24 | 1.5 | Word-wrap boundaries |
| thick_rotated_fb_text | zpl | 0.11 | 1.0 | Rotated ^FB font-0 text (overlay now blends alpha; was stamped full-black AA skirt) |
| ups | zpl | 2.88 | 8.0 | MaxiCode + font metrics |
| ups_import_control | zpl | 3.41 | 4.5 | MaxiCode + font metrics |
| cjlogistics | zpl | 1.53 | 3.0 | Real CJ Logistics production template (adapted fonts): rotated PDF417 + Code128 + CJK blank parity |
| correiosbr | zpl | 3.47 | 3.7 | Real Correios postagem template (adapted): DataMatrix + Z64 SEDEX logo + font metrics; `Recebedor:______` signature lines improved by the font-0 underscore ink calibration (issue #78) |
| kuehnenagel_intl | zpl | 4.80 | 7.0 | Real Kuehne+Nagel capture: ^GFA logos + font metrics |
| kuehnenagel_eselect | zpl | 2.27 | 3.5 | Real Kuehne+Nagel capture: mid-format ^LH honored + ^BY wide:narrow ratio 2.7 applied to 2of5; remaining = font metrics + ^GFA graphics |
| bring | zpl | 2.00 | 3.5 | Real Bring capture: ~DG graphic + font metrics |
| ups_surepost | zpl | 3.58 | 10.0 | MaxiCode + font metrics |
| usps | zpl | 2.58 | 5.0 | Font metrics + ® superscript glyph |
| tnt_express | zpl | 2.40 | 4.0 | Real TNT capture (Centiro): mid-format ^LH honored; ^GFA logo + font metrics |
| royalmail | zpl | 1.64 | 4.5 | QR code + font metrics |
| canadapost | zpl | 1.98 | 3.5 | QR code + font (PDF417 now matches reference) |
| auspost | zpl | 2.04 | 5.0 | QR code + font metrics |
| colissimo | zpl | 2.20 | 4.5 | DataMatrix + font metrics |
| postnl | zpl | 1.91 | 5.0 | QR code + font metrics |
| bpost | zpl | 1.94 | 4.5 | QR code + font metrics |
| correos | zpl | 2.00 | 5.0 | QR code + font metrics |
| dbschenker | zpl | 2.20 | 4.0 | Font metrics (PDF417 now matches reference) |
| evri | zpl | 1.52 | 4.5 | QR code + font metrics |
| dpdde | zpl | 2.00 | 3.5 | Font metrics (PDF417 now matches reference) |
| ontrac | zpl | 1.97 | 4.5 | QR code + font metrics |
| seur | zpl | 1.93 | 3.5 | Font metrics (PDF417 now matches reference) |
| purolator | zpl | 2.08 | 4.0 | DataMatrix + font metrics |
| inpost | zpl | 3.10 | 5.5 | QR code + font metrics |
| yodel | zpl | 1.86 | 4.5 | QR code + font metrics |
| dhl_express | zpl | 2.21 | 3.5 | Real DHL Express capture (Centiro): mid-format ^LH honored; ^GFA logo + font metrics |
| dhl_home_delivery | zpl | 1.84 | — | ^GFA logo + font metrics |
| usps_apo | zpl | 2.84 | 3.5 | Font metrics (rotated I/B pen-anchor fixed; glyph-top re-anchored to Labelary after #64 unclipping; residual = glyph weight + 1-bit AA fringe) |
| usps_intl | zpl | 2.46 | 4.0 | Font metrics (^POI rotated text) |
| usps_priority_mail | zpl | 0.32 | — | Font metrics (^FB centered) |
| usps_test_merchant | zpl | 0.07 | — | Font metrics |
| cjk_font0_ci28 | zpl | 0.11 | 1.0 | ^CI28 font-0 CJK: rendered blank with calibrated advance like Labelary (probe-measured, see tuning::FONT0_MISSING_GLYPH_ADVANCE_EM); residual = Roboto Condensed metrics on the Latin prefix |
| font0_latin_ext_a | zpl | 0.21 | 1.0 | Issue #65: font-0 Latin Extended-A glyphs Labelary renders (Ă ă Đ đ Ţ ţ) drawn with the Roboto Condensed subset; Ș ș Ț ț Ħ ħ stay blank with calibrated advance on both sides |
| font0_symbols | zpl | 0.57 | 1.0 | Issue #78: font-0 symbol ink calibration (tuning::FONT0_INK_TARGETS) stretches dashes/operators/`%`/braces to Labelary's ink box, `©`/`®` render as equal-size superscripts, ASCII quotes as curly shapes; en/em dash advances re-probed (Labelary advances the whole dash family uniformly at 0.9 × cell height); residual = Roboto glyph shapes vs Labelary's Helvetica-class face on the letters |

## Known Limitations

### MaxiCode (ups, ups_surepost, ups_import_control)
MaxiCode is a proprietary 2D symbology used by UPS. The encoder implements
proper GF(64) Reed-Solomon ECC (primary 10+10, secondary 42+20 even/odd tracks)
and greedy Set-A character encoding. Remaining diff (~3-4%) is due to minor
hex module placement differences vs Labelary and font metric differences.
Verified by visual inspection: the MaxiCode symbol renders at the correct
position/size on all three; only the interior module bits differ, consistent
with Labelary's secondary-message Set-switching heuristic diverging from our
greedy Set-A-first one on the same (spec-valid) input.

### PDF417 (fedex, fedex_express, fedex_ground, dbschenker, dpdde, seur, tnt_express)
The PDF417 pipeline (`src/barcodes/pdf417_encoding.rs`) reproduces the reference
renderer's compaction segment-for-segment: ISO/IEC 15438 text sub-mode switching,
numeric compaction for medium digit runs, byte-run absorption of short blocks, and
the exact descriptor/padding/RS-ECC assembly (verified codeword-identical on the
fedex secondary-message symbol and byte-identical on `pdf417_basic`). Residual
diff on these labels is the font-metrics floor, not the barcode. The automatic
column count for `^B7` without a `columns` parameter is the classic near-cubic
sqrt rule and can still differ from the reference on auto-sized symbols.

### Aztec (aztec_ec, pnldpd)
The `rxing` crate's Aztec writer produces proper Aztec codes. Minor differences
stem from error correction level defaults and symbol sizing when the ZPL
parameters leave the size open.

### DataMatrix (glsdk_return, purolator, posteit)
The `datamatrix` crate produces a spec-correct ECC 200 symbol (no quiet zone,
matching Labelary) at the same size and position as the reference. Pixel
comparison shows the interior module pattern is still largely mismatched —
confirmed by inspection to be a genuine bit-level difference, not a rendering
bug (no rotation, mirroring, or size mismatch). ECC 200 leaves the encodation
mode (ASCII/C40/text/base256) selection open when multiple paths are equally
efficient for a given input; the `datamatrix` crate and Labelary's encoder
resolve that ambiguity differently, producing different but equally valid
codeword sequences for the same data.

### Font Rendering
Labelize uses `ab_glyph` with an Apache-2.0 Roboto Condensed Bold subset for
font 0 (ink-fitted width ratio 1.2968 + cap-scale 1.3913, see
`tuning::FONT0_*`) and DejaVu Sans Mono variants for bitmap fonts A–H. Labelary
renders font 0 with a face that is metrically near-identical to Adobe
Helvetica Condensed Bold (probe-verified), so residual diffs come from the
design difference between the two faces, concentrated on dense small text
(dhlecommercetr, dhlparcelit). The previous Helvetica-substitute calibration
was replaced in 2026-10 after the old TTF turned out to be an unlicensed Adobe
derivative (issue #65).

### GFA Graphics
Embedded `^GFA` hex graphics are decoded and rasterised accurately. Remaining
differences (< 2 %) are primarily from anti-aliasing at logo edges and slight
coordinate rounding.

### Stored Graphics (dhlparcelit, brtit)
`~DG` downloads and `^XG` recalls render correctly. The remaining ~2.4 % diff on
`dhlparcelit` is font metrics; the rotated I/B pen-anchor offset that used to
dominate its ^A0I text was fixed (see `tuning::ROTATED_ADVANCE_OFFSET`). The
`^XG.GRF` (unnamed recall) does not match the stored `CMR.GRF` key — both our
implementation and Labelary skip it.

### Single-line field-block overflow

`fb_single_line_overflow` isolates the `C2PQQ…` shipping-label header with
`^A0,60,0^FB400,1,0,L,0`. Its independent Labelary reference was fetched on
2026-10-28 using the standard 8 dpmm, 4.005 × 8.01 inch endpoint. Labelary
breaks the oversized identifier at character boundaries and overlays the
wrapped lines in the final line slot, as required for overflow by the
[Zebra ^FB reference](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-fb.html).
The full-canvas diff fell from 1.36% to 0.12%. The renderer regression also
checks the header's 420 × 80 pixel region directly (29.04% → 4.88%, ceiling
8%) so blank surrounding pixels cannot hide a broken header.

## Updating References

`datamatrix_dimensions` uses a **0.0%** tolerance against its independent
Labelary reference. It covers ECC200 rectangular size constraints and the
row-only capacity boundary; see [DATAMATRIX_DIMENSIONS.md](DATAMATRIX_DIMENSIONS.md).

`qr_mask_selection` has a **0.0%** tolerance against an independent Labelary
reference for five isolated EC-H Byte cases. See [QR_MASK_SELECTION.md](QR_MASK_SELECTION.md).

The `print_mirror`, `print_mirror_width`, and `print_mirror_inverted` fixtures
have a strict pixel-for-pixel check in `tests/unit_print_mirror.rs` in addition
to the golden suite. Their independent Labelary references and measured
coordinate expectations are documented in [PRINT_MIRROR.md](PRINT_MIRROR.md).

To regenerate all Labelary reference images:

```sh
# ZPL labels (Labelary API)
for f in testdata/labels/*.zpl testdata/unit/*.zpl; do
  name=$(basename "$f" .zpl)
  dir=$(dirname "$f")
  curl -s -X POST http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/ \
    -F "file=@$f" -o "${dir}/${name}.png"
done

# EPL labels — Labelary does not support EPL.
# Use the Go renderer or keep existing references.
```

## Running the Diff Report

```sh
# Full report (no failure on HIGH)
cargo test --test e2e diff_report -- --nocapture

# Golden tests with per-label tolerances (fails on regression)
cargo test --test e2e e2e_golden -- --test-threads=4
```
