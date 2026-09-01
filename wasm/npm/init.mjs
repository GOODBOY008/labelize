// @goodboy008/labelize-wasm — Node.js entry.
//
// The main package entry (`@goodboy008/labelize-wasm`) is the wasm-bindgen
// bundler glue for webpack/Vite-style bundlers. This `init` subpath is for
// Node.js (and any environment that can read the wasm file directly): it
// instantiates the engine from the packaged .wasm without bundler support or
// flags, then re-exports the wasm-bindgen wrappers.
//
// Usage (Node ≥ 20):
//   import { lz_render } from "@goodboy008/labelize-wasm/init";
//   const png = lz_render(zplBytes, 102.0, 152.0, 8, false, false, false);
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  __wbg_set_wasm,
  lz_render,
  lz_playground_html,
  __wbindgen_init_externref_table,
  __wbindgen_cast_0000000000000001,
} from "./labelize_wasm_bg.js";

const bytes = readFileSync(
  fileURLToPath(new URL("./labelize_wasm_bg.wasm", import.meta.url)),
);

const { instance } = await WebAssembly.instantiate(bytes, {
  "./labelize_wasm_bg.js": {
    __wbindgen_init_externref_table,
    __wbindgen_cast_0000000000000001,
  },
});
__wbg_set_wasm(instance.exports);
instance.exports.__wbindgen_start();

export { lz_render, lz_playground_html };