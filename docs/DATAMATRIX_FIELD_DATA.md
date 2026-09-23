# ZPL DataMatrix ECC 200 field data

The `^BX` renderer evaluates field-data escapes only for quality 200. The default
escape is `_`, matching the newer firmware behavior described in Zebra's
[^BX reference](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html).
An explicit `g` parameter overrides it. Only that character is active; `_1` and
`~1` are not both interpreted automatically. The old firmware's separate literal
tilde rules are not emulated. Labelize's existing command splitter also treats
raw tilde as a command prefix. Use, for example, `^CT!` when testing an explicit
tilde escape in raw ZPL; context-sensitive command splitting is not changed here.

Processing order is the parser's existing `^FH` handling, followed by DataMatrix
escape parsing. Literal/control/decimal bytes are distinct from function
codewords. `_1` emits codeword 232; `_d029` emits a data byte with value 29.
Only a leading FNC1 establishes GS1 DataMatrix. FNC1 inside the data becomes a
separator when decoded; an ordinary GS byte does not establish GS1.

Supported escape tokens are a doubled escape, control shifts, `dNNN` bytes,
PAD (`0`), FNC1 (`1`), structured-append FNC2 (`2` plus three three-digit values),
FNC3 (`3`), and ECI (`5NNN`). Malformed or unsupported escapes return errors.
The direct `barcodes::datamatrix::encode` API remains literal; `encode_zpl`
accepts byte slices with ZPL escapes. EPL disables this escape processing.

Byte-only payloads retain the existing automatic encodation. Payloads containing
function codewords use ASCII encodation with numeric-pair compaction and upper
shift. The existing DataMatrix library supplies capacity selection, Reed-Solomon
ECC, and placement. This path prioritizes correct function-codeword positions;
it does not optimize C40/Text/Base256 segments around functions and may produce
larger symbols. Existing dimension selection is unchanged.

Validation includes exact byte/token/codeword tests and an independent RXing
matrix decoder. Tests assert both decoded content and the GS1 symbology modifier
(`]d2`), plus internal FNC1, literal GS, custom escapes, `^FH`-generated controls,
upper-shift bytes, padding, and EPL isolation. These checks do not use rendered
images as replacement reference data.

This ECC 200 path is separate from the ECC 000-140 implementation documented in
`DATAMATRIX_LEGACY.md`; Legacy fields never pass through these substitutions.
It does not redesign the ECC 200 string-based ZPL input/charset pipeline: literal
text follows its existing UTF-8 behavior, and non-UTF-8 high-byte `^FH` sequences
remain a separate parser limitation. Decimal escapes (`_d000` through `_d255`)
and the byte-slice `encode_zpl` API preserve all 256 byte values.
