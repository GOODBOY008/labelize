---
name: tspl-reference
description: "Look up TSPL/TSPL2 command specifications from official TSC programming manuals. Use when: implementing TSPL commands, debugging TSPL parsing, verifying parameter syntax, checking defaults, coordinate units, text or bitmap font behavior, graphics, bitmap modes, barcode/QR/PDF417/DataMatrix/Aztec/MaxiCode semantics, or fixing TSPL rendering differences."
argument-hint: "TSPL command name or rendering question (e.g. 'SIZE units', 'TEXT alignment', 'BARCODE parameters')"
---

# TSPL Official Reference Lookup

## Purpose

Fetch and interpret TSPL/TSPL2 command specifications from the official TSC programming manual to resolve ambiguities in labelize's TSPL parser and renderer.

## When to Use

- Implementing or fixing a TSPL command parser
- A `.tspl` label renders incorrectly
- Unsure about command syntax, optional parameters, defaults, ranges, or units
- Text, graphics, bitmap, or barcode behavior is unclear
- Comparing labelize output with a real TSC printer, TSC docs, or Zebra TSPL compatibility docs

## Official Documentation

Primary source:

```
https://fs.tscprinters.com/en/dl/4/2541
```

Known direct PDF URL:

```
https://fs.tscprinters.com/system/files/31-0000001-00_tspl_tspl2_programming_3_0.pdf
```

If the direct PDF stops working, start from the TSC downloads page and search for `TSPL/ TSPL2 Programming Manual (English)`:

```
https://usca.tscprinters.com/en/downloads
```

Supplementary compatibility reference:

```
https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/software/zd100series-zd230series-zd888series-proman-en.pdf
```

## Common TSPL Command Areas

| Area | Commands |
|------|----------|
| Setup and media | `SIZE`, `GAP`, `BLINE`, `OFFSET`, `SPEED`, `DENSITY`, `DIRECTION`, `REFERENCE`, `SHIFT`, `CODEPAGE`, `CLS`, `PRINT` |
| Text | `TEXT`, `BLOCK`, resident bitmap fonts, downloaded fonts, code pages |
| Graphics | `BAR`, `BOX`, `CIRCLE`, `ELLIPSE`, `ERASE`, `REVERSE`, `BITMAP`, `PUTBMP`, `PUTPCX`, `PUTPNG` |
| 1D barcodes | `BARCODE`, `TLC39`, `RSS` |
| 2D barcodes | `QRCODE`, `PDF417`, `MPDF417`, `DMATRIX`, `MAXICODE`, `AZTEC` |
| Memory/files | `DOWNLOAD`, `EOP`, `FILES`, `KILL`, `MOVE`, `RUN` |
| Device control | `FEED`, `BACKFEED`, `FORMFEED`, `HOME`, `CUT`, `SELFTEST`, `SOUND` |

## Procedure

1. **Identify the command** — Extract the TSPL command name from the user's question, fixture, or parser code. TSPL command names are case-insensitive in practice; normalize to uppercase when searching.

2. **Fetch the official manual** — Use the web or fetch tool to load the TSC manual PDF. Search within the PDF for the command heading, then read the full command section.

3. **Extract key details:**
   - Syntax line and exact parameter order
   - Parameter types, required vs optional fields, defaults, ranges, and units
   - Coordinate behavior, especially dot units and effects of `REFERENCE`, `SHIFT`, and `DIRECTION`
   - Bitmap and barcode mode semantics
   - Printer-model or TSPL-vs-TSPL2 compatibility notes

4. **Compare with labelize implementation:**
   - Parser: `src/parsers/tspl_parser.rs`
   - Rendered elements: `src/elements/`
   - Renderer: `src/drawers/renderer.rs`
   - Barcode encoders: `src/barcodes/`
   - TSPL parser tests: `tests/unit_tspl_parser.rs`
   - Fixture labels: `testdata/tspl/`

5. **Report discrepancies** between the official spec and labelize's behavior before editing code. Include parameter position, official default, current implementation, and the expected rendering effect.

6. **Verify focused changes** with TSPL-specific tests first:
   ```bash
   cargo test --test unit_tspl_parser
   cargo test --test unit_renderer
   ```

   For parser or rendering changes with wider impact, also run:
   ```bash
   cargo test
   ```

## Labelize TSPL Notes

- `SIZE` may update per-label `DrawerOptions`; explicit CLI/API dimensions can override it per axis.
- Coordinates are interpreted in dots after applying `REFERENCE` and `SHIFT`.
- Supported render-focused commands currently include `SIZE`, `DIRECTION`, `REFERENCE`, `SHIFT`, `CLS`, `PRINT`, `TEXT`, `BAR`, `BOX`, `CIRCLE`, `ELLIPSE`, `ERASE`, `REVERSE`, `BITMAP`, `BARCODE`, `QRCODE`, and `PDF417`.
- Some TSPL commands may be accepted as device no-ops because they affect printer hardware rather than rendered output.
- Do not add command behavior from secondary references without checking the official TSC manual first.

## Example

User asks: "Why is TSPL TEXT too large?"

1. Fetch the TSC TSPL/TSPL2 Programming Manual.
2. Search for the `TEXT` command and resident font table.
3. Check font width/height, x/y multiplication, rotation, and alignment parameters.
4. Compare with `parse_text`, `tspl_font_info`, and TSPL font metrics in `src/elements/font.rs`.
5. Add or adjust focused tests in `tests/unit_tspl_parser.rs`, then run `cargo test --test unit_tspl_parser`.
