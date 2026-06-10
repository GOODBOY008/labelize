# Labelize — ZPL / EPL Label Renderer

[![Crates.io](https://img.shields.io/crates/v/labelize)](https://crates.io/crates/labelize)
[![License](https://img.shields.io/github/license/GOODBOY008/labelize)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/GOODBOY008/labelize/ci.yml?branch=main)](https://github.com/GOODBOY008/labelize/actions)

> **Turn ZPL/EPL into pixels — label rendering, simplified.**

Labelize is a fast, open-source Rust engine that parses **ZPL** (Zebra Programming Language) and **EPL** (Eltron Programming Language) label data and renders it to **PNG** or **PDF**. Use it as a **CLI tool**, an **HTTP microservice**, or embed it as a **Rust library** — no printer hardware required.

If you need a self-hosted, offline alternative to [Labelary](http://labelary.com/) for previewing and converting thermal label formats, Labelize has you covered.

Existing solutions either depend on external services (Labelary API — data privacy concerns), require paid licenses (often hundreds of dollars), or produce poor rendering quality. Labelize gives you a fast, free, open-source option that runs entirely on your own infrastructure.

## Why Labelize?

| | Labelize | Labelary (web) | Zebra Printer |
|---|---|---|---|
| **Offline / self-hosted** | ✅ | ❌ | ✅ |
| **No hardware needed** | ✅ | ✅ | ❌ |
| **Open source** | ✅ | ❌ | ❌ |
| **EPL support** | ✅ | ❌ | ✅ |
| **PDF output** | ✅ | ❌ | ❌ |
| **Embeddable library** | ✅ | ❌ | ❌ |
| **REST API** | ✅ | ✅ | ❌ |
| **Cost** | Free | Free / paid | Hardware cost |

## Performance

Benchmarked against the Labelary API on the same set of labels:

| Metric | Labelize | Labelary API |
|--------|----------|--------------|
| Avg render time | ~5 ms | ~388 ms |
| Offline | ✅ | ❌ |
| Data privacy | Full control | Third-party |

## Features

- **ZPL Parser** — 30+ ZPL commands: text, barcodes, graphics, stored formats, graphic fields, field blocks, and more
- **EPL Parser** — EPL command support for text, barcodes, line draw, and reference points
- **10 Barcode Symbologies** — Code 128, Code 39, EAN-13, Interleaved 2-of-5, PDF417, Aztec, DataMatrix, QR Code, MaxiCode
- **PNG & PDF Output** — Monochrome 1-bit PNG or single-page embedded PDF output
- **CLI Tool** — Convert ZPL/EPL files from the command line with format auto-detection, multi-label support, and customizable label dimensions
- **HTTP Microservice** — RESTful API for label conversion with format detection via `Content-Type` header; deploy anywhere with Docker or bare metal
- **Web Playground** — Built-in browser UI at `GET /` — paste or open a `.zpl`/`.epl` file, choose a label size (4×6, 4×4, etc.), render PNG inline, and download PNG or PDF with one click
- **Embedded Fonts** — Zero runtime font dependencies; bundles Helvetica Bold Condensed, DejaVu Sans Mono, and ZPL GS fonts
- **Rust Library** — Integrate label rendering directly into your Rust application via the public API

## Quick Start

### Installation

```bash
# Via Homebrew (macOS / Linux)
brew tap GOODBOY008/homebrew-labelize && brew install labelize

# From source (requires Rust toolchain)
cargo install --path .
```

### Convert a ZPL label to PNG

```bash
labelize convert label.zpl          # → label.png  (format auto-detected)
labelize convert label.epl          # EPL works too
labelize convert label.zpl -t pdf   # output as PDF
labelize convert label.zpl --width 100 --height 62 --dpmm 12  # custom dimensions
```

### Run as an HTTP microservice

```bash
labelize serve --port 8080
```

Open **http://localhost:8080/** in your browser to use the built-in **interactive playground** — paste ZPL/EPL, pick a label size, and render PNG instantly. Download PNG or PDF directly from the page.

```bash
# Convert via REST API
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ' \
  -o label.png
```
### Run as Docker Webservice
'''bash
docker compose up -d --build
'''


## CLI Reference

```
Usage: labelize <COMMAND>

Commands:
  convert  Convert a ZPL/EPL file to PNG or PDF
  serve    Start HTTP server for label conversion

Convert Options:
  <INPUT>               Input file path (.zpl or .epl)
  -o, --output <PATH>   Output file path (default: input stem + .png/.pdf)
  -f, --format <FMT>    Input format override: zpl | epl
  -t, --type <TYPE>     Output type: png | pdf [default: png]
  --width <MM>          Label width in mm [default: 102]
  --height <MM>         Label height in mm [default: 152]
  --dpmm <N>            Dots per mm [default: 8]

Serve Options:
  --host <HOST>         Bind address [default: 0.0.0.0]
  -p, --port <PORT>     Listen port [default: 8080]
```

## HTTP API

| Endpoint       | Method | Description                                   |
|---------------|--------|-----------------------------------------------|
| `/`           | GET    | Interactive web playground (browser UI)       |
| `/health`     | GET    | Health check → `{"status":"ok"}`             |
| `/convert`    | POST   | Convert label data → PNG or PDF              |

**POST /convert** query parameters:

| Parameter | Default | Description            |
|-----------|---------|------------------------|
| `width`   | 102     | Label width in mm      |
| `height`  | 152     | Label height in mm     |
| `dpmm`    | 8       | Dots per mm            |
| `output`  | png     | Output format: png/pdf |

Set `Content-Type: application/zpl` or `Content-Type: application/epl` to select the parser.

## Library Usage

```rust
use std::io::Cursor;
use labelize::{ZplParser, Renderer, DrawerOptions};

let zpl = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";
let mut parser = ZplParser::new();
let labels = parser.parse(zpl).unwrap();

let renderer = Renderer::new();
let mut buf = Cursor::new(Vec::new());
renderer.draw_label_as_png(&labels[0], &mut buf, DrawerOptions::default()).unwrap();

std::fs::write("output.png", buf.into_inner()).unwrap();
```

## Supported ZPL & EPL Commands

### ZPL Commands

| Category | Commands |
|----------|----------|
| **Text & Font** | `^FO` `^FT` `^FD` `^FS` `^A` `^CF` `^FB` `^FR` `^FH` `^FN` `^FW` `^FV` |
| **Barcodes** | `^BC` (Code 128) `^BE` (EAN-13) `^B2` (Interleaved 2-of-5) `^B3` (Code 39) `^B7` (PDF417) `^BO` (Aztec) `^BX` (DataMatrix) `^BQ` (QR Code) `^BD` (MaxiCode) `^BY` (defaults) |
| **Graphics** | `^GB` (box) `^GC` (circle) `^GD` (diagonal) `^GF` (graphic field) `^GS` (symbol) `~DG` (download graphic) `^IL` `^XG` |
| **Label Control** | `^XA` `^XZ` `^PW` `^PO` `^LH` `^LR` `^CI` |
| **Stored Formats** | `^DF` `^XF` |

### EPL Commands

`N` (new label) · `A` (text) · `B` (barcode) · `LO` (line draw) · `R` (reference point) · `P` (print)

## Architecture

```
  ZPL/EPL input
       │
       ▼
  ┌─────────┐     ┌──────────┐     ┌─────────┐
  │  Parser  │ ──▶ │ Renderer │ ──▶ │ Encoder │
  └─────────┘     └──────────┘     └─────────┘
       │                │                │
   LabelInfo        RgbaImage       PNG / PDF
```

- **Parser** — Tokenizes input, maintains VirtualPrinter state, produces `Vec<LabelElement>`
- **Renderer** — Creates canvas, iterates elements, dispatches drawing (text, graphics, barcodes), handles reverse print and label inversion
- **Encoder** — Converts RGBA image to monochrome PNG or embeds into single-page PDF

## Render Comparison

Side-by-side comparison against [Labelary](http://labelary.com/) reference renderer. Left = Labelary, Right = Labelize.

All diff images are in [`testdata/diffs/`](testdata/diffs/) — browse them to review any label.

| Label | Diff | Preview |
|-------|------|---------|
| amazon | 2.26% | <img src="testdata/diffs/amazon.png" height="150"> |
| fedex | 5.77% | <img src="testdata/diffs/fedex.png" height="150"> |
| ups | 5.74% | <img src="testdata/diffs/ups.png" height="150"> |
| dhlpaket | 2.17% | <img src="testdata/diffs/dhlpaket.png" height="150"> |
| usps | 4.05% | <img src="testdata/diffs/usps.png" height="150"> |
| swisspost | 1.49% | <img src="testdata/diffs/swisspost.png" height="150"> |

**83 labels tested** — 6 perfect · 27 good (<1%) · 39 minor (<5%) · 11 moderate (<15%) · 0 high

> Full report: [`testdata/diffs/diff_report.txt`](testdata/diffs/diff_report.txt)

## Testing

```bash
cargo test                               # all tests
cargo test --test 'e2e_*'               # golden-file E2E tests
cargo test --test 'unit_*'             # unit tests

# E2E shell scripts (requires labelize on PATH)
PATH="$PATH:target/debug" bash e2e/http/test_http.sh   # HTTP microservice tests
PATH="$PATH:target/debug" bash e2e/cli/test_cli.sh     # CLI tests
```

82 golden-file E2E tests compare rendered output pixel-by-pixel against reference PNGs from the Labelary reference renderer.

## Building from Source

```bash
cargo build --release
# Binary: target/release/labelize
```

## Use Cases

- **Shipping label preview** — Render a ZPL label before sending it to the printer
- **Warehouse management** — Batch-convert label templates to PDF for archival
- **E-commerce integrations** — Embed as a microservice to generate shipping label PNGs on the fly
- **Automated QA** — Validate label content in CI/CD pipelines with golden-file tests
- **Label design tools** — Use the library API to add real-time ZPL preview to custom applications

## Related Projects & Keywords

Looking for a **ZPL renderer**, **ZPL to PNG converter**, **ZPL to PDF**, **EPL parser**, **Zebra label preview**, **thermal label rendering**, or **Labelary alternative**? Labelize covers all of these.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

See [LICENSE](LICENSE) in the repository root.
