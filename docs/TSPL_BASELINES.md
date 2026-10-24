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
  `PDF417`, `ITF14`, `DMATRIX` (returns a blank label; measured 2026-10-21
  with area-only, `x#`, and manual-sample shapes — DMATRIX is therefore
  calibrated against the TSC TSPL manual + its result figure alone). Its
  `BITMAP` is also untrustworthy (see the arbitration section below), and
  it skips `|` characters in text. Canvas is floor-rounded (812×406 where
  labelize produces 813×407) — pad to union before diffing.
- **portakal** parses a subset (`doc_QRCODE_sample_1` → 0 elements,
  `doc_BARCODE_sample_1` → 1 of 3 elements) and drops `BITMAP` payloads
  entirely. Its SVG viewport adds a
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

### `BITMAP` — hex-text form added; polarity confirmed 0 = black (2026-10-24, #108)

**Polarity: 0 = printed (black) dot.** Issue #108 claimed 1 = black with
TX610 production evidence, and an earlier revision of the fix briefly
flipped labelize to 1 = black; deeper arbitration reversed that. Six
independent sources say 0 = black:

1. **TSC TSPL/TSPL2 manual (both the 264p and 287p 2014 editions)** — the
   BITMAP example figure (printed p.43/45) draws the dot grid with dark
   cells exactly at the **0** bits (solid top bar + left shaft + right-byte
   diagonal), and the sample-code Result glyph is the matching "↖" arrow.
   The sample's wire form is raw binary.
2. **[tspl-core](https://github.com/thermal-label/tspl-core)** (dedicated
   TSPL II wire encoder): "The TSC TSPL II manual specifies that within
   the `BITMAP` payload, bit `0` represents a printed dot and bit `1`
   represents an un-printed dot" — it one's-complements its 1 = dark
   raster before emitting.
3. **[hackernoon TSPL/JS guide](https://hackernoon.com/how-to-print-images-with-tspl-and-javascript)**
   (working Node.js → TSC prints): "mark every dot as 'one'-not painted or
   'zero'-painted"; reproduces the manual's arrow and sends raw bytes.
4. **[RayforLoy/TSC-Printer `bitmap.py`](https://github.com/RayforLoy/TSC-Printer)**:
   maps dark pixels → 0 bits.
5. **labelixa**: prints 0 bits black (probed with the manual's sample).
6. labelize's pre-#108 behavior (calibrated in PR #79) was already 0 = black.

The reporter's binary-path observations are explained by their writer's
convention ("the TSC convention **we build against** is 1 = black") rather
than the wire spec; with 0 = black the manual's 16-row data table
reproduces its own figure exactly.

**Hex-text payload form: accepted (sniffed).** The manual's wire form is
raw binary and every surveyed writer emits binary; but the #108 reporter's
writer emits ASCII hex text and reports TSC TX610 firmware printing it
correctly in production — the only attestation of firmware hex acceptance,
so the sniff is a strict superset of the binary cursor: a payload run that
is all hex digits (space/tab separators tolerated, as the manual's data
table prints them) ending at a line boundary — or exactly
`2 × widthBytes × height` digits — is decoded; short runs zero-pad; odd
digit counts and anything containing non-hex bytes stay on the binary
cursor.

**labelixa remains disqualified as a BITMAP arbiter**: it consumes hex
text as raw bytes one character per pixel (same mis-parse as pre-#108
labelize) and its binary parsing drops/misaligns a byte (probe records
2026-10-24). **portakal** parses the BITMAP header but drops the payload
entirely. The `tspl_graphics*` / `tspl_bitmap_hex` goldens therefore use
renderer-baseline references — a future labelixa fetch must not overwrite
them.

### Text scaling — resolved by probe (2026-10-12, single-element battery)

Font cells 1–5 × multipliers (1x1, 2x1, 1x2, 2x2, 3x3), ink bbox of
`HHHHHH` on both engines:

- **Ink heights match within ±1 px** across every font/multiplier — the
  "magnified text renders small" impression was wrong for height.
- **Width**: at `ymul > 1` labelixa widens glyphs by ymul (e.g. font 2:
  1x1 → 70 px both engines; 1x2 → labelixa 138 vs labelize 70; 2x2 → 276
  vs 138). The TSC manual defines xmul as horizontal and ymul as vertical,
  independent — labelize follows the manual; labelixa's proportional face
  evidently scales both axes. portakal drops ymul entirely. **Verdict:
  labelize correct; labelixa quirk recorded, not calibrated to.**
- 1x advances differ by ≤1.5 px/char vs labelixa with no consistent ratio;
  labelize lands nearer the manual's nominal cell widths.

### Code 128 `wide == narrow` (`BARCODE …,2,2`)

labelixa draws visibly wider bars than labelize; portakal differs from
both. Per the manual, `wide` applies to discrete two-width codes (39, ITF);
continuous codes like 128 use `narrow` only — labelize follows that.
Needs hardware arbitration to close (low priority).

### `REVERSE` — resolved (see this PR's fix)

Manual + corpus intent: print-time region inversion (white-on-black
text). labelize now inverts the region; labelixa skips the command
entirely, portakal draws a plain black rect (both blind to the documented
behavior — the fix landed on manual semantics, not on either baseline).

### QR version/quiet zone on CJK content (~4.4% on doc_QRCODE_sample_1)

Both engines pick versions automatically; module counts not yet compared
per symbol. Low priority.
