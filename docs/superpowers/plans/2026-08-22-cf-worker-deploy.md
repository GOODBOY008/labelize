# Cloudflare Workers Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy the Labelize playground (ZPL/EPL → PNG/PDF convert & preview) to a Cloudflare Worker, using a pure-WASM build of the labelize engine behind a ~150-line JS worker.

**Architecture:** A new Rust cdylib crate (`workers/labelize-wasm`) compiles labelize to `wasm32-unknown-unknown` and exports C-ABI functions (`lz_alloc`, `lz_free`, `lz_render`, `lz_playground_html`). A zero-dependency JS worker (`workers/worker.js`) implements the same three routes as the existing axum server (`/`, `/health`, `/convert`) and calls the wasm engine. The playground HTML stays a single source of truth in `src/playground.rs:PLAYGROUND_HTML`, exported by the wasm module.

**Tech Stack:** Rust (wasm32-unknown-unknown, `-Oz`/LTO), plain JS worker, Wrangler (`npx wrangler`), Cloudflare Workers free plan.

**Design spec:** `docs/superpowers/specs/2026-08-22-cf-worker-deploy-design.md` (committed as 59b2320).

---

## File Structure

| File | Responsibility |
|---|---|
| `Cargo.toml` (root) | Add `playground` feature; `serve` depends on it |
| `src/lib.rs` | Gate `pub mod playground` on `feature = "playground"` instead of `serve` |
| `workers/labelize-wasm/Cargo.toml` | New cdylib crate, path dep on labelize, release profile tuned for size |
| `workers/labelize-wasm/src/lib.rs` | C-ABI exports + `render_payload` core + unit tests |
| `workers/worker.js` | HTTP routes, wasm memory plumbing, error mapping |
| `workers/wrangler.toml` | Worker name, main entry, compatibility date |
| `workers/build.sh` | Cross-compile → optional wasm-opt → copy `engine.wasm` → size check |
| `workers/.gitignore` | Ignore built `engine.wasm` and `target/` |
| `docs/superpowers/plans/2026-08-22-cf-worker-deploy.md` | This plan |

**Environment notes (verified 2026-08-22):**
- `wasm32-unknown-unknown` target is NOT installed (`rustup target add wasm32-unknown-unknown` needed).
- `wasm-opt`/`wasm-tools` NOT installed → step is optional in build.sh.
- `wrangler` NOT installed globally → use `npx wrangler` (npx 11.5.2 available).
- Sample ZPL: inline strings suffice. Sample EPL: `testdata/labels/dpduk.epl` (real, 1900 bytes, already golden-tested).
- Frontend posts `Content-Type: application/zpl` or `application/epl`; server checks `content_type.contains("epl")`.

---

### Task 1: Split the `playground` feature from `serve`

**Files:**
- Modify: `Cargo.toml` (root), `src/lib.rs`

- [ ] **Step 1: Change the feature definition in `Cargo.toml`**

In the `[features]` section, add a `playground` feature and make `serve` include it:

```toml
[features]
default = []
cli = ["dep:clap"]
playground = []
serve = ["cli", "playground", "dep:axum", "dep:tokio", "dep:serde"]
skill = []
```

- [ ] **Step 2: Change the module gate in `src/lib.rs`**

```rust
#[cfg(feature = "serve")]
pub mod playground;
```
becomes:
```rust
#[cfg(feature = "playground")]
pub mod playground;
```

- [ ] **Step 3: Verify existing feature combinations still build and pass tests**

