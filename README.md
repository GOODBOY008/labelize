<div align="center">

<img src="docs/logo.svg" alt="Labelize" width="190"/>

# Labelize

**Parse and render ZPL / EPL labels to PNG & PDF — fast, offline, open source.**

[![CI](https://img.shields.io/github/actions/workflow/status/GOODBOY008/labelize/ci.yml?branch=main&label=CI)](https://github.com/GOODBOY008/labelize/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/labelize)](https://crates.io/crates/labelize)
[![npm](https://img.shields.io/npm/v/@goodboy008/labelize-wasm)](https://www.npmjs.com/package/@goodboy008/labelize-wasm)
[![Docker](https://img.shields.io/github/actions/workflow/status/GOODBOY008/labelize/docker.yml?branch=main&label=docker)](https://github.com/GOODBOY008/labelize/actions/workflows/docker.yml)
[![License](https://img.shields.io/github/license/GOODBOY008/labelize)](LICENSE)

[🌐 Playground](https://labelize.764629910.workers.dev) · [📚 Tutorials](docs/tutorials/README.md) · [📜 Changelog](CHANGELOG.md) · [🚀 Releases](https://github.com/GOODBOY008/labelize/releases)

<img src="docs/playground.png" alt="The Labelize web playground rendering a shipping label" width="860"/>

</div>

---

Labelize is a Rust engine that parses **ZPL** (Zebra Programming Language) and **EPL** (Eltron Programming Language) label data and renders it to **PNG** or **PDF** — no printer hardware required. The same engine ships as a **CLI tool**, an **HTTP microservice**, a **WebAssembly package**, an **Android library**, and a plain **Rust crate**, producing identical output on every surface.

A free public playground is hosted on Cloudflare Workers at **<https://labelize.764629910.workers.dev>** — paste a label, see it render, download PNG or PDF. No install, no signup, no data leaving the browser tab beyond the render call.

## ✨ Why Labelize?

Previewing thermal labels today usually means one of three compromises: send your label data to the [Labelary](http://labelary.com/) web API (third-party service, data-privacy concerns, ~400 ms per render), pay for a commercial SDK (often hundreds of dollars), or keep a physical printer around just to see what would print. Labelize is the self-hosted alternative:

|  | **Labelize** | Labelary (web) | Zebra Printer |
|---|---|---|---|
| Offline / self-hosted | ✅ | ❌ | ✅ |
| No hardware needed | ✅ | ✅ | ❌ |
| Open source | ✅ | ❌ | ❌ |
| EPL support | ✅ | ❌ | ✅ |
| PDF output | ✅ | ❌ | ❌ |
| Embeddable library | ✅ | ❌ | ❌ |
| REST API | ✅ | ✅ | ❌ |
| Cost | Free | Free / paid tiers | Hardware cost |

**Features at a glance**

- 🖨️ **38+ ZPL commands** — text & fonts, 12 barcode symbologies (Code 128, EAN-13/8, UPC-A/E, Code 39, Interleaved 2-of-5, PDF417, Aztec, DataMatrix, QR, MaxiCode), boxes/circles/diagonals/ellipses, graphic fields, stored formats (`^DF`/`^XF`), label rotation & inversion
- 🏷️ **EPL2 support** — text, the full 1D/2D barcode command set (`B`/`b`), lines, diagonals, boxes, binary graphics (`GW`)
- 🖼️ **PNG & PDF output** — thermal-faithful 1-bit monochrome by default, optional antialiased greyscale
- 🔤 **Embedded fonts** — zero runtime font dependencies (Roboto Condensed, DejaVu Sans Mono, ZPL GS — all permissively licensed)
- ⚡ **~5 ms per render** — no network, no interpreter, no printer
- 🧪 **124 golden-file E2E tests** against Labelary reference renders on every push

## 🚀 Quick Start

Pick the surface that fits your stack — every path below produces the same pixels.

### Command line (macOS · Linux · Windows)

```bash
# Homebrew (macOS / Linux)
brew tap GOODBOY008/homebrew-labelize && brew install labelize

# Or from source / crates.io (requires a Rust toolchain)
cargo install labelize --features cli

# Windows: download labelize-x86_64-pc-windows-msvc.zip from Releases
```

```bash
labelize convert label.zpl                    # → label.png (format auto-detected)
labelize convert label.epl -t pdf             # EPL in, PDF out
labelize convert label.zpl --width 100 --height 62 --dpmm 12
```

### Docker

Pre-built multi-arch images (`linux/amd64` + `linux/arm64`) on every release:

```bash
docker run -p 8080:8080 goodboy008/labelize:latest   # or ghcr.io/goodboy008/labelize

curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello Docker^FS^XZ' \
  -o label.png
```

Open `http://localhost:8080/` for the interactive playground. Tags: `latest`, `1.6.0` / `1.6` / `1`, `edge` (current `main`).

### JavaScript / TypeScript

The engine compiled to WebAssembly — renders in browsers, Node.js, and bundlers with **no server**:

```bash
npm install @goodboy008/labelize-wasm
```

```js
import { lz_render } from "@goodboy008/labelize-wasm/init";   // Node ≥ 20

const zpl = Buffer.from("^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ", "ascii");
const png = lz_render(zpl, 102.0, 152.0, 8, false, false, false); // Uint8Array
```

Bundlers (webpack 5, Vite + `vite-plugin-wasm`) import from the package root instead. Full API in the [JavaScript tutorial](docs/tutorials/javascript-wasm.md).

### Android

Self-contained AAR (`com.goodboy008.labelize`) with native libraries for `arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86` (minSdk 24) — bit-identical output to the desktop builds. Download `labelize-android-aar.zip` from [Releases](https://github.com/GOODBOY008/labelize/releases):

```kotlin
// app/build.gradle.kts
repositories {
    flatDir { dirs("libs") }
}
dependencies {
    implementation(name = "labelize-android-release", ext = "aar")
}

// then, from Kotlin or Java:
val png = Labelize.renderZplToPng(zpl.toByteArray(), widthMm = 102.0, heightMm = 152.0)
```

More in the [Android tutorial](docs/tutorials/android.md) and [`android/README.md`](android/README.md).

### Rust

```bash
cargo add labelize
```

```rust
use std::io::Cursor;
use labelize::{ZplParser, Renderer, DrawerOptions};

let mut parser = ZplParser::new();
let labels = parser.parse(b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ")?;

let mut buf = Cursor::new(Vec::new());
Renderer::new().draw_label_as_png(&labels[0], &mut buf, DrawerOptions::default())?;
std::fs::write("label.png", buf.into_inner())?;
```

A complete runnable example lives at [`examples/render_label.rs`](examples/render_label.rs)
(`cargo run --example render_label -- label.zpl`), with a walkthrough in the
[Rust tutorial](docs/tutorials/rust-library.md).

## 🎨 Web Playground

Every Labelize server serves an interactive UI at `GET /` — the same page running
at the public instance:

- Paste ZPL/EPL or open a `.zpl` / `.epl` file, pick a label size (4×6, 4×4, …)
- **Live preview** with debounced auto-render, zoom fit / percent, dark & light themes, English / 简体中文
- One-click **PNG / PDF download**, copy-PNG to clipboard, `Ctrl+S`
- **Compare with Labelary** (ZPL) — fetches the reference render and scores the diff on the same scale as CI
- Shareable permalinks that encode the label + settings in the URL hash

```bash
labelize serve --port 8080     # or: docker run -p 8080:8080 goodboy008/labelize
```

## 🖼️ Render Quality

Labelize is calibrated against the Labelary reference renderer with per-label
pixel-diff tolerance thresholds — 128 real-world carrier and synthetic labels
are compared on every push. Left = Labelary reference, right = Labelize.

| Label | Diff | Preview |
|-------|------|---------|
| amazon | 1.08% | <img src="testdata/diffs/amazon.png" height="140"> |
| dhlpaket | 1.48% | <img src="testdata/diffs/dhlpaket.png" height="140"> |
| ups | 2.98% | <img src="testdata/diffs/ups.png" height="140"> |
| fedex | 4.85% | <img src="testdata/diffs/fedex.png" height="140"> |

**128 labels tested** — 8 perfect · 61 good (<1%) · 52 minor (<5%) · 7 moderate (<15%) · 0 high

All side-by-side images: [`testdata/diffs/`](testdata/diffs/) ·
full reports: [labels](testdata/diffs/diff_report_labels.txt) · [unit](testdata/diffs/diff_report_unit.txt) ·
thresholds: [`docs/DIFF_THRESHOLDS.md`](docs/DIFF_THRESHOLDS.md)

## 📚 Documentation

Step-by-step tutorials for every platform live in [`docs/tutorials/`](docs/tutorials/README.md):

| Guide | For |
|-------|-----|
| [Command Line](docs/tutorials/cli.md) | Converting files from a shell |
| [Docker](docs/tutorials/docker.md) | Running the service in a container |
| [HTTP Service](docs/tutorials/http-service.md) | REST API & playground integration |
| [JavaScript / WebAssembly](docs/tutorials/javascript-wasm.md) | Client-side rendering |
| [Android](docs/tutorials/android.md) | Rendering inside an app |
| [Rust Library](docs/tutorials/rust-library.md) | Embedding the engine in Rust |

Reference docs: [Usage](docs/USAGE.md) · [ZPL command matrix](docs/ZPL_COMMANDS_REFERENCE.md) · [Diff thresholds](docs/DIFF_THRESHOLDS.md) · [Changelog](CHANGELOG.md)

## 📖 Reference

<details>
<summary><b>CLI reference</b></summary>

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
  --dpmm <N>            Dots per mm: 6, 8, 12, or 24 [default: 8]
  --antialias           8-bit grayscale output (default: 1-bit)

Serve Options:
  --host <HOST>         Bind address [default: 0.0.0.0]
  -p, --port <PORT>     Listen port [default: 8080]
```

</details>

<details>
<summary><b>HTTP API</b></summary>

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/`       | GET  | Interactive web playground |
| `/health` | GET  | Health check → `{"status":"ok"}` |
| `/convert` | POST | Convert label data → PNG or PDF |

**POST /convert** — the parser is selected by `Content-Type` (`application/zpl` / `application/epl`):

| Parameter | Default | Description |
|-----------|---------|-------------|
| `width`   | 102     | Label width in mm |
| `height`  | 152     | Label height in mm |
| `dpmm`    | 8       | Dots per mm |
| `output`  | png     | Output format: png/pdf |
| `antialias` | false | Preserve antialiased greys instead of 1-bit |

Status codes: `200` success (body is PNG/PDF bytes) · `400` parse error (bad label data) · `500` render error. Only the first label in the body is rendered — split multi-label files, or use the CLI.

</details>

<details>
<summary><b>Supported commands</b></summary>

### ZPL

DataMatrix ECC 200 field-data escapes and firmware defaults are described in
[DataMatrix field data](docs/DATAMATRIX_FIELD_DATA.md).

| Category | Commands |
|----------|----------|
| **Text & Font** | `^FO` `^FT` `^FD` `^FS` `^A` `^A@` (named font) `^CF` `^CW` (font identifier) `^FB` `^FR` `^FH` `^FN` `^FW` `^FV` |
| **Barcodes** | `^BC` (Code 128) `^BE` (EAN-13) `^B8` (EAN-8) `^B9` (UPC-E) `^BU` (UPC-A) `^B2` (Interleaved 2-of-5) `^B3` (Code 39) `^B7` (PDF417) `^BO` (Aztec) `^BX` (DataMatrix) `^BQ` (QR Code) `^BD` (MaxiCode) `^BY` (defaults) |
| **Graphics** | `^GB` (box) `^GC` (circle) `^GD` (diagonal) `^GE` (ellipse) `^GF` (graphic field) `^GS` (symbol) `~DG` (download graphic) `^IL` `^XG` `^ID` `^IM` `^IS` `~EG` |
| **Label Control** | `^XA` `^XZ` `^PW` `^PO` `^PM` (persistent mirror image) `^LH` `^LR` `^LT` (label top) `^LS` (label shift) `^LL` (label length) `^CI` `^MU` (units) `^PQ` (print quantity) `^FX` (comment) `^SN`/`^SF` (serial state) |
| **Stored Formats** | `^DF` `^XF` |

DataMatrix rendering supports **ECC 000, 050, 080, 100, 140 and 200**. Omitted or empty ZPL
`^BX` quality defaults to ECC 000, as specified by Zebra; use `^BXN,4,200`
for modern ECC 200. The Legacy path supports six encodation formats,
CRC, convolutional protection, randomization and square symbols up to 49 modules.
Legacy ZPL preserves raw field bytes and `^FH` bytes, including stored-format
recalls. Backslashes and pipes remain literal, matching the recorded CI13/CI27
printer probes; control bytes can be supplied through `^FH`.
Numeric records above 511 remain explicitly unsupported.
They never silently become ECC 200. The raw Legacy encoder API accepts bytes.
EPL DataMatrix continues to use ECC 200. See [Legacy scope and evidence](docs/DATAMATRIX_LEGACY.md)
for the norm-based implementation, printer observations, limitations and source
attribution. No independent overall validation is claimed.

### EPL

`N` (new label) · `A` (text) · `B` (1D barcodes — Code 128, Code 39, EAN-13/8, UPC-A/E, 2-of-5, Codabar, …) · `b` (2D: QR, DataMatrix, Aztec, PDF417, MaxiCode) · `LO`/`LW` (black/white line) · `LS` (diagonal) · `X` (box) · `GW` (graphic write) · `R` (reference point) · `P` (print)

</details>

<details>
<summary><b>Architecture</b></summary>

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

- **Parser** (`src/parsers/`) — tokenizes input, maintains `VirtualPrinter` state, produces typed `LabelElement`s
- **Renderer** (`src/drawers/`) — rasterizes elements onto an `RgbaImage` canvas (text, graphics, barcodes, reverse print, label inversion)
- **Encoder** (`src/images/`) — emits 1-bit monochrome PNG or a single-page PDF

</details>

## 🛠️ Development

```bash
cargo build --release                 # binary: target/release/labelize
cargo test                            # all tests (124 golden E2E + unit)
cargo test --test 'e2e_*'             # golden-file tests vs Labelary references
cargo test --test 'unit_*'            # unit tests
PATH="$PATH:target/debug" bash e2e/http/test_http.sh   # HTTP integration
PATH="$PATH:target/debug" bash e2e/cli/test_cli.sh     # CLI integration
```

After any rendering change, regenerate and commit the diff artifacts
(`cargo test --test e2e_diff_report -- --nocapture`) — CI enforces this via the
golden-staleness check. See [AGENTS.md](AGENTS.md) for the full workflow.

## 💡 Use Cases

- **Shipping label preview** — see exactly what prints before it hits the printer
- **Warehouse management** — batch-convert label templates to PDF for archival
- **E-commerce integrations** — generate label PNGs on the fly from a microservice
- **Automated QA** — validate label content in CI/CD with golden-file tests
- **Label design tools** — add real-time ZPL preview with the library or wasm API

## 🤝 Contributing

Contributions are welcome! Bug reports, new ZPL/EPL commands, rendering
improvements, and docs fixes all count — see [AGENTS.md](AGENTS.md) for the
development workflow, then open an issue or submit a pull request.

## 📄 License

[MIT AND BSD-3-Clause](LICENSE) — see [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) for embedded assets.

---

<sub>Looking for a <b>ZPL renderer</b>, <b>ZPL to PNG converter</b>, <b>ZPL to PDF</b>, <b>EPL parser</b>, <b>Zebra label preview</b>, <b>thermal label rendering</b>, or a <b>Labelary alternative</b>? Labelize covers all of these.</sub>
