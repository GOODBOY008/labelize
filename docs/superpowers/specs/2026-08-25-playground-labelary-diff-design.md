# Playground "Compare with Labelary" Tool — Design

Date: 2026-08-25
Status: Approved (design choices confirmed interactively; implementation follows)

## Goal

Give playground users a one-click tool to **estimate the rendering diff between
Labelary (reference) and Labelize** for the ZPL they typed: a numeric score on
the same scale CI uses, plus a visual red-pixel diff map and side-by-side
previews.

## Constraints & Verified Facts

These were verified against the live API on 2026-08-25 (curl + response headers):

- `https://api.labelary.com/v1/printers/8dpmm/labels/4x6/0/` responds `200` with a
  PNG over **HTTPS** and sends `Access-Control-Allow-Origin: *` — the browser can
  call Labelary **directly**, no proxy required.
- Request format that works (mirrors `tests/common/labelary_client.rs`):
  `POST` with header `Content-Type: application/x-www-form-urlencoded` and the raw
  ZPL as body. This content type is a CORS-safelisted "simple" header, so **no
  preflight** is triggered. `Accept: image/png` is also safelisted.
- `Content-Type: application/epl` would trigger a preflight whose
  `Access-Control-Allow-Headers` does not list `Content-Type` — do not use it.
- Labelary **does not support EPL** (404 on EPL input; also documented in
  `docs/DIFF_THRESHOLDS.md`). The tool is ZPL-only; the button is disabled with a
  tooltip when format = EPL.
- 4×6 in @ 8 dpmm: Labelary returns 812×1218 px; Labelize renderer
  (`src/drawers/renderer.rs`, `(width_mm * dpmm).ceil()`) produces 813×1220 px.
  The 1–2 px off-by-one must be normalized before comparing (same philosophy as
  `pad_png_to_size` in the e2e suite), otherwise every label shows ~0.3 % fake
  size-mismatch noise.
- Labelary rate limit is ~3 req/s (e2e client sleeps 334 ms between calls);
  the UI must debounce and surface HTTP 429 as a "wait and retry" message.

## Architecture

Pure front-end, single source of truth stays `src/playground.rs`. No changes to
`worker.js`, the wasm crate exports, the npm package, or server routes.

```
ZPL in editor
  → click "⇄ Compare with Labelary"
      ├─ A) POST /convert (same origin)          → Labelize PNG
      ├─ B) POST https://api.labelary.com/...    → Labelary reference PNG (browser direct, CORS *)
      │      (both requests run in parallel)
      ├─ decode both PNGs via createImageBitmap → canvas → ImageData
      ├─ normalize: draw each at natural size onto a W×H canvas,
      │     W = max(w1,w2), H = max(h1,h2), white background
      ├─ pixel loop: any channel |a−e| > 32 → diff pixel   (same rule as
      │     tests/common/image_compare.rs, all 4 channels)
      ├─ diff% = diff pixels ÷ (W×H) × 100
      └─ show: verdict badge + diff% + sizes + elapsed,
            3 image columns (Labelary | Labelize | Diff overlay)
```

The compare always refetches both images (does not trust a previous Render), so
it works even before the user has pressed Render and never shows stale output.

## Metric & Verdict

Matches CI conventions (`docs/DIFF_THRESHOLDS.md` categories):

| Verdict   | diff%      | Meaning                              |
|-----------|------------|--------------------------------------|
| PERFECT   | 0 %        | Pixel-identical                      |
| GOOD      | < 1 %      | Sub-pixel / anti-alias noise         |
| MINOR     | 1 – 5 %    | Small font or position deltas        |
| MODERATE  | 5 – 15 %   | Font engine, graphics, 2D barcode    |
| HIGH      | ≥ 15 %     | Missing encoder / structural mismatch|

Note: the page renders with `antialias=false` (1-bit) while CI golden tests
currently run with `antialias=true`, so playground numbers are the same *scale*
but not bit-identical to CI numbers. The tool is explicitly an estimator; the
formula and threshold (32) are identical to `image_compare.rs`.

## UI

- Settings bar: new secondary button `⇄ Compare with Labelary` left of the
  Render button (`dl-btn-png` style). Disabled + spinner while running; disabled
  with tooltip "Labelary does not support EPL — ZPL only" when format is EPL.
- Preview panel: new `#compare-section` under the download bar, hidden by
  default:
  - Verdict row: colored badge, diff %, both images' real sizes, elapsed ms,
    one-line category explanation.
  - Three columns (flex, wrap on narrow screens): **Labelary** (reference),
    **Labelize** (local render), **Diff** (white background + opaque red
    diff pixels, same visual language as e2e diff images).
- Reuses the existing loading overlay, error banner, and status bar.

## Error Handling & Edge Cases

- Labelary network failure / timeout / non-2xx → error banner with the status;
  HTTP 429 gets a specific "Labelary is rate-limited (~3 req/s), wait a moment
  and retry" message.
- Oversized canvas: if either side exceeds 4096 px (e.g. dpmm=24 with a large
  custom size), abort with a "canvas too large, lower dpmm or size" message to
  avoid mobile OOM. Computation: `ceil(inches * 25.4 * dpmm)`.
- Dimension mismatch of 1–2 px is normal → normalized, reported in the verdict
  row, not an error.
- Parse failure of the ZPL → existing error path from POST /convert.

## Testing

- Wasm smoke test in `workers/labelize-wasm/src/lib.rs`:
  `exports_playground_html` gains an assertion that the HTML contains the
  compare button id (`compare-btn`).
- No golden/e2e impact (no rendering logic changed; `PLAYGROUND_HTML` is a
  constant with no golden coverage).
- Manual acceptance checklist:
  1. Default sample ZPL → Compare → three images render, verdict shown, no
     console errors.
  2. Same ZPL cross-checked visually against the Labelary viewer website.
  3. EPL selected → button disabled with tooltip.
  4. Labelary unreachable (offline / blocked) → error banner text correct.
  5. Oversized canvas (dpmm=24 + large custom size) → soft-limit message.
  6. Rapid double-click → only one request pair.

## Files Touched

- `src/playground.rs` — all HTML/CSS/JS for the feature.
- `workers/labelize-wasm/src/lib.rs` — one test assertion.
- `docs/superpowers/specs/2026-08-25-playground-labelary-diff-design.md` — this
  document.

## Deployment

- CF Worker: `workers/build.sh` (rebuilds wasm embedding `PLAYGROUND_HTML`) +
  `wrangler deploy`, or push to `main` (auto-deploy via CI).
- Self-hosted axum server: plain `cargo build`.
- No npm package release needed.