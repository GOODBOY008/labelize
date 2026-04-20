# Tasks

## Task 1: Create Data Models and Error Types

- [x] 1.1 Create `src/skill/mod.rs` module file and register it in `src/lib.rs`, exposing all sub-modules
- [x] 1.2 Create `src/skill/models.rs` with data model structs: `DiffReportEntry`, `DiffStatus`, `DiffReport`, `ElementBBox`, `ElementType`, `ElementDiffContribution`, `ZplSnippet`, `ZplCommandSpan`, `ZplCommand`, `FixCategory` — all with `Debug`, `Clone` derives
- [x] 1.3 Create `src/skill/error.rs` with error types: `ScanError`, `AnalyzeError`, `ExtractError`, `SkillError` using `thiserror` — includes `LabelNotFound` variant with available labels list and edit-distance suggestion

## Task 2: Implement DiffScanner

- [x] 2.1 Create `src/skill/diff_scanner.rs` with `parse_diff_report()`, `load_diff_report()`, and helper functions
- [x] 2.2 Implement `parse_diff_report()` — parses the Unicode table format from `testdata/diffs/diff_report.txt` into `DiffReport` extracting label name, extension, diff percentage, dimensions, and status
- [x] 2.3 Implement `DiffStatus::from_percent()` — maps diff percentage to status category: Perfect=0%, Good=(0%,1%), Minor=[1%,5%), Moderate=[5%,15%), High=[15%,100%]
- [x] 2.4 Implement `find_high_diff_labels()` — filters entries where diff_percent > tolerance (or MODERATE/HIGH if no tolerance set)
- [x] 2.5 Implement `find_labels_above_threshold()` — filters entries by a specific diff% threshold
- [x] 2.6 Implement `scan_label()` — looks up a single label by name, returns `ScanError::LabelNotFound` with closest match suggestion
- [x] 2.7 Implement `suggest_closest_label()` using Levenshtein edit distance
- [x] 2.8 Implement `enrich_with_tolerances()` — parses `docs/DIFF_THRESHOLDS.md` markdown table and enriches report entries with tolerance values

## Task 3: Implement ElementAnalyzer

- [x] 3.1 Create `src/skill/element_analyzer.rs` with analysis functions
- [x] 3.2 Implement `compute_element_bbox()` for all drawable element types: Text, GraphicBox, GraphicCircle, DiagonalLine, GraphicField, Barcode128, BarcodeEan13, Barcode2of5, Barcode39, BarcodePdf417, BarcodeAztec, BarcodeDatamatrix, BarcodeQr, Maxicode
- [x] 3.3 Implement `count_red_pixels()` and `count_red_pixels_in_rect()` — identify diff pixels using threshold R>200, G<50, B<50, A>200
- [x] 3.4 Implement `correlate_diff_regions()` — computes `ElementDiffContribution` for each bbox with local_diff_percent and contribution_to_total, sorted descending
- [x] 3.5 Implement `analyze_label()` — orchestrates: compute bboxes → load diff image → correlate regions → return ranked contributions
- [x] 3.6 Implement `format_analysis_report()` — human-readable tabular output of analysis results

## Task 4: Implement SnippetExtractor

- [x] 4.1 Create `src/skill/snippet_extractor.rs` with extraction functions
- [x] 4.2 Implement `split_zpl_commands()` — splits raw ZPL text into individual `ZplCommand` structs at `^` and `~` boundaries (2-char prefix)
- [x] 4.3 Implement `group_commands_into_spans()` — groups commands from `^FO`/`^FT` through `^FS` into `ZplCommandSpan` entries, one per drawable element
- [x] 4.4 Implement `extract_global_state_commands()` — identifies global state commands: `^LH`, `^PW`, `^CF`, `^BY`, `^CI`, `^FW`, `^PO`, `^LR`, `^LL`
- [x] 4.5 Implement `extract_element()` — builds standalone ZPL snippet with `^XA`/`^XZ` wrapper, deduplicated global state, and element span; writes to `testdata/snippets/`
- [x] 4.6 Implement `extract_element_group()` — combines multiple element spans into a single standalone ZPL file
- [x] 4.7 Implement `write_snippet()` and `extract_all_high_diff_elements()` — disk I/O helpers

## Task 5: Create SKILL.md for Auto-Fix Workflow

- [x] 5.1 Create `.github/skills/zpl-diff-auto-fix/SKILL.md` with the full agent workflow
- [x] 5.2 Document Phase 1 (Scan), Phase 2 (Analyze), Phase 3 (Extract), Phase 4 (Fix), Phase 5 (Finalize) procedures
- [x] 5.3 Include element type → fix category → target source file mapping table
- [x] 5.4 Document auto-fix loop strategy for batch mode
- [x] 5.5 Document known limitations (MaxiCode, PDF417, font metrics) that cannot reach <1%
- [x] 5.6 Reference related skills (fix-zpl-render, zpl-reference) and utility code

## Task 6: Write Tests

