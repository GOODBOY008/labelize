# Languages and examples

Labelize parses five printer languages into the same renderer. Support means
a documented subset of each language, not complete printer firmware emulation.
Hardware commands such as feed, cut, heat and communications may have no
visible effect. A successful preview does not certify a printer's output.

## ZPL

Zebra labels usually start with `^XA` and end with `^XZ`. Most positions use
printer dots. Start with the annotated [first label](GETTING_STARTED.md),
then use the [ZPL command reference](ZPL_COMMANDS_REFERENCE.md) for support
and parameter details. Set the canvas explicitly: `^LL` is not parsed.

## EPL

EPL uses line-oriented commands: `N` clears the image buffer and `P1` prints.
This example places text at (40, 40) dots and a box around it.
Use a **100 × 60 mm** canvas at **8 dpmm**.

```epl playground width=100 height=60 dpmm=8
N
A40,40,0,4,1,1,N,"Hello, EPL"
X30,30,2,420,100
P1
```

Resident font numbers have fixed metrics; they are not ZPL font names.
Inside quoted text (`A`) and barcode data (`B` or `b`), use `\"` for a
literal double quote and `\\` for a literal backslash. For example,
`"A\"B\\C"` prints `A"B\C`. Upload binary `GW` graphics as bytes.

## TSPL / TSC

TSPL mixes physical dimensions (`SIZE`) and dot coordinates (`TEXT`, `BAR`).
This example declares 100 × 60 mm. Native APIs can use that declared size;
the playground's dimensions override it.

```tspl playground width=100 height=60 dpmm=8
SIZE 100 mm,60 mm
CLS
TEXT 40,40,"3",0,1,1,"Hello, TSPL"
BAR 40,90,400,3
PRINT 1,1
```

