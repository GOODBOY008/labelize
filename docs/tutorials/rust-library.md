# Tutorial: Rust Library

[All tutorials](README.md) · [Documentation index](../README.md)

Use Labelize as a crate when the caller is already Rust — a batch converter,
a print spooler, an HTTP endpoint of your own, or a test harness that must
assert on rendered labels.

## 1. Add the dependency

Use a current stable Rust toolchain. Cargo selects the edition for each crate
independently, so a Rust 2021 application can depend on this Rust 2024 library.
[Rust Edition Guide](https://doc.rust-lang.org/edition-guide/editions/index.html)

```sh
cargo add labelize
```

The default library renders all five printer languages to PNG or raster PDF.
Enable `vector` for searchable PDF or scaled PNG output. The
[Rust API reference](https://docs.rs/labelize) lists the optional features.

## 2. One call: `render_png` / `render_pdf`

`render_png` and `render_pdf` detect the language, choose the canvas size and
encode the first label. The result also tells you how many labels the input contains.

```rust
use labelize::{render_png, RenderOptions};

let input = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";

let out = render_png(input, RenderOptions::default())?;
// out.language == Language::Zpl (auto-detected)
// out.labels  == 1             (total labels in the input)
// out.bytes   — PNG file bytes
std::fs::write("output.png", out.bytes)?;
```

`render_pdf` takes the same arguments and returns PDF bytes. Everything is
steerable through `RenderOptions`:

```rust
use labelize::{render_pdf, Language, RenderOptions};

let opts = RenderOptions {
    language: Some(Language::Epl), // None = auto-detect
    width_mm: Some(100.0),         // None = label-declared, else 102×152
    height_mm: Some(62.0),
    dpmm: 12,
    antialias: false,
};
let out = render_pdf(epl_bytes, opts)?;
```

Errors are a single `LabelizeError`: `Parse` variants are user-input
problems, everything else is engine-side — the same split the built-in HTTP
service maps to 400 vs 500.

### Render a filled printer template

Load a Studio export with `PrintTemplate::load`, then call `fill` with
`TemplateValues` for each label. Pass the returned printer bytes directly to
`render_png` or `render_pdf`. [Complete Rust example](../PRINT_TEMPLATES.md#rust-sdk)

## 3. The composable path: detect → parse → render

Use `parse_labels` to inspect elements, iterate through a batch or render into
your own writer. It accepts ZPL, EPL, TSPL/TSC, CPCL and DPL.

```rust
use std::io::Cursor;
use labelize::{detect_language, parse_labels, DrawerOptions, Renderer};

let input = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";

let language = detect_language(input);

// Parse — one input can contain several labels (^XA…^XZ repeated, ^PQ
// quantities), so you always get a Vec.
let labels = parse_labels(language, input, 8)?;

// Render + encode — pixels go straight into any `impl Write`.
let renderer = Renderer::new();
let mut buf = Cursor::new(Vec::new());
renderer
    .draw_label_as_png(&labels[0], &mut buf, DrawerOptions::default())?;
std::fs::write("output.png", buf.into_inner())?;
```

When you already know the language, skip the sniff:

```rust
use labelize::{parse_labels, Language};

let labels = parse_labels(Language::Tspl, tspl_bytes, 8)?;
```

For language-specific settings, use the corresponding parser directly. For
example, `ZplParser::parse_with_options` accepts `ZplParseOptions`.

## 4. Validate without rendering

Before you spend a render — or before you show a user an error — check the
source and get the exact parser failure. `validate` never fails; the outcome
travels in the report:

```rust
use labelize::{validate, Language};

let report = validate(b"N\nq812\nB10,10,0,9,2,4,80,B,\"oops\"\nP1\n", 8, None);

assert_eq!(report.language, Language::Epl); // auto-detected
assert!(!report.ok);
assert!(report.error.as_deref().unwrap().contains("unsupported bar code type"));

let ok = validate(b"^XA^FDHi^FS^XZ", 8, Some(Language::Zpl));
assert!(ok.ok && ok.labels == 1);
```

`ValidationReport` fields: `language`, `ok`, `labels` (label count), `error`
(`Some(msg)` when parsing failed). Passing `Some(language)` forces a parser
and skips detection; empty input reports `ok: true` with `labels: 0` —
"parses to nothing", not "invalid".

## 5. Format (pretty-print) label source

`format_source` reflows source one command per line — `^XA` blocks indented,
CRLF normalized, blank runs collapsed — without changing what renders:

```rust
use labelize::{format_source, Language};

let (language, formatted) = format_source(b"^XA^CF0,60^FO50,50^FDHello^FS^XZ", None);

assert_eq!(language, Language::Zpl);
assert_eq!(formatted, b"^XA\n  ^CF0,60\n  ^FO50,50\n  ^FDHello\n^FS\n^XZ\n");
```

The return is raw `Vec<u8>`, not `String`: binary payloads (`^FD` field
data, `GW`/`BITMAP` graphics) pass through byte-for-byte, and those are not
valid UTF-8. On real corpus labels the formatted source renders
pixel-identically (sha256-verified), so it is safe to expose as a
"beautify" feature. Formatting never fails and never rejects input.

## 6. Control the canvas with `DrawerOptions`

```rust
let options = DrawerOptions {
    label_width_mm: 101.6,     // default 101.6 (4")
    label_height_mm: 152.0,    // default 203.2 (8") — set to your stock size
    dpmm: 8,                   // 6 / 8 / 12 / 24
    enable_inverted_labels: true, // honor ^PO I (default true)
    antialias: false,          // true → grayscale coverage instead of black/white
};
```

`DrawerOptions::with_defaults()` fills any zero field with the defaults —
handy when dimensions come from user input that may be unset
(`draw_label_as_png` applies it for you).

TSPL labels carry their own physical size (`SIZE 60 mm,30 mm`) in
[`LabelInfo`](../../src/elements/label_info.rs). The built-in CLI and HTTP
service prefer an explicit user size, then the label's declared one, then
the fallback — copy the pattern when dimensions are optional for you:

```rust
use labelize::{DEFAULT_LABEL_HEIGHT_MM, DEFAULT_LABEL_WIDTH_MM};

let label = &labels[0];
let options = DrawerOptions {
    label_width_mm: user_width.or(label.label_width_mm).unwrap_or(DEFAULT_LABEL_WIDTH_MM),
    label_height_mm: user_height.or(label.label_height_mm).unwrap_or(DEFAULT_LABEL_HEIGHT_MM),
    dpmm: 8,
    ..DrawerOptions::default()
};
```

## 7. PDF output

Use `render_pdf(input, RenderOptions::default())?` for a single raster page.
To preserve searchable text and combine labels into one PDF, enable `vector`.

```sh
cargo add labelize --features vector
```

```rust
use labelize::{render_pdf_with_output, OutputMode, OutputOptions, RenderOptions};

let job = concat!(
    "^XA^FO20,20^A0N,32,32^FDParcel A^FS^XZ",
    "^XA^FO20,20^A0N,32,32^FDParcel B^FS^XZ",
);
let output = OutputOptions {
    mode: OutputMode::Vector,
    all_pages: true,
    ..OutputOptions::default()
};
let pdf = render_pdf_with_output(job.as_bytes(), RenderOptions::default(), output)?;
std::fs::write("parcels.pdf", pdf.bytes)?;
```

Each label gets its own page. Ordinary text stays searchable, while embedded
images retain their source resolution. See [Vector output](../VECTOR_OUTPUT.md)
for scaling and raster fallback details.

## 8. Multiple labels per input

`^XA…^XZ` blocks repeat, and `^PQ` multiplies a single block into a print
quantity — both arrive as extra entries in the `Vec`:

```rust
let labels = parse_labels(language, input, 8)?;
for (i, label) in labels.iter().enumerate() {
    let mut buf = Cursor::new(Vec::new());
    renderer.draw_label_as_png(label, &mut buf, options.clone())?;
    std::fs::write(format!("label-{i}.png"), buf.into_inner())?;
}
```

## 9. Error handling

Two stages fail differently — keep them distinct in your messages, they map
to HTTP 400 vs 500 in the built-in service:

| Stage | Call | Error type |
|---|---|---|
| Any | `render_png` / `render_pdf` | `LabelizeError` — `Parse` variants are user-input, the rest engine-side |
| Parse | `parse_labels` | `LabelizeError::Parse(String)` — bad input, user's fault |
| Render | `draw_label_as_png` | `String` — low-level; the one-call API wraps this into `LabelizeError::Render` |
| Encode | `encode_pdf` / `encode_png` | `LabelizeError` |

`validate` sits outside the table: it converts parse failures into data, so
"what is wrong with this snippet" is a question you ask *before* rendering
instead of an `Err` you unpack after.

## 10. A complete, runnable example

From a checkout of the project, run the ZPL/EPL converter example to write a
PNG or PDF.

```bash
cargo run --example render_label -- testdata/labels/amazon.zpl out.png
```

Source: [`examples/render_label.rs`](../../examples/render_label.rs).

## 11. What's next

- Explore the modules — `parsers/` (ZPL/EPL/TSPL/CPCL/DPL), `elements/` (typed label
  elements), `drawers/renderer.rs` (rasterizer), `images/` (PNG/PDF encoders)
- [`docs/ZPL_COMMANDS_REFERENCE.md`](../ZPL_COMMANDS_REFERENCE.md) — the
  supported command matrix
- [HTTP service tutorial](http-service.md) — wrap this API in axum yourself,
  or reuse the built-in server (`/convert`, `/validate`, `/format`)
