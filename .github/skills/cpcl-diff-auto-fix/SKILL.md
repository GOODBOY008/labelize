---
name: cpcl-diff-auto-fix
description: "Auto-detect high-diff CPCL labels (cpcl_* / cpcl_real_*), isolate problematic commands into standalone CPCL snippets, and iteratively fix rendering until the diff against the labelixa CPCL baseline drops. Use when: CPCL label diff is too high, want automated fix cycle for CPCL, batch fix all CPCL goldens."
argument-hint: "Label name to fix (e.g. 'cpcl_real_route_label') or 'all' for batch mode"
---

# CPCL Diff Auto-Fix Skill

Sibling of `zpl-diff-auto-fix` / `tspl-diff-auto-fix` for CPCL (Zebra
mobile printers). Same loop (scan → analyze → extract → fix → verify);
this file defines what changes per language.

## Language Matrix

| Concern | CPCL |
|---|---|
| Reference renderer | labelixa CPCL API — `POST https://api.labelixa.com/v1/cpcl/render` (text/plain body → PNG; subset coverage at 203 dpi) |
| Reference cache | `testdata/labelixa_cache/` (`tests/common/labelixa_client.rs::labelixa_render_cpcl`; endpoint-keyed, shares the TSPL quota) |
| Official spec | Zebra CPCL Programming Manual — `zebra.com/.../cpcl-pm-en.pdf` (font tables are Appendix D **image pages**; `pdftoppm` + vision to read them) |
| Fix surface | `src/parsers/cpcl/` only for language calibration |
| Command syntax | space-separated fields; sessions `! {offset} {hdpi} {vdpi} {height} {qty}` … PRINT/END/ABORT; 2-D codes take data lines until `ENDQR`/`ENDPDF`/`ENDMAXICODE`/`ENDAZTEC`; `EG`/`CG` payloads are byte-counted and may start on the command line |
| Unit model | dots at the header dpi using the manual's own classes (8 dots/mm @200-203 dpi, 12 @300) — a 203-dpi file renders dot-identical at the engine's 8 dpmm |

## Critical Rules (beyond the ZPL skill's)

1. **Never weaken tests** — fix rendering, not tolerances.
2. **Architecture boundary**: CPCL calibration lives in `src/parsers/cpcl/`
   (`fonts.rs` Appendix-D cells, header dpi classes, QR payload
   conventions in `qr_payload.rs`). Renderer changes must stay
   language-blind; `tests/unit_architecture.rs` fails CI on `cpcl`
   tokens in drawers.
3. **Arbitration limits** (labelixa skips these — it cannot arbitrate
   them): QR, PDF-417, Aztec, MaxiCode, `EG`/`CG` graphics, `ML` blocks,
   `SETMAG` magnification, `SCALE-TEXT`, justification effects beyond
   CENTER on text. `cpcl_2d_graphics` is a renderer-baseline golden
   (0.01%) for exactly this reason — do not "calibrate" those elements
   against labelixa.

## Procedure

### Phase 1: Scan

1. `cargo test --test e2e_diff_report -- --nocapture` — CPCL rows live in
   `testdata/diffs/diff_report_unit.txt` with ext `cpcl`.
2. Cross-reference tolerances in `docs/DIFF_THRESHOLDS.md`; corpus
   provenance in `docs/CPCL_DPL_CORPUS.md` (real corpus = the
   Labelixa/thermal-printer-examples MIT collection).
3. Highest diff first; `cpcl_shipping` (16.12%) and
   `cpcl_real_asset_tag` (17.76%) are dominated by font-substitute width
   — measure before touching code (Phase 2 step 6).

### Phase 2: Analyze

4. Read the source: `testdata/unit/<name>.cpcl`. Fields are
   space-separated; correlate `_diff.png` red regions to commands by
   their `{x} {y}` args.
5. Classify per element: ContentDiff / PositionDiff / Mixed. Watch the
   session state that shifts positions: header offset, `IN-*` units,
   `CENTER`/`RIGHT [end]` blocks.
6. For font issues, quantify the reference's glyph box:

   ```bash
   curl -s -X POST -H "Content-Type: text/plain" \
     --data-binary @testdata/unit/<name>.cpcl \
     https://api.labelixa.com/v1/cpcl/render -o /tmp/ref.png
   ```

### Phase 3: Extract

7. Standalone snippet: copy the file, keep the `!` header (+`PAGE-WIDTH`,
   any `IN-*`/SETMAG/justification state under test), the ONE command
   under test, `PRINT`. Save as `testdata/unit/<name>_<n>.cpcl` (commit
   only if it becomes a regression golden).
8. Render locally: `cargo run -- convert <file>` (header height +
   PAGE-WIDTH drive the canvas). Fetch the same snippet from labelixa
   for comparison.

### Phase 4: Fix (iterative)

9. Fix surface by symptom:
   | Symptom | Files |
   |---|---|
   | Command semantics/params | `src/parsers/cpcl/parser.rs` |
   | Font cells/Appendix-D tables/SETMAG | `src/parsers/cpcl/fonts.rs` |
   | QR payload conventions (`MA,` / `H0M,N…`) | `src/parsers/cpcl/qr_payload.rs` |
   | Header/units/dpi-class conversion | `src/parsers/cpcl/parser.rs` (`header_dpmm`, `x_mm`/`y_mm`) |
   | Renderer lacks neutral capability | IR field + parser resolution + renderer branch (language-blind!) |
10. Spec check: the CPCL Programming Manual PDF (see matrix). Command
    format blocks are text-extractable; only the Appendix D font tables
    need page rendering.
11. Verify loop: `cargo build` → snippet render →
    `cargo test --test e2e_golden -- golden_cpcl` → full
    `cargo test --all-features` → `cargo test --test e2e_diff_report`.
    **Other languages' goldens must stay byte-identical** (`git status
    testdata/` may show only cpcl_* changes).
12. Regressions → revert, next approach. Improvement → keep, re-measure,
    record before/after in the commit message.

### Phase 5: Finalize

13. Update `docs/DIFF_THRESHOLDS.md` (new diff%, tolerance slightly above).
14. Full suite + fmt + clippy. Commit with before/after numbers.

## CPCL-specific gotchas

- labelixa's CPCL renderer is an interpretation of the guide, not
  printer-verified — same single-arbiter caution as labelixa TSPL. Where
  the manual is explicit (font tables, ratio codes, UNITS retroactivity),
  the manual wins over labelixa.
- The `EG`/`CG` payload starts right after the 4th parameter's separator
  space and runs across lines byte-counted; a parser that only reads
  following lines swallows the next command (this was the original bug).
- `COUNT` is inert serial state (like `^SN`) — diffs on later copies are
  out of scope; we render copy 1.
- Justified fields become one-line `FieldBlock`s spanning [x .. end]:
  a position diff on a CENTER/RIGHT field may be a justify-end problem,
  not a glyph problem.
- Real corpus (`cpcl_real_*`) comes from the Labelixa example collection
  — treat the files as immutable provenance; extract snippets instead of
  editing them.

## Known open questions (do not "fix" without arbitration)

- Font-substitute width: our resident-bitmap faces render narrower than
  labelixa's (~20% on magnified font 5) — the dominant diff source on
  `cpcl_shipping`/`cpcl_real_asset_tag`. Needs a measured advance model
  before any "fix".
- Barcode interpretation-line metrics and ratio geometry deltas.
