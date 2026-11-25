# Fill an exported printer template

Export the design from Studio and bundle the original template file with your
application. Labelize fills each order and returns a native printer job. Keep
the job as bytes through printing and previewing so binary artwork survives.

| Where your application runs | Integration |
| --- | --- |
| Shell script or batch job | Use the CLI to write a printer file, PNG or PDF. |
| Backend in any language | Call the HTTP service to fill the template, then post the returned bytes for a preview. |
| Rust application | Load the template once with the Rust SDK and reuse it across orders. |

The examples below use the [shipping template](../testdata/templates/printers/shipping.tpl.zpl)
and [sample order](../testdata/templates/first.json). Save them as
`shipping.tpl.zpl` and `order.json`. The sample label stock is 100 × 50 mm.

## CLI

`order.json` contains one object of string values. For example:

```json
{"order_no":"000123","product":"Thermal labels"}
```

```sh
labelize template fill shipping.tpl.zpl --data order.json -o order.zpl
labelize template render shipping.tpl.zpl --data order.json --dpmm 8 --width 100 --height 50 -o order.png
labelize template render shipping.tpl.zpl --data order.json --dpmm 8 --width 100 --height 50 -o order.pdf
```

Choose the export that matches the printer language, device fonts and printhead
resolution. Supply the physical dimensions of the stock when the native job
does not declare them. The preview uses the first label by default. Builds with
the `vector` feature also support `--mode vector`, including `--all-pages` for
a multipage PDF. The CLI replaces the destination only after the entire record
has been validated, rendered if requested, and written successfully.

## HTTP

Start a build with the `serve` feature using `labelize serve`, or use the URL
of your Labelize deployment. Send one JSON object to `POST /template/fill`.
For `template_base64`, encode the original file bytes in standard Base64.
Each value is a string, including identifiers with leading zeros.

```sh
python3 - <<'PY' > request.json
import base64, json
from pathlib import Path
print(json.dumps({
    "language": "zpl",
    "template_base64": base64.b64encode(Path("shipping.tpl.zpl").read_bytes()).decode("ascii"),
    "values": json.loads(Path("order.json").read_text(encoding="utf-8")),
}))
PY

base_url=http://127.0.0.1:8080
curl --fail --show-error --silent "$base_url/template/fill" \
  -H 'Content-Type: application/json' --data-binary @request.json -o order.zpl
curl --fail --show-error --silent "$base_url/convert?output=png&width=100&height=50&dpmm=8" \
  -H 'Content-Type: application/zpl' --data-binary @order.zpl -o order.png
```

The first response is `application/octet-stream`, ready to send to the
printer. The second call previews that exact job. For PDF, set `output=pdf`
and save the result as `order.pdf`. For other printers, set both the request's
`language` and the conversion media type, for example `dpl` and
`application/dpl`. The request JSON and the filled job each have a 2 MiB
limit. Invalid template or order data returns HTTP 400 with a
diagnostic. [HTTP API and status codes](API.md)

## Rust SDK

Use the SDK to fill orders in process. The [Node.js/WebAssembly example](../wasm/npm/README.md)
shows the equivalent call for JavaScript applications.

```rust
use labelize::{render_png, Language, PrintTemplate, RenderOptions, TemplateValues};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("shipping.tpl.zpl")?;
    let template = PrintTemplate::load(Language::Zpl, &bytes)?;
    let order = TemplateValues::from([
        ("order_no".into(), "000123".into()),
        ("product".into(), "Thermal labels".into()),
    ]);
    let job = template.fill(&order)?;
    std::fs::write("order.zpl", &job)?;
    let options = RenderOptions {
        language: Some(Language::Zpl),
        dpmm: 8,
        width_mm: Some(100.0),
        height_mm: Some(50.0),
        ..Default::default()
    };
    std::fs::write("order.png", render_png(&job, options)?.bytes)?;
    Ok(())
}
```

1. **Load once.** `PrintTemplate` owns the native bytes, the ordered ranges
   that may be replaced, and the parsed literal/variable expressions. It
   checks directory boundaries and compiles the printer-specific options.
   `fields()` lists names in first-use order with their object numbers and uses.
