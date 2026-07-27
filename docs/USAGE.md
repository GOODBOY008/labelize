# Usage Guide

## Installation

### From Source

```bash
cd labelize
cargo install --path .
```

### From Homebrew (macOS/Linux)

```bash
brew tap GOODBOY008/homebrew-labelize
brew install labelize
```

### From GitHub Releases

Download the pre-built binary for your platform from [Releases](https://github.com/GOODBOY008/labelize/releases), extract, and place in your PATH.

---

## CLI Tool

### Convert a ZPL file to PNG

```bash
labelize convert label.zpl
# → label.png (same directory)
```

### Convert an EPL file to PNG

```bash
labelize convert label.epl
# → label.png (auto-detected from extension)
```

### Convert a TSPL file to PNG

```bash
labelize convert label.tspl
# → label.png (TSPL SIZE controls dimensions unless overridden)
```

### Specify output path

```bash
labelize convert label.zpl -o output/my_label.png
```

### Generate PDF instead of PNG

```bash
labelize convert label.zpl -t pdf
# → label.pdf
```

### Override input format detection

```bash
labelize convert data.txt --format zpl
labelize convert data.txt --format tspl
```

### Custom label dimensions

```bash
labelize convert label.zpl --width 100 --height 150 --dpmm 12
```

| Option       | Default | Description                              |
|-------------|---------|------------------------------------------|
| `--width`   | 102 or TSPL `SIZE` | Label width override in mm       |
| `--height`  | 152 or TSPL `SIZE` | Label height override in mm      |
| `--dpmm`    | 8       | Dots per mm (6, 8, 12, or 24)           |
| `-f, --format`  | auto    | Input format: `zpl`, `epl`, or `tspl` |
| `-t, --type`    | png     | Output type: `png` or `pdf`          |
| `-o, --output`  | auto    | Output file path                     |

### Multi-label files

When a ZPL file contains multiple `^XA…^XZ` blocks, each label is output as a separate file with a numeric suffix:

```bash
labelize convert multi.zpl
# → multi_1.png, multi_2.png, multi_3.png, …
```

---

## HTTP Server

### Start the server

```bash
labelize serve
# Listening on 0.0.0.0:8080
```

### Custom host and port

```bash
labelize serve --host 127.0.0.1 --port 3000
```

### Health check

```bash
curl http://localhost:8080/health
# {"status":"ok"}
```

### Convert ZPL to PNG via HTTP

```bash
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ' \
  -o label.png
```

### Convert EPL to PNG via HTTP

```bash
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/epl" \
  --data-binary @label.epl \
  -o label.png
```

### Convert TSPL to PNG via HTTP

```bash
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/tspl" \
  --data-binary @label.tspl \
  -o label.png
```

### Convert to PDF via HTTP

Add `?output=pdf` to the URL:

```bash
curl -X POST "http://localhost:8080/convert?output=pdf" \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ' \
  -o label.pdf
```

### Custom dimensions via query parameters

```bash
curl -X POST "http://localhost:8080/convert?width=100&height=62&dpmm=12" \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ' \
  -o label.png
```

| Parameter | Default | Description             |
|-----------|---------|-------------------------|
| `width`   | 102 or TSPL `SIZE` | Label width override in mm |
| `height`  | 152 or TSPL `SIZE` | Label height override in mm |
| `dpmm`    | 8       | Dots per mm             |
| `output`  | png     | Output format: png/pdf  |

---

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
labelize = { path = "." }
```

### Parse and render a ZPL string

```rust
use std::io::Cursor;
use labelize::{ZplParser, Renderer, DrawerOptions};

fn main() {
    let zpl = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";
    let mut parser = ZplParser::new();
    let labels = parser.parse(zpl).unwrap();

    let renderer = Renderer::new();
    let opts = DrawerOptions::default();
    let mut buf = Cursor::new(Vec::new());
    renderer.draw_label_as_png(&labels[0], &mut buf, opts).unwrap();
    std::fs::write("output.png", buf.into_inner()).unwrap();
}
```

### Render to PDF

```rust
use std::io::Cursor;
use labelize::{ZplParser, Renderer, DrawerOptions, encode_pdf};

fn main() {
    let zpl = b"^XA^FO50,50^A0N,40,40^FDHello^FS^XZ";
    let mut parser = ZplParser::new();
    let labels = parser.parse(zpl).unwrap();

    let opts = DrawerOptions::default();
    let renderer = Renderer::new();

    // Render to image
    let mut png_buf = Cursor::new(Vec::new());
    renderer.draw_label_as_png(&labels[0], &mut png_buf, opts.clone()).unwrap();

    let img = image::load_from_memory(&png_buf.into_inner()).unwrap().to_rgba8();

    // Encode as PDF
    let mut pdf_buf = Cursor::new(Vec::new());
    encode_pdf(&img, &opts, &mut pdf_buf).unwrap();
    std::fs::write("output.pdf", pdf_buf.into_inner()).unwrap();
}
```

---

## Supported ZPL Commands

| Command | Description                    |
|---------|--------------------------------|
| `^XA`   | Start label format             |
| `^XZ`   | End label format               |
| `^FO`   | Field origin                   |
| `^FT`   | Field typeset                  |
| `^FD`   | Field data                     |
| `^FS`   | Field separator                |
| `^A`    | Font selection                 |
| `^CF`   | Default font                   |
| `^FB`   | Field block (word wrap)        |
| `^FR`   | Field reverse print            |
| `^FH`   | Hex escape in field data       |
| `^FN`   | Field number (stored formats)  |
| `^FW`   | Field orientation              |
| `^BC`   | Code 128 barcode               |
| `^BE`   | EAN-13 barcode                 |
| `^B3`   | Code 39 barcode                |
| `^B2`   | Interleaved 2-of-5 barcode    |
| `^BO`   | Aztec barcode                  |
| `^BX`   | DataMatrix barcode             |
| `^BQ`   | QR Code                        |
| `^B7`   | PDF417 barcode                 |
| `^BD`   | MaxiCode                       |
| `^BY`   | Barcode defaults               |
| `^GB`   | Graphic box                    |
| `^GC`   | Graphic circle                 |
| `^GD`   | Graphic diagonal line          |
| `^GF`   | Graphic field                  |
| `^GS`   | Graphic symbol                 |
| `^PW`   | Print width                    |
| `^PO`   | Print orientation              |
| `^LH`   | Label home                     |
| `^LR`   | Label reverse                  |
| `^CI`   | Character set                  |
| `~DG`   | Download graphics              |
| `^IL`   | Recall stored image            |
| `^XG`   | Recall graphic                 |
| `^DF`   | Download format                |
| `^XF`   | Recall format                  |

## Supported EPL Commands

| Command | Description                    |
|---------|--------------------------------|
| `N`     | New label                      |
| `A`     | Text field                     |
| `B`     | Barcode                        |
| `LO`    | Line draw (graphic box)        |
| `R`     | Reference point                |
| `P`     | Print label                    |

## Supported TSPL Commands

| Category | Commands |
|----------|----------|
| **Layout** | `SIZE` `DIRECTION` `REFERENCE` `SHIFT` `CLS` `PRINT` |
| **Text** | `TEXT` |
| **Graphics** | `BAR` `BOX` `CIRCLE` `ELLIPSE` `ERASE` `REVERSE` `BITMAP` |
| **Barcodes** | `BARCODE` (`128`, `EAN128`, `39`, `39C`, `EAN13`) `QRCODE` `PDF417` |

Printer-control commands such as `GAP`, `SPEED`, `DENSITY`, and `SET` are parsed as no-ops because they do not affect rendered output.
