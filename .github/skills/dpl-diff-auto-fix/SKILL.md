---
name: dpl-diff-auto-fix
description: "Auto-detect high-diff DPL labels (dpl_* / dpl_real_*), isolate problematic records into standalone DPL snippets, and iteratively fix rendering until the diff against the Evolabel layout-view reference drops. Use when: DPL label diff is too high, want automated fix cycle for DPL, batch fix all DPL goldens."
argument-hint: "Label name to fix (e.g. 'dpl_real_manual_component') or 'all' for batch mode"
---

# DPL Diff Auto-Fix Skill

Sibling of `zpl-diff-auto-fix` / `tspl-diff-auto-fix` for DPL
(Datamax-O'Neil / Honeywell). Same loop (scan → analyze → extract → fix
→ verify); this file defines what changes per language.

## Language Matrix

| Concern | DPL |
|---|---|
| Reference renderer | Evolabel layout view — `POST https://layoutview.evolabel.com/` (multipart form `code`/`width`/`height`/`variables` → HTML embedding a base64 PNG at ~11.8 px/mm ≈ 300 dpi; **not printer-verified**) |
| Reference client | `tests/common/evolabel_client.rs` (`evolabel_render(dpl, w_mm, h_mm)`); cache in `testdata/labelixa_cache/`, None-on-failure → renderer baseline |
| Official spec | *M-Class DPL Programmer's Manual* Rev B (`scaleservice.net/manuals/datamax/MDPL.pdf`); font cells Table C-2, smooth sizes Table C-3, barcode letters Appendix F, record structures §Generating Label Formats |
| Fix surface | `src/parsers/dpl/` only for language calibration |
| Command syntax | `<STX>L … E`/`X`; print records = fixed-width header `{rot 1-4}{type}{c}{d}{eee}{ffff row}{gggg col}` (15 bytes; **17 for W-expanded 2-D**) + data + CR; config lines are letter+digits |
| Unit model | hundredths of an inch (`m` = tenths of a millimetre); **home = bottom-left, rows grow up** → the IR's `LabelPosition.from_bottom` |

## Critical Rules (beyond the ZPL skill's)

1. **Never weaken tests** — fix rendering, not tolerances.
2. **Architecture boundary**: DPL calibration lives in `src/parsers/dpl/`
   (Table C-2 cells in `fonts.rs`, record parsing, `qr_payload.rs`).
   The bottom-left home is the neutral `LabelPosition.from_bottom` anchor
   the renderer resolves — renderer changes stay language-blind;
   `tests/unit_architecture.rs` fails CI on `dpl` tokens in drawers.
3. **Single-arbiter caution**: evolabel is one unverified preview —
   where the manual is explicit (font cells, multiplier base-25, W1C
   byte-count form, PDF-417 option layout), the manual wins. Do not
   calibrate to evolabel alone on anything the manual pins.
4. **Canvas provenance**: synthetic `dpl_*` goldens use the 812×1624
   unit canvas; `dpl_real_*` goldens derive their canvas from the stored
   reference at test time (dpmm 12, mm = ref_px/12 − ε) so bottom-anchored
   content aligns. Keep both mechanisms intact when touching bootstraps.

## Procedure

### Phase 1: Scan

1. `cargo test --test e2e_diff_report -- --nocapture` — DPL rows in
   `testdata/diffs/diff_report_unit.txt`, ext `dpl`; real-corpus rows
   show the honest evolabel distance, synthetic rows are 0.00%
   renderer-baseline.
2. Cross-reference `docs/DIFF_THRESHOLDS.md`; provenance in
   `docs/CPCL_DPL_CORPUS.md`.
3. Highest diff first. The dominant component is font-substitute width
   (~20% narrow vs evolabel on the Appendix B component label) — measure
   before touching code (Phase 2 step 6).

### Phase 2: Analyze

4. Read the source: `testdata/unit/<name>.dpl` (binary — `0x02` STX,
   CR-terminated records). Decode record headers by column positions:
   bytes 0=rotation, 1=type, 2/3=multipliers, 4-6=eee, 7-10=row,
   11-14=col (W records shift by the two expansion chars).