Run from the repo root:
```bash
cargo build
cargo build --features serve
cargo test
```
Expected: all build clean, all tests pass (`src/playground.rs` contains no axum/tokio code, only the HTML constant, so gating it on the lighter feature compiles standalone).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "feat: gate playground HTML module behind its own feature"
```

---

### Task 2: Scaffold the `workers/` directory and wasm crate

**Files:**
- Create: `workers/.gitignore`, `workers/labelize-wasm/Cargo.toml`, `workers/labelize-wasm/src/lib.rs` (empty module for now)

- [ ] **Step 1: Create the directory layout and `workers/.gitignore`**

```bash
mkdir -p workers/labelize-wasm/src
```

`workers/.gitignore`:
```gitignore
engine.wasm
target/
node_modules/
```

- [ ] **Step 2: Write `workers/labelize-wasm/Cargo.toml`**

```toml
[package]
name = "labelize-wasm"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
labelize = { path = "../..", features = ["playground"] }
image = "0.25"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```

- [ ] **Step 3: Create an empty `workers/labelize-wasm/src/lib.rs`**

```rust
//! labelize compiled to wasm32-unknown-unknown with a C-ABI surface for JS.
```

- [ ] **Step 4: Verify the empty crate builds and install the wasm target**

```bash
cd workers/labelize-wasm
cargo test
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```
Expected: `cargo test` passes (0 tests), wasm build produces `target/wasm32-unknown-unknown/release/labelize_wasm.wasm`.

- [ ] **Step 5: Commit**

```bash
cd ../..
git add workers/
git commit -m "feat(workers): scaffold labelize-wasm cdylib crate"
```

---

### Task 3: `render_payload` core (TDD)

**Files:**
- Modify: `workers/labelize-wasm/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Append to `workers/labelize-wasm/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ZPL: &str = "^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ";
    const SAMPLE_EPL: &str = include_str!("../../../testdata/labels/dpduk.epl");
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn png(bytes: &[u8]) -> bool {
        bytes.len() >= 8 && bytes[..8] == PNG_MAGIC
    }

    #[test]
    fn renders_zpl_to_png() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, false, false)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_epl_to_png() {
        let out = render_payload(SAMPLE_EPL.as_bytes(), 102.0, 152.0, 8, false, false, true)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_to_pdf() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, true, false)
            .expect("render ok");
        assert!(out.starts_with(b"%PDF"), "expected PDF header");
    }

    #[test]
    fn parse_error_is_reported() {
        let err = render_payload(b"this is not a label", 102.0, 152.0, 8, false, false, false)
            .expect_err("garbage input must fail");
        assert_eq!(err.0, Stage::Parse);
    }

    #[test]
    fn empty_input_yields_no_labels_error() {
        let err = render_payload(b"", 102.0, 152.0, 8, false, false, false)
            .expect_err("empty input must fail");
        assert_eq!(err.1, "No labels found".to_string());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail to compile**

```bash
cd /Volumes/AidenExternal/aiden/IdeaProjects/labelize/workers/labelize-wasm
cargo test
```
Expected: compile error — `render_payload` and `Stage` are undefined.

- [ ] **Step 3: Implement `Stage`, `render_payload`, and the allocator helpers**

Add to `workers/labelize-wasm/src/lib.rs` (above the test module):

```rust
use std::alloc::{alloc, dealloc, Layout};
use std::io::Cursor;
use std::slice;

use labelize::drawers::renderer::Renderer;
use labelize::elements::drawer_options::DrawerOptions;
use labelize::parsers::epl_parser::EplParser;
use labelize::parsers::zpl_parser::ZplParser;

/// Failure stage, which maps to HTTP status in the JS worker:
/// Parse → 400 (bad input), Render → 500 (internal).
#[derive(Debug, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Render,
}

fn alloc_buf(len: usize) -> *mut u8 {
    unsafe { alloc(Layout::from_size_align(len, 1).expect("len fits usize")) }
}

