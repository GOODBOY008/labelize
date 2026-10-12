# TSPL Baseline Renderers

TSPL has no equivalent of Labelary (which is ZPL-only and the golden
reference for the ZPL side). Two independent baseline renderers are used for
TSPL calibration, with a strict triangulation policy so we never overfit one
approximation.

| Baseline | Type | Access | License | Role |
|---|---|---|---|---|
| **labelixa.com** | Online renderer (MVP subset) | `POST https://api.labelixa.com/v1/tspl/render` (raw TSPL body → PNG, 203 dpi preview). Diagnostics: `POST /v1/tspl/diagnostics`. Anonymous IP-quota, remaining reported in `X-RateLimit-Remaining`, resets 00:00 UTC; 512 KB body cap. npm `labelixa` / PyPI `labelixa` packages are thin API clients, **not** offline renderers. | Proprietary service | **Primary** — only hosted TSPL renderer found (6 search rounds, EN+CN, incl. GitHub sweep) |
| **portakal** | Local parser + SVG preview | `git clone productdevbook/portakal` (npm `portakal` ≥0.5 lacks the parser; use `main`), `parseTSC(src) → ResolvedLabel`, `renderPreview(label) → SVG`; rasterize SVG with `@resvg/resvg-js` (`fitTo width=812`, background white, `DejaVu Sans Mono`). Run via `npx tsx`. | MIT | **Secondary** — offline, unlimited iterations for auto-fix loops |

## Capability boundaries (measured, 2026-10-12)

- **labelixa** does not render: `DIRECTION` (declared "not applied to the
  MVP preview"; verified 0–4 identical), `CIRCLE`, `ELLIPSE`, rounded `BOX`,
  `PDF417`, `ITF14`. It also skips `|` characters in text. Canvas is
  floor-rounded (812×406 where labelize produces 813×407) — pad to union
  before diffing.
- **portakal** parses a subset (`doc_QRCODE_sample_1` → 0 elements,
  `doc_BARCODE_sample_1` → 1 of 3 elements). Its SVG viewport adds a
  decorative margin; rasterize with `fitTo` and re-check dimensions. Font
  rendering depends on the rasterizer's system fonts — treat font-metric
  disagreement involving portakal as weak evidence.

## Triangulation policy

1. **Both baselines agree, labelize differs** → labelize is wrong: fix
   (parser-side calibration only; `tests/unit_architecture.rs` guards the
   language boundary).
2. **Baselines disagree with each other** → open question unless the TSC
   TSPL/TSPL2 manual states the semantics; never calibrate to one side.
3. **A baseline skips a feature** the other renders → compare against the
   rendering one only, and note the coverage gap.
4. No hardware samples exist in the corpus yet; a TSC print scan always
   outranks both baselines.

## Known engine disagreements (open questions, 2026-10-12)

- **Code 128 `wide == narrow`** (`BARCODE …,2,2`): labelixa draws visibly
  wider bars than labelize; portakal differs from both. No manual clause
  found; needs hardware arbitration.
- **Magnified text size** (`TEXT x,y,"4",0,2,2,…`): labelixa draws larger
  than labelize.
- **`REVERSE`**: labelize fills a black box (see fix in this PR — TSC
  semantics is an inverted print area); labelixa skips the command.
