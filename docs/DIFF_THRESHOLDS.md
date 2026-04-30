# Rendering Diff Thresholds

Labelize renders ZPL/EPL labels to pixel images. This document tracks the
expected pixel-difference percentage for every test label compared against
reference images from the [Labelary ZPL viewer](https://labelary.com/viewer.html).

## Reference Setup

| Parameter | Value |
|-----------|-------|
| DPI | 8 dpmm (≈ 203 dpi) |
| Label size | 4.005 × 8.01 inches (101.625 × 203.25 mm) |
| Pixel dims | 813 × 1626 |
| Source | Labelary API `http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/` |

The EPL label `dpduk.epl` uses a reference rendered by the Go-based labelize
predecessor because Labelary does not support EPL.

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

| Label | Ext | Diff % | Tolerance | Primary diff source |
|-------|-----|--------|-----------|---------------------|
| amazon | zpl | 2.81 | 4.0 | Font metrics |
| aztec_ec | zpl | 6.82 | 7.5 | Aztec barcode encoding (correct symbol size, different internal patterns from rxing) |
| barcode128_default_width | zpl | 0.63 | 2.0 | Sub-pixel barcode bars |
| barcode128_line | zpl | 0.23 | 2.0 | Sub-pixel |
| barcode128_line_above | zpl | 0.26 | 2.0 | Sub-pixel |
| barcode128_mode_a | zpl | 0.63 | 2.0 | Sub-pixel |
| barcode128_mode_d | zpl | 0.63 | 2.0 | Sub-pixel barcode bars |
| barcode128_mode_n | zpl | 0.63 | 2.0 | Sub-pixel |
| barcode128_mode_n_cba_sets | zpl | 0.38 | 2.0 | Barcode set switching |
| barcode128_mode_u | zpl | 2.20 | 3.0 | Font metrics |
| barcode128_rotated | zpl | 0.27 | 2.0 | Sub-pixel |
| bstc | zpl | 0.00 | 1.0 | Perfect |
| dbs | zpl | 5.27 | 6.0 | Font metrics |
| dhlecommercetr | zpl | 5.00 | 6.0 | Font metrics |
| dhlpaket | zpl | 2.68 | 4.0 | Font metrics |
| dhlparceluk | zpl | 5.14 | 5.7 | Font metrics (^FT Rotated270 multi-line x fixed) |
| dpdpl | zpl | 6.38 | 7.0 | Font metrics |
| dpduk | epl | 5.79 | 6.5 | EPL reference from Go renderer |
| ean13 | zpl | 2.84 | 3.0 | Sub-pixel barcode bars |
| edi_triangle | zpl | 0.63 | 2.0 | Sub-pixel |
| encodings_013 | zpl | 1.91 | 3.0 | Character encoding |
| fedex | zpl | 7.91 | 9.0 | PDF417 encoding + font |
| gb_0_height | zpl | 0.00 | 1.0 | Perfect |
| gb_0_width | zpl | 0.00 | 1.0 | Perfect |
| gb_normal | zpl | 0.00 | 1.0 | Perfect |
| gb_rounded | zpl | 0.07 | 1.0 | Rounding artefacts |
| gd_default_params | zpl | 0.29 | 1.0 | Sub-pixel diagonal |
| gd_thick | zpl | 3.90 | 4.5 | Diagonal rendering |
| gd_thin_l | zpl | 0.07 | 2.0 | Sub-pixel |
| gd_thin_r | zpl | 0.08 | 2.0 | Sub-pixel |
| glscz | zpl | 3.19 | 5.0 | Font metrics |
| glsdk_return | zpl | 6.45 | 7.0 | DataMatrix + font metrics |
| gs | zpl | 1.01 | 2.0 | Graphic symbol font |
| icapaket | zpl | 5.39 | 6.0 | Font metrics |
| jcpenney | zpl | 6.88 | 8.0 | Font metrics |
| kmart | zpl | 7.03 | 7.5 | Font metrics |
| labelary | zpl | 4.48 | 5.5 | Font metrics + Code128 |
| pnldpd | zpl | 13.78 | 14.5 | Aztec + font metrics |
| pocztex | zpl | 4.56 | 5.5 | Font metrics |
| porterbuddy | zpl | 6.36 | 7.5 | QR code + font metrics |
| posten | zpl | 3.61 | 5.0 | Font metrics |
| qr_code_ft_manual | zpl | 2.30 | 3.0 | QR barcode + position |
| qr_code_offset | zpl | 0.41 | 1.5 | QR position offset |
| return_qrcode | zpl | 3.10 | 4.0 | QR + font |
| reverse | zpl | 0.80 | 2.0 | Sub-pixel |
| reverse_qr | zpl | 0.33 | 1.5 | QR barcode |
| swisspost | zpl | 1.81 | 3.0 | Font metrics |
| templating | zpl | 1.17 | 2.0 | Font metrics |
| text_fallback_default | zpl | 4.25 | 5.0 | Font metrics |
| text_fo_b | zpl | 0.10 | 1.0 | Sub-pixel |
| text_fo_i | zpl | 0.10 | 1.0 | Sub-pixel |
| text_fo_n | zpl | 0.10 | 1.0 | Sub-pixel |
| text_fo_r | zpl | 0.10 | 1.0 | Sub-pixel |
| text_ft_auto_pos | zpl | 1.32 | 3.0 | Auto-position cursor |
| text_ft_b | zpl | 0.03 | 1.0 | Sub-pixel |
| text_ft_i | zpl | 0.04 | 1.0 | Sub-pixel |
| text_ft_n | zpl | 0.10 | 1.0 | Sub-pixel |
| text_ft_r | zpl | 0.16 | 1.0 | Sub-pixel |
| text_multiline | zpl | 0.68 | 2.0 | Word-wrap boundaries |
| ups | zpl | 5.74 | 7.0 | MaxiCode + font metrics |
| ups_surepost | zpl | 7.61 | 8.5 | MaxiCode + font metrics |
| usps | zpl | 4.09 | 5.0 | Font metrics |
| tnt_express | zpl | 5.25 | 6.0 | Font metrics + PDF417 |
| royalmail | zpl | 3.33 | 4.0 | QR code + font metrics |
| canadapost | zpl | 4.04 | 5.0 | QR code + PDF417 + font |
| auspost | zpl | 3.93 | 4.5 | QR code + font metrics |
| colissimo | zpl | 3.88 | 5.0 | DataMatrix + font metrics |
| postnl | zpl | 3.67 | 4.5 | QR code + font metrics |
| bpost | zpl | 3.59 | 4.5 | QR code + font metrics |
| correos | zpl | 3.73 | 4.5 | QR code + font metrics |
| dbschenker | zpl | 5.28 | 6.0 | PDF417 + font metrics |
| evri | zpl | 3.06 | 4.0 | QR code + font metrics |
| dpdde | zpl | 5.00 | 5.5 | PDF417 + font metrics |
| ontrac | zpl | 3.53 | 4.5 | QR code + font metrics |
| seur | zpl | 4.70 | 5.5 | PDF417 + font metrics |
| purolator | zpl | 3.78 | 4.5 | DataMatrix + font metrics |
| inpost | zpl | 4.18 | 5.0 | QR code + font metrics |
| yodel | zpl | 3.54 | 4.5 | QR code + font metrics |

## Known Limitations

### MaxiCode (ups, ups_surepost)
MaxiCode is a proprietary 2D symbology used by UPS. The encoder implements
proper GF(64) Reed-Solomon ECC (primary 10+10, secondary 42+20 even/odd tracks)
and greedy Set-A character encoding. Remaining diff (~5-7%) is due to minor
hex module placement differences vs Labelary and font metric differences.

### PDF417 (fedex)
The `pdf417` crate produces **valid, scannable** barcodes, but the specific
codeword arrangement differs from Labelary's encoder. Both are correct per the
ISO 15438 specification; different encoders may choose different text/byte/numeric
compaction modes resulting in visually different (but equivalent) barcodes.

### Aztec (aztec_ec, pnldpd)
The `rxing` crate's Aztec writer produces proper Aztec codes. Minor differences
stem from error correction level defaults and symbol sizing when the ZPL
parameters leave the size open.

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
`~DG` downloads and `^XG` recalls render correctly. The remaining ~9.5 % diff on
`dhlparcelit` is due to font metrics (^A0I text) and label size mismatch
(813×1626 vs 812×1624 reference). The `^XG.GRF` (unnamed recall) does not match
the stored `CMR.GRF` key — both our implementation and Labelary skip it.

## Updating References

To regenerate all Labelary reference images:

```sh
# ZPL labels (Labelary API)
for f in testdata/*.zpl; do
  name=$(basename "$f" .zpl)
  curl -s -X POST http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/ \
    -F "file=@$f" -o "testdata/${name}.png"
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
