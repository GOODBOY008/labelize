# Labelize Tutorials

Step-by-step guides for using Labelize on each platform. Every platform uses
the same Rust rendering engine and supports ZPL, EPL, TSPL/TSC, CPCL and DPL.
Match input bytes, dimensions and output options when comparing platforms;
wrapper defaults can differ. For reference material, see the
[documentation index](../README.md).

| Tutorial | Platform | Start here if you want to… |
|----------|----------|---------------------------|
| [Command Line](cli.md) | macOS · Linux · Windows | Convert label files to PNG/PDF from a shell |
| [Docker](docker.md) | Any host with Docker | Run a self-hosted conversion service in a container |
| [HTTP Service](http-service.md) | Bare metal · container · Cloudflare Workers | Convert labels over a REST API, use the web playground |
| [JavaScript / WebAssembly](javascript-wasm.md) | Node.js · bundlers · browsers | Render labels client-side with `@goodboy008/labelize-wasm` |
| [Android](android.md) | Android (minSdk 24) | Render labels inside an Android app via the AAR |
| [Rust Library](rust-library.md) | Any Rust project | Call the parser/renderer directly from Rust code |

## Picking a surface

- **One-off conversions** → [CLI](cli.md)
- **Service for other systems to call** → [Docker](docker.md) or [HTTP service](http-service.md)
- **No server, render in the user's browser** → [JavaScript/WASM](javascript-wasm.md)
- **Mobile app** → [Android](android.md)
- **Already in Rust** → [Rust library](rust-library.md)

## Common concepts

A few terms shared by every tutorial:

- **Input language** — ZPL, EPL, TSPL/TSC, CPCL or DPL. Select it explicitly
  or use the detection supported by the platform: file extension, content
  type or input bytes. See each tutorial for its defaults.
- **Label size** — width × height in millimetres. Native high-level APIs use
  explicit dimensions, then supported label-declared dimensions, then
  102 × 152 mm (approximately 4″ × 6″). The hosted Worker's raster mode defaults
  omitted dimensions to 102 × 152 mm; its vector mode uses declared dimensions
  before that fallback. Send explicit dimensions when needed.
- **dpmm** — dots per millimetre, i.e. print resolution. 8 dpmm = 203 dpi
  (most common), 6 dpmm = 152 dpi, 12 dpmm = 300 dpi, 24 dpmm = 600 dpi.
  Higher dpmm → larger pixel dimensions for the same physical size.
- **PNG vs PDF** — raster output is the default: PNG is black/white unless
  antialiasing is enabled, and PDF embeds the rendered image. Both PNG modes
  use 8-bit grayscale samples. Opt-in [vector output](../VECTOR_OUTPUT.md)
  adds searchable native PDF text, scalable geometry, multipage PDF and PNG
  scaling independent of the source printer resolution.

## Try it without installing anything

The public playground runs the latest `main` build:
<https://labelize.764629910.workers.dev> — paste ZPL/EPL/TSPL/CPCL/DPL, preview, download
PNG/PDF, and optionally score your render against the
[Labelary](http://labelary.com/) reference.
