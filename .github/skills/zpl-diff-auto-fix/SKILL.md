---
name: zpl-diff-auto-fix
description: "Auto-detect high-diff ZPL labels, extract problematic elements into standalone ZPL files, and iteratively fix rendering logic until diff drops below 1%. Use when: label diff is too high, need to reduce rendering differences, want automated fix cycle, batch fix all labels."
argument-hint: "Label name to fix (e.g. 'fedex', 'ups') or 'all' for batch mode"
---

# ZPL Diff Auto-Fix Skill

## Purpose

Automatically detect labels with high rendering diff percentages, isolate the problematic ZPL elements into standalone test files for focused debugging, and iteratively fix the rendering logic until the diff drops below a target threshold (default 1%).

## Critical Rule — Fix Rendering Code, Never Tests

**DO NOT** modify diff tests, golden test infrastructure, reference images, tolerance thresholds
(to hide regressions), or test comparison logic to make diffs pass. The goal is to improve
rendering accuracy, not to weaken the test harness.

**DO** fix the root cause in rendering code and logic:

- Parsers: `src/parsers/`
- Elements: `src/elements/`
- Barcode encoders: `src/barcodes/`
- Renderer / drawing: `src/drawers/`
- Font handling: `src/assets/`

Threshold updates in `docs/DIFF_THRESHOLDS.md` are only allowed **after** a rendering fix
genuinely lowers the diff percentage — set the new tolerance slightly above the new measured
diff, never raise it to paper over a regression.

## When to Use

- A label has a diff percentage higher than desired
- You want to systematically reduce rendering differences across all labels
- You need to extract standalone ZPL snippets to isolate a rendering bug
- After implementing a rendering change, you want to verify and fine-tune it

## Prerequisites

Before starting, ensure:
1. The diff report is up to date: `cargo test --test e2e_diff_report -- --nocapture`
2. All current tests pass: `cargo test`
3. You have reference PNG images in `testdata/` for the target label(s)

## Procedure

### Phase 1: Scan — Identify High-Diff Labels

1. **Read the diff report** from [testdata/diffs/diff_report.txt](../../testdata/diffs/diff_report.txt)

2. **Read the tolerance thresholds** from [docs/DIFF_THRESHOLDS.md](../../docs/DIFF_THRESHOLDS.md)

3. **Identify target labels** — either a specific label from the user's request, or all labels with diff > 1%:
   - List labels sorted by diff percentage (highest first)
   - For each, note the current diff%, tolerance, and primary diff source

4. **Pick the highest-impact label** to fix first (highest diff% that is potentially fixable — skip MaxiCode/PDF417 unless specifically requested)

### Phase 2: Analyze — Find Problematic Elements

5. **Read the ZPL source** from `testdata/labels/<name>.zpl` or `testdata/unit/<name>.zpl`

6. **Parse the ZPL** to identify all commands and elements:
   ```
   Use src/skill/snippet_extractor.rs → split_zpl_commands() to list all commands
   Use src/skill/snippet_extractor.rs → group_commands_into_spans() to group into element spans
   ```

7. **Inspect the diff image** at `testdata/diffs/<name>_diff.png` — red pixels show where rendering differs from Labelary

8. **Correlate diff regions with elements** using `src/skill/element_analyzer.rs`:
   - Parse the ZPL to get `LabelInfo`
   - Compute bounding boxes for each element via `compute_element_bbox()`
   - Load the diff image and count red pixels in each bbox via `correlate_diff_regions()`
   - Report which elements contribute most to the total diff

9. **Rank elements by diff contribution** — focus on the top 3-5 contributors

9b. **Classify each element's diff** as ContentDiff, PositionDiff, or Mixed using
    `src/skill/diff_classifier.rs → classify_element_diffs()`:

    ```
    // Full pipeline with classification:
    Use element_analyzer::analyze_label_with_classification(label, diff_path, zpl, use_labelary)
    ```

    The classifier renders each element in isolation, optionally fetches the Labelary reference
    for the same snippet, and compares the two renders:

    | Isolated snippet diff | Full-label bbox diff | Classification |
    |----------------------|----------------------|----------------|
    | Low (< 2%)           | High (> 2%)          | **PositionDiff** — element placed wrong |
    | High (> 2%)          | Any                  | **ContentDiff** — element renders wrong |
    | High (> 2%)          | High (> 2%)          | **Mixed** — both |
    | Low (< 2%)           | Low (< 2%)           | No classification (no meaningful diff) |

    For **PositionDiff** elements, the classifier also attempts to detect the offset vector (dx, dy):
    - Shadow detection: two clusters of red pixels → direct offset measurement
    - Centroid shift: centroid of diff pixels vs. expected element center

