# Deploy Labelize to Cloudflare Workers — Design

**Date:** 2026-08-22
**Status:** Approved (design reviewed by user)

## Goal

Deploy the existing Labelize playground (ZPL/EPL → PNG/PDF convert & preview web app) to a Cloudflare Worker so anyone can open a website URL, paste ZPL, and preview the rendered label as PNG. Public access; no auth.

## Constraints & Decisions (from brainstorming)

- **Approach:** JS worker + pure-WASM core module (option A). workers-rs rejected (maintenance mode, no axum reuse); container + reverse proxy rejected (contradicts "deploy to worker").
- **PDF:** keep (`/convert?output=pdf` must work, same as axum version). Drop only as a size fallback if the wasm bundle exceeds CF limits.
- **Access:** public, no auth. If abuse becomes an issue, add Turnstile/rate limiting later.
- **Deployment:** owner provides `CF_API_TOKEN`; assistant runs `wrangler deploy`.
- **Worker name:** `labelize` (workers.dev subdomain), rename if taken.
- **HTML single source of truth:** `PLAYGROUND_HTML` stays the Rust constant in `src/playground.rs`; both the axum server and the worker serve it via the same source (wasm exports it).
- axum/Docker version stays untouched and keeps working.

## Architecture

```
Browser ──→ Cloudflare Worker (labelize.<account>.workers.dev)
             ├── GET  /          → playground HTML (exported from wasm)
             ├── GET  /health    → {"status":"ok"}
             └── POST /convert   → JS calls wasm engine
                                     ├─ labelize-wasm (wasm32-unknown-unknown)
                                     │    parse ZPL/EPL → Renderer → PNG (or PDF)
                                     └─ returns image/png | application/pdf
```

Routes, parameter semantics (`width`, `height`, `dpmm`, `antialias`, `output=pdf`, Content-Type containing `epl` selects EPL parser) mirror the existing axum implementation in `src/main.rs:convert_handler`, so the shipped frontend needs zero changes.

## Components

### 1. `workers/labelize-wasm/` — new Rust cdylib crate

Path dependency on `labelize` (workspace-free: standalone crate in `workers/` subdir, `crate-type = ["cdylib"]`).

C-ABI exports (wasm32-unknown-unknown, no wasm-bindgen):

- `lz_alloc(len: usize) -> *mut u8` / `lz_free(ptr: *mut u8, len: usize)` — buffer management for the JS↔wasm boundary. All buffers handed to JS (render result, error text, HTML) are `lz_alloc`-allocated and freed with `lz_free` on the returned pointer+len.
- `lz_render(src: *const u8, src_len: usize, width_mm: f64, height_mm: f64, dpmm: i32, antialias: bool, want_pdf: bool) -> *const RenderResult` where `RenderResult` has fixed layout `{ data_ptr: *const u8, data_len: usize, err_ptr: *const u8, err_len: usize }` (plus an `is_error: bool` flag implied by `err_len > 0`). Returns PNG or PDF bytes (success) or an error string (failure).
- `lz_playground_html() -> *const RenderResult`-like pair — returns the `PLAYGROUND_HTML` string (single source of truth).

Conversion logic: port `convert_handler`'s body (parse params → `ZplParser::with_dpmm(dpmm)` or `EplParser::new()` by Content-Type `epl` marker → `DrawerOptions` → `Renderer::draw_label_as_png` → optional `encode_pdf` from the PNG). Pure synchronous code — no tokio needed.

Panic strategy: `panic = "abort"` in release profile. A Rust panic traps; JS wraps every wasm call in try/catch and maps the trap to a 500.

Build: `cargo build --release --target wasm32-unknown-unknown`, profile tuned (`opt-level="z"`, `lto=true`, `codegen-units=1`), optional `wasm-opt -Oz` if installed. Output: `workers/engine.wasm`. Build script checks the resulting size.

Size: image crate's multi-format decoders dominate. Expected 2–4 MB; must verify against CF WASM limits (10 MB). Fallbacks if over: `image` with `default-features = false, features = ["png"]` or drop PDF.

### 2. `workers/worker.js` — ~150 lines, zero dependencies

- `fetch` handler with route dispatch for `/`, `/health`, `/convert`.
- Instantiates the wasm module (wrangler `[wasm_modules]` binding).
- `/convert`: validates params (same defaults as axum: width 102.0, height 152.0, dpmm 8, antialias false). EPL detection matches the shipped frontend exactly: it posts `Content-Type: application/zpl` or `application/epl`, and the existing server-side check is `content_type.contains("epl")` — reuse it verbatim. Copies body bytes into wasm memory via `lz_alloc` + `new Uint8Array(memory.buffer)`, calls `lz_render`, copies result out (data or err), frees both.
- Error mapping: parse errors → 400 with error text; wasm trap / internal error → 500. `Cache-Control: no-cache` on HTML like the axum version.

### 3. `workers/wrangler.toml`

`name = "labelize"`, `main = "worker.js"`, `compatibility_date`, `[wasm_modules] engine = "./engine.wasm"`.

### 4. `workers/build.sh` (and/or Makefile)

Cross-compile → wasm-opt → size check → copy to `workers/engine.wasm`. Idempotent, rerunnable.

## Data Flow

POST body bytes → JS allocates wasm buffer, copies in → `lz_render` → Rust parses & renders synchronously → PNG/PDF bytes copied back to JS → HTTP response.

## Error Handling

| Case | HTTP | Mechanism |
|---|---|---|
| ZPL/EPL parse error | 400 | error string from RenderResult |
| Rendering/encoding error | 500 | error string from RenderResult |
| Rust panic | 500 | wasm trap caught by JS try/catch |
| Invalid/absent body | 400 | JS-side validation |

## Testing

1. Rust unit tests in `labelize-wasm` (run on native target — same logic, both targets).
2. Local smoke via `npx wrangler dev`:
   - `GET /` returns HTML containing the playground textarea.
   - `POST /convert` with a ZPL from `testdata/unit/` returns bytes with PNG magic (`89 50 4E 47`).
   - `POST /convert?output=pdf` returns `%PDF`.
   - EPL path (`Content-Type` with `epl`).
3. Post-deploy curl against the live URL for the three routes.
4. Existing e2e/golden suite untouched (no rendering logic changes).

## Deploy

- Owner provides `CF_API_TOKEN` (with Workers Scripts edit permission) — the assistant runs `wrangler login`-less deploy via `CLOUDFLARE_API_TOKEN` env var.
- `wrangler deploy` from `workers/`.
- Verify live URL: `/`, `/health`, a sample convert.

## Out of Scope (YAGNI)

- CI/CD pipeline for the worker (manual deploy for now; can add GitHub Actions later).
- Custom domain, KV/D1 storage, auth/Turnstile, rate limiting.
- Modifying the axum server or Docker publishing pipeline.
- Rendering behavior changes of any kind.