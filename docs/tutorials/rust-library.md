# Tutorial: Rust Library

Use Labelize as a crate when the caller is already Rust — a batch converter,
a print spooler, an HTTP endpoint of your own, or a test harness that must
assert on rendered labels.

## 1. Add the dependency

```toml
[dependencies]
labelize = "1.6"
image = "0.25"   # only if you post-process the rendered pixels
```

The default features are enough for library use; `cli` / `serve` features are
only needed for the binaries.

## 2. Parse → render → encode

The pipeline has three steps, matching the crate layout:

```rust
use std::io::Cursor;
use labelize::{DrawerOptions, Renderer, ZplParser};

let zpl = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";

// 1. Parse — one input can contain several labels (^XA…^XZ repeated)
let mut parser = ZplParser::new();
let labels = parser.parse(zpl).unwrap();

// 2+3. Render + encode — pixels go straight into your writer
let renderer = Renderer::new();
let mut buf = Cursor::new(Vec::new());
renderer
    .draw_label_as_png(&labels[0], &mut buf, DrawerOptions::default())
    .unwrap();

std::fs::write("output.png", buf.into_inner()).unwrap();
```

EPL is identical with `EplParser::new()`.

## 3. Control the canvas with `DrawerOptions`

```rust
let options = DrawerOptions {
    label_width_mm: 101.6,     // default 101.6 (4")
    label_height_mm: 152.0,    // default 203.2 (8") — set to your stock size
    dpmm: 8,                   // 6 / 8 / 12 / 24
    enable_inverted_labels: true, // honor ^PO I (default true)
    antialias: false,          // true → 8-bit greys instead of 1-bit
};
```

`DrawerOptions::with_defaults()` fills any zero field with the defaults —
handy when dimensions come from user input that may be unset.

## 4. PDF output

PDF wraps the rendered pixels in a single-page document:

```rust
use labelize::encode_pdf;

let mut png = Cursor::new(Vec::new());
renderer.draw_label_as_png(&labels[0], &mut png, options.clone()).unwrap();
let img = image::load_from_memory(&png.into_inner()).unwrap().to_rgba8();
let mut pdf = Cursor::new(Vec::new());
encode_pdf(&img, &options, &mut pdf).unwrap();
std::fs::write("output.pdf", pdf.into_inner()).unwrap();
```

## 5. Error handling

Parsing and rendering return `Result<_, LabelizeError>` (see `src/error.rs`).
A malformed label fails at parse; a label that parses but cannot be drawn
fails at render — keep those stages distinct in your error messages, they map
to HTTP 400 vs 500 in the built-in service.

## 6. A complete, runnable example

The repo ships a ~90-line example that turns these pieces into a mini
`convert` command with PNG/PDF output:

```bash
cargo run --example render_label -- testdata/labels/amazon.zpl out.png
```

Source: [`examples/render_label.rs`](../../examples/render_label.rs).

## 7. What's next

- Explore the modules — `parsers/` (ZPL/EPL), `elements/` (typed label
  elements), `drawers/renderer.rs` (rasterizer), `images/` (PNG/PDF encoders)
- [`docs/ZPL_COMMANDS_REFERENCE.md`](../ZPL_COMMANDS_REFERENCE.md) — the
  supported command matrix
- [HTTP service tutorial](http-service.md) — wrap this API in axum yourself,
  or reuse the built-in server