### Phase 3: Extract — Create Standalone Snippets

10. **Extract each high-diff element** into a standalone ZPL file:
    ```
    Use src/skill/snippet_extractor.rs → extract_element()
    ```
    This creates `testdata/unit/<label>_<index>.zpl` with:
    - `^XA` / `^XZ` wrapper
    - Global state commands (`^PW`, `^CF`, `^BY`, `^CI`, `^FW`, `^PO`, `^LR`) preserved
    - The specific element's command span (`^FO`/`^FT` through `^FS`)

11. **Render each snippet** to verify it renders correctly in isolation:
    ```bash
    cargo run -- convert --width 50 --height 80 --dpmm 8 testdata/unit/<label>_<index>.zpl
    ```
    Unit tests use a smaller canvas (`50×80 mm` at 8 dpmm → `400×640 px`) for easier
    visual comparison. Carrier labels in `testdata/labels/` use the full Labelary size
    (`101.625×203.25 mm` → `813×1626 px`).

12. **Get Labelary reference** for each snippet — post the snippet ZPL to:
    ```
    POST http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/
    ```
    Save the reference as `testdata/unit/<label>_<index>_ref.png`.
    Note: Labelary always renders at 813×1626. The local render uses 400×640 for unit
    tests, so direct pixel comparison requires resizing the Labelary reference to match.

13. **Compare snippet renders** — this isolates the rendering difference to a single element

### Phase 4: Fix — Iterative Rendering Fixes

For each high-diff element, starting with the highest contributor:

14. **Identify the element type, diff classification, and fix category:**

    Use `FixCategory::from_classification(element_type, classification)` to select the fix strategy:

    | Classification | Fix Strategy | Target Source Files |
    |---------------|-------------|-------------------|
    | **ContentDiff** + Text | FontMetrics | `src/drawers/renderer.rs`, `src/elements/font.rs` |
    | **ContentDiff** + Barcode128 | BarcodeEncoding | `src/barcodes/code128.rs`, `src/drawers/renderer.rs` |
    | **ContentDiff** + BarcodeEan13 | BarcodeEncoding | `src/barcodes/ean13.rs` |
    | **ContentDiff** + Barcode2of5 | BarcodeEncoding | `src/barcodes/interleaved2of5.rs` |
    | **ContentDiff** + Barcode39 | BarcodeEncoding | `src/barcodes/code39.rs` |
    | **ContentDiff** + BarcodePdf417 | BarcodeEncoding | `src/barcodes/pdf417.rs` |
    | **ContentDiff** + BarcodeAztec | BarcodeEncoding | `src/barcodes/aztec.rs` |
    | **ContentDiff** + BarcodeDatamatrix | BarcodeEncoding | `src/barcodes/datamatrix.rs` |
    | **ContentDiff** + BarcodeQr | BarcodeEncoding | `src/barcodes/qrcode.rs` |
    | **ContentDiff** + Maxicode | BarcodeEncoding | `src/barcodes/maxicode.rs` |
    | **ContentDiff** + GraphicBox/Circle/Line/Field | GraphicRendering | `src/drawers/renderer.rs` |
    | **PositionDiff** (any element) | PositionOffset | `src/parsers/zpl_parser.rs`, `src/elements/field_alignment.rs`, `src/elements/drawer_options.rs` |
    | **Mixed** (any element) | PositionOffset first, then ContentDiff fix | Both sets above |

    For **PositionDiff** elements, use the detected offset vector (dx, dy) from `PositionOffsetInfo`
    to guide the fix — look for coordinate calculation that differs by that many pixels.

15. **Fetch the official Zebra documentation** for the suspect ZPL command using the `zpl-reference` skill:
    ```
    https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-<slug>.html
    ```

16. **Compare spec vs implementation** using the `fix-zpl-render` skill's Phase 3 comparison table

17. **Apply the fix** to the relevant source file

18. **Verify the fix:**
    ```bash
    # Quick check — does it compile?
    cargo build

    # Render the snippet
    cargo run -- convert testdata/unit/<label>_<index>.zpl

    # Run the specific golden test
    cargo test --test e2e_golden -- <label_name> --nocapture

    # Check for regressions on ALL labels
    cargo test --test e2e_golden
    ```

