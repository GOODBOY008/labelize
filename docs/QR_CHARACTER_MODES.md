# Explicit QR character modes

ZPL manual input (`^FD<level>M,<mode><data>`) now reaches the encoder as a single
explicit segment. Previously the renderer discarded the parsed mode and always
used automatic segmentation. For example, `QM,B002012345678901234567890` produced
a Numeric segment; it now produces a Byte segment with the same 20 payload bytes.

| Input mode | Behavior |
|---|---|
| Automatic (`A,`) | Existing automatic encoder, version selection, and mask selection |
| Manual `N` | One Numeric segment; ASCII digits 0-9 only |
| Manual `A` | One Alphanumeric segment; 0-9, A-Z, space, `$%*+-./:` |
| Manual `Bdddd` | One Byte segment; existing parsed UTF-8 bytes and byte-count handling |
| Manual `K` | Explicit error: Shift-JIS/Kanji encoding is not implemented in this Rust path |
| Unknown manual indicator | Explicit error instead of falling back to automatic |

The existing `qrcode::encode(content, magnification, ec)` API remains automatic.
`qrcode::encode_with_mode(content, magnification, ec, mode)` adds explicit mode
selection without changing the existing API. Standard QR versions 1 through 40
are tried in order while retaining the selected mode. Invalid or oversized data
returns an error; there is no automatic-mode fallback. Input is validated before
calling the low-level encoder, whose Numeric/Alphanumeric methods assume valid
characters and can otherwise panic or silently substitute values.

This change retains existing non-binary `|` stripping and binary length handling,
including clamping a declared length larger than the available content and
rejecting a length that splits a UTF-8 character. It does not add a raw-byte API,
JIS character-set conversion, QR Model 1, explicit mask selection, or Mixed Mode.
The JavaScript toolkit has additional Kanji/Mixed Mode support that is not ported
in this focused change.

EPL's existing `b ... Q` parser synthesizes a manual Byte header to preserve its
payload (including `|`). The shared renderer now honors that mode as well, so its
module pattern can change while the payload remains intact. A decoded-byte test
covers this path.

## Specification and toolkit reference

