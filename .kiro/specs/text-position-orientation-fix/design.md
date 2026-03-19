# Text Position & Orientation Fix — Bugfix Design

## Overview

The labelize ZPL renderer produces text that diverges from Labelary reference output in two ways: (1) incorrect pixel-level positioning due to spurious fractional font-height offsets in `get_text_top_left_pos()`, and (2) missing glyph rotation — text is always drawn left-to-right regardless of the `FieldOrientation` setting. Additionally, `DrawerState::update_automatic_text_position()` advances the auto-position cursor incorrectly for rotated text. The fix will correct the position calculations, implement render-then-rotate for text (mirroring the existing barcode rotation approach), and fix auto-position advancement.

## Glossary

- **Bug_Condition (C)**: Any text rendering call where the current `get_text_top_left_pos()` produces incorrect coordinates OR the orientation is non-Normal and glyphs are not rotated
- **Property (P)**: Text is rendered at the correct top-left position and with the correct glyph rotation for the given orientation, matching Labelary reference output
- **Preservation**: All non-text elements (barcodes, graphic boxes, circles, diagonal lines, QR codes) and Normal-orientation text continue to render identically
- **`get_text_top_left_pos()`**: Function in `src/drawers/renderer.rs` that computes the (x, y) drawing origin for a `TextField` given its position mode (`^FO` vs `^FT`), orientation, and font metrics
- **`DrawerState::update_automatic_text_position()`**: Method in `src/drawers/drawer_state.rs` that advances the auto-position cursor after each `^FT` text field
- **`^FO` (Field Origin)**: ZPL command setting the top-left origin of a field (`calculate_from_bottom = false`)
- **`^FT` (Field Typeset)**: ZPL command setting the baseline/bottom-left origin of a field (`calculate_from_bottom = true`)
- **`FieldOrientation`**: Enum with variants `Normal`, `Rotated90` (R), `Rotated180` (I), `Rotated270` (B)

## Bug Details

### Bug Condition

The bug manifests in three interrelated ways:

1. **Position offsets**: `get_text_top_left_pos()` applies incorrect fractional font-height offsets (e.g., `y + 3h/4` for Normal, `x + h/4` for Rotated90) that shift text away from the expected position for both `^FO` and `^FT` commands.
2. **Missing rotation**: `draw_text()` calls `drawing::draw_text_mut()` directly onto the canvas without rotating the rendered glyphs, so text always appears left-to-right regardless of orientation.
3. **Auto-position error**: `update_automatic_text_position()` advances the cursor by measured text width in orientation-dependent directions, but the advancement direction and magnitude are incorrect for rotated text.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type TextField (with position, font.orientation, text, block)
  OUTPUT: boolean

  hasPositionOffsetBug :=
    (NOT input.position.calculate_from_bottom
      AND input.font.orientation == Normal AND appliedOffset != (0, 0))
    OR (NOT input.position.calculate_from_bottom
      AND input.font.orientation IN {Rotated90, Rotated180, Rotated270}
      AND appliedOffset != expectedRotatedOffset(input))
    OR (input.position.calculate_from_bottom
      AND computedTopLeft != expectedFTTopLeft(input))

  hasMissingRotationBug :=
    input.font.orientation IN {Rotated90, Rotated180, Rotated270}
    AND glyphsNotRotated(input)

  hasAutoPositionBug :=
    input.position.calculate_from_bottom
    AND input.position.automatic_position
    AND cursorAdvancement != expectedCursorAdvancement(input)

  RETURN hasPositionOffsetBug OR hasMissingRotationBug OR hasAutoPositionBug
