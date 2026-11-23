# Labelize

Render thermal printer labels to PNG or PDF in Rust. Labelize reads **ZPL, EPL,
TSPL/TSC, CPCL and DPL** and runs offline with embedded fonts. It can also turn a
raster image into a ZPL or TSPL graphic for printing.

## Render your first label

In a Cargo binary project, add the library.

```sh
cargo add labelize
```

Save this as `src/main.rs` and run `cargo run --quiet > label.png`.

```rust
use labelize::{render_png, LabelizeError, RenderOptions};
use std::io::{self, Write};

fn main() -> Result<(), LabelizeError> {
    let zpl = b"^XA^FO24,24^A0N,36,36^FDHello, Labelize!^FS^XZ";
    let png = render_png(zpl, RenderOptions::default())?;
    io::stdout().lock().write_all(&png.bytes)?;
    Ok(())
}
```

`render_png` and `render_pdf` detect the language and render the **first label**.
The result's `bytes` hold the encoded file, `language` identifies the parser used,
and `labels` counts **all labels in the input**. Set `width_mm`, `height_mm` or
`dpmm` in `RenderOptions` to match your label stock and printhead.

## Common tasks

| Task | How |
| --- | --- |
| Preview every label in a batch | [`parse_labels`](https://docs.rs/labelize/latest/labelize/lang/fn.parse_labels.html) returns one `LabelInfo` per label. Pass each one to `Renderer::draw_label_as_png`. |
| Check input before rendering | [`validate`](https://docs.rs/labelize/latest/labelize/lang/fn.validate.html) reports the detected language, label count and any parse error. |
| Print an existing image | [`encode_image_bytes`](https://docs.rs/labelize/latest/labelize/reverse/fn.encode_image_bytes.html) converts encoded image bytes to ZPL graphic commands or a TSPL command stream with a binary bitmap. |
| Export a searchable, multipage PDF | Enable `vector` and call [`render_pdf_with_output`](https://docs.rs/labelize/latest/labelize/output/fn.render_pdf_with_output.html) with `OutputMode::Vector` and `all_pages: true`. Each parsed label gets its own page. |

## Cargo features

The default library supports all five input languages, raster PNG and PDF output,
validation and image-to-printer-code conversion. Opt in to additional features as needed.

| Feature | Use it to |
| --- | --- |
| `vector` | Export PDF with searchable text, combine labels into one PDF, or scale PNG output independently of printer resolution. Enable it with `cargo add labelize --features vector`. |
| `serde` | Serialize the detected `Language` and `ValidationReport`. |
| `cli` | Install the `labelize` command with `cargo install labelize --features cli`. |
| `serve` | Run the HTTP rendering service and web playground. Includes `cli`, `playground` and `serde`. |
| `playground` | Embed the web playground in your own application. |
| `skill` | Analyze rendering differences and extract ZPL snippets for diagnosis. |

[Rust API reference](https://docs.rs/labelize) · [Rust integration tutorial](https://labelize.764629910.workers.dev/#docs/rust) · [Try the playground](https://labelize.764629910.workers.dev)
