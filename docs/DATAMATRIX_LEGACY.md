# Historical Data Matrix ECC 000-140

ZPL BX with omitted/empty quality or explicit 0/000 uses ECC000. Qualities
50, 80, 100 and 140 use their respective convolutional encoders. Explicit
ECC200 and EPL keep their own paths; no Legacy input silently becomes ECC200.

## Scope and byte handling

The raw `barcodes::datamatrix_legacy::encode` API accepts literal bytes for
ECC000; `encode_with_ecc` additionally selects 0, 50, 80, 100 or 140. Both accept
format 1..6 and an optional complete odd square size. The pipeline implements
format encoding, CRC, nine-bit record length, convolution where applicable,
ECC header, zero fill, randomization, placement and finder border.

| ECC | Input bits/cycle | Output bits/cycle | Flush cycles | Minimum side | Maximum format-6 bytes at 49x49 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 000 | 1 | 1 | 0 | 9 | 271 |
| 050 | 3 | 4 | 3 | 11 | 200 |
| 080 | 2 | 3 | 11 | 13 | 176 |
| 100 | 1 | 2 | 15 | 13 | 131 |
| 140 | 1 | 4 | 13 | 17 | 63 |

Convolution excludes the ECC header. A partial final input group is zero-padded,
then the state is flushed before adding the header and filling the data area.
Each call starts with zero state. Capacity errors never truncate the payload.
Legacy dimensions follow the larger requested row/column value; values above
49 are automatic, invalid small/even dimensions and rectangular aspect requests
are rejected. Existing orientation, position and module scaling are reused.

The ZPL parser preserves original field bytes through tokenization, FH decoding,
field resets and stored-format recall. Literal transport CR/LF/TAB are ignored
as before; FH can insert them after tokenization. Legacy consumes these bytes
without additional character-set transcoding, including invalid UTF-8. Formats
1..5 still enforce their byte repertoires. Backslashes and pipes stay literal
after FH, matching the scoped CI13/CI27 printer observations below. Parameter g
is irrelevant for Legacy. The raw encoder never interprets ZPL escapes.
ECC200 has its own [field-data rules](DATAMATRIX_FIELD_DATA.md).

Public API migration: `BarcodeDatamatrixWithData`, `RecalledFieldData` and
`RecalledField` gain `data_bytes: Option<Vec<u8>>`. Struct-literal callers must
initialize it. Parsed Legacy bytes are authoritative: callers replacing `data`
must update or clear `data_bytes`. Manual ASCII fields may use None; manual
non-ASCII Legacy fields require explicit bytes. EPL retains its ECC200 text path.

## Historical norms and source attribution

Development worked directly from ISO/IEC 16022:2000, the FCD draft for the next
edition and GOST R ISO/IEC 16022-2008: Annex H (placement), I (formats), J (CRC),
K (convolution), L (randomization), and Q (intermediate example values).
Standard PDFs are not distributed here. Source SHA-256 identifiers:

- FCD: `c618f525c53d884614237349e0e83a85dd4378bca6ad64982d484bec6242fcaa`
- GOST: `ce038dcde4cd82c8626b9782abb2c379868f7d9bd709dd836cd69c5459b9888d`
- ISO 2000: `3726076a792673241f9dfef2253e6e1dc815573e3fd564cd176638001ba2d317`

