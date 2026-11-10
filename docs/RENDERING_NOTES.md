# Rendering behavior and evidence

[Documentation index](README.md)

Focused semantics and reference measurements that complement the
[ZPL command matrix](ZPL_COMMANDS_REFERENCE.md) and
[diff thresholds](DIFF_THRESHOLDS.md). Dated measurements describe the named
renderer or device; they are not universal printer conformance claims.

- [Print mirror](#print-mirror): persistent state, transforms and geometry.
- [Field parameters](#field-parameters): text direction, character spacing and
  measured Labelary compatibility.
- [QR mask selection](#qr-mask-selection): scoring, independent matrices and
  observed firmware differences.

DataMatrix has a separate [reference](DATAMATRIX.md); TSPL calibration and
open questions are in [TSPL baselines](TSPL_BASELINES.md).

## Field parameters

`^FPd,g` configures the current text field. `H` places upright characters
left to right, `R` places them right to left, and `V` stacks them vertically
without rotating the individual characters. The field's font orientation
still rotates the resulting arrangement. This is distinct from `^FR`, which
reverses field colors.

The gap `g` is an additional character spacing in printer dots; `^MU` does
not rescale it. Each `^FP` replaces both parameters: omitted or invalid
direction selects `H`, and an omitted gap selects zero. `^FS` restores the
default horizontal direction and zero gap for the next field.

### Independent Labelary measurements (November 6, 2026)

Synthetic `ABCDE` fields were rendered through the public Labelary API at
8 dots/mm, using scalable font 0 (`^A0N,40,32`) and bitmap font A
(`^AAN,36,20`). Visible ink widths were:

| Gap command | Font 0 | Font A | Change from zero gap |
| --- | ---: | ---: | ---: |
| `^FPH,0` | 86 | 115 | 0 |
| `^FPH,5` | 106 | 135 | +20 |
| `^FPH,15` | 146 | 175 | +60 |
| `^FPH,30` | 206 | 235 | +120 |
| `^FPH,-5` | 66 | 95 | -20 |

Five characters have four between-character spaces: the measured width
change is exactly `4 * g`, without stretching the glyphs. `^FPR,10` likewise
adds ten dots between upright characters in reverse order.

Font A's standard matrix is 9 dots high by 5 body dots wide, with a separate
one-dot natural gap. Width selects magnification from the five-dot body;
the resolved advance includes the gap. For example, `^AAN,36,20` uses a
fourfold horizontal magnification and a 24-dot advance, before adding `g`.
Using the six-dot advance as the width divisor incorrectly selects threefold
magnification and compresses the line. A Labelary width sweep at 5, 7, 8,
10, 15 and 20 dots confirmed advances of 6, 6, 12, 12, 18 and 24 dots.

For **both tested fonts**, `^FPV,0`, `^FPV,5`, `^FPV,10` and `^FPV,30`
produced pixel-identical PNGs. Labelize deliberately reproduces this
Labelary behavior: the gap is ignored for vertical glyph placement. Vertical
placement uses the font's cell height as the character pitch. With `^FB`,
the gap still affects wrapping, so different gaps can produce different
blocks even though the pitch within each vertical run is unchanged.

Labelary accepts negative gaps down to -10 (more-negative values clamp to
-10); the Zebra guide documents a nonnegative range instead. Very large
positive gaps can also be discarded by Labelary based on the selected font
width. These are compatibility observations, **not verified Zebra printer
behavior**. Physical-printer tests remain outstanding.

The independent full-page reference is
[`testdata/unit/field_parameters.zpl`](../testdata/unit/field_parameters.zpl)
and its PNG; the adjacent provenance JSON records the endpoint, source hash
and original reference hash. Tests additionally check individual field
geometry and spacing so sparse whole-page tolerances cannot hide an ignored
command. Existing substitute-font shape differences remain visible in the
comparison artifacts.

The full-page golden has a 3.0% ceiling: correcting Font A's width
magnification reduced the comparison from 2.7771% to 2.4873%, including
1.1299% in the unchanged annotation fields alone. The
unmodified `db25929` renderer, which ignores `^FP`, measured 3.8829% against
the same source and reference and fails this ceiling. Isolated geometry
checks separately require the exact requested per-character displacement;
font-outline tolerance cannot excuse an ignored or incorrectly scaled gap.
Every horizontal and reverse glyph origin must also stay within three dots
of the independent capture, allowing substitute bearings without accepting
an incorrect natural advance. Both raster and vector output run these checks.

The [additional frozen captures](../testdata/field-parameters/provenance.json)
pin rotated origins, explicit alignment and field-block interactions. For
centered blocks, Labelary includes a trailing word separator in alignment
measurement only when it fits the block width. Acceptance checks compare
spacing-induced shifts against the zero-gap control, preserving the existing
substitute-font baseline while rejecting new alignment errors.

Sources: [Zebra Programming Guide](https://cpws.zebra.com/cpws/docs/zpl/zpl-zbi2-pm-en.pdf),
[Zebra font matrices](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-font-barcodes-fonts-andbar-codes/r-zpl-font-barcodes-font-matrices.html),
[Labelary command reference](https://labelary.com/reference.html), and the
captured independent images. Unicode combining sequences and complex field
interactions must not be inferred to match physical printer firmware from
these synthetic measurements.

## Print mirror

`^PMY` mirrors the whole label horizontally. The setting persists across
`^XA`/`^XZ` and subsequent calls to the same `ZplParser`. `^PMN` disables it;
constructing a new parser starts with mirroring disabled. Missing or invalid
parameters leave the current state unchanged. The last valid setting in a
format applies to every field and every `^PQ` copy of that label.

The completed image is mirrored after print-width clipping/centering and `^PO`
rotation. `DrawerOptions::enable_inverted_labels` controls `^PO` only; disabling
that option does not disable `^PM`.

Downloaded formats store an optional explicit mirror instruction. Defining a
format does not change the printer's mirror setting. Recalling a format replays
its instruction, if present; a format without `^PM` inherits the printer's state
at recall time. A later `^PMN` in the calling format takes precedence.

### Sources and comparison (September 10, 2026)

- The authoritative [Zebra Programming Guide, ^PM section](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/software/zpl-zbi2-pg-en.pdf)
  specifies horizontal reflection, persistence until explicit disable/power-off,
  and ignoring missing/invalid parameters.
- [Labelary lists ^PM as supported](https://labelary.com/docs.html). Live requests
  confirmed persistence, reset, ignored invalid/missing parameters, whole-label
  application, print-width/rotation composition, and stored-format behavior.
- The inspected [ZPLR interpreter at 590f03c](https://github.com/le2ni/zplr/blob/590f03c58c79663be37708cd97f8cff5eb0d797e/src/core/interpreter.ts)
  initializes mirror state per label and uses `yesNo(args[0], true)` for `^PM`.
  Its [raster transform](https://github.com/le2ni/zplr/blob/590f03c58c79663be37708cd97f8cff5eb0d797e/src/core/raster.ts)
  implements horizontal reflection composed with 180-degree rotation. Its
  parameter/state handling is not used as the authority for this implementation.

The Labelary HTTP endpoint also ignored a `^PMY` placed before the first `^XA`.
That observation does not establish physical-printer behavior outside formats.
Like other existing printer settings in Labelize, `^PM` is interpreted wherever
the command parser receives it; the conformance cases below use complete formats.

### Independent geometry measurements

Requests used `POST https://api.labelary.com/v1/printers/8dpmm/labels/1x0.5/0/`
with `Accept: image/png` and the synthetic field
`^FO10,20^GB20,10,10^FS` inside `^XA`/`^XZ`. The returned canvas was 203 x 101
dots. Coordinates below are inclusive black-pixel bounds.

| Settings | Left, top, right, bottom |
|---|---|
| Default | 10, 20, 29, 29 |
| `^PMY` | 173, 20, 192, 29 |
| `^PW100` | 61, 20, 80, 29 |
| `^PW100^PMY` | 122, 20, 141, 29 |
| `^PW100^POI` | 122, 71, 141, 80 |
| `^PW100^PMY^POI` | 61, 71, 80, 80 |
| `^PW300^PMY` | 173, 20, 192, 29 |

The odd 103-dot margin for `^PW100` distinguishes mirroring the completed canvas
from mirroring the narrower printable region before centering.

### Regression fixtures

`testdata/unit/print_mirror{,_width,_inverted}.zpl` have independent PNG references
downloaded from Labelary at `8dpmm/labels/4.005x8.01/0/` (813 x 1626 pixels).
These contain only asymmetric filled graphics, including print-width clipping,
so font rasterization does not require a tolerance. `unit_print_mirror` compares
all three decoded images pixel-for-pixel; the normal golden suite also runs them.
Reference generation has no local-renderer fallback in this investigation.

Other tests cover persistent state, separate parser instances, invalid inputs,
stored formats containing only settings, rotated text, QR codes, clipping,
antialiasing, and the inversion option. No physical printer was used.

### Library data model

`LabelInfo::mirrored` is the resolved per-label snapshot. Code constructing
`LabelInfo` with a struct literal must supply `mirrored: false` for normal output.
`StoredFormat::mirrored` and `RecalledFormat::mirrored` are `Option<bool>`:
`None` inherits the current setting, while `Some(false)` explicitly disables it.
EPL labels initialize the flag to false.

## QR mask selection

`qrcode 0.14.1` evaluates N4 (dark/light balance) with an approximately linear
score. ISO/IEC 18004:2015 section 7.8.3.1, Table 11 requires ten points for each
complete five-percentage-point deviation from 50%. The approximation can change
the winning mask, even when segmentation, version, correction level and
codewords are identical.

### Reproduced counterexamples

The following inputs use one Byte segment and no ECI. The H case requires
version 2; the other four fit version 1. The automatic Labelize API is tested;
these lowercase payloads are encoded as Byte by the existing optimizer.
Mask numbers are zero-based. Automatic segmentation is unchanged.

| Payload | EC | Previous mask | Corrected mask | Standard score, previous -> corrected |
|---|---|---|---|---|
| `mask-24` | L | 7 | 1 | 1041 -> 1037 |
| `mask-37` | M | 0 | 7 | 1035 -> 1034 |
| `mask-46` | Q | 2 | 3 | 1041 -> 1039 |
| `mask-64` | L | 1 | 0 | 1019 -> 1017 |
| `mask-907` | H | 7 | 2 | 1190 -> 1184 |

A ZPL equivalent of the first case is
`^XA^FO20,20^BQN,2,4^FDLM,B0007mask-24^FS^XZ`.

The first regression failed against the unmodified renderer: `mask-24 L
Automatic: must select standard mask 1`. A valid, decodable QR symbol alone
does not establish correct mask selection.

### Implementation and scope

Labelize continues using the dependency's public `Bits`, Reed-Solomon codeword
construction and `Canvas` mask application. It evaluates the eight completed
candidate matrices itself, including format information. Lowest score wins;
ties choose the lowest mask number. This avoids vendoring the dependency or
modifying the local Cargo registry.

N4 uses integer arithmetic: `abs(2 * dark - total) * 10 / total * 10`.
N1-N3 retain the existing evaluation: runs, overlapping 2x2 blocks, and one
penalty per finder-like core with a light area before or after (including the
quiet zone). The dependency subtracts a mask-independent 720 points from N3;
our totals retain those points, which cannot change candidate order. Scores use
`u32` so even an artificial all-dark version-40 matrix cannot overflow.

This fixes automatic mask evaluation for the existing Model 2 path. It does not
implement the ZPL `^BQ` explicit mask parameter, Micro QR, Model 1, or change
segment selection, correction levels, padding, or symbol sizes. It is not a
certification of every aspect of QR compliance or physical print quality.

### Independent references and verification

The five text fixtures in `testdata/qr-mask/` were generated on September 10,
2026 from the existing JavaScript toolkit at `096b5d42fc6ed50ba49b8e3a2fb6ab98171b1d7a`.
Its `QrCore` generated all eight forced-mask matrices using Byte mode, no ECI,
and the specified EC level. `calculateQrMaskPenalty` evaluated each candidate.
Each fixture stores payload, EC, winning mask, all eight standard scores, and
the winning matrix. No Rust renderer output was used to generate these references.
The toolkit itself is unchanged.

Tests compare all 40 Rust candidate scores to the recorded independent scores,
then compare complete output matrices for automatic input.
The independent rxing decoder checks Byte mode and payload bytes. Additional
tests cover N4 boundaries, symmetry and large scores. Existing automatic-mode
regressions preserve decoded data codewords, correction level, dimensions,
quiet zones and magnification while allowing the intended mask changes.

Existing Labelary PNGs remain unchanged. Pixel agreement with Labelary is a
separate measurement: a changed mask can increase or decrease image differences.
Different error correction levels or character modes must not be mistaken for
a mask-scoring defect. N3 deliberately retains the dependency's interpretation;
this change does not claim to resolve every ambiguity in ISO mask evaluation.

Sources:
- [ISO/IEC 18004:2015, section 7.8.3.1, Table 11](https://nuintun.github.io/qrcode/spec/ISO-IEC-18004-2015.pdf)
- [qrcode-rust canvas implementation](https://docs.rs/qrcode/0.14.1/src/qrcode/canvas.rs.html)
- Toolkit: `src/barcodes/qr-core.js`, `src/barcodes/qr-mask-penalty.js`
### Labelary and printer study, 2026-10-06

Five new lowercase, seven-byte EC-H inputs isolate mask selection: their version,
Byte segment, error correction, and unmasked data codewords are identical across
the before/after encoders and independently decoded Labelary references.

| Payload | Previous mask | Corrected / Labelary mask | Matrix difference before -> after |
| --- | ---: | ---: | --- |
| `abcabaa` | 5 | 1 | 20.4082% -> 0% |
| `abcucaa` | 3 | 5 | 30.8390% -> 0% |
| `abccdaa` | 3 | 2 | 24.0363% -> 0% |
| `abcndaa` | 6 | 4 | 26.3039% -> 0% |
| `abcaeaa` | 5 | 6 | 13.1519% -> 0% |

These are barcode-module differences, not percentages diluted by white label
space. `testdata/unit/qr_mask_selection.zpl` includes all five fields and uses
an independently fetched Labelary PNG with a strict 0.0% golden tolerance.
Reference endpoint: `https://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/`.
Original PNG SHA-256:
`865fff0a526ead419ef512e7b8afeea69699a1441c87003795011a81717169ab`.

A physical native `^BQ` print of `abcabaa` on a ZD421 (300 dpi, V93.21.17Z)
uses mask 2, whereas Labelary and this correction use mask 1. The native photo
was independently decoded with the same payload and data codewords. This is
evidence of a separate firmware-mask-selection difference; the correction
improves Labelary compatibility on the five verified cases and does not claim
universal Zebra matrix identity.