END FUNCTION
```

### Examples

- **Normal + ^FO**: `^FO30,30^A0N,,28^FDGOGREEN^FS` — Current code draws at (30, 30 + 3h/4) ≈ (30, 51). Expected: draw at (30, 30). Text appears shifted ~21px downward.
- **Rotated90 + ^FO**: `^FO30,30^A0R,,28^FDGOGREEN^FS` — Current code draws at (30 + h/4, 30) with no glyph rotation. Expected: render text to buffer, rotate 90°, place at correct origin. Text appears unrotated and offset.
- **Rotated180 + ^FO**: `^FO30,30^A0I,,28^FDGOGREEN^FS` — Current code draws at (30 + w, 30 + h/4) with no glyph rotation. Expected: render to buffer, rotate 180°, place at correct origin.
- **Rotated270 + ^FO**: `^FO30,30^A0B,,28^FDGOGREEN^FS` — Current code draws at (30 + 3h/4, 30 + w) with no glyph rotation. Expected: render to buffer, rotate 270°, place at correct origin.
- **Normal + ^FT**: `^FT30,30^A0N,,28^FDGOGREEN^FS` — `^FT` specifies baseline position. Current code applies multiline offset but doesn't correctly convert baseline y to top-left y. Expected: y_top = y_baseline - font_height.
- **Auto-position**: `^FT10,200^A0N,30,20^FDACME ^FS^FT^A0N,30,20^FDSummer ^FS` — After "ACME ", cursor should advance by text width in x. Current advancement may be incorrect for rotated orientations.

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Mouse/barcode rendering with `overlay_with_rotation()` and `rotate_90`/`rotate_180`/`rotate_270` must continue to work identically
- Graphic boxes, circles, diagonal lines, and graphic fields must render at their specified positions
- `^FB` field block word wrapping and alignment logic must continue to work within the block boundaries
- Print width centering (`^PW`) must continue to center content
- Reverse print (`^FR`) must continue to invert text rendering correctly
- All existing golden tests for non-text elements must continue to pass within current tolerance

**Scope:**
All inputs that do NOT involve text rendering (`LabelElement::Text`) should be completely unaffected by this fix. This includes:
- All barcode types (Code128, EAN13, 2of5, Code39, PDF417, Aztec, DataMatrix, QR)
- Graphic elements (boxes, circles, diagonal lines, graphic fields)
- Label-level operations (print width, inversion, reverse print compositing)

## Hypothesized Root Cause

Based on the bug description and code analysis, the most likely issues are:

1. **Spurious fractional offsets in `get_text_top_left_pos()`**: The `^FO` branch applies orientation-dependent offsets like `y + 3h/4` (Normal) and `x + h/4` (Rotated90) that do not correspond to any ZPL specification requirement. These appear to be ad-hoc attempts to compensate for font metric differences but produce incorrect results. For `^FO`, the position should be the direct top-left corner of the bounding box (Normal) or the rotated equivalent.

2. **Missing render-then-rotate pattern for text**: Barcodes already use the pattern: render to buffer → call `rotate_90`/`rotate_180`/`rotate_270` → overlay onto canvas via `overlay_with_rotation()`. Text rendering skips this entirely — `draw_text_mut()` always draws left-to-right glyphs directly onto the canvas. The position adjustments in `get_text_top_left_pos()` were likely an attempt to simulate rotation by moving the origin, but this cannot produce correctly rotated glyphs.

3. **Incorrect `^FT` baseline-to-top-left conversion**: The `calculate_from_bottom` branch computes multiline offsets but doesn't properly subtract font height to convert from the ZPL baseline reference point to the top-left pixel coordinate needed by `draw_text_mut()`. The offset formula `(lines - 1) * (h + spacing)` handles multiline but misses the single-line baseline conversion.

4. **Auto-position cursor advancement direction**: `update_automatic_text_position()` advances by `w` (measured text width) in orientation-dependent directions, but after implementing actual rotation, the effective width/height of the rendered text buffer will be swapped for 90°/270° rotations, requiring the advancement to account for the rotated dimensions.

## Correctness Properties

Property 1: Bug Condition — Text Position Matches Reference for All Orientations

_For any_ `TextField` input with any `FieldOrientation` (Normal, Rotated90, Rotated180, Rotated270) and any position mode (`^FO` or `^FT`), the fixed `get_text_top_left_pos()` function SHALL compute a top-left drawing coordinate that, when combined with the rotated text buffer, produces pixel output matching the Labelary reference within the golden test tolerance.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

Property 2: Bug Condition — Text Glyphs Are Rotated for Non-Normal Orientations

_For any_ `TextField` input with `FieldOrientation` in {Rotated90, Rotated180, Rotated270}, the fixed `draw_text()` function SHALL render text glyphs to an intermediate buffer and rotate that buffer by the corresponding angle (90°, 180°, 270°) before compositing onto the canvas, producing correctly oriented text matching the Labelary reference.

**Validates: Requirements 2.6**

Property 3: Bug Condition — Automatic Position Advancement Is Correct

_For any_ sequence of `TextField` inputs using `^FT` with automatic positioning, the fixed `update_automatic_text_position()` SHALL advance the cursor by the correct dimension (width for Normal/Rotated180, height-equivalent for Rotated90/Rotated270) in the correct direction, so that subsequent fields are placed at positions matching the Labelary reference.

**Validates: Requirements 2.7**

Property 4: Preservation — Non-Text Elements Unchanged

_For any_ label input containing only non-text elements (barcodes, graphic boxes, circles, diagonal lines, graphic fields), the fixed code SHALL produce exactly the same rendered output as the original code, preserving all existing rendering behavior.

**Validates: Requirements 3.2, 3.3, 3.5, 3.6, 3.7**

Property 5: Preservation — Field Block Wrapping Unchanged

_For any_ `TextField` input with a `^FB` field block, the fixed code SHALL continue to wrap and align text within the block boundaries identically to the original code for Normal orientation, preserving word-wrap and alignment behavior.

**Validates: Requirements 3.4**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct:

**File**: `src/drawers/renderer.rs`

**Function**: `get_text_top_left_pos()`

**Specific Changes**:
1. **Remove spurious fractional offsets for `^FO` (calculate_from_bottom=false)**: For Normal orientation, return `(x, y)` directly without adding `3h/4`. For rotated orientations, compute the top-left of the rotated bounding box without the current ad-hoc offsets. After rotation, a text block of size (w, h) becomes (h, w) for 90°/270° and (w, h) for 180°. The `^FO` position is the top-left of the rotated bounding box.
2. **Fix `^FT` baseline conversion (calculate_from_bottom=true)**: For Normal, convert baseline y to top-left: `y_top = y_baseline - h`. For Rotated90, the baseline reference shifts: `x_top = x_baseline`, `y_top = y_baseline`. For Rotated180: `x_top = x_baseline - w`, `y_top = y_baseline`. For Rotated270: `x_top = x_baseline - h`, `y_top = y_baseline - w`. Account for multiline offsets in the correct axis per orientation.

**Function**: `draw_text()`

**Specific Changes**:
3. **Implement render-then-rotate for non-Normal orientations**: For Rotated90/180/270, render text to a temporary `RgbaImage` buffer (transparent background), then call the existing `rotate_90()`/`rotate_180()`/`rotate_270()` functions, then overlay the rotated buffer onto the canvas using `overlay_at()`. This mirrors the existing barcode rotation pattern.
4. **Handle field block text with rotation**: When `^FB` is present and orientation is non-Normal, render the entire block (with word wrapping and alignment) to the temporary buffer before rotating.

**File**: `src/drawers/drawer_state.rs`

**Function**: `update_automatic_text_position()`

**Specific Changes**:
5. **Fix cursor advancement for rotated text**: After implementing actual rotation, the effective rendered dimensions change. For Normal, advance x by text width. For Rotated90, advance y by text width (the rotated buffer's height is the original width). For Rotated180, advance x by negative text width (text extends leftward). For Rotated270, advance y by negative text width. Verify advancement directions match Labelary behavior.

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm or refute the root cause analysis. If we refute, we will need to re-hypothesize.

**Test Plan**: Render the existing text orientation test ZPL files (`text_fo_n.zpl`, `text_fo_r.zpl`, `text_fo_b.zpl`, `text_fo_i.zpl`, `text_ft_n.zpl`, `text_ft_r.zpl`, `text_ft_b.zpl`, `text_ft_i.zpl`, `text_ft_auto_pos.zpl`) and compare against their Labelary reference PNGs. Measure pixel diff percentages to quantify the divergence.

**Test Cases**:
1. **Normal ^FO Test**: Render `text_fo_n.zpl` and compare — expect vertical offset mismatch (will fail on unfixed code)
2. **Rotated90 ^FO Test**: Render `text_fo_r.zpl` and compare — expect position + rotation mismatch (will fail on unfixed code)
3. **Rotated180 ^FO Test**: Render `text_fo_i.zpl` and compare — expect position + rotation mismatch (will fail on unfixed code)
4. **Rotated270 ^FO Test**: Render `text_fo_b.zpl` and compare — expect position + rotation mismatch (will fail on unfixed code)
5. **Normal ^FT Test**: Render `text_ft_n.zpl` and compare — expect baseline conversion mismatch (will fail on unfixed code)
6. **Rotated ^FT Tests**: Render `text_ft_r.zpl`, `text_ft_i.zpl`, `text_ft_b.zpl` — expect combined position + rotation mismatch (will fail on unfixed code)
7. **Auto-Position Test**: Render `text_ft_auto_pos.zpl` — expect cumulative position drift (will fail on unfixed code)

**Expected Counterexamples**:
- Text pixels appear at wrong coordinates (shifted by fractional font-height amounts)
- Text glyphs appear unrotated for R/I/B orientations
- Possible causes: spurious offsets in `get_text_top_left_pos()`, missing rotation in `draw_text()`, incorrect auto-position advancement

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed function produces the expected behavior.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := draw_text_fixed(input)
  ASSERT pixelDiff(result, labelaryReference(input)) <= tolerance
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed function produces the same result as the original function.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT render_original(input) = render_fixed(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many test cases automatically across the input domain
- It catches edge cases that manual unit tests might miss
- It provides strong guarantees that behavior is unchanged for all non-buggy inputs

**Test Plan**: Observe behavior on UNFIXED code first for non-text elements and Normal-orientation text, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Barcode Rendering Preservation**: Render labels with various barcode types and orientations on unfixed code, then verify identical output after fix
2. **Graphic Element Preservation**: Render labels with graphic boxes, circles, diagonal lines on unfixed code, then verify identical output after fix
3. **Normal Text Preservation**: Render Normal-orientation text with ^FO on unfixed code, observe current output, then verify the fix improves (not regresses) alignment with Labelary reference
4. **Field Block Preservation**: Render multiline text with ^FB on unfixed code, then verify word wrapping and alignment continue to work after fix

### Unit Tests

- Test `get_text_top_left_pos()` with each orientation × position mode combination, asserting expected (x, y) output
- Test `update_automatic_text_position()` with sequential ^FT fields in each orientation, asserting cursor position after each call
- Test edge cases: zero-width text, empty string, single character, very large font sizes
- Test that `draw_text()` produces a rotated buffer for non-Normal orientations

### Property-Based Tests

- Generate random `TextField` inputs with arbitrary orientations, positions, font sizes, and text content; verify that `get_text_top_left_pos()` returns coordinates within the canvas bounds
- Generate random non-text label elements; verify that rendering output is byte-identical before and after the fix
- Generate random `^FT` auto-position sequences; verify cursor advancement is monotonic in the expected direction per orientation

### Integration Tests

- Run all existing golden tests and verify non-text tests pass within current tolerance
- Run text golden tests (`text_fo_*`, `text_ft_*`, `text_multiline`, `text_ft_auto_pos`) and verify improved pixel diff against Labelary reference
- Test full shipping label ZPLs (amazon, fedex, ups, usps) that contain mixed text and barcode elements