5. Classify per element: ContentDiff / PositionDiff / Mixed. Position
   diffs on DPL usually mean a row/col field misread or a missed
   bottom-anchor conversion — check `from_bottom` handling first.
6. For font issues, quantify the reference glyph box:

   ```bash
   curl -s -X POST "https://layoutview.evolabel.com/" \
     -F "code=<testdata/unit/<name>.dpl" \
     -F "width=100" -F "height=50" -F "variables={}" \
   | python3 -c "import sys,re,base64; m=re.search(r'base64,([A-Za-z0-9+/=]+)', sys.stdin.read()); open('/tmp/ref.png','wb').write(base64.b64decode(m.group(1)))"
   ```

   (The `code` field must NOT carry a `filename=` — the server 400s.)

### Phase 3: Extract

7. Standalone snippet: copy the file, keep `<STX>L`, `D11` and any
   config under test, the ONE record under test, `E`. Save as
   `testdata/unit/<name>_<n>.dpl`. Generate headers programmatically
   (a Python f-string with explicit widths) — hand-counted 15/17-byte
   headers misplace fields silently.
8. Render locally: `cargo run -- convert <file> --width W --height H
   --dpmm 12` (match the evolabel canvas: W/H mm = ref px / 12).

### Phase 4: Fix (iterative)

9. Fix surface by symptom:
   | Symptom | Files |
   |---|---|
   | Record parsing/field boundaries | `src/parsers/dpl/parser.rs` |
   | Font cells/multipliers/smooth sizes | `src/parsers/dpl/fonts.rs` |
   | QR payload conventions (W1D manual form) | `src/parsers/dpl/qr_payload.rs` |
   | Bottom-anchor placement (shared) | `src/drawers/renderer.rs` (`flip_from_bottom`, `adjust_image_typeset_position`) — must stay language-blind |
   | Config semantics (metric, C/R, `T` terminator) | `src/parsers/dpl/parser.rs` (`parse_config`) |
10. Spec check: the MDPL PDF (see matrix). Record-structure tables and
    Appendix G barcode details are text-extractable.
11. Verify loop: `cargo build` → snippet render →
    `cargo test --test e2e_golden -- golden_dpl` → full
    `cargo test --all-features` → `cargo test --test e2e_diff_report`.
    **Other languages' goldens must stay byte-identical** (`git status
    testdata/` may show only dpl_* changes).
12. Regressions → revert, next approach. Improvement → keep, re-measure,
    record before/after in the commit message.

### Phase 5: Finalize

13. Update `docs/DIFF_THRESHOLDS.md` (new diff%, tolerance slightly above).
14. Full suite + fmt + clippy. Commit with before/after numbers.

## DPL-specific gotchas

- 15 vs 17-byte headers: W-expanded records (`W1c` DataMatrix, `W1d/D`
  QR, `W1f` Aztec) carry the type ID **inside** field b, so row/col sit
  two bytes later. PDF-417 (`z`/`Z`) and MaxiCode (`u`/`U`) stay 15-byte
  with their own data layouts.
- The byte-count field exists **only** in uppercase forms (`W1C`, `Z`);
  the lowercase W1c's `iii` ECC digits are numeric too — never sniff the
  count from digits.
- Base-25 multipliers: `1`-`9`=1-9, `A`-`O`=10-24, and `0` means
  "use the default" (per-command default, not 1).
- Uppercase barcode letters print the HRI line; lowercase do not.
- `T` reassigns the record terminator — a `T~` format terminates records
  at `~`, and CR becomes data. The formatter and parser share the
  cursor; keep them in sync.
- DPL declares no in-band label size: canvas always comes from the
  caller (`--width/--height`, unit_options, or the reference-derived
  canvas for the real corpus).

## Known open questions (do not "fix" without arbitration)

- Font-substitute width vs evolabel's face (~20%) — the dominant
  `dpl_real_manual_component` diff component. Needs a measured advance
  model for the CondensedBold substitute before any "fix".
- Barcode geometry deltas (module rounding at 12 dpmm, HRI font).
- evolabel is ~11.8 px/mm, not exactly 300 dpi: sub-pixel bar-edge
  differences are preview artifacts, not engine bugs.
