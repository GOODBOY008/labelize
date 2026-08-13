# @goodboy008/labelize-wasm

The [Labelize](https://github.com/GOODBOY008/labelize) ZPL/EPL label engine compiled to WebAssembly.
Parse and render ZPL/EPL label data to PNG (or PDF) entirely in the browser, Node.js, or your bundler —
no server needed.

## Usage

### Bundlers (webpack 5, Vite with `vite-plugin-wasm`)

```js
import { lz_render } from "@goodboy008/labelize-wasm";

const zpl = new TextEncoder().encode("^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ");
const png = lz_render(zpl, 102.0, 152.0, 8, false, false, false);
// png is a Uint8Array — pass it to an <img> via URL.createObjectURL
```

The package is the standard wasm-bindgen bundler output: `labelize_wasm.js` initializes
the engine at module top level and exports `lz_render` / `lz_playground_html`.

### Node.js (≥ 20)

```js
import { lz_render } from "@goodboy008/labelize-wasm/init";

const zpl = Buffer.from("^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ", "ascii");
const png = lz_render(zpl, 102.0, 152.0, 8, false, false, false);
```

The `/init` subpath instantiates the engine from the packaged `.wasm` directly,
without bundler support or Node flags.

### API

`lz_render(src, width_mm, height_mm, dpmm, grayscale, want_pdf, is_epl) -> Uint8Array`

| param | meaning |
|---|---|
| `src` | raw label bytes (ZPL, or EPL when `is_epl = true`) |
| `width_mm` / `height_mm` | label canvas size in millimetres (defaults: 102 × 152) |
| `dpmm` | dots per millimetre (default 8) |
| `grayscale` | emit 8-bit grayscale output keeping renderer greys instead of thresholding to 1-bit |
| `want_pdf` | return a PDF instead of PNG |
| `is_epl` | parse input as EPL instead of ZPL |

Errors throw a string prefixed with the failure stage: `1:` parse error, `2:` rendering error.

`lz_playground_html() -> string` returns the Labelize playground page (identical to the
HTTP server's `/`).

## Release assets

GitHub releases also attach `labelize-wasm-wasm32.zip` with the raw glue + `.wasm` files,
for direct use without npm. See `wasm/build.sh` to rebuild the engine:

```bash
cd wasm && ./build.sh   # requires Rust wasm32 target + wasm-bindgen-cli
```

## License

MIT AND BSD-3-Clause