fn render_payload(
    bytes: &[u8],
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: bool,
    want_pdf: bool,
    is_epl: bool,
) -> Result<Vec<u8>, (Stage, String)> {
    let mut labels = if is_epl {
        EplParser::new().parse(bytes)
    } else {
        ZplParser::with_dpmm(dpmm).parse(bytes)
    }
    .map_err(|e| (Stage::Parse, e.to_string()))?;

    let label = labels
        .into_iter()
        .next()
        .ok_or_else(|| (Stage::Parse, "No labels found".to_string()))?;

    let options = DrawerOptions {
        label_width_mm: width_mm,
        label_height_mm: height_mm,
        dpmm,
        antialias,
        ..Default::default()
    };

    let renderer = Renderer::new();
    let mut png_buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(&label, &mut png_buf, options.clone())
        .map_err(|e| (Stage::Render, e.to_string()))?;

    if want_pdf {
        let img = image::load_from_memory(png_buf.get_ref())
            .map_err(|e| (Stage::Render, format!("image decode: {e}")))?
            .to_rgba8();
        let mut pdf_buf = Cursor::new(Vec::new());
        labelize::encode_pdf(&img, &options, &mut pdf_buf)
            .map_err(|e| (Stage::Render, e.to_string()))?;
        Ok(pdf_buf.into_inner())
    } else {
        Ok(png_buf.into_inner())
    }
}
```

(Note: `png_buf.get_ref()` — the Cursor is still owned here; `png_buf.into_inner()` would move it, so `get_ref()` keeps it borrowable for `load_from_memory`. The axum original decodes from the inner buffer the same way.)

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test
```
Expected: 5 tests pass. (This runs on the native target — same code path as wasm.)

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "feat(workers): port convert_handler core as render_payload"
```

---

### Task 4: C-ABI exports `lz_render`, `lz_alloc`, `lz_free`, `lz_playground_html` (TDD)

**Files:**
- Modify: `workers/labelize-wasm/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
    #[test]
    fn exports_roundtrip_zpl() {
        let src = SAMPLE_ZPL.as_bytes();
        let src_ptr = lz_alloc(src.len() as u32);
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), src_ptr, src.len()) };

        let out = unsafe { *lz_render(src_ptr, src.len() as u32, 102.0, 152.0, 8, 0, 0, 0) };
        assert_eq!(out.code, 0, "ok outcome");
        let payload = unsafe { slice::from_raw_parts(out.ptr, out.len as usize) }.to_vec();
        assert!(png(&payload), "expected PNG magic");

        lz_free(src_ptr, src.len() as u32);
        lz_free(out.ptr, out.len as u32);
    }

    #[test]
    fn exports_report_parse_error() {
        let bad = b"NOPE";
        let src_ptr = lz_alloc(bad.len() as u32);
        unsafe { std::ptr::copy_nonoverlapping(bad.as_ptr(), src_ptr, bad.len()) };
        let out = unsafe { *lz_render(src_ptr, bad.len() as u32, 102.0, 152.0, 8, 0, 0, 0) };
        assert_eq!(out.code, 1, "parse error code");
        lz_free(src_ptr, bad.len() as u32);
        lz_free(out.ptr, out.len as u32);
    }

    #[test]
    fn exports_playground_html() {
        let out = unsafe { *lz_playground_html() };
        assert_eq!(out.code, 0);
        let html = unsafe { slice::from_raw_parts(out.ptr, out.len as usize) }.to_vec();
        let html = String::from_utf8(html).expect("utf8 html");
        assert!(html.contains("<textarea"), "playground page has editor");
        lz_free(out.ptr, out.len as u32);
    }
```

- [ ] **Step 2: Run tests to verify they fail to compile**

```bash
cargo test
```
Expected: compile error — `lz_render`, `lz_playground_html`, `out_ptr_token` undefined, and `RenderOutcome` has no `code` field.

- [ ] **Step 3: Implement the exports**

Add to the top level of `workers/labelize-wasm/src/lib.rs` (replacing the empty doc comment):

```rust
/// Result handed back to JS. Fixed layout on wasm32 (address space is u32):
/// offset 0 `code: u32` (0 = payload, 1 = parse error → 400, 2 = render error → 500),
/// offset 4 `ptr: *mut u8` (payload or error text), offset 8 `len: u32`. Total 12 bytes.
#[repr(C)]
pub struct RenderOutcome {
    pub code: u32,
    pub ptr: *mut u8,
    pub len: u32,
}

const OUTCOME_SIZE: usize = std::mem::size_of::<RenderOutcome>();

