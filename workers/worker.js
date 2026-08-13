// Labelize Cloudflare Worker: / , /health, /convert (mirrors the axum server).
//
// The wasm-bindgen thin entry (labelize_wasm.js) auto-initializes with the
// bundler's wasm import, which workerd resolves to a module (not an instance),
// so we skip it: import the core glue functions and instantiate the engine
// manually. Workerd gives us the .wasm as bytes via esbuild's binary loader.
import {
  __wbg_set_wasm,
  lz_render,
  lz_playground_html,
  __wbindgen_init_externref_table,
  __wbindgen_cast_0000000000000001,
} from "../wasm/npm/labelize_wasm_bg.js";
import wasmBytes from "../wasm/npm/labelize_wasm_bg.wasm";

let ready = null;
let htmlPage = null;

function ensureEngine() {
  if (!ready) {
    ready = WebAssembly.instantiate(wasmBytes, {
      "./labelize_wasm_bg.js": {
        __wbindgen_init_externref_table,
        __wbindgen_cast_0000000000000001,
      },
    }).then((result) => {
      // Wrangler resolves the .wasm import to a WebAssembly.Module, so
      // instantiate() returns the plain instance; for byte-based loaders it
      // returns { module, instance }.
      const instance = result instanceof WebAssembly.Instance ? result : result.instance;
      __wbg_set_wasm(instance.exports);
      instance.exports.__wbindgen_start();
    });
  }
  return ready;
}

function htmlResponse() {
  return new Response(htmlPage, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-cache",
    },
  });
}

function jsonResponse(obj, status = 200) {
  return new Response(JSON.stringify(obj), {
    status,
    headers: { "content-type": "application/json" },
  });
}

// wasm-bindgen throws the Err string of Result<_, String>; unwrap it and map
// the "1:" / "2:" stage prefix back to HTTP 400 / 500.
function renderStatus(err) {
  const msg = err && err.message !== undefined ? err.message : String(err);
  const m = /^(\d):([\s\S]*)$/.exec(msg);
  if (m && m[1] === "1") return { status: 400, text: m[2] };
  return { status: 500, text: m ? m[2] : msg };
}

async function convert(request, url) {
  const ct = request.headers.get("content-type") || "application/zpl";
  const width = parseFloat(url.searchParams.get("width")) || 102.0;
  const height = parseFloat(url.searchParams.get("height")) || 152.0;
  const dpmm = parseInt(url.searchParams.get("dpmm"), 10) || 8;
  const grayscale =
    url.searchParams.get("grayscale") === "true" ||
    url.searchParams.get("antialias") === "true"; // legacy alias
  const wantPdf = url.searchParams.get("output") === "pdf";
  const isEpl = ct.includes("epl");

  const body = new Uint8Array(await request.arrayBuffer());
  if (body.length === 0) {
    return new Response("empty body", { status: 400 });
  }

  try {
    const png = lz_render(body, width, height, dpmm, grayscale, wantPdf, isEpl);
    return new Response(png, {
      headers: { "content-type": wantPdf ? "application/pdf" : "image/png" },
    });
  } catch (err) {
    // Rust panic with panic=abort surfaces here as a wasm trap or thrown Error.
    const { status, text } = renderStatus(err);
    return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8" } });
  }
}

export default {
  async fetch(request, env, ctx) {
    await ensureEngine();
    if (!htmlPage) {
      htmlPage = lz_playground_html();
    }

    const url = new URL(request.url);
    if (request.method === "GET" && url.pathname === "/") return htmlResponse();
    if (request.method === "GET" && url.pathname === "/health") {
      return jsonResponse({ status: "ok" });
    }
    if (request.method === "POST" && url.pathname === "/convert") {
      return convert(request, url);
    }
    return jsonResponse({ error: "Not Found" }, 404);
  },
};