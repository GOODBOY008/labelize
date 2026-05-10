---
name: fix-zpl-render
description: "Diagnose and fix ZPL rendering differences by auto-learning from official Zebra documentation. Use when: rendered label doesn't match expected output, pixel diff is too high, barcode looks wrong, text positioning is off, graphic elements are misplaced, golden test fails, rendering bug, wrong render, visual mismatch, diff threshold exceeded."
argument-hint: "Describe the rendering problem or label name (e.g. 'fedex label PDF417 wrong', 'text baseline too high')"
---

# Fix ZPL Rendering — Auto-Learn from Official Docs

## Purpose

Systematically diagnose rendering mismatches in labelize by:
1. Identifying which ZPL command(s) cause the visual difference
2. Fetching the official Zebra spec for those commands
3. Comparing spec behavior with labelize implementation
4. Applying the fix

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

- A golden test exceeds its diff threshold
- Visual inspection shows wrong barcode, text position, or graphic
- A new ZPL label renders incorrectly
- Need to reduce diff percentage for a specific label

## Procedure

### Phase 1: Identify the Problem

1. **Get the label source** — Read the ZPL file from `testdata/labels/<name>.zpl` or `testdata/unit/<name>.zpl`
2. **Check current diff** — Look up the label in [docs/DIFF_THRESHOLDS.md](../../docs/DIFF_THRESHOLDS.md) for the current diff percentage and known cause
3. **Run the golden test** for the specific label:
   ```bash
   cargo test --test e2e -- <label_name> --nocapture
   ```
4. **Inspect the diff image** — Check `testdata/diffs/<name>.png` to see which pixels differ (red = mismatch)
5. **Parse the ZPL** — Read through the `.zpl` file and list every ZPL command used

### Phase 2: Learn from Official Docs

6. **For each suspect command**, fetch the official Zebra documentation:
   ```
   https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-<slug>.html
   ```

   Use the `#tool:fetch_webpage` tool. Common command slugs:

   | Command | Slug | Category |
   |---------|------|----------|
   | `^A` | `a` | Font/text |
   | `^BC` | `bc` | Code 128 barcode |
   | `^BQ` | `bq` | QR Code |
   | `^B7` | `b7` | PDF417 |
   | `^BO` | `b0` | Aztec |
   | `^BX` | `bx` | DataMatrix |
   | `^BD` | `bd` | MaxiCode |
   | `^BY` | `by` | Barcode defaults |
   | `^FB` | `fb` | Field block (multiline) |
   | `^FO` | `fo` | Field origin |
   | `^FT` | `ft` | Field typeset |
   | `^FW` | `fw` | Field orientation |
   | `^GB` | `gb` | Graphic box |
   | `^GD` | `gd` | Diagonal line |
   | `^GF` | `gf` | Graphic field |
   | `^CF` | `cf` | Default font |
   | `^CI` | `ci` | Character set |
   | `^LH` | `lh` | Label home |
   | `^PO` | `po` | Print orientation |
   | `^PW` | `pw` | Print width |

7. **Extract from the doc:**
   - Exact parameter order, types, and default values
   - Parameter interaction rules (e.g., "if omitted, uses value set by `^BY`")
   - Coordinate system details (top-left vs baseline, dot units)
   - Any special behavior notes

8. **For field interactions**, also fetch:
   ```
   https://docs.zebra.com/us/en/printers/software/zpl-pg/r-zpl-interactions-field-interactions.html
   ```

### Phase 3: Compare with Implementation

9. **Find the parser code** — Search in `src/parsers/zpl_parser.rs` for the command prefix
10. **Find the element struct** — Check `src/elements/<relevant_element>.rs`
11. **Find the renderer code** — Check `src/drawers/renderer.rs` for how the element is drawn
12. **For barcodes** — Also check `src/barcodes/<barcode_type>.rs` for encoding logic

13. **Compare systematically:**

    | Aspect | Official Spec | Labelize Code | Match? |
    |--------|--------------|---------------|--------|
    | Parameter order | ... | ... | ✓/✗ |
    | Default values | ... | ... | ✓/✗ |
    | Coordinate origin | ... | ... | ✓/✗ |
    | Unit interpretation | ... | ... | ✓/✗ |
    | Interaction with ^BY/^FW/etc | ... | ... | ✓/✗ |

### Phase 4: Fix and Verify

14. **Implement the fix** — Edit the parser, element, renderer, or barcode encoder as needed
15. **Run the specific test:**
    ```bash
    cargo test --test e2e -- <label_name> --nocapture
    ```
16. **Run all tests** to check for regressions:
    ```bash
    cargo test
    ```
17. **Update threshold** — If the diff improved, update `docs/DIFF_THRESHOLDS.md` with the new percentage

## Diagnostic Patterns

### Common Root Causes

| Symptom | Likely Cause | Where to Look |
|---------|-------------|---------------|
| Text shifted vertically | `^FT` baseline vs `^FO` top-left confusion | `renderer.rs` → `get_text_top_left_pos` |
| Text too wide/narrow | Font width ratio wrong | `elements/font.rs` → width calculation |
| Barcode too wide | `^BY` module width default wrong | `zpl_parser.rs` → `^BY` parsing |
| Barcode wrong encoding | Barcode encoder bug | `barcodes/<type>.rs` |
| Rotated element in wrong position | Rotation pivot point off | `renderer.rs` → rotation transforms |
| Graphic box wrong size | Border thickness double-counted | `renderer.rs` → `draw_box` |
| Reverse print inverted | Compositing logic error | `renderer.rs` → reverse print buffer |
| Missing element | Parser doesn't handle command | `zpl_parser.rs` → command matching |
| GFA graphic corrupt | Hex/compressed data parsing | `parsers/zpl_parser.rs` → `^GF` handler |

### Known High-Diff Labels and Focus Areas

| Label | Diff | Primary Issue | Fix Focus |
|-------|------|---------------|-----------|
| ups, ups_surepost | 25-36% | MaxiCode encoding | `barcodes/maxicode.rs` — needs compliant encoder |
| fedex | 18% | PDF417 encoding | `barcodes/pdf417.rs` — data encoding gaps |
| pnldpd | 14% | GFA + Aztec + Code128 | Multiple: graphic field parsing, barcode encoding |
| porterbuddy | 14% | GFA + QR | Graphic field Z/LZ4 decompression, QR sizing |

## References

- [ZPL Command Index](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands.html)
- [Field Interactions](https://docs.zebra.com/us/en/printers/software/zpl-pg/r-zpl-interactions-field-interactions.html)
- [Fonts and Barcodes](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-font-barcodes-fonts-andbar-codes.html)
- [Diff Thresholds](../../docs/DIFF_THRESHOLDS.md)
- [Rendering Fixes History](/memories/repo/labelize-rendering-fixes.md)