The encoder and placement generator are ported from the author's own
[QR Atelier](https://github.com/Streetblock/QR-Atelier), commit
`432aa76de2116adc055062f91ef622f4dc9568cb`, files `libs/DMlegacy.js` and
`libs/DMlegacyPlacementGenerator.js`. Copyright 2026 David Block. This port
uses the MIT option of MIT OR Apache-2.0; see the included
[license](../licenses/QR-Atelier-MIT.txt) and source headers.

The pinned source already includes random byte BC at offset 211 and the ECC100
correction (no delay-12 tap). Rust uses byte slices, BitMatrix and a bounded lazy
cache. Placement uses filtered bit reversal, inverse permutation, cyclic row
shifts and corner swaps. This formula was empirically reconstructed and checked
for all 21 supported sizes; it is not a normative algorithm or a proof beyond
that finite domain. Production generates/caches grids; fixed grids are test-only.

## Evidence and limitations

There is **no independent overall verification of the Legacy encoder**. Our
JavaScript encoder and our own decoder can share interpretation mistakes.
Independent RXing checks in this branch concern ECC200, not Legacy.

Fixed references in `testdata/legacy/` distinguish the evidence:

- `placement-grids.txt`: all 21 grids / 18,389 cells extracted from the FCD PDF
  with two methods sharing that source, supplemented by visual edition checks.
- `ecc050-annex-q-matrix.txt`: the published 13x13 ECC050 / format3 / AB12-X
  example. Its 96 protected bits are also tested; the CRC example is 7559.
- `ecc000-printer-matrices.txt`: three recorded 200/300/400-digit matrices,
  sides 31/35/41. The 200-digit case came from a Zebra photo; the older 300/400
  records do not independently identify their encoder.
- `ecc000-js-port-vectors.txt`: 12 same-author JS outputs, six formats and
  automatic/49-module sizes. These are port regressions, not independent evidence.
- `convolution-js-stages.txt`: 200 same-source stage vectors, lengths 0..49 for
  each quality. Input bit i is 1 when `(i*i + 3*i + 7) % 11 < 5`.
- `convolution-js-matrices.txt`: 48 same-source outputs, four qualities and six
  formats, automatic/49-module sizes; format6 includes 00/80/FF/96/01 bytes.
- `ci13-ci27-photo-grids.txt`: 12 sampled 23x23 grids / 6,348 modules from the
  field probes. The optional `extract-photo-grids.mjs` copies measured rows from
  observation JSON, never calls an encoder. Geometry, thresholds and photo hashes
  are preserved there; these are scoped comparisons, not independent decoding.

Fixtures are frozen and read-only in tests. Stage rows use
`quality|input_bits|protected_bits`; convolution matrix headers use
`quality|format|side|payload_hex` followed by binary rows. JS vectors were produced
from the pinned source exports `encodeLegacyEcc050/080/100/140` and
`generateLegacyDataMatrix`, not regenerated by Rust tests.

## Printer observations

The recorded ZD421 at 300 dpi, firmware V93.21.17Z, prints Legacy symbols.
ECC000/format6 tests with CI13 and CI27 preserve backslash-ampersand, doubled
backslashes and doubled pipes literally. FH 0D/0A produces CR/LF. Each of the
12 sampled matrices agrees with its known-byte candidate at all 529 modules.
The BX prose suggests PDF417-like substitutions, but applying those substitutions
contradicted these observations. Other ECC levels/firmware are not hardware-verified
by this experiment; cross-quality tests establish implementation consistency only.

For repeated format1 digit 1, 500 digits print with AUTO and requested 49x49.
The supplied N500/N501 photo shows INVALID-L in both N501 fields. The contributor
reports the same rejection for 511, 512, 596 and 597. Rejected fields print no
DataMatrix; with CV active the printer emits INVALID-L. These observations do
not establish a universal 500-character cap. Model/firmware context was retained
from the experiment, not freshly queried for each photo. Reproducible ZPL and
scoped results are in [the printer study](../examples/legacy-printer-study/README.md).

The generic encoder accepts fitting records through 511 and rejects longer
records explicitly. FCD section 6.5.3 (printed p24 / PDF p32) specifies a nine-bit
character count, but Table G.1 (printed p68 / PDF p76) lists 560/596 numeric
characters at sides 47/49. Both pages were visually checked. Zebra also lists
596. No extended record was produced by this printer experiment, so wrapping
the count, adding a tenth bit or claiming full 596 support is not justified.
All other 29 Zebra maximum-field table cells are tested at and above capacity.

Device-specific field suppression and CV diagnostic rendering are separate
follow-ups. Labelize returns rendering errors for unsupported input. The 501..511
generic behavior deliberately differs from the tested device's restriction.

ISO/IEC 16022:2024 removed Legacy ECC000-140, but existing printer commands still
use it. This is compatibility work, not a recommendation for new labels or a
claim about every modern Zebra device.

- [Zebra BX](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html)
- [ISO/IEC 16022:2024](https://www.iso.org/standard/80926.html)
- [2024 foreword preview](https://gso-sims-preview-doc-aws.s3-eu-west-1.amazonaws.com/iso-iec-16022-2024-en.html)
