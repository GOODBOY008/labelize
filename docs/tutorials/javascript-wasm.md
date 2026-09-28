# Tutorial: JavaScript / WebAssembly

Render ZPL/EPL entirely client-side with
[`@goodboy008/labelize-wasm`](https://www.npmjs.com/package/@goodboy008/labelize-wasm) —
no server round-trip, works in Node.js, browsers, and bundlers. The public
playground at <https://labelize.764629910.workers.dev> is built on this
package.

## 1. Install

```bash
npm install @goodboy008/labelize-wasm
```

## 2. Node.js (≥ 20)

Import the `/init` subpath — it instantiates the engine from the packaged
`.wasm` directly, no bundler support or Node flags needed:

```js
import { lz_render } from "@goodboy008/labelize-wasm/init";
import { writeFileSync } from "node:fs";

const zpl = Buffer.from("^XA^FO50,50^A0N,40,40^FDHello Node^FS^XZ", "ascii");
const png = lz_render(zpl, 102.0, 152.0, 8, false, false, false);

writeFileSync("label.png", png); // Uint8Array of PNG bytes
```

## 3. Bundlers (webpack 5 · Vite)

Vite needs `vite-plugin-wasm`; webpack 5 supports wasm natively. With a
bundler, import the package root — initialization happens at module load:

```js
import { lz_render } from "@goodboy008/labelize-wasm";

const zpl = new TextEncoder().encode(
  "^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ"
);
const png = lz_render(zpl, 102.0, 152.0, 8, false, false, false);
```

Show it in the browser:

```js
const blob = new Blob([png], { type: "image/png" });
img.src = URL.createObjectURL(blob);
```

## 4. The `lz_render` API

```ts
lz_render(src, width_mm, height_mm, dpmm, antialias, want_pdf, is_epl)
  → Uint8Array
```

| Param | Meaning |
|-------|---------|
| `src` | raw label bytes (ZPL, or EPL when `is_epl = true`) |
| `width_mm` / `height_mm` | label canvas size in millimetres (defaults 102 × 152) |
| `dpmm` | dots per millimetre (default 8) |
| `antialias` | keep renderer greys instead of thresholding to 1-bit |
| `want_pdf` | return a PDF instead of PNG |
| `is_epl` | parse input as EPL instead of ZPL |

**Errors** throw a string prefixed with the failure stage:

- `1:…` — parse error (bad label data)
- `2:…` — rendering error (input parsed but rendering failed)

```js
try {
  lz_render(zpl, 102, 152, 8, false, false, false);
} catch (e) {
  if (String(e).startsWith("1:")) console.error("bad label data:", e);
  else throw e;
}
```

The package also exports `lz_playground_html(): string`, the standalone
playground page (same HTML the HTTP service serves at `/`).

## 5. Without npm

Every GitHub Release attaches `labelize-wasm-wasm32.zip` containing the raw
wasm-bindgen glue + `.wasm` files, for direct `<script>`-style use or
non-npm environments. To rebuild from source:

```bash
cd wasm && ./build.sh   # requires the Rust wasm32 target + wasm-bindgen-cli
```

## 6. What's next

- [Android tutorial](android.md) — the same engine as an AAR
- [HTTP service tutorial](http-service.md) — if you'd rather render server-side