2. **Fill each record.** Every occurrence of a name uses the same value.
   Expressions expand once, so braces in a business value stay literal.
   Each command encodes its field before fixed bytes and encoded ranges are
   joined in printer order. The immutable template can be shared across threads.
3. **Preview the filled job.** Pass the returned bytes to `render_png` or
   `render_pdf` with the export language and render options. The CLI follows
   this same path for raster and vector output.

`validate(&order)` returns every invalid occurrence, including its field
names, native kind, object number and reason. `fill` also validates. Errors
include missing keys, invalid retail check digits, unsupported characters and
barcode capacity limits. Extra order keys are ignored. Pass identifiers as
strings so that leading zeros survive.

## ZPL field markers

A Studio ZPL export places a marker on its own line, immediately before a
single native field. For example:

```text
^FXLabelize field text^FS
^FO20,20^A0N,30,30^FH^FD_49_44_3A_20{{order_no}}^FS
```

The marker supplies the field kind. After `^FH^FD`, `_HH` encodes one
literal UTF-8 byte in uppercase hex. `{{name}}` denotes a variable matching
`[A-Za-z_][A-Za-z_0-9]*`. The example prints `ID: 000123`. QR fields also
carry their native correction/input-mode prefix, such as `MA,`. Text blocks,
linear barcodes, captions and matrix symbols each apply the corresponding
printer rules. For example, a literal `^` in a value becomes `_5E` under
`^FH`. Use a Studio named-font export for text outside the resident font's
character range. [Example Studio ZPL export](../testdata/templates/printers/shipping.tpl.zpl)

## TSPL/TSC, EPL2, CPCL and DPL byte directories

The first line is `LABELIZE-TPL/1 <language> <charset>`. Directory lines
are ASCII with LF separators. Each subsequent row has six space-separated
fields: `offset length object kind codec parameters`. A blank line ends the
directory and the native job starts immediately afterward. For example:

```text
LABELIZE-TPL/1 tspl US-ASCII
20 12 1 text tspl -

TEXT 0,0,"3",0,1,1,"{{order_no}}"
PRINT 1
```

Offsets start at zero in the native body. Lengths are in bytes. Ranges must
be in order, nonempty, nonoverlapping and inside the body. Object numbers start
at one. Each range contains the same expression grammar as ZPL. Bytes outside
the ranges are copied verbatim, including graphics, line endings and fixed
text containing braces. The directory itself is removed from the printer job.

| Directory language | Charset | Codec names and native output |
| --- | --- | --- |
| `tspl` | `US-ASCII`, `UTF-8` | `tspl` escapes quotes in a quoted field. PDF417 uses `tspl-length` to write `L<count>,<data>`. Aztec uses `aztec-count` to write `<count>,<data>`. Both count encoded bytes. |
| `epl` | `US-ASCII` | `epl` escapes quotes and backslashes in quoted fields. |
| `cpcl` | `US-ASCII`, `UTF-8` | `line` is a single data line. Data Matrix uses `dm-line` to escape controls, literal underscores and terminator-like data. `pdf-line` and `aztec-line` reject the respective end markers. |
| `dpl` | `US-ASCII` | `line` writes a single data line. For Data Matrix and PDF417, `count4:<options>` writes four decimal digits counting both options and data, followed by those bytes. `dpl-aztec` prefixes four default option digits; earlier Aztec entries using `line` receive the same prefix. |

Linear kinds are `code128`, `code39`, `ean13`, `ean8`, `upca`, and `itf`;
their `-caption` variants print human-readable values. Retail symbols accept
the body or the complete value with a valid check digit. The symbol receives
the body and a caption receives the complete number. Matrix kinds are `qr`
(correction L/M/Q/H), `datamatrix` (square or rectangle), `pdf417` (columns
and correction joined by a comma, e.g. `3,2`) and `aztec`. For DPL counted
Data Matrix the options are `200` plus seven digits; for PDF417 they are `F`,
the correction digit (0–8) and six further digits. Ordinary text and the other kinds use `-` for
directory parameters. A UTF-8 profile can carry Chinese text such as
`保温杯` when the device has the selected font.

The fixtures include two records per language and their expected native bytes.
The tests compare the Rust output with the expected jobs, render both forms at
8 and 12 dots/mm, and read the order number from the rendered labels.
[Shared templates and verification instructions](../testdata/templates/)