19. **Measure improvement:**
    ```bash
    cargo test --test e2e_diff_report -- --nocapture
    ```
    Compare the new diff% against the previous value.

20. **If diff improved without regressions:** Keep the change and proceed to the next element.

21. **If diff regressed on other labels:** Revert the change and try a different approach.

22. **Repeat** from step 14 for the next highest-contributing element.

### Phase 5: Finalize — Update Thresholds and Report

23. **Update diff thresholds** in [docs/DIFF_THRESHOLDS.md](../../docs/DIFF_THRESHOLDS.md):
    - Set new diff% values for improved labels
    - Set new tolerance ceilings slightly above the new diff%

24. **Regenerate diff images:**
    ```bash
    cargo test --test e2e_diff_report -- --nocapture
    ```

25. **Run full test suite:**
    ```bash
    cargo test
    ```

26. **Commit all changes** including:
    - Source code fixes
    - Updated `docs/DIFF_THRESHOLDS.md`
    - Updated diff images in `testdata/diffs/`
    - New snippet files in `testdata/unit/` (if useful for ongoing debugging)

## Auto-Fix Loop Strategy

When running in fully automated mode ("fix all labels above 1%"):

```
1. Generate diff report
2. Sort labels by diff% descending
3. For each label above 1%:
   a. Analyze element contributions
   b. For each element (highest contribution first):
      i.   Extract standalone snippet
      ii.  Identify fix category
      iii. Fetch Zebra spec for the command
      iv.  Compare spec vs implementation
      v.   Apply fix
      vi.  Build & test
      vii. If regression: revert, try next element
      viii.If improved: keep, re-measure
   c. After all elements tried:
      - If diff ≤ 1%: SUCCESS, move to next label
      - If diff > 1% but improved: PARTIAL, record improvement
      - If no improvement possible: SKIP (e.g., font metrics, library limitations)
4. Generate final report
```

## Known Limitations

These labels cannot reach <1% diff due to fundamental limitations:

| Label | Blocker | Reason |
|-------|---------|--------|
| ups, ups_surepost | MaxiCode encoder | Proprietary encoding, no compliant open-source encoder |
| fedex | PDF417 encoding | rxing encoder uses different compaction modes (valid but visually different) |
| aztec_ec | Aztec encoding | rxing Aztec encoder differs in symbol sizing |
| pnldpd | Multiple (Aztec + GFA + font) | Compound issues from barcode encoding + font metrics |
| Most text-heavy labels | Font metrics | Helvetica Bold vs Zebra proprietary fonts |

## Utility Code Reference

The skill uses Rust utilities in `src/skill/`:

- **`diff_scanner.rs`** — Parse diff report, find high-diff labels, suggest closest label names
- **`element_analyzer.rs`** — Compute element bounding boxes, correlate diff pixels with elements
- **`snippet_extractor.rs`** — Split ZPL commands, group into spans, extract standalone snippets
- **`models.rs`** — Data structures: `DiffReport`, `ElementBBox`, `ZplSnippet`, etc.
- **`error.rs`** — Error types: `ScanError`, `AnalyzeError`, `ExtractError`

### Phase 6: Commit — Generate Summary

27. **Generate a commit message** summarizing the changes:
    - List labels improved with before/after diff percentages
    - Summarize which rendering modules were modified
    - Note any threshold updates
    - Format: `fix(render): reduce diff for <labels> — <brief description>`

    Example:
    ```
    fix(render): reduce diff for amazon, dhl — fix ^FO baseline offset and Code128 quiet zone

    Labels improved:
    - amazon: 4.2% → 0.8%
    - dhl: 3.1% → 0.6%

    Changes:
    - src/parsers/zpl_parser.rs: correct ^FO y-offset calculation for rotated fields
    - src/barcodes/code128.rs: remove extra quiet zone module on right side
    - docs/DIFF_THRESHOLDS.md: update tolerances for amazon (1.0%), dhl (0.8%)
    ```

## References

- [Fix ZPL Render Skill](../fix-zpl-render/SKILL.md) — Detailed diagnostic procedures
- [ZPL Reference Skill](../zpl-reference/SKILL.md) — Official Zebra documentation lookup
- [Diff Thresholds](../../docs/DIFF_THRESHOLDS.md) — Per-label tolerance configuration
- [Rendering Fixes History](/memories/repo/labelize-rendering-fixes.md) — Previously applied fixes