For Simplified Chinese text, select `CODEPAGE 936` for GBK input or
`CODEPAGE UTF-8` for UTF-8. For example, `TEXT 20,20,"TSS24.BF2",0,1,1,"保温杯"`
uses 24-dot cells. The setting applies to subsequent text fields across
`CLS` and `PRINT` in the same job. A malformed sequence in the selected
encoding produces a parse error. Send the original file bytes when a job
also contains binary graphics. [TSC programming manual](https://fs.tscprinters.com/system/files/31-0000001-00_tspl_tspl2_programming_3_0.pdf)

`BARCODE` accepts Code 128/39, EAN-13, EAN-8 (`EAN8`), UPC-A (`UPCA`),
Interleaved 2 of 5 (`25`) and ITF-14. Supply seven data digits for EAN-8 or
eleven for UPC-A to generate the retail check digit. Type `25` takes an even
number of digits, including leading zeros. A narrow width of 2 dots and a wide
width of 4 dots gives a 2:1 ratio that both ZBar and ZXing-C++ can scan.

The 2D commands are `QRCODE`, `DMATRIX`, `PDF417` and `AZTEC`. For example,
`AZTEC 10,10,0,3,0,0,0,0,0,"ABC123"` prints a single symbol at (10, 10)
with three-dot modules. Aztec supports quoted and byte-counted data, quarter
turns, reversed output, fixed compact/full layers and Runes. Menu symbols,
FLG(n) controls and multi-symbol output return explicit errors.

Use TSC's `\["]` for a double quote in a quoted field. A backslash
immediately before that escape remains literal, so `"A\\["]B"`
encodes the four bytes `41 5c 22 42`. At the end of a command, `"A\"`
encodes `A` and one backslash. Labelize also accepts the older `\"`
quote escape within data. At a physical line ending, a following command
recognized by Labelize or end of input closes the field. Use `\["]` for a data quote
at the end of a line in a multiline field. [TSC Programming Manual](https://fs.tscprinters.com/system/files/31-0000001-00_tspl_tspl2_programming_3_0.pdf)

For `PDF417`, commas inside quoted data are literal. In the counted form,
the comma after `L<n>` separates the length from the payload. For example,
`L5,,ABCD` encodes the five bytes `,ABCD`. The rotation parameter of
`QRCODE` accepts 0, 90, 180 or 270 degrees clockwise.

Text, boxes, lines and raw `BITMAP` graphics are also supported. Send binary
payloads as bytes. `MPDF417`, `PUTPCX` and `PUTBMP` return unsupported-command
errors. See [TSPL baselines](TSPL_BASELINES.md) for calibration and
reference coverage.

## CPCL

CPCL sessions begin with a `!` header and finish with `PRINT` or `END`.
The header below declares a 480-dot height at the 203-dpi class; `PAGE-WIDTH`
sets 800 dots. At 8 dpmm the canvas is **100 × 60 mm**.

```cpcl playground width=100 height=60 dpmm=8
! 0 203 203 480 1
PAGE-WIDTH 800
TEXT 4 0 40 40 Hello, CPCL
LINE 40 110 440 110 3
FORM
PRINT
```

Use `COUNTRY GB18030` or `ENCODING GB18030` for GB18030 text, including
four-byte characters. Use `UTF-8` with either command for UTF-8 input. For
example, `TEXT GBUNSG24.CPF 0 20 20 保温杯` selects 24-dot cells.
`GBUNSG16.CPF` selects 16-dot cells. Each new `!` session starts with the
default encoding. [Zebra mobile printer guide](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/product/zq310plus-zq320plus-ug-en.pdf)

Labelize previews these CPCL fonts and the TSPL `TSS`, `TST` and `TTT`
16/24-dot fonts with an embedded GNU Unifont subset. Han characters use a
full cell and ASCII uses half a cell. The 24-dot preview scales the 16-dot
outlines. Glyph shapes can differ from the fonts installed on the printer.

Text, resident fonts, scalable text, multiline fields, several 1D/2D barcodes
and counted `EG`/`CG` graphics are supported. `VBARCODE QR` rotates the
symbol 90 degrees counterclockwise from `BARCODE QR`. For Data Matrix, use
`B DATAMATRIX x y H scale S 200`, followed by the data and a separate
`ENDDATAMATRIX` line. Data lines are joined without line breaks. The default
module is six printer dots. `C` or `R` can select a fixed square size.
Use `G` to change the escape byte (default `_`). Supported escapes encode
literal bytes and ASCII controls. Legacy ECC, FNC1, structured append and
other function escapes return an error.
Stored formats, PCX, font groups
and some barcode types are unsupported. Reference coverage is described in
[the test corpus](TEST_CORPUS.md#cpcl-and-dpl).

## DPL

DPL formats start with the byte `0x02` (STX) followed by `L` and finish with
`E`. Coordinates normally use hundredths of an inch from a **bottom-left**
origin; `m` switches to tenths of a millimetre. Text records encode rotation,
font, scaling and row/column fields before their data.

Use the playground's **DPL showcase** sample for a complete runnable format.
For integrations, read a `.dpl` file as bytes and send it with
`Content-Type: application/dpl`. The visible characters `<STX>` are explanatory
notation, not a substitute for the actual control byte.

Bitmap/smooth fonts, common linear barcodes, QR, DataMatrix, PDF417, Aztec,
MaxiCode and line/box records are supported. To print in-job artwork, download
it to RAM with `<STX>IDFname` (native hexadecimal rows, also `IDAF`) or
`<STX>IDbname` (binary BMP). End the header with CR, supply the image data,
then recall its case-sensitive name in a rotation-1 `Y` record. The image
remains available across label formats within that job. Other image encodings,
polygons, circles and several legacy barcodes are unsupported. External DPL comparisons use a
layout preview, not a printer-grade renderer; see [corpus provenance](TEST_CORPUS.md#cpcl-and-dpl).

## Choose the parser explicitly

| Surface | Selection |
| --- | --- |
| Playground | Format selector |
| CLI | File extension, or `--format zpl` (also `epl`, `tspl`, `cpcl`, `dpl`) |
| HTTP conversion | `Content-Type: application/zpl` and the equivalent for each language |
| HTTP validation/formatting | `?language=zpl` or another supported language |
| Rust / WASM | Explicit language option; see the platform tutorial |

Prefer explicit selection for files with unusual prefixes and binary payloads.
Do not switch languages to hide a parse error; consult the support reference
and reduce the failing label to the command that triggers it.