- [x] 6.1 Unit tests for DiffScanner: `test_parse_dims`, `test_classify_status`, `test_edit_distance`, `test_suggest_closest`, `test_parse_report_sample`, `test_scan_label_not_found` (6 tests)
- [x] 6.2 Unit tests for SnippetExtractor: `test_split_commands`, `test_group_spans`, `test_extract_global_state`, `test_extract_element_snippet`, `test_extract_barcode_snippet`, `test_extract_element_out_of_range`, `test_extract_group` (7 tests)
- [x] 6.3 Integration test `test_load_and_parse_real_diff_report` — validates parsing against real diff_report.txt (83+ labels)
- [x] 6.4 Integration test `test_load_report_with_tolerances` — validates tolerance enrichment from DIFF_THRESHOLDS.md
- [x] 6.5 Integration test `test_analyze_bstc_label` — verifies PERFECT label has zero diff contributions
- [x] 6.6 Integration test `test_analyze_label_with_diff` — verifies amazon label analysis produces ranked contributions
- [x] 6.7 Integration test `test_extract_snippets_from_real_label` — verifies snippet extraction and rendering from real ZPL
- [x] 6.8 Integration test `test_full_analysis_pipeline` — runs full scan→analyze→extract pipeline on real data

## Task 7: Local Verification

- [x] 7.1 All 13 unit tests pass (`cargo test --lib skill`)
- [x] 7.2 All 6 integration tests pass (`cargo test --test unit_skill`)
- [x] 7.3 All 82 golden e2e tests pass — no rendering regressions
- [x] 7.4 Full test suite: 232 tests pass, 0 failures

## Task 8: Implement Diff Classification (Content vs Position)

- [ ] 8.1 Add `DiffClassification` enum (`ContentDiff`, `PositionDiff`, `Mixed`) and `PositionOffsetInfo` struct with `OffsetDetectionMethod` enum to `src/skill/models.rs`
- [ ] 8.2 Add `classification: Option<DiffClassification>` field to `ElementDiffContribution` in `src/skill/models.rs`
- [ ] 8.3 Update `FixCategory::from_element_type()` to `FixCategory::from_classification()` that takes both `ElementType` and `DiffClassification` — ContentDiff uses element-type-based mapping, PositionDiff always maps to `PositionOffset` or `CommandParsing`
- [ ] 8.4 Add `position_offset: Option<PositionOffsetInfo>` and `classification: DiffClassification` fields to `FixHypothesis` struct (design only — not yet implemented in code)

## Task 9: Implement Snippet-Based Diff Classification

- [ ] 9.1 Create `src/skill/diff_classifier.rs` with `classify_element_diffs()` function
- [ ] 9.2 Implement `render_snippet_isolated()` — renders a ZPL snippet using the existing Labelize renderer and returns the image
- [ ] 9.3 Implement `fetch_labelary_snippet_render()` — sends the snippet ZPL to Labelary API and returns the reference image (with caching to `testdata/snippets/labelary_cache/`)
- [ ] 9.4 Implement `classify_element_diffs()` — for each element: render snippet isolated, fetch Labelary render, compare the two, classify as ContentDiff/PositionDiff/Mixed based on snippet_threshold (default 2%)
- [ ] 9.5 Implement `detect_shadow_pattern()` — analyzes diff pixel distribution in bbox to detect "double image" offset pattern (element appears at both correct and incorrect positions)
- [ ] 9.6 Implement `compute_diff_centroid()` — computes the centroid of diff pixels within a bbox region and compares to expected element center
- [ ] 9.7 Implement `detect_position_offset()` — orchestrates shadow detection, cross-correlation, and centroid shift to produce `PositionOffsetInfo` with confidence score
- [ ] 9.8 Register `diff_classifier` module in `src/skill/mod.rs`

## Task 10: Update Analysis Pipeline for Classification

- [ ] 10.1 Update `analyze_label()` in `src/skill/element_analyzer.rs` to call `classify_element_diffs()` after correlating diff regions
- [ ] 10.2 Update `format_analysis_report()` to include classification column (Content/Position/Mixed) and offset info for PositionDiff elements
- [ ] 10.3 Update SKILL.md to document the diff classification step in the analysis phase and the classification-aware fix strategy

## Task 11: Write Tests for Diff Classification

- [ ] 11.1 Unit test `test_classify_content_diff` — synthetic case where isolated snippet diff is high → ContentDiff
- [ ] 11.2 Unit test `test_classify_position_diff` — synthetic case where isolated snippet matches but full-label diff is high → PositionDiff
- [ ] 11.3 Unit test `test_classify_mixed` — synthetic case where both diffs are high → Mixed
- [ ] 11.4 Unit test `test_classify_no_diff` — element with zero diff pixels gets no classification
- [ ] 11.5 Unit test `test_fix_category_from_classification` — verify ContentDiff uses element-type mapping, PositionDiff always maps to PositionOffset/CommandParsing
- [ ] 11.6 Unit test `test_centroid_shift_detection` — verify centroid computation on synthetic diff image
- [ ] 11.7 Integration test `test_classify_real_label_diffs` — run classification on a real high-diff label and verify classifications are reasonable

## Task 12: Local Verification (Post-Classification)

- [ ] 12.1 All new unit tests pass (`cargo test --lib skill`)
- [ ] 12.2 All new integration tests pass (`cargo test --test unit_skill`)
- [ ] 12.3 All existing tests still pass — no regressions from model changes
- [ ] 12.4 Full test suite passes
