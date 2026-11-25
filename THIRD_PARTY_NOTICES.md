# Source attribution

## Labelize Studio printer templates

The expression syntax, field encodings and validation rules in `src/template/`
follow Labelize Studio's template format, copyright © 2025 gongzhongqiang,
distributed under the [MIT license](LICENSE). The shared fixtures include
Studio exports and the independently captured native printer jobs.
[Studio source](https://github.com/GOODBOY008/labelize-studio) · [Fixture provenance](testdata/templates/README.md)

## Legacy Data Matrix ECC 000-140

`src/barcodes/datamatrix_legacy.rs` is adapted from David Block's QR Atelier
Legacy encoder and placement generator, commit
`432aa76de2116adc055062f91ef622f4dc9568cb`:
https://github.com/Streetblock/QR-Atelier/tree/432aa76de2116adc055062f91ef622f4dc9568cb

Copyright 2026 David Block. The original implementation offers MIT OR
Apache-2.0; this Rust adaptation uses the MIT option. The full MIT notice is
retained in `licenses/QR-Atelier-MIT.txt`, and the source file identifies its
origin and adaptation. This attribution applies to the listed implementation,
not ownership of the underlying standard, mathematical rules or reference data.

See [Legacy DataMatrix](docs/DATAMATRIX.md#legacy-ecc-000-140) for the historical norm sources and the distinction
between norm-derived tests, recorded printer examples and same-author port tests.
The implementation has not received independent overall verification.

## Additional barcode encoders

Labelize uses the MIT-licensed [anyd encoders](https://github.com/KarpelesLab/anydcode) and [BWIPP's Rust port](https://github.com/erdzan12/bwipp-rs) for linear, stacked and composite barcodes. The Code 49 numeric compaction and symbol checks follow the [Zint reference implementation](https://github.com/zint/zint/blob/master/backend/code49.c). The framing for TCIF Linked Code 39 follows the [published symbology](https://patents.google.com/patent/US20010045461A1/en). The existing Zint BSD notice is in [Third-party licenses](THIRD_PARTY_LICENSES.md).