fn make_outcome(code: u32, data: Vec<u8>) -> *mut RenderOutcome {
    let len = data.len();
    let payload_ptr = alloc_buf(len);
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), payload_ptr, len) };
    let outcome_ptr = alloc_buf(OUTCOME_SIZE) as *mut RenderOutcome;
    unsafe {
        std::ptr::write(outcome_ptr, RenderOutcome { code, ptr: payload_ptr, len: len as u32 });
    }
    outcome_ptr
}

#[no_mangle]
pub extern "C" fn lz_alloc(len: u32) -> *mut u8 {
    alloc_buf(len as usize)
}

// lz_free rebuilds the exact 1-aligned Layout from the length both sides track,
// matching how the buffer was allocated. JS passes the source length for input
// buffers and the outcome's len for payload buffers.
#[no_mangle]
pub extern "C" fn lz_free(ptr: *mut u8, len: u32) {
    if ptr.is_null() {
        return;
    }
    unsafe { dealloc(ptr, Layout::from_size_align(len as usize, 1).expect("valid layout")) };
}

```rust
#[no_mangle]
pub extern "C" fn lz_render(
    src: *const u8,
    src_len: u32,
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: u32,
    want_pdf: u32,
    is_epl: u32,
) -> *mut RenderOutcome {
    let bytes = unsafe { slice::from_raw_parts(src, src_len as usize) };
    match render_payload(
        bytes,
        width_mm,
        height_mm,
        dpmm,
        antialias != 0,
        want_pdf != 0,
        is_epl != 0,
    ) {
        Ok(data) => make_outcome(0, data),
        Err((Stage::Parse, msg)) => make_outcome(1, msg.into_bytes()),
        Err((Stage::Render, msg)) => make_outcome(2, msg.into_bytes()),
    }
}

#[no_mangle]
pub extern "C" fn lz_playground_html() -> *mut RenderOutcome {
    make_outcome(0, labelize::playground::PLAYGROUND_HTML.as_bytes().to_vec())
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test
```
Expected: 8 tests pass (5 from Task 3 + 3 new). Native target: `lz_alloc`/`lz_free` use the system allocator here, which is a valid stand-in for dlmalloc on wasm.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "feat(workers): export C-ABI render and html functions"
```

---

### Task 5: JS worker, wrangler config, and build script

**Files:**
- Create: `workers/worker.js`, `workers/wrangler.toml`, `workers/build.sh`

- [ ] **Step 1: Write `workers/worker.js`**

```js
// Labelize Cloudflare Worker: / , /health, /convert (mirrors the axum server).
import engine from "./engine.wasm";

const OUTCOME_SIZE = 12; // wasm32 RenderOutcome: code u32 + ptr u32 + len u32

let htmlPage = null;

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

function copyIn(bytes) {
  if (bytes.length === 0) return { ptr: 0, len: 0 };
  const ptr = engine.lz_alloc(bytes.length);
  if (!ptr) throw new Error("wasm alloc failed");
  new Uint8Array(engine.memory.buffer, ptr, bytes.length).set(bytes);
  return { ptr, len: bytes.length };
}

function readOutcome(outPtr) {
  const view = new DataView(engine.memory.buffer, outPtr, OUTCOME_SIZE);
  const code = view.getUint32(0, true);
  const payloadPtr = view.getUint32(4, true);
  const payloadLen = view.getUint32(8, true);
  const payload = new Uint8Array(engine.memory.buffer, payloadPtr, payloadLen).slice();
  engine.lz_free(outPtr, OUTCOME_SIZE);
  if (payloadLen > 0) engine.lz_free(payloadPtr, payloadLen);
  return { code, payload };
}

