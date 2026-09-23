# Labelize Tutorials

Step-by-step guides for using Labelize on each platform. All tutorials use the
same Rust rendering engine — output is identical whether you convert from the
CLI, an HTTP call, WebAssembly, or Android.

| Tutorial | Platform | Start here if you want to… |
|----------|----------|---------------------------|
| [Command Line](cli.md) | macOS · Linux · Windows | Convert `.zpl`/`.epl` files to PNG/PDF from a shell |
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

- **ZPL / EPL** — the two label languages Labelize parses. Format is
  auto-detected from the file extension (`.zpl` / `.epl`) or the
  `Content-Type` header (`application/zpl` / `application/epl`) depending on
  the surface.
- **Label size** — width × height in millimetres. The default is 102 × 152 mm
  (4″ × 6″). If you don't know the size, it's usually encoded in the label
  itself (`^PW` for width in ZPL) or known from the printer stock.
- **dpmm** — dots per millimetre, i.e. print resolution. 8 dpmm = 203 dpi
  (most common), 6 dpmm = 152 dpi, 12 dpmm = 300 dpi, 24 dpmm = 600 dpi.
  Higher dpmm → larger pixel dimensions for the same physical size.
- **PNG vs PDF** — every surface can emit both. PNG is monochrome (thermal
  faithful); PDF embeds the same render in a single-page document.

## Try it without installing anything

The public playground runs the latest `main` build:
<https://labelize.764629910.workers.dev> — paste ZPL/EPL, preview, download
PNG/PDF, and optionally score your render against the
[Labelary](http://labelary.com/) reference.
