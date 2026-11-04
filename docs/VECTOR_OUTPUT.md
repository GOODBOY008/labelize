# Vector PDF and scalable PNG

[Documentation index](README.md)

The optional `vector` feature compiles a language-neutral drawing scene from
the existing label IR. Positioned glyphs, barcode modules, curves, clipping,
rotations and physical dimensions are shared between the PDF and PNG consumers.
The PDF backend uses krilla with embedded subset fonts and Unicode mappings;
the PNG backend rasterizes those same coordinates with tiny-skia. It does not
serialize or reparse SVG, and never changes source parsing resolution to scale
an output image.

Raster PNG/PDF remains the default. Existing `render_png`, `render_pdf`,
`lz_render`, `lz_render_lang` and Android methods retain their contracts.
The new output functions validate their options before rendering. The release
build configurations, Docker, WASM and Android include the optional backend;
Rust library users explicitly enable `features = ["vector"]`. krilla 0.8.2
requires Rust 1.92 or later.

## CLI and HTTP

```sh
cargo build --release --features serve,vector
labelize convert label.zpl --mode vector --type pdf
labelize convert label.zpl --mode vector --scale 3 --antialias
labelize convert batch.tspl --mode vector --type pdf --all-pages -o batch.pdf
```

Without `--all-pages`, the CLI retains its numbered-file behavior for inputs
containing several labels. A multipage PDF preserves each label's declared
size unless the caller supplies explicit width/height overrides.

```sh
curl --data-binary @label.zpl -H 'Content-Type: application/zpl' \
  'http://localhost:8080/convert?mode=vector&output=pdf' -o label.pdf
curl --data-binary @label.zpl -H 'Content-Type: application/zpl' \
  'http://localhost:8080/convert?mode=vector&scale=3&antialias=true' -o label.png
curl --data-binary @batch.tspl -H 'Content-Type: application/tspl' \
  'http://localhost:8080/convert?mode=vector&output=pdf&pages=all' -o batch.pdf
```

| Option | Default | Contract |
| --- | --- | --- |
| `mode` | `raster` | `raster` or `vector`; unavailable builds reject `vector` |
| `scale` | `1` | Finite positive PNG pixels per source dot; only vector PNG accepts another value |
| `pages` | `first` | `first` or `all`; `all` requires vector PDF |
| `dpmm` | `8` | Source printer resolution; unaffected by `scale` |
| `antialias` | `false` | Keep grayscale PNG coverage instead of black/white thresholding |

Invalid settings return HTTP 400. Parse, render and encoding failures keep the
existing stage distinction. PNG stores eight-bit grayscale samples in both
visual modes; it writes `pHYs` resolution as `dpmm × scale × 1000` pixels/metre.
`scale=3` at 8 dpmm means 24 output pixels/mm (609.6 dpi), while coordinates
and font metrics are still interpreted at 8 dpmm.

The scene limits a source canvas to 16 Mi pixels, an output PNG to 64 Mi pixels,
and either dimension to 32000 dots/pixels. PDF dimensions are millimetres
converted to points independently of pixel rounding. Scaling does not add
detail to source bitmaps or local raster regions.

## Rust

```rust
use labelize::{render_pdf_with_output, OutputMode, OutputOptions, RenderOptions};

let output = OutputOptions {
    mode: OutputMode::Vector,
    all_pages: true,
    ..Default::default()
};
let pdf = render_pdf_with_output(b"^XA^FO20,20^A0N,30,20^FDHello^FS^XZ",
    RenderOptions::default(), output)?;
# Ok::<(), labelize::LabelizeError>(())
```

Use `render_png_with_output` for PNG, or `render_label_with_output` for an
already parsed label. The existing `RenderOptions` struct was not expanded.
For repeated exports, create `LabelScene::new(&label, drawer_options)` once,
then call `to_pdf()` or `to_png(ScenePngOptions { scale, antialias })`.
`encode_scenes_pdf(&scenes)` combines scenes with different page sizes.
`LabelScene::stats()` reports actual glyph/path/image counts and each local
fallback's source element index, reason and bounds. Enable `serde` to serialize
those diagnostics.

## WASM, playground and Android

The new WASM export, also available from the Node `init` subpath, is:

```js
lz_render_output(src, widthMm, heightMm, dpmm, antialias, wantPdf,
                 language, "vector", scale, allPages);
```

Zero width/height selects declared sizes and then the standard fallback.
An empty language requests detection; unknown nonempty language names fail.
Errors retain `1:` (input/options) and `2:` (render/encode) prefixes.
The Worker exposes the same HTTP query parameters as the server.

The playground adds **Output**, **PNG scale**, and **All PDF pages** controls
when the build supports vector output. Share links retain these settings.
PDF downloads omit PNG scale; Labelary pixel comparison requires scale 1.

Android provides `renderOutput(...)`, `renderVectorPdf(...)` and
`renderScaledPng(...)`. They use the same option validation and retain
`LabelizeException` stages. WASM and Android enable the feature by default;
`cargo build --no-default-features` produces their smaller raster-only build.

## Hybrid output and fidelity boundaries

This mode does not promise a PDF containing only paths and fonts. Bitmap
fields retain their source resolution. The scene records local pixel regions for:

- Background-dependent field inversion and bitmap XOR.
- Shallow multi-dot diagonals whose horizontal band is narrower than one
  source dot perpendicular to the line; preserving the raster polygon's ink
  keeps these lines visible in monochrome output.
- Print-time region inversion, applied after ordinary fields.
- Source text overlapping barcode modules or their quiet zone: preserve the existing pixels in
  that region because small changes in text edges can damage the symbol.

The PDF backend represents opaque black/white regions as vertically merged
pixel rectangles, up to 4,096 rectangles per region. This avoids viewer image
resampling changing barcode modules. A 0.0001-source-dot inward edge guard
prevents floating-point boundary coverage from tinting neighboring white pixels.
Grayscale, transparency and more complex images keep their image encoding.
These pixel rectangles do not reconstruct smooth detail from a bitmap, and
`LabelScene::stats().images` counts scene image operations, not PDF resources.

Ordinary text, text blocks, all rotations, barcode interpretation lines,
barcode modules and geometric shapes have native scene representations.
One-dot diagonals retain one dot of perpendicular stroke width at shallow
and steep angles, so they remain visible in monochrome scaled PNG output.
MaxiCode uses shared module centers and circular geometry. The resident-font
zero repair emits corrected outlines with an invisible searchable text layer.
Every native glyph uses its explicit layout position, avoiding accumulated
rounding of fractional CID font widths in PDF readers.
`ActualText` spans preserve logical strings, including justified spaces. Some
extractors (notably pypdf) ignore that metadata and infer extra spaces in wide
resident fonts or superscripts; PDFium and Poppler preserve those strings.
Reader-inferred line order and text-selection boxes can differ; lopdf's raw
extractor inserts a newline between separately positioned glyphs.
Missing font characters remain subject to the existing font/parser coverage.

PDF viewers use their own font hinting, antialiasing and pixel snapping.
Their low-resolution renderings need not be pixel-identical to legacy PNG or
scene PNG, and low-resolution whole-page barcode detection can vary. Use the
legacy output when exact existing pixels are the requirement. Vector output
preserves physical layout and scales continuous text/shape geometry; it does
not make an overlapping, clipped or otherwise invalid source label valid.

See [Testing](TESTING.md#vector-pdf-and-png) for the separate structural,
search, visual and process checks, and the
[independent verification guide](../e2e/vector/README.md) for corpus commands
and generated reports.
