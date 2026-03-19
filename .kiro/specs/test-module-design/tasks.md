# Implementation Plan: Test Module Design for Labelize

## Overview

Build a comprehensive, multi-layered test suite for the labelize Rust project. Tasks are ordered so shared infrastructure is built first, then unit tests, then property tests, then E2E tests. Each task builds incrementally on the previous ones, ensuring no orphaned code.

## Tasks

- [ ] 1. Add dev-dependencies and set up test directory structure
  - [ ] 1.1 Add `reqwest` (blocking) and `sha2` to `[dev-dependencies]` in `Cargo.toml`
    - Add `reqwest = { version = "0.12", features = ["blocking"] }` and `sha2 = "0.10"`
    - _Requirements: 12.4, 9.1_

  - [ ] 1.2 Create test directory skeleton and crate roots
    - Create `tests/common/mod.rs`, `tests/common/render_helpers.rs`, `tests/common/image_compare.rs`, `tests/common/labelary_client.rs`, `tests/common/proptest_strategies.rs`
    - Create `tests/unit.rs` (crate root declaring `mod common; mod unit;`) and `tests/unit/mod.rs` with submodule declarations
    - Create `tests/e2e.rs` (crate root declaring `mod common; mod e2e;`) and `tests/e2e/mod.rs` with submodule declarations
    - Create stub files for all unit test modules: `zpl_parser.rs`, `epl_parser.rs`, `barcodes.rs`, `renderer.rs`, `png_encoder.rs`, `pdf_encoder.rs`, `hex_encoding.rs`, `property_tests.rs`
    - Create stub files for E2E test modules: `golden.rs`, `labelary.rs`
    - _Requirements: 12.6_

- [ ] 2. Implement shared test helpers (`tests/common/`)
  - [ ] 2.1 Implement `render_helpers.rs`
    - Implement `render_zpl_to_png(zpl: &str, options: DrawerOptions) -> Vec<u8>` — full parse→render→encode pipeline
    - Implement `render_epl_to_png(epl: &str, options: DrawerOptions) -> Vec<u8>` — full parse→render→encode pipeline
    - Implement `parse_zpl(zpl: &[u8]) -> Vec<LabelInfo>` and `parse_epl(epl: &[u8]) -> Vec<LabelInfo>`
    - Implement `default_options() -> DrawerOptions` returning 102mm × 152mm, 8 dpmm
    - _Requirements: 12.1, 12.2_

  - [ ] 2.2 Implement `image_compare.rs`
    - Define `CompareResult` struct with `diff_percent`, `dimensions_match`, `actual_dims`, `expected_dims`, `diff_image`
    - Implement `compare_images(actual: &[u8], expected: &[u8], tolerance: f64) -> CompareResult` with per-channel threshold of 32
    - Implement `compare_ssim(actual: &[u8], expected: &[u8]) -> f64` for structural similarity
    - Implement `save_diff_image(name: &str, diff: &RgbaImage)` saving to `testdata/diffs/`
    - Assert dimensions separately before pixel comparison
    - _Requirements: 12.3, 10.2, 10.5, 10.6_

  - [ ] 2.3 Implement `labelary_client.rs`
    - Implement `labelary_render(zpl: &str, dpmm: u8, width_inches: f64, height_inches: f64) -> Option<Vec<u8>>`
    - POST to `http://api.labelary.com/v1/printers/{dpmm}/labels/{width}x{height}/0/` with `Accept: image/png`
    - Implement SHA-256 cache key from `(zpl, dpmm, width, height)`, cache in `testdata/labelary_cache/`
    - Implement token-bucket rate limiter at 3 requests/second using `std::time::Instant`
    - Return `None` on network errors, HTTP errors, or malformed responses (skip test gracefully)
    - _Requirements: 12.4, 9.1, 9.2, 9.6, 9.7_

  - [ ] 2.4 Implement `proptest_strategies.rs`
    - Implement `arb_drawer_options()` — positive dimensions, valid dpmm (6, 8, 12, 24)
    - Implement `arb_zpl_label()` — `^XA` + random commands + `^XZ`
    - Implement `arb_code128_input()` — non-empty ASCII strings (chars 32-127)
    - Implement `arb_2of5_input()` — even-length digit strings
    - Implement `arb_ean13_input()` — 12-digit strings
    - Implement `arb_qr_input()` — 1-100 ASCII characters
    - Implement `arb_hex_string()` — strings of hex characters (0-9, a-f, A-F)
    - Implement `arb_rgba_image()` — random RgbaImage (1-200 × 1-200)
    - _Requirements: 12.5_

