# Troubleshooting

Start with the same source bytes, language, physical size and resolution on
both systems. Change one setting at a time. Keep a copy of the original input.

## Empty or cropped preview

1. Confirm the format and complete label boundaries (`^XA…^XZ`, `N…P1`,
   `CLS…PRINT`, a CPCL session, or a DPL format).
2. Check the syntax status. A parse result with zero labels cannot render.
3. Set width and height explicitly. At 8 dpmm, a field at x=800 is already
   beyond a 100 mm canvas. ZPL `^LL` does not set height in Labelize.
4. Check the coordinate origin. DPL rows grow upward from the bottom; most
   other language examples position from the top-left.

A failed live render keeps the last successful preview. Read the error banner
before assuming that the visible image belongs to your latest edit.

## The label prints at the wrong size

Use actual-size printing with page fitting disabled. Check the document's
physical dimensions, printer resolution and stock dimensions. Preview zoom
changes only the screen display. In vector mode, PNG scale changes exported
pixel density without changing source dot coordinates.

## Text differs or characters are missing

Labelize uses embedded substitute fonts and calibrated resident-font metrics.
It does not automatically load fonts installed on your printer. Compare font
selection, encoding and rotation first. A character needs both the correct
input encoding and a glyph in the selected font. Image-to-code preserves
appearance as a bitmap when editable text is unnecessary.

## Barcode looks different or will not scan

Check payload, symbology, module width and available whitespace. Avoid text
or boxes overlapping modules, and leave quiet space around the symbol.
Default preview behavior does not add a protective quiet-zone canvas for you.
Print at the intended resolution and test with the scanner you will use.
Neither successful parsing nor a low visual diff proves scan reliability.

## Binary graphics are corrupted

Use `--data-binary @file` with curl and byte arrays in SDKs. Do not pass TSPL
`BITMAP`, EPL `GW`, CPCL compressed graphics or image-to-TSPL output through a
UTF-8 string conversion. Keep control bytes and counted payload lengths intact.

## API returns an error

Read the response body before treating it as an image. Use curl's
`--fail-with-body` or your HTTP client's status check. For `/validate`, HTTP
200 means a report was produced: inspect `ok` and `error` inside the JSON.
Unsupported output settings (such as `pages=all` with raster output) are input
errors. See the [HTTP contract](tutorials/http-service.md).

## Vector controls are absent

The running build lacks the `vector` feature. Build the native service with
`cargo build --features serve,vector` and run that binary. The presence of
vector documentation does not mean every deployed instance enables it.

## Report a reproducible issue

Include the Labelize version or commit, execution surface, a minimal source
file, language, dimensions, dpmm and output mode. Attach the actual output
and describe the expected output with its source (printer model, reference
service or specification). Remove customer addresses and other private data.
See [testing](TESTING.md) for fixture and reference-image conventions.
