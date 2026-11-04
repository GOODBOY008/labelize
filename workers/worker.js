// Labelize Cloudflare Worker: shared playground and public synchronous API.
//
// The wasm-bindgen thin entry (labelize_wasm.js) auto-initializes with the
// bundler's wasm import, which workerd resolves to a module (not an instance),
// so we skip it: import the core glue functions and instantiate the engine
// manually. Workerd gives us the .wasm as bytes via esbuild's binary loader.
import * as engine from "../wasm/npm/labelize_wasm_bg.js";
const { lz_render_output, lz_playground_html, lz_validate, lz_format, lz_encode } = engine;
import wasmBytes from "../wasm/npm/labelize_wasm_bg.wasm";
import openapi from "../src/playground/openapi.json";

let ready = null;
let htmlPage = null;

function ensureEngine() {
  if (!ready) {
    ready = WebAssembly.instantiate(wasmBytes, {
      "./labelize_wasm_bg.js": engine,
    }).then((result) => {
      // Wrangler resolves the .wasm import to a WebAssembly.Module, so
      // instantiate() returns the plain instance; for byte-based loaders it
      // returns { module, instance }.
      const instance = result instanceof WebAssembly.Instance ? result : result.instance;
      engine.__wbg_set_wasm(instance.exports);
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

// Canonical public origin of the hosted playground. Once a custom domain is
// attached, set PUBLIC_BASE_URL (wrangler.toml [vars] or the dashboard) so
// canonical/OG/JSON-LD URLs, robots.txt and the sitemap consolidate on it
// and workers.dev 301-redirects to the domain.
const DEFAULT_BASE_URL = "https://labelize.764629910.workers.dev";

function baseUrl(env) {
  const raw = String((env && env.PUBLIC_BASE_URL) || "").trim().replace(/\/+$/, "");
  return raw || DEFAULT_BASE_URL;
}

// Mirrors labelize::playground::robots_txt / sitemap_xml.
function robotsTxt(base) {
  return `User-agent: *\nAllow: /\n\nSitemap: ${base}/sitemap.xml\n`;
}

function sitemapXml(base) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>${base}/</loc>
    <changefreq>monthly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
`;
}

function textResponse(body, type) {
  return new Response(body, {
    headers: { "content-type": type, "cache-control": "max-age=86400" },
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
  const ct = request.headers.get("content-type") || "";
  let width, height, dpmm, scale, antialias, wantPdf, mode, allPages;
  try {
    const parameter = (name, fallback) => {
      const values = url.searchParams.getAll(name);
      if (values.length > 1) throw new Error(`duplicate ${name}`);
      return values.length ? values[0] : fallback;
    };
    const positive = (name, fallback) => {
      const value = parameter(name, String(fallback));
      if (!/^(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/.test(value)) throw new Error(`invalid ${name}`);
      const number = Number(value);
      if (!Number.isFinite(number) || number <= 0) throw new Error(`invalid ${name}`);
      return number;
    };
    mode = parameter("mode", "raster");
    // The new mode uses each label's declared dimensions unless overridden.
    // Preserve the existing Worker's fixed defaults for raster requests.
    const dimension = (name, fallback) => mode.toLowerCase() === "vector" && !url.searchParams.has(name)
      ? 0 : positive(name, fallback);
    width = dimension("width", 102);
    height = dimension("height", 152);
    dpmm = positive("dpmm", 8);
    if (!Number.isInteger(dpmm) || dpmm > 2147483647) throw new Error("invalid dpmm");
    scale = positive("scale", 1);
    const output = parameter("output", "png");
    if (!["png", "pdf"].includes(output)) throw new Error("output must be png or pdf");
    wantPdf = output === "pdf";
    const aa = parameter("antialias", "false");
    if (!["true", "false"].includes(aa)) throw new Error("antialias must be true or false");
    antialias = aa === "true";
    const pages = parameter("pages", "first");
    if (!["first", "all"].includes(pages)) throw new Error("pages must be first or all");
    allPages = pages === "all";
  } catch (err) {
    return new Response(err.message, { status: 400 });
  }
  // The new API can detect a language from bytes; preserve the historical
  // ZPL fallback for raster requests with an unspecified Content-Type.
  const lang = ct.includes("epl")
    ? "epl"
    : ct.includes("tspl")
      ? "tspl"
      : ct.includes("cpcl")
        ? "cpcl"
        : ct.includes("dpl")
          ? "dpl"
          : ct.includes("zpl") || mode.toLowerCase() !== "vector"
            ? "zpl"
            : "";

  const body = new Uint8Array(await request.arrayBuffer());
  if (body.length === 0) {
    return new Response("empty body", { status: 400 });
  }

  try {
    const png = lz_render_output(body, width, height, dpmm, antialias, wantPdf, lang, mode, scale, allPages);
    return new Response(png, {
      headers: { "content-type": wantPdf ? "application/pdf" : "image/png" },
    });
  } catch (err) {
    // Rust panic with panic=abort surfaces here as a wasm trap or thrown Error.
    const { status, text } = renderStatus(err);
    return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8" } });
  }
}

// Validate: lz_validate never throws — the report JSON carries ok/error,
// mirroring the axum server's POST /validate response shape.
async function validate(request, url) {
  const dpmm = parseInt(url.searchParams.get("dpmm"), 10) || 8;
  const lang = url.searchParams.get("language") || "";
  const bad = badLanguage(lang);
  if (bad) return bad;
  const body = new Uint8Array(await request.arrayBuffer());
  return jsonResponse(JSON.parse(lz_validate(body, dpmm, lang)));
}

// Format: reflows the source one command per line; raw bytes so binary
// payloads (GW/BITMAP) survive unchanged, mirroring POST /format.
async function formatLabel(request, url) {
  const lang = url.searchParams.get("language") || "";
  const bad = badLanguage(lang);
  if (bad) return bad;
  const body = new Uint8Array(await request.arrayBuffer());
  const formatted = lz_format(body, lang);
  return new Response(formatted, {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}

// Encode: reverse rendering — a raster image in, printer code out.
// `language` picks the target: "zpl" → ASCII text, "tspl" → binary bytes.
// Mirrors the axum server's POST /encode.
async function encodeImage(request, url) {
  // Defaults apply only when absent. parseInt()/|| would silently accept
  // dpmm=0, 8junk, fractional values and overflowing wasm i32/u8 arguments.
  function integerParam(name, fallback, min, max) {
    const values = url.searchParams.getAll(name);
    if (!values.length) return fallback;
    const raw = values[0];
    const value = Number(raw);
    if (values.length !== 1 || !/^[+-]?\d+$/.test(raw) ||
        !Number.isSafeInteger(value) || (min === 0 && raw.startsWith("-")) || value < min || value > max) {
      throw new Error(`invalid ${name}`);
    }
    return value;
  }
  let dpmm, threshold;
  try {
    dpmm = integerParam("dpmm", 8, 1, 2147483647);
    threshold = integerParam("threshold", 128, 0, 255);
  } catch (err) {
    return new Response(err.message, { status: 400 });
  }
  const languages = url.searchParams.getAll("language");
  if (languages.length > 1) return new Response("duplicate language", { status: 400 });
  const lang = (languages[0] ?? "zpl").toLowerCase();
  if (lang !== "zpl" && lang !== "tspl") {
    return new Response(`unknown language ${lang} — expected zpl or tspl`, {
      status: 400,
      headers: { "content-type": "text/plain; charset=utf-8" },
    });
  }
  const body = new Uint8Array(await request.arrayBuffer());
  if (body.length === 0) {
    return new Response("empty body", { status: 400 });
  }
  try {
    const out = lz_encode(body, dpmm, threshold, lang);
    return new Response(out, {
      headers: {
        "content-type":
          lang === "tspl" ? "application/octet-stream" : "text/plain; charset=utf-8",
      },
    });
  } catch (err) {
    const { status, text } = renderStatus(err);
    return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8" } });
  }
}

function badLanguage(lang) {
  if (lang && !["zpl", "epl", "tspl", "cpcl", "dpl"].includes(lang.toLowerCase())) {
    return jsonResponse(
      { error: `unknown language ${lang.toLowerCase()} — expected zpl, epl, tspl, cpcl or dpl` },
      400,
    );
  }
  return null;
}

// CORS is limited to API routes. No cookies or credentials are needed.
const API_METHODS = new Map([
  ...Object.entries(openapi.paths).map(([path, operation]) => [path, operation.post ? "POST" : "GET"]),
  ["/openapi.json", "GET"],
]);
function corsResponse(response, method) {
  const headers = new Headers(response.headers);
  headers.set("access-control-allow-origin", "*");
  headers.set("access-control-allow-methods", `${method}, OPTIONS`);
  headers.set("access-control-allow-headers", "Content-Type");
  headers.set("access-control-max-age", "86400");
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
}

async function route(request, env, url) {
  if (request.method === "GET" && url.pathname === "/openapi.json") return jsonResponse(openapi);
  if (request.method === "GET" && url.pathname === "/health") return jsonResponse({ status: "ok" });
  // Static API metadata and OPTIONS must work without initializing WASM.
  await ensureEngine();
  if (request.method === "GET" && url.pathname === "/" && !htmlPage) htmlPage = lz_playground_html();
  const base = baseUrl(env);
  if (request.method === "GET") {
    if (url.pathname === "/") {
      // Once PUBLIC_BASE_URL names a custom domain, send the workers.dev
      // HTML page there so search engines index a single origin. API
      // endpoints keep answering on both hosts.
      if (base !== DEFAULT_BASE_URL && url.hostname !== new URL(base).hostname) {
        return Response.redirect(base + "/", 301);
      }
      if (base !== DEFAULT_BASE_URL && !htmlPage.includes(base)) {
        htmlPage = htmlPage.split(DEFAULT_BASE_URL).join(base);
      }
      return htmlResponse();
    }
    if (url.pathname === "/robots.txt") {
      return textResponse(robotsTxt(base), "text/plain; charset=utf-8");
    }
    if (url.pathname === "/sitemap.xml") {
      return textResponse(sitemapXml(base), "application/xml; charset=utf-8");
    }
  }
  if (request.method === "POST" && url.pathname === "/convert") return convert(request, url);
  if (request.method === "POST" && url.pathname === "/validate") return validate(request, url);
  if (request.method === "POST" && url.pathname === "/format") return formatLabel(request, url);
  if (request.method === "POST" && url.pathname === "/encode") return encodeImage(request, url);
  return jsonResponse({ error: "Not Found" }, 404);
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const method = API_METHODS.get(url.pathname);
    if (!method) return route(request, env, url);
    if (request.method === "OPTIONS") return corsResponse(new Response(null, { status: 204 }), method);
    try {
      return corsResponse(await route(request, env, url), method);
    } catch (error) {
      // Unexpected errors (including WASM initialization failures) must also
      // be readable by callers in other browsers, rather than look like CORS.
      return corsResponse(new Response("Internal server error", {
        status: 500, headers: { "content-type": "text/plain; charset=utf-8" },
      }), method);
    }
  },
};
