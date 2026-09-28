# QR mask selection: exact N4 balance penalty

`qrcode 0.14.1` evaluates N4 (dark/light balance) with an approximately linear
score. ISO/IEC 18004:2015 section 7.8.3.1, Table 11 requires ten points for each
complete five-percentage-point deviation from 50%. The approximation can change
the winning mask, even when segmentation, version, correction level and
codewords are identical.

## Reproduced counterexamples

The following inputs use one Byte segment and no ECI. The H case requires
version 2; the other four fit version 1. Both the automatic
and explicit Byte Labelize APIs are tested, since these lowercase payloads are
also encoded as Byte by the existing optimizer. Mask numbers are zero-based.

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
then compare complete output matrices for automatic and manual Byte input.
The independent rxing decoder checks Byte mode and payload bytes. Additional
tests cover N4 boundaries, symmetry and large scores. Existing automatic-mode
regressions preserve decoded data codewords, correction level, dimensions,
quiet zones and magnification while allowing the intended mask changes.

Existing Labelary PNGs remain unchanged. Pixel agreement with Labelary is a
separate measurement: a standards-correct mask can increase or decrease image
differences. Different error correction levels or character modes must not be
mistaken for a mask-scoring defect (see [QR character modes](QR_CHARACTER_MODES.md)).

Sources:
- [ISO/IEC 18004:2015, section 7.8.3.1, Table 11](https://nuintun.github.io/qrcode/spec/ISO-IEC-18004-2015.pdf)
- [qrcode-rust canvas implementation](https://docs.rs/qrcode/0.14.1/src/qrcode/canvas.rs.html)
- Toolkit: `src/barcodes/qr-core.js`, `src/barcodes/qr-mask-penalty.js`
## Effect on the existing Labelary corpus

Both complete diff reports were regenerated after the fix: 51 label cases and
76 unit cases, with no skipped/error cases. All 127 comparison images and both
reports are byte-for-byte unchanged from the character-mode branch. Thus this
fix neither reduces nor increases the current corpus's Labelary differences.
The five new independent counterexamples exercise inputs absent from that corpus.