- [ ] 3. Checkpoint - Verify shared infrastructure compiles
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 4. Implement unit tests for ZPL parser (`tests/unit/zpl_parser.rs`)
  - [ ] 4.1 Implement ZPL parser unit tests for font, barcode, field, graphic, config, and template commands
    - Test single-command label blocks produce correct `LabelElement` variants (one test per command group)
    - Test multi-label blocks (`^XA...^XZ` repeated) produce correct `Vec<LabelInfo>` length
    - Test font commands (`^A`, `^CF`, `^CW`) set `FontInfo` fields correctly
    - Test barcode config commands (`^BY`, `^BC`, `^B3`, `^B7`, `^B0`, `^BE`, `^BQ`, `^BX`, `^BD`) produce correct barcode elements
    - Test field positioning (`^FO`, `^FT`) sets `LabelPosition` correctly
    - Test graphic commands (`^GB`, `^GC`, `^GD`, `^GF`, `^GS`, `^GE`) produce correct graphic elements
    - Test label config commands (`^LH`, `^LL`, `^PW`, `^PO`, `^PM`, `^LR`) update printer state
    - Test field block (`^FB`) produces correct `FieldBlock` configuration
    - Test hex-escaped field data (`^FH` + `^FD`) decodes hex sequences
    - Test template commands (`^DF`, `^XF`, `^XG`, `^FN`) store/recall formats
    - Test malformed commands use defaults or return descriptive errors
    - Test field reversal (`^FR`) and label reversal (`^LR`) set `reverse_print` flag
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 1.12, 11.1_

- [ ] 5. Implement unit tests for EPL parser (`tests/unit/epl_parser.rs`)
  - [ ] 5.1 Implement EPL parser unit tests
    - Test text command (`A`) produces `LabelElement::Text` with correct position, font, content
    - Test barcode command (`B`) produces correct barcode `LabelElement` variant
    - Test line command (`LO`) produces `LabelElement::GraphicBox` with correct dimensions
    - Test reference point command (`R`) offsets subsequent element positions
    - Test clear command (`N`) resets element list
    - Test print command (`P`) finalizes label
    - Test malformed EPL commands are skipped or produce descriptive errors
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [ ] 6. Implement unit tests for barcode generators (`tests/unit/barcodes.rs`)
  - [ ] 6.1 Implement barcode generator unit tests for all symbologies
    - Test Code128 (`encode_auto`, `encode_no_mode`) produces non-zero dimension output
    - Test Code39 produces `BitMatrix` with correct module count for input length
    - Test EAN-13 produces `BitMatrix` encoding 13 digits (12 data + 1 check)
    - Test Interleaved 2-of-5 produces `BitMatrix` encoding digit pairs
    - Test PDF417 produces 2D `BitMatrix` with valid row structure
    - Test Aztec produces square `BitMatrix`
    - Test DataMatrix produces `BitMatrix` with even width and height
    - Test QR code produces square `BitMatrix`
    - Test MaxiCode produces `BitMatrix` with fixed MaxiCode dimensions
    - Test empty/invalid input returns `Err` with descriptive message for each symbology
    - Test `BitMatrix` `width()` × `height()` matches actual data dimensions
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 11.2_

- [ ] 7. Implement unit tests for renderer (`tests/unit/renderer.rs`)
  - [ ] 7.1 Implement renderer unit tests
    - Test `GraphicBox` renders black pixels in box region, white outside
    - Test `GraphicCircle` renders circular outline with correct diameter and border thickness
    - Test `Text` element renders non-white pixels in text region
    - Test barcode element renders barcode pattern at specified position
    - Test `inverted = true` with `enable_inverted_labels = true` produces 180° rotated output
    - Test `print_width` smaller than label width centers content
    - Test `reverse_print` element inverts pixels against canvas
    - Test different `dpmm` values (6, 8, 12, 24) scale canvas dimensions proportionally
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 11.3, 11.4_

- [ ] 8. Implement unit tests for encoders and hex utilities
  - [ ] 8.1 Implement PNG encoder unit tests (`tests/unit/png_encoder.rs`)
    - Test encode/decode round-trip preserves dimensions
    - Test pixels above threshold (128) map to white (255)
    - Test pixels at or below threshold (128) map to black (0)
    - Test valid PNG bytes are produced from known pixel values
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

  - [ ] 8.2 Implement PDF encoder unit tests (`tests/unit/pdf_encoder.rs`)
    - Test non-empty PDF bytes are produced
    - Test PDF output starts with `%PDF-` header
    - Test `MediaBox` dimensions match `DrawerOptions` (mm × 2.834645669)
    - Test label image is embedded as grayscale XObject
    - _Requirements: 6.1, 6.2, 6.3, 6.4_

  - [ ] 8.3 Implement hex and encoding utility unit tests (`tests/unit/hex_encoding.rs`)
    - Test valid hex string decodes to correct bytes
    - Test odd-length hex string pads with trailing zero
    - Test invalid hex characters return error
    - Test compressed graphic field data (G-Y, g-z) expands correctly
    - Test Z64-encoded data (`:Z64:` prefix) decodes and decompresses
    - Test ZPL charset conversion produces correct Unicode
    - Test hex encode/decode round-trip
    - Test escaped string with hex escape character replaces sequences correctly
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

- [ ] 9. Checkpoint - Ensure all unit tests compile and pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 10. Implement property-based tests (`tests/unit/property_tests.rs`)
  - [ ] 10.1 Implement parser property tests
    - Write proptest for Property 1: ZPL block count invariant — N `^XA...^XZ` blocks produce `Vec<LabelInfo>` of length N
    - Write proptest for Property 2: ZPL parser robustness — arbitrary content between `^XA`/`^XZ` never panics
    - Write proptest for Property 3: ZPL field positioning preservation — `^FO`/`^FT` values preserved in `LabelPosition`
    - Write proptest for Property 4: EPL reference point offset — `R{rx},{ry}` offsets subsequent element positions
    - Write proptest for Property 5: EPL parser robustness — arbitrary string input never panics
    - _Requirements: 1.2, 1.5, 1.11, 2.4, 2.7, 8.1_

  - [ ]* 10.2 Implement barcode property tests
    - **Property 6: Code128 no-panic and non-zero output**
    - **Property 7: 1D barcode width proportional to input length**
    - **Property 8: EAN-13 fixed module width (95 modules)**
    - **Property 9: 2D barcode square invariant (Aztec, QR)**
    - **Property 10: DataMatrix even dimensions**
    - **Property 11: MaxiCode fixed dimensions**
    - **Property 12: Barcode invalid input produces error**
    - **Property 13: BitMatrix dimensions consistency**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 8.2, 8.3, 8.7, 8.8**

  - [ ]* 10.3 Implement renderer property tests
    - **Property 14: GraphicBox pixel correctness**
    - **Property 15: Text rendering produces non-white pixels**
    - **Property 16: Label inversion is 180° rotation**
    - **Property 17: Renderer canvas dimensions match DrawerOptions**
    - **Validates: Requirements 4.1, 4.3, 4.5, 4.8, 8.5**

  - [ ]* 10.4 Implement encoder property tests
    - **Property 18: PNG encode/decode round-trip**
    - **Property 19: PNG monochrome threshold mapping**
    - **Property 20: PDF output validity**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 8.4**

  - [ ]* 10.5 Implement hex/encoding property tests
    - **Property 21: Hex encode/decode round-trip**
    - **Property 22: Invalid hex produces error**
    - **Property 23: Hex decode output length**
    - **Property 24: Z64 encode/decode round-trip**
    - **Property 25: Hex escape sequence decoding**
    - **Validates: Requirements 7.1, 7.3, 7.5, 7.7, 7.8, 8.6**

- [ ] 11. Checkpoint - Ensure all unit and property tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 12. Implement improved golden-file E2E tests (`tests/e2e/golden.rs`)
  - [ ] 12.1 Implement golden-file test infrastructure
    - Define `TestCaseConfig` struct with `name`, `input_file`, `golden_file`, `tolerance`, `category`
    - Define `TestCategory` enum (Text, Barcode, Graphic, Mixed)
    - Implement `golden_zpl` and `golden_epl` test runners using `render_helpers` and `image_compare`
    - Support per-test tolerance values (default 15% for new tests, 50% for migrated tests)
    - Assert dimensions separately before pixel comparison
    - Generate diff images on failure and save to `testdata/diffs/`
    - Support `LABELIZE_UPDATE_GOLDEN` env var to overwrite golden files
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.7, 12.7_

  - [ ] 12.2 Migrate existing golden tests and add command coverage tests
    - Migrate all existing tests from `tests/e2e_golden.rs` into the new `golden.rs` module with per-test tolerances
    - Add golden tests covering each ZPL command group (fonts, barcodes, fields, graphics, config, download, recall, print, extensions)
    - Add golden tests for each `FieldOrientation` variant (Normal, Rotated90, Rotated180, Rotated270)
    - Add golden tests for each `dpmm` value (6, 8, 12, 24)
    - _Requirements: 11.1, 11.3, 11.4_

- [ ] 13. Implement Labelary comparison E2E tests (`tests/e2e/labelary.rs`)
  - [ ] 13.1 Implement Labelary comparison tests
    - Implement one test per ZPL command category: fonts, barcodes, fields, graphics, label config
    - Each test renders ZPL with labelize and fetches Labelary reference via `labelary_render()`
    - Compare using `compare_images()` with 15% tolerance
    - Skip gracefully when Labelary API is unreachable
    - Log pixel-difference percentage for each comparison
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 11.1_

- [ ] 14. Remove old test file and final wiring
  - [ ] 14.1 Remove `tests/e2e_golden.rs`
    - Delete the old single-file golden test now that all tests are migrated to `tests/e2e/golden.rs`
    - Verify no references to the old file remain
    - _Requirements: 12.6_

- [ ] 15. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document (Properties 1-25)
- The implementation language is Rust throughout, matching the existing codebase and design document
- Shared infrastructure (tasks 1-2) must be completed before any test modules
- Unit tests (tasks 4-8) should be completed before property tests (task 10) since property tests reuse the same module structure
- E2E tests (tasks 12-13) depend on shared helpers from task 2
