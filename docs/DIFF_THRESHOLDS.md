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
| amazonshipping | zpl | 2.29 | 4.0 | DataMatrix in 4 orientations + ^FR + font metrics |
| aztec_ec_1_ec23 | zpl | 0.36 | 7.5 | Aztec encoder pattern differences (rxing vs Labelary), EC 23% |
| aztec_ec_2_ec45 | zpl | 0.53 | 7.5 | Aztec encoder pattern differences, EC 45% |
| aztec_ec_3_ec70 | zpl | 0.87 | 7.5 | Aztec encoder pattern differences, EC 70% |
| aztec_ec_4_ec95 | zpl | 5.06 | 7.5 | Aztec encoder pattern differences, EC 95% |
| brtit | zpl | 1.87 | 3.0 | ^POI orientation + ~DG logo + font metrics |
| cf_font_designator | zpl | 0.13 | 5.0 | ^CF default font metrics |
| cf_font_no_orientation | zpl | 0.18 | 5.0 | ^CF default font metrics |
| code128_mode_d_fnc1 | zpl | 0.23 | 1.0 | Code128 mode D FNC1 display |
| dhlparcelit | zpl | 3.92 | 7.0 | ~DG/^XG stored graphics + font metrics |
| dhlparceluk_dhl_text | zpl | 0.23 | 5.5 | Font metrics |
| dhlparceluk_ver | zpl | 0.05 | 5.5 | Font metrics |
| fo_lenient_coord | zpl | 0.02 | 5.0 | ^FO coordinate parsing leniency |
| maxicode_default_mode2 | zpl | 0.59 | 1.0 | MaxiCode mode 2 module placement |
| maxicode_mode4 | zpl | 0.58 | 1.0 | MaxiCode mode 4 module placement |
| mu_dpi_conversion | zpl | 0.06 | 2.0 | ^MU dpi conversion + font metrics |
| mu_millimeters | zpl | 3.18 | 8.0 | ^MU millimeter units + font metrics |
| pdf417_basic | zpl | 0.29 | 1.0 | PDF417 compaction mode selection |
| posteit | zpl | 3.30 | 7.5 | ^GFA Z64 logo + DataMatrix + font metrics |
| postnl_qr | zpl | 0.00 | 5.0 | Perfect |
| qr_ft_600 | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| qr_ft_by100 | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| qr_ft_test | zpl | 0.47 | 1.0 | QR render with ^FT positioning |
| ups_maxicode | zpl | 0.62 | 5.0 | MaxiCode + font metrics |
| ean8_upca | zpl | 0.41 | 2.0 | EAN-8/UPC-A ^B8/^BU (bars pixel-perfect, module-centered interpretation font) |
| ge_ellipse | zpl | 0.22 | 2.0 | ^GE ellipse (exact ring; Labelary rasterizer cap differs slightly) |
| lt_ls | zpl | 0.73 | 2.0 | ^LT/^LS content shift (position-perfect, font substitution on text) |
| upce | zpl | 0.73 | 2.0 | UPC-E ^B9 (bars pixel-perfect, module-centered interpretation font) |
| amazon | zpl | 1.08 | 3.5 | Font metrics |
| aztec_ec | zpl | 6.82 | 7.5 | Aztec barcode encoding (correct symbol size, different internal patterns from rxing) |
| barcode128_default_width | zpl | 0.54 | 2.0 | Sub-pixel barcode bars |
| barcode128_line | zpl | 0.23 | 2.0 | Sub-pixel |
| barcode128_line_above | zpl | 0.27 | 2.0 | Sub-pixel |
| barcode128_mode_a | zpl | 0.54 | 2.0 | Sub-pixel |
| barcode128_mode_d | zpl | 0.54 | 2.0 | Sub-pixel barcode bars |
| barcode128_mode_n | zpl | 0.54 | 2.0 | Sub-pixel |
| barcode128_mode_n_cba_sets | zpl | 0.23 | 2.0 | Barcode set switching |
| barcode128_mode_u | zpl | 0.54 | 2.0 | Font metrics |
| barcode128_rotated | zpl | 0.24 | 2.0 | Sub-pixel |
| bstc | zpl | 0.00 | 1.0 | Perfect |
| dbs | zpl | 1.87 | 5.0 | Font metrics |
| dhlecommercetr | zpl | 2.51 | 4.5 | Font metrics |
| dhlpaket | zpl | 1.48 | 3.5 | Font metrics |
| dhlparceluk | zpl | 4.45 | 5.5 | Font metrics (^FT Rotated270 multi-line x fixed) |
| dpdpl | zpl | 5.60 | 7.5 | Font metrics |
| dpduk | epl | 5.79 | 6.5 | EPL reference from Go renderer |
| epl2_showcase | epl | 0.00 | 2.0 | Renderer baseline reference (Labelary has no EPL) |
| ean13 | zpl | 0.71 | 2.0 | Module-centered interpretation line (bars pixel-perfect) |
| edi_triangle | zpl | 0.03 | 2.0 | Sub-pixel |
| encodings_013 | zpl | 1.56 | 2.5 | Character encoding |
| fedex | zpl | 4.85 | 7.0 | PDF417 encoding + font |
| fedex_express | zpl | 6.24 | 7.0 | PDF417 encoding + font |
| fedex_ground | zpl | 5.45 | 6.0 | PDF417 encoding + font |
| font_p | zpl | 0.17 | 1.0 | Bitmap font P (20x18 base, DejaVu Mono Bold substitute) |
| font_q | zpl | 0.17 | 1.0 | Bitmap font Q (28x24 base, DejaVu Mono Bold substitute) |
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
| glscz | zpl | 1.97 | 3.5 | Font metrics |
| glsdk_return | zpl | 2.84 | 5.5 | DataMatrix + font metrics |
| gs | zpl | 1.14 | 2.0 | Graphic symbol font |
| icapaket | zpl | 3.06 | 5.5 | Font metrics |
| jcpenney | zpl | 2.84 | 6.0 | Font metrics |
| kmart | zpl | 3.95 | 8.0 | Font metrics |
| labelary | zpl | 1.84 | 4.5 | Font metrics + Code128 |
| pnldpd | zpl | 7.57 | 11.5 | Aztec + font metrics |
| pocztex | zpl | 2.36 | 4.5 | Font metrics |
| porterbuddy | zpl | 5.64 | 7.0 | QR code + font metrics |
| posten | zpl | 1.23 | 3.0 | Font metrics |
| qr_code_ft_manual | zpl | 0.29 | 1.0 | Perfect |
| qr_code_offset | zpl | 0.00 | 1.0 | Perfect |
| return_qrcode | zpl | 1.96 | 4.0 | QR + font |
| reverse | zpl | 0.25 | 1.5 | Sub-pixel |
| reverse_qr | zpl | 0.12 | 1.5 | QR barcode |
| swisspost | zpl | 0.98 | 2.5 | Font metrics |
| templating | zpl | 1.17 | 2.5 | Font metrics |
| text_fallback_default | zpl | 2.84 | 5.0 | Font metrics |
| text_fo_b | zpl | 0.09 | 1.0 | Sub-pixel |
| text_fo_i | zpl | 0.11 | 1.0 | Sub-pixel |
| text_fo_n | zpl | 0.03 | 1.0 | Sub-pixel |
| text_fo_r | zpl | 0.06 | 1.0 | Sub-pixel |
| text_ft_auto_pos | zpl | 0.93 | 2.5 | Auto-position cursor |
| text_ft_b | zpl | 0.03 | 1.0 | Sub-pixel |
| text_ft_i | zpl | 0.03 | 1.0 | Sub-pixel |
| text_ft_n | zpl | 0.05 | 1.0 | Sub-pixel |
| text_ft_r | zpl | 0.06 | 1.0 | Sub-pixel |
| text_multiline | zpl | 0.24 | 1.5 | Word-wrap boundaries |
| ups | zpl | 2.98 | 8.0 | MaxiCode + font metrics |
| ups_import_control | zpl | 3.59 | 4.5 | MaxiCode + font metrics |
| ups_surepost | zpl | 3.74 | 10.0 | MaxiCode + font metrics |
| usps | zpl | 2.72 | 5.0 | Font metrics + ® superscript glyph |
| tnt_express | zpl | 2.71 | 5.0 | Font metrics + PDF417 |
| royalmail | zpl | 1.48 | 4.5 | QR code + font metrics |
| canadapost | zpl | 2.22 | 5.0 | QR code + PDF417 + font |
| auspost | zpl | 1.93 | 5.0 | QR code + font metrics |
| colissimo | zpl | 2.09 | 4.5 | DataMatrix + font metrics |
| postnl | zpl | 1.78 | 5.0 | QR code + font metrics |
| bpost | zpl | 1.78 | 4.5 | QR code + font metrics |
| correos | zpl | 1.91 | 5.0 | QR code + font metrics |
| dbschenker | zpl | 2.70 | 5.5 | PDF417 + font metrics |
| evri | zpl | 1.40 | 4.5 | QR code + font metrics |
| dpdde | zpl | 2.55 | 4.5 | PDF417 + font metrics |
| ontrac | zpl | 1.89 | 4.5 | QR code + font metrics |
| seur | zpl | 2.34 | 4.5 | PDF417 + font metrics |
| purolator | zpl | 1.94 | 4.0 | DataMatrix + font metrics |
| inpost | zpl | 2.99 | 5.5 | QR code + font metrics |
| yodel | zpl | 1.74 | 4.5 | QR code + font metrics |
| dhl_express | zpl | 1.26 | — | Font metrics (A0 font) |
| dhl_home_delivery | zpl | 1.84 | — | ^GFA logo + font metrics |
| usps_apo | zpl | 5.55 | — | Font metrics (^POI rotated text) |
| usps_intl | zpl | 2.83 | — | Font metrics (^POI rotated text) |
| usps_priority_mail | zpl | 0.32 | — | Font metrics (^FB centered) |
| usps_test_merchant | zpl | 0.07 | — | Font metrics |

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
The `pdf417` crate produces **valid, scannable** barcodes, but the specific
codeword arrangement differs from Labelary's encoder. Both are correct per the
ISO 15438 specification; different encoders may choose different text/byte/numeric
compaction modes resulting in visually different (but equivalent) barcodes.
`fedex_express`/`fedex_ground` use identical `^B7` parameters to `fedex` — their
slightly higher diff (~5.5-6.2% vs ~4.9%) is because their secondary message is
longer, so a larger fraction of the label is barcode area exposed to this mismatch.

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
Labelize uses `ab_glyph` with Helvetica Bold Condensed for font 0 (width ratio
0.55) and DejaVu Sans Mono variants for bitmap fonts A–H (width ratio 1.661).
Labelary uses its own proprietary font set. Character advance widths and hinting
differ between engines, causing 1–10 % diffs on text-heavy labels. The font 0
ratio of 0.55 was calibrated via systematic sweep over 0.53–0.60 to minimize
the number of moderate-diff labels.

### GFA Graphics
Embedded `^GFA` hex graphics are decoded and rasterised accurately. Remaining
differences (< 2 %) are primarily from anti-aliasing at logo edges and slight
coordinate rounding.

### Stored Graphics (dhlparcelit, brtit)
`~DG` downloads and `^XG` recalls render correctly. The remaining ~3.9 % diff on
`dhlparcelit` is due to font metrics (^A0I text). The `^XG.GRF` (unnamed recall)
does not match the stored `CMR.GRF` key — both our implementation and Labelary
skip it.

## Updating References

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
