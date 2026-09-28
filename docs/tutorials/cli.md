# Tutorial: Command Line (macOS · Linux · Windows)

Convert `.zpl` / `.epl` files to PNG or PDF from a shell. Format is
auto-detected from the file extension.

## 1. Install

**macOS / Linux (Homebrew)**

```bash
brew tap GOODBOY008/homebrew-labelize
brew install labelize
```

**Any OS with a Rust toolchain** (builds from source, ~1 min)

```bash
cargo install labelize --features cli
# or, from a checkout of this repo:
cargo install --path . --features cli
```

**Windows (pre-built binary)**

1. Go to <https://github.com/GOODBOY008/labelize/releases>
2. Download `labelize-x86_64-pc-windows-msvc.zip`
3. Extract `labelize.exe` and put its folder on your `PATH`

(Windows with a Rust toolchain: `cargo install labelize --features cli` works too.)

Verify:

```bash
labelize --help
```

## 2. Your first conversion

```bash
labelize convert label.zpl          # → label.png, next to the input
```

EPL works the same way — `labelize convert label.epl`. If the file has a
different extension, force the format:

```bash
labelize convert data.txt --format zpl
```

A file containing several labels (`^XA…^XZ` repeated, or several `P` commands
in EPL) produces one output per label: `label_1.png`, `label_2.png`, …

## 3. Choose the output type and path

```bash
labelize convert label.zpl -t pdf            # → label.pdf (single-page PDF)
labelize convert label.zpl -o out/label.png  # explicit output path
```

## 4. Get the size right

The default canvas is 102 × 152 mm (4″ × 6″) at 8 dpmm. If your label looks
cropped or has excess whitespace, override:

```bash
labelize convert label.zpl --width 100 --height 62   # 100 × 62 mm
labelize convert label.zpl --dpmm 12                 # 300 dpi → bigger pixels
labelize convert label.zpl --dpmm 6                  # 152 dpi
```

Tip: many carrier labels declare their width with `^PW` (in dots). Divide by
the dpmm to get millimetres — e.g. `^PW812` at 8 dpmm ≈ 101.5 mm.

## 5. Antialiased output (optional)

By default the PNG is 1-bit black/white, which is what a thermal printer
actually prints. To keep antialiased greys (closer to Labelary's preview):

```bash
labelize convert label.zpl --antialias
```

## 6. Batch conversion

The CLI converts one file per invocation; loop in your shell:

```bash
for f in labels/*.zpl; do labelize convert "$f"; done
```

For recurring service-style workloads, consider the
[HTTP service](http-service.md) instead.

## 7. What's next

- [HTTP service tutorial](http-service.md) — same engine as a REST API with a
  built-in web playground
- [Docker tutorial](docker.md) — run the service in a container
- `labelize convert --help` — the authoritative flag list