async function convert(request, url) {
  const ct = request.headers.get("content-type") || "application/zpl";
  const width = parseFloat(url.searchParams.get("width")) || 102.0;
  const height = parseFloat(url.searchParams.get("height")) || 152.0;
  const dpmm = parseInt(url.searchParams.get("dpmm"), 10) || 8;
  const antialias = url.searchParams.get("antialias") === "true";
  const wantPdf = url.searchParams.get("output") === "pdf";
  const isEpl = ct.includes("epl");

  const body = new Uint8Array(await request.arrayBuffer());
  if (body.length === 0) {
    return new Response("empty body", { status: 400 });
  }

  const { ptr, len } = copyIn(body);
  try {
    const outPtr = engine.lz_render(
      ptr, len, width, height, dpmm,
      antialias ? 1 : 0, wantPdf ? 1 : 0, isEpl ? 1 : 0,
    );
    const { code, payload } = readOutcome(outPtr);
    if (code === 1) {
      return new Response(new TextDecoder().decode(payload), {
        status: 400,
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }
    if (code === 2) {
      return new Response(new TextDecoder().decode(payload), {
        status: 500,
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }
    return new Response(payload, {
      headers: { "content-type": wantPdf ? "application/pdf" : "image/png" },
    });
  } catch (err) {
    // Rust panic with panic=abort surfaces here as a wasm trap.
    return new Response("internal error: " + err.message, { status: 500 });
  } finally {
    if (len > 0) engine.lz_free(ptr, len);
  }
}

export default {
  async fetch(request, env, ctx) {
    if (!htmlPage) {
      const out = readOutcome(engine.lz_playground_html());
      htmlPage = new TextDecoder().decode(out.payload);
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
```

- [ ] **Step 2: Write `workers/wrangler.toml`**

```toml
name = "labelize"
main = "worker.js"
compatibility_date = "2026-08-01"
```

(No `[wasm_modules]` binding needed — Wrangler resolves `import engine from "./engine.wasm"` directly. If the deployed version errors about the wasm import, add `[wasm_modules] engine = "./engine.wasm"` instead and change the import to `import engine from "engine"`.)

- [ ] **Step 3: Write `workers/build.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-unknown-unknown
rustup target add $TARGET

# labelize-wasm is its own crate root (no workspace), so build from inside it.
cd labelize-wasm
cargo build --release --target $TARGET
cp target/$TARGET/release/labelize_wasm.wasm ../engine.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz ../engine.wasm -o ../engine.wasm
fi

SIZE=$(stat -f%z ../engine.wasm 2>/dev/null || stat -c%s ../engine.wasm)
echo "engine.wasm: $((SIZE / 1024)) KB"
if [ "$SIZE" -gt 10485760 ]; then
  echo "WARNING: wasm exceeds 10 MB — Cloudflare may reject it."
  echo "Remedies: image with default-features=false,features=[\"png\"] in labelize-wasm/Cargo.toml, or drop PDF."
  exit 1
fi
```

Note: the wasm build runs from inside `labelize-wasm` (its own crate root), so cargo output lands in `labelize-wasm/target/`.

- [ ] **Step 4: Build and record the wasm size**

```bash
chmod +x workers/build.sh
cd workers && ./build.sh
```
Expected: `engine.wasm: NNN KB` printed. Record NNN — if it approaches the 10 MB ceiling, apply the image-features remedy from the warning before continuing.

- [ ] **Step 5: Commit**

```bash
cd ..
git add workers/
git commit -m "feat(workers): add JS worker, wrangler config, and build script"
```

---

### Task 6: Local smoke test with `wrangler dev`

**Files:** none (verification only; fix files if smoke fails)

- [ ] **Step 1: Start the dev server**

```bash
cd /Volumes/AidenExternal/aiden/IdeaProjects/labelize/workers
npx wrangler dev --port 8787
```
Keep this running in a separate terminal / background task. First run downloads wrangler and prompts to create an account if missing — answer interactively or reuse an existing login.

- [ ] **Step 2: Smoke `GET /`**

```bash
curl -s http://127.0.0.1:8787/ | grep -c "<textarea"
```
Expected: `1`.

- [ ] **Step 3: Smoke `GET /health`**

```bash
curl -s http://127.0.0.1:8787/health
```
Expected: `{"status":"ok"}`.

- [ ] **Step 4: Smoke `POST /convert` (ZPL → PNG)**

```bash
printf '^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ' | \
  curl -s -X POST -H "Content-Type: application/zpl" \
  --data-binary @- http://127.0.0.1:8787/convert -o /tmp/lz.png -w "%{http_code} %{content_type}\n"
xxd -l 8 /tmp/lz.png
```
Expected: `200 image/png` and `00000000: 8950 4e47 0d0a 1a0a` (PNG magic).

- [ ] **Step 5: Smoke `POST /convert?output=pdf`**

```bash
printf '^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ' | \
  curl -s -X POST -H "Content-Type: application/zpl" \
  --data-binary @- "http://127.0.0.1:8787/convert?output=pdf" -o /tmp/lz.pdf -w "%{http_code} %{content_type}\n"
head -c 4 /tmp/lz.pdf
```
Expected: `200 application/pdf` and `%PDF`.

- [ ] **Step 6: Smoke EPL path**

```bash
curl -s -X POST -H "Content-Type: application/epl" \
  --data-binary @/Volumes/AidenExternal/aiden/IdeaProjects/labelize/testdata/labels/dpduk.epl \
  http://127.0.0.1:8787/convert -o /tmp/lz-epl.png -w "%{http_code} %{content_type}\n"
xxd -l 8 /tmp/lz-epl.png
```
Expected: `200 image/png` and PNG magic.

- [ ] **Step 7: Smoke error mapping**

```bash
printf 'not a label' | curl -s -X POST -H "Content-Type: application/zpl" \
  --data-binary @- http://127.0.0.1:8787/convert -w "\n%{http_code}\n"
```
Expected: error text and `400`.

If any step fails, fix `worker.js` or `src/lib.rs` accordingly and re-run; commit fixes as a separate commit before Task 7.

- [ ] **Step 8: Stop the dev server**

---

### Task 7: Deploy and verify live (needs `CF_API_TOKEN`)

**Files:** none

- [ ] **Step 1: Obtain the API token**

Ask the owner for a Cloudflare API token with `Workers Scripts: Edit` permission (account-scoped). Export it in the deploy shell:

```bash
export CLOUDFLARE_API_TOKEN="<token>"
```

- [ ] **Step 2: Deploy**

```bash
cd /Volumes/AidenExternal/aiden/IdeaProjects/labelize/workers
npx wrangler deploy
```
Expected: `Uploaded labelize` plus the workers.dev URL, e.g. `https://labelize.<account-subdomain>.workers.dev`. If the name `labelize` is taken, add `name = "labelize-playground"` to `wrangler.toml` and redeploy.

- [ ] **Step 3: Verify the live endpoints**

Repeat the Task 6 smoke steps (Steps 2–7) against `https://labelize.<account-subdomain>.workers.dev` instead of `127.0.0.1:8787`. All must pass with the same expected output.

- [ ] **Step 4: Open the site**

Report the final URL to the owner and confirm the playground page loads in a browser (textarea present, Render button produces a preview).

- [ ] **Step 5: Commit any deploy-time fixes**

```bash
git add -A workers/
git commit -m "fix(workers): adjustments found during live verification"
```

---

## Execution Notes (2026-08-22 — deviations found during execution)

The plan was executed as written through Task 3; Tasks 4 and 5 changed materially after two discoveries. Everything below was verified locally (all six smoke checks pass).

### 1. wasm-bindgen instead of raw C-ABI (getrandom story)

`imageproc 0.26.2` pulls `rand 0.9.5`; `lopdf 0.40` pulls `rand 0.10.2`; both rand versions enable `sys_rng`, so both `getrandom 0.3.4` and `0.4.3` enter the wasm graph. On `wasm32-unknown-unknown` getrandom refuses to compile without its `wasm_js` backend, which is built on **wasm-bindgen** (0.3 and 0.4 alike). The raw C-ABI design could not avoid it, so:

- `workers/labelize-wasm/Cargo.toml` adds `wasm-bindgen = "0.2"` and `getrandom = { version = "0.4", features = ["wasm_js"] }` (feature unification covers rand 0.10's chain; imageproc already enables `wasm_js` on 0.3).
- `workers/labelize-wasm/.cargo/config.toml` sets `rustflags = ["--cfg", "getrandom_backend=\"wasm_js\""]` for the wasm target only.
- Exports changed from C-ABI pointers to `#[wasm_bindgen]` functions. Memory management is automatic; `lz_render(&[u8], ...) -> Result<Vec<u8>, String>` and `lz_playground_html() -> String`. Errors are prefixed strings (`1:` → HTTP 400, `2:` → HTTP 500) because exported structs must be Copy.
- The engine never calls the random source in the render/PDF paths (lopdf only uses rand in its encryption code), so the JS-side Web Crypto bridge is unused at runtime.

### 2. workerd wasm import semantics → manual instantiation

Two workerd/wrangler quirks broke the stock wasm-bindgen bundler glue:

- Wrangler (esbuild) resolves `.wasm` imports to a `WebAssembly.Module`, while bundler glue assumes an instantiated instance (`wasm.__wbindgen_start()` → "not a function").
- `--no-bundle` cannot resolve the relative glue imports at all.

Fix in `workers/worker.js`: skip the thin entry `labelize_wasm.js` entirely; import the core glue (`__wbg_set_wasm`, `lz_render`, `lz_playground_html`, `__wbindgen_init_externref_table`, `__wbindgen_cast_0000000000000001`) from `labelize_wasm_bg.js`, import `labelize_wasm_bg.wasm` (the module), and manually `WebAssembly.instantiate(module, { "./labelize_wasm_bg.js": { ... } })`. The two wasm imports are never invoked during instantiation (verified experimentally), then `instance.exports.__wbindgen_start()` runs the real init. `instantiate()` returns a plain instance when given a module, `{ module, instance }` when given bytes — the code handles both.

### 3. build.sh and glue artifacts

`build.sh` now runs `wasm-bindgen --target bundler --out-dir .. --out-name labelize_wasm`, producing `labelize_wasm.js` (unused), `labelize_wasm_bg.js`, and `labelize_wasm_bg.wasm` (2.7 MB, well under the 10 MB ceiling). All `labelize_wasm*` artifacts are gitignored; `.gitignore` covers `labelize_wasm*.js`, `labelize_wasm*.wasm`, `labelize_wasm*.d.ts`. `labelize-wasm/Cargo.lock` is committed for reproducible builds.

### 4. Task 7 requires the owner's `CF_API_TOKEN`

Blocked on the owner providing a Cloudflare API token with Workers Scripts: Edit permission.

## Self-Review (updated after execution)

- **Spec coverage:** spec's four components (wasm crate, worker.js, wrangler.toml, build.sh) map to Tasks 2/3/4, 5, 5, 5. Error mapping table (400/500/panic → 500) is implemented in worker.js and tested in Tasks 6. Data flow section matches Task 4 exports. Testing section matches Tasks 3/6/7. Deploy section matches Task 7. HTML single-source-of-truth is Task 1 + `lz_playground_html`. PDF keep-decision is exercised in Task 3 test + Task 6 smoke. The C-ABI details in the Self-Review below are superseded by the Execution Notes above (wasm-bindgen surface, `1:`/`2:` string errors, manual instantiation); the architecture-level claims (single source HTML, route parity, error mapping, size ceiling) all still hold.
- **Placeholder scan:** no TBD/TODO; every step has concrete code or commands.
- **Type consistency:** `RenderOutcome { code, ptr, len }` is consistent across lib.rs and worker.js (code 0/1/2; OUTCOME_SIZE 12 — wasm32 `u32(4) + ptr(4) + u32(4)`). `lz_render` argument order matches between the Rust signature and the JS call site (src, src_len, width_mm, height_mm, dpmm, antialias, want_pdf, is_epl). `render_payload` argument order matches its only caller. Feature name `playground` is consistent between root Cargo.toml, lib.rs, and labelize-wasm's dependency declaration.