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
Binary `GW` graphics must be uploaded as bytes through the API.

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

`TEXT`, boxes, lines, Code 128/39/EAN-13, QR, DataMatrix, PDF417 and `BITMAP`
are supported. `AZTEC`, `MPDF417`, `PUTPCX` and `PUTBMP` return explicit
unsupported-command errors. See [TSPL baselines](TSPL_BASELINES.md) for
calibration and reference coverage. Binary `BITMAP` data must remain bytes.

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

Text, resident fonts, scalable text, multiline fields, several 1D/2D barcodes
and counted `EG`/`CG` graphics are supported. Stored formats, PCX, font groups
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
MaxiCode and line/box records are supported. Stored images, polygons, circles
and several legacy barcodes are unsupported. External DPL comparisons use a
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
