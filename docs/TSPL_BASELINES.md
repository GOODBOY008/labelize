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
  calibrated against the TSC TSPL manual + its result figure alone). It also
  skips `|` characters in text. Canvas is floor-rounded (812×406 where
  labelize produces 813×407) — pad to union before diffing.
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

### `BITMAP` — exact hex compatibility and polarity (2026-10-30, #108)

The [TSC TSPL/TSPL2 manual, Chinese edition](https://fs.chinatsc.cn/system/files/tspl_tspl2_programming_2018_5_17_scjiantizhongwen.pdf)
shows the BITMAP cell grid on printed p.44 and its byte table/sample stream
on p.45 (PDF pages 56–57). Dark cells correspond to **0** bits; the printed
result is an arrow pointing up-left. The sample sends **raw binary** after
the fifth comma. The hexadecimal column is a representation of those wire
bytes, not evidence of an ASCII-hex firmware mode.

Labelize therefore keeps **0 = black, 1 = white**, unchanged from 1.9.0.
Issue #108 reports a TX610 accepting ASCII hex and using a different
polarity; no physical print scan is attached, so that firmware behavior
remains unverified. We support hex as a Labelize compatibility extension
without claiming that every TSC printer accepts it or attributing the
reporter's observations to a particular writer bug.

For `N = widthBytes × heightDots`, the compatibility form requires a whole
payload line with exactly `2 × N` hex digits. Upper/lowercase and spaces/tabs
are accepted; CR, LF, CRLF or EOF ends the line. Other payloads use the
existing `N`-byte binary cursor. In particular, short/odd/overlong runs and
hex prefixes followed by non-hex bytes are not decoded, padded or truncated.
The formatter uses the same boundary check and preserves payload bytes.

**The issue's literal repro is ambiguous:** its `4,8` header declares 32
bytes, but its 32 hex characters encode only 16 bytes. Those 32 characters
are also a valid 32-byte binary bitmap, so guessing hex would corrupt valid
binary input. Use **64 hex digits** for 4×8 bytes (e.g. `FF00FF00` repeated
eight times), or change the header to `4,4` for the original 32 digits.
With the documented polarity, each full row is
`........########........########` (`.` white, `#` black).

For JS/WASM, pass raw binary in a `Uint8Array` (or Node `Buffer`). Do not
round-trip binary through a UTF-8 string: bytes such as `0xFF` change when
text-encoded. A hex-text command stream is ASCII and can be text-encoded.
This is a transport requirement, not a diagnosis of the reporter's missing
small-binary reproduction.

Verification and reference provenance:

- `tspl_bitmap_hex.png` is a 32×8 stripe grid constructed independently
  from the expected pixels, with a complete 64-digit payload.
- `tspl_bitmap_manual.png` is a 16×16 transcription of the manual's dark
  cell grid. `tests/e2e_tspl_bitmap.rs` separately pins the byte table and
  expected pixel rows, exercising both binary and hex paths.
- Both new fixtures require **0.00%** pixel difference. They are analytical
  references, not renderer-generated or labelixa-derived goldens.
- `tspl_graphics*` remain renderer-baseline integration fixtures. Their
  old hex data contained 62 and 61 bytes respectively despite `4,16`
  headers; the missing trailing `00` bytes are now explicit in the input.
  Their references are regenerated for the decoded bitmap. Do not replace
  these with labelixa captures: the earlier #109 investigation reported
  hex misparsing and binary misalignment there, and no new hardware
  evidence supersedes the manual.
- Existing reverse-render goldens and byte-exact parser/formatter tests
  protect raw payloads, including ASCII hex bytes, CR/LF and non-UTF-8 data.

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
