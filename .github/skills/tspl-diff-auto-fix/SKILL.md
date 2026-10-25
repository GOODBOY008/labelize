---
name: tspl-diff-auto-fix
description: "Auto-detect high-diff TSPL labels (tspl_* / tspl_real_*), isolate problematic commands into standalone TSPL snippets, and iteratively fix rendering until the diff against the labelixa baseline drops. Use when: TSPL label diff is too high, want automated fix cycle for TSPL, batch fix all TSPL goldens."
argument-hint: "Label name to fix (e.g. 'tspl_real_drybox') or 'all' for batch mode"
---

# TSPL Diff Auto-Fix Skill

Sibling of `zpl-diff-auto-fix` for the TSPL/TSC language (the family
also covers CPCL and DPL — see `cpcl-diff-auto-fix` / `dpl-diff-auto-fix`).
Same loop (scan → analyze → extract → fix → verify), TSPL-specific
references and guardrails. Read that skill for the general philosophy;
this file defines what changes per language.

## Language Matrix (ZPL vs TSPL; the sibling skills carry their own)

| Concern | ZPL | TSPL |
|---|---|---|
| Reference renderer | Labelary API (4.005x8.01 @ 8dpmm) | labelixa API (`POST /v1/tspl/render`), portakal as secondary arbiter — see docs/TSPL_BASELINES.md |
| Reference cache | testdata/labelary_cache/ | testdata/labelixa_cache/ (tests/common/labelixa_client.rs) |
| Official spec | zpl-reference skill (docs.zebra.com) | TSC TSPL/TSPL2 Programming Manual (web search; no hosted command pages) |
| Fix surface | parsers/zpl/, shared pipeline | **parsers/tspl/ only** for language calibration |
| Command syntax | ^-prefixed stream | line commands; **each element is one self-contained line** |

## Critical Rules (beyond the ZPL skill's)

1. **Never weaken tests** — same as ZPL: fix rendering, not tolerances.
2. **Architecture boundary**: TSPL calibration lives in `src/parsers/tspl/`
   (fonts, command semantics, SIZE conversions). A renderer/element change
   must be language-neutral (new IR field + parser-side resolution);
   `tests/unit_architecture.rs` fails CI on leaks — if it trips, you added
   language specifics to the shared pipeline. Redesign, don't bypass.
3. **Triangulation before calibration** (docs/TSPL_BASELINES.md):
   - labelixa AND portakal agree, labelize differs → labelize bug, fix.
   - Baselines disagree → open question; do NOT calibrate to labelixa alone
     (it is an unverified MVP approximation). Record in the PR/open-questions
     list instead.
   - A baseline skips the command (DIRECTION, CIRCLE, ELLIPSE, rounded BOX,
     PDF417, ITF14 on labelixa) → it cannot arbitrate that element.

## Procedure

### Phase 1: Scan

1. `cargo test --test e2e_diff_report -- --nocapture` — TSPL rows live in
   `testdata/diffs/diff_report_unit.txt` with ext `tspl`.
2. Cross-reference tolerances in `docs/DIFF_THRESHOLDS.md`; corpus
   provenance in `docs/TSPL_CORPUS.md`.
3. Target order: highest diff first, but discount known open questions
   (wide==narrow barcode ratios, REVERSE pre-fix) — check the PR comments
   for the current open-question list before burning cycles.

### Phase 2: Analyze

4. Read the source: `testdata/unit/<name>.tspl`. TSPL elements are one line
   each — correlate `_diff.png` red regions to lines by their `x,y` args.
5. Classify per element (no src/skill extractor needed — manual bbox from
   the command's own coordinates):
   - **ContentDiff**: shape/size/ink differs in place.
   - **PositionDiff**: same ink, shifted (check REFERENCE/SHIFT state and
     the y-axis anchor).
   - **Mixed**: both.
6. For font/text issues, measure the labelixa reference's glyph box
   (script below) to quantify "how much bigger" before touching code.

   ```bash
   # fetch/refresh a labelixa reference (respects cache + quota)
   # tests/common/labelixa_client.rs is the in-crate path; ad-hoc:
   curl -s -X POST -H "Content-Type: text/plain" \
     --data-binary @testdata/unit/<name>.tspl \
     https://api.labelixa.com/v1/tspl/render -o /tmp/ref.png
   ```

### Phase 3: Extract

7. A standalone snippet is trivial in TSPL — copy the file, keep
   `SIZE/DIRECTION/REFERENCE/SHIFT/CLS`, the ONE element line under test,
   `PRINT 1`. Save as `testdata/unit/<name>_<n>.tspl` (treat as synthetic
   unit fixtures; commit only if they become regression goldens).
8. Render locally: `cargo run -- convert <file>` (SIZE drives the canvas;
   no --width/--height needed). Compare against `/tmp/ref.png` for the same
   snippet (fetch it).

### Phase 4: Fix (iterative)

9. Fix surface by symptom:
   | Symptom | Files |
   |---|---|
   | Command semantics/params | `src/parsers/tspl/parser.rs` |
   | Font cells/faces/magnification | `src/parsers/tspl/fonts.rs` (+ neutral `FontFace` additions if a new rendering class is truly needed) |
   | Element maps wrong onto IR | `src/parsers/tspl/parser.rs` (constructor mapping) |
   | Renderer genuinely lacks neutral capability | add IR field + parser resolution + renderer branch (language-blind!) |
10. Spec check: TSC TSPL/TSPL2 Programming Manual (web search the command;
    the Zebra-hosted TSPL guide also mirrors the command set).
11. Verify loop (same as ZPL skill steps 18-21):
    `cargo build` → snippet render → `cargo test --test e2e_golden -- golden_tspl` →
    full `cargo test --all-features` → `cargo test --test e2e_diff_report`.
    **ZPL/EPL goldens must stay byte-identical** (`git status testdata/` may
    show only tspl_* changes).
12. Regressions → revert, next approach. Improvement → keep, re-measure,
    record before/after in the commit message.

### Phase 5: Finalize

13. Update `docs/DIFF_THRESHOLDS.md` (new diff%, tolerance slightly above).
14. Full suite + fmt + clippy. Commit with before/after numbers.

## TSPL-specific gotchas

- labelixa canvases floor-round (812 vs our 813) — pad to union before
  pixel-diffing; the golden bootstrap already pads to the SIZE canvas.
- The anonymous labelixa quota counts labels/day; the cached client stops
  after `X-RateLimit-Remaining: 0` — renderer-baseline fallback makes CI
  offline-safe, but for calibration sessions prefer the disk cache or wait
  for the 00:00 UTC reset.
- `DIRECTION 0` inverts the whole label; labelixa ignores DIRECTION, so any
  DIRECTION-0 fixture diff includes that gap — calibrate such labels against
  portakal semantics only if portakal implements it, else open question.
- Real captures may carry `;` comments and `${placeholder}` template text —
  both parse as literal content (documented in TSPL_CORPUS.md).

## Known open questions (do not "fix" without arbitration)

See docs/TSPL_BASELINES.md §Known engine disagreements — currently:
Code 128 `wide==narrow` ratios, magnified-text size, and anything where
labelixa and portakal disagree.