- [Zebra ^BQ programming reference](https://docs.zebra.com/content/tcm/us/en/printers/software/zpl-pg/zpl-commands/%5Ebq.html)
  defines explicit manual character modes and their character sets.
- Our toolkit at [096b5d42](https://github.com/Streetblock/zpl-toolkit/tree/096b5d42fc6ed50ba49b8e3a2fb6ab98171b1d7a)
  already implements the intended approach in `src/barcodes/qr-zpl.js` and
  `src/barcodes/qr-core.js`: preserve the selected mode, validate the character
  set, and choose a fitting version for that segment. It was inspected and run
  as a reference; no toolkit files were changed. Its nine QR field-data/roundtrip
  tests passed during this investigation.

## Labelary limitation observed September 10, 2026

Live requests used 8 dpmm and `4.005x8.01` inches (813 x 1626 pixels), with only
`^FO30,50^BQN,2,4` and the following field data. Independent QR decoders read the
mode indicator from the actual modules, not just the decoded text:

| Field data | Labelary segment | Toolkit segment | Labelize after fix |
|---|---|---|---|
| `QM,B002012345678901234567890` | Numeric (0001) | Byte (0100) | Byte (0100) |
| `QM,A12345678901234567890` | Numeric (0001) | Alphanumeric (0010) | Alphanumeric (0010) |
| `QM,N12345678901234567890` | Numeric (0001) | Numeric (0001) | Numeric (0001) |

All three decode to the same text. Labelary's optimization of the first two
requests therefore cannot be used as the authority for explicit segment choice.
The original ZPL/PNG observations are retained separately in
`testdata/qr-mode-probes/`; they are not substituted for the corrected renderer's
expected manual-mode behavior. The test records that discrepancy explicitly.
No physical printer was used in this investigation.

The regular `qr_manual_{byte,numeric,alphanumeric}` golden fixtures use
`QM,B0009lowercase`, `QM,N12345678901234567890`, and `QM,AAC-42`, respectively.
For these inputs Labelary chooses the documented mode. Their PNGs were downloaded
directly from Labelary without a local fallback. Existing reference PNGs were not
regenerated. Older fixtures containing manual QR data can have changed QR sizes
or patterns because their explicit mode is now honored, while Labelary optimized
it; the accompanying diff reports show the effect.

## Verification

`unit_qr_modes` decodes generated modules using the separate `rxing` decoder and
checks the mode nibble in recovered data codewords, decoded text, and Byte
segments. It covers explicit digit payloads, all error correction levels, UTF-8
bytes, allowed characters, invalid inputs, oversized data, version >= 10 count
fields, and the EPL path. Automatic codewords and correction levels are compared against the existing
automatic encoder; magnification and quiet zones are checked separately so the
subsequent N4 mask correction can change the selected matrix.

These segment checks are the functional regression tests. Golden pixel comparisons
are supplemental: valid QR masks and optimization choices can produce different
patterns even when decoded payloads agree. The UTF-8 panic regressions from #47
remain enabled; this branch builds on that fix.

The reviewed diff reports contain 127 cases (51 labels, 76 unit fixtures), with
no skipped or errored cases. New Byte/Numeric fixtures are pixel-identical to
Labelary; Alphanumeric differs by 0.14%. Existing `postnl_qr` changes from 0.00%
to 0.68%, `qr_code_ft_manual` from 0.29% to 2.79%, and the EPL showcase from 0.00%
to 0.19%. The manual `^FT` symbol grows while retaining its baseline. These
changes are confined to QR mode selection; existing references and numeric test
tolerances are unchanged.

The old 1% full-image assertion for `qr_code_ft_manual` cannot describe both
Labelary's optimized Numeric symbol and the requested Byte symbol. Its golden
test now decodes the Byte/H payload at the fixed 29-module, 10-dot scale and
`^FT` position, verifies every module pixel, and compares the surrounding
pixels with the unchanged reference at the original 1% limit. It does not relax
the threshold or rely on the pre-#31 8% unit-tolerance override.
### Decoded format details

A follow-up inspection with the separate ZXing JavaScript decoder explains the
pixel differences more precisely. The same saved images were used; no reference
was regenerated.

| Fixture | Labelary mode / EC / mask / modules | Labelize mode / EC / mask / modules |
|---|---|---|
| `qr_manual_alphanumeric` (`AC-42`) | Alphanumeric / H / 4 / 21x21 | Alphanumeric / Q / 3 / 21x21 |
| `qr_manual_byte` (`lowercase`) | Byte / Q / 7 / 21x21 | Byte / Q / 7 / 21x21 |
| `qr_manual_numeric` (20 digits) | Numeric / Q / 0 / 21x21 | Numeric / Q / 0 / 21x21 |
| `qr_code_ft_manual` (20 digits) | Numeric / H / 2 / 25x25 | Byte / H / 2 / 29x29 |

All decoded payloads agree. For the Byte/Numeric controls, unmasked codewords
also agree exactly. For Alphanumeric, Labelary emits H although the fixture
requests Q. Thus its difference is not merely mask selection: the correction
level and codewords differ too. Raising correction without enlarging the symbol
is a possible explanation, but Labelary's internal reason was not verified.

The 0.1404% is 1,856 changed pixels divided by the entire 813x1626 label. Within
the 84x84 QR square, 26.30% of pixels differ. The manual Byte fixture has larger
10-dot modules and a different symbol size (290x290 instead of 250x250 pixels),
so its 36,872 changed label pixels produce 2.7892%. That whole-image number also
includes the pre-existing one-dot horizontal positioning difference. Neither
percentage measures decoded-data correctness or scanner reliability.

A subsequent [N4 mask-selection fix](QR_MASK_SELECTION.md) preserves these
encoding modes and codewords, but may choose a different mask. The observations
above describe the original character-mode PR before that follow-up.
