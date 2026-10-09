# QR mask selection: exact N4 balance penalty

`qrcode 0.14.1` evaluates N4 (dark/light balance) with an approximately linear
score. ISO/IEC 18004:2015 section 7.8.3.1, Table 11 requires ten points for each
complete five-percentage-point deviation from 50%. The approximation can change
the winning mask, even when segmentation, version, correction level and
codewords are identical.

## Reproduced counterexamples

The following inputs use one Byte segment and no ECI. The H case requires
version 2; the other four fit version 1. The automatic Labelize API is tested;
these lowercase payloads are encoded as Byte by the existing optimizer.
Mask numbers are zero-based. This PR now retains main's automatic segmentation
and has no dependency on the reverted explicit-character-mode PR #52.

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

## Implementation and scope

Labelize continues using the dependency's public `Bits`, Reed-Solomon codeword
construction and `Canvas` mask application. It now evaluates the eight completed
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

## Independent references and verification

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
## Original corpus check (2026-09-10)

Both complete diff reports were regenerated after the fix: 51 label cases and
76 unit cases, with no skipped/error cases. All 127 comparison images and both
reports are byte-for-byte unchanged from the character-mode branch. Thus this
fix neither reduces nor increases the current corpus's Labelary differences.
The five original independent counterexamples exercise inputs absent from that corpus.

## Labelary and printer study, 2026-10-06

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

The branch is updated against main `6465f51`, preserving the PR52 revert. Only
automatic mask selection changes; QR parser, character-mode interpretation,
segmentation and error correction selection remain main's behavior. The
explicit-mode work can be considered separately for a future Zebra profile.

Current validation against main `6465f51`: 127 library/QR/barcode/property
regressions passed, as did 130 goldens and both diff reports (134 fixtures,
no HIGH/SKIP/ERR). The new five-symbol golden is pixel-identical to Labelary;
all existing comparison images and scores remain unchanged relative to main.
Formatting and all-feature Clippy with warnings denied also passed.

## Payload boundaries (`^FD` parsing)

Mask selection presupposes the right codewords: a second, independent
mismatch source is the `^BQ` payload itself. Empirically (52 payload pairs
decoded from Labelary symbols), Labelary consumes the `^FD` ECC char and the
input-mode designator, then **one separator char only when the designator is a
letter** (the comma in `QA,data`), and **nothing more when it is a digit**:
`^FD9876543210` encodes `76543210`. Manual mode additionally consumes its
submode char and, for `B`, a four-digit byte count. Pipes are field separators
only for the recognized manual modes and the explicit automatic designator —
kept verbatim otherwise. `get_input_data` previously dropped three characters
unconditionally, mangling every digit-leading payload (`9876543210` encoded as
`6543210`).

`examples/qr_mask_probe.rs` measures this: it renders a minimal one-QR ZPL
locally and against Labelary, compares the module grids, reads each side's
mask id and EC level from the (BCH-validated) format information, and decodes
both payloads with the independent rxing decoder. `examples/qr_penalty_fit.rs`
fits candidate penalty functions against measured (payload, EC, mask) triples.
On 29 same-codeword symbol pairs the standard penalty reproduces Labelary's
mask 29/29; across 68 probed payload classes the two fixes together make 49
symbol-perfect.

## Error-correction re-optimization (accepted divergence)

For unrecognized or explicitly overridden EC designators Labelary
re-optimizes the level and sometimes the segmentation (JSON payloads -> M and a
smaller version; explicit M -> Q; explicit L -> H). Emulating that
non-standard behaviour was evaluated and rejected in the #52 review (reverted
in d6fe62c; re-measured 2026-10-06: keeping strict modes costs +4.29 pp across
11 labels +3.32 pp in unit on the then-current corpus while buying parity only
where none is needed). The divergence is documented, not matched.
