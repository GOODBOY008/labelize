# Requirements Document

## Introduction

This document specifies the requirements for comprehensive unit test (UT) and end-to-end (E2E) test modules for the labelize project — a Rust-based ZPL/EPL label renderer. The test modules cover individual component testing (parsers, barcode generators, renderer, encoders), property-based testing with proptest, and E2E comparison testing against the Labelary API as ground truth. The goal is to replace the current high-tolerance (50%) golden-file approach with a rigorous, multi-layered test strategy.

## Glossary

- **Test_Harness**: The overall test infrastructure including helpers, fixtures, and configuration shared across UT and E2E tests
- **ZPL_Parser**: The `ZplParser` struct that converts ZPL byte input into `Vec<LabelInfo>`
- **EPL_Parser**: The `EplParser` struct that converts EPL byte input into `Vec<LabelInfo>`
- **Barcode_Generator**: Any of the barcode encoding modules (`code128`, `code39`, `ean13`, `twooffive`, `pdf417`, `aztec`, `datamatrix`, `qrcode`, `maxicode`) that produce `BitMatrix` or `RgbaImage` output
- **Renderer**: The `Renderer` struct that draws `LabelInfo` elements onto an `RgbaImage` canvas
- **PNG_Encoder**: The `encode_png` function that converts `RgbaImage` to monochrome PNG bytes
- **PDF_Encoder**: The `encode_pdf` function that converts `RgbaImage` to a PDF document
- **Labelary_Client**: A test helper that sends ZPL to the Labelary API (`http://api.labelary.com`) and retrieves reference PNG/PDF output
- **Pixel_Diff_Comparator**: A test utility that compares two images and reports a pixel-difference percentage
- **BitMatrix**: The `BitMatrix` struct representing a 2D grid of boolean values used by barcode generators
- **DrawerOptions**: Configuration struct specifying label dimensions (`label_width_mm`, `label_height_mm`, `dpmm`, `enable_inverted_labels`)
- **LabelInfo**: Parsed label representation containing `print_width`, `inverted` flag, and `Vec<LabelElement>`
- **LabelElement**: Enum of all drawable element types (text, graphics, barcodes) and parser config constructs
- **Virtual_Printer**: The `VirtualPrinter` state machine that tracks parser state across ZPL commands
- **Hex_Decoder**: The `hex` module utilities for decoding hex strings and compressed graphic field data
- **Encoding_Converter**: The `encodings` module that converts ZPL charset text to Unicode
- **Golden_File**: A reference PNG image stored in `testdata/` used for regression comparison
- **Tolerance_Threshold**: The maximum allowed pixel-difference percentage for a comparison test to pass

## Requirements

### Requirement 1: Unit Test Module for ZPL Parser

**User Story:** As a developer, I want unit tests for the ZPL parser, so that I can verify each ZPL command is parsed into the correct `LabelElement` variants with accurate field values.

#### Acceptance Criteria

1. WHEN a valid ZPL label block (`^XA...^XZ`) containing a single command is provided, THE ZPL_Parser SHALL produce a `Vec<LabelInfo>` with exactly one `LabelInfo` containing the expected `LabelElement` variant
2. WHEN a ZPL input contains multiple label blocks, THE ZPL_Parser SHALL produce one `LabelInfo` per `^XA...^XZ` block
3. WHEN a ZPL input contains font commands (`^A`, `^CF`, `^CW`), THE ZPL_Parser SHALL set the corresponding `FontInfo` fields (name, height, width, orientation) on subsequent text elements
4. WHEN a ZPL input contains barcode configuration commands (`^BY`, `^BC`, `^B3`, `^B7`, `^B0`, `^BE`, `^BQ`, `^BX`, `^BD`), THE ZPL_Parser SHALL produce the corresponding barcode `LabelElement` variant with correct parameters
5. WHEN a ZPL input contains field positioning commands (`^FO`, `^FT`), THE ZPL_Parser SHALL set the `LabelPosition` (x, y) on the next element
6. WHEN a ZPL input contains graphic commands (`^GB`, `^GC`, `^GD`, `^GF`, `^GS`, `^GE`), THE ZPL_Parser SHALL produce the corresponding graphic `LabelElement` variant with correct dimensions and color
7. WHEN a ZPL input contains label configuration commands (`^LH`, `^LL`, `^PW`, `^PO`, `^PM`, `^LR`), THE ZPL_Parser SHALL update the Virtual_Printer state accordingly
8. WHEN a ZPL input contains field block commands (`^FB`), THE ZPL_Parser SHALL produce text elements with the correct `FieldBlock` configuration (width, max lines, alignment)
9. WHEN a ZPL input contains hex-escaped field data (`^FH` with `^FD`), THE ZPL_Parser SHALL decode the hex sequences into the correct character values
10. WHEN a ZPL input contains template commands (`^DF`, `^XF`, `^XG`, `^FN`), THE ZPL_Parser SHALL store and recall formats correctly
11. IF a ZPL input contains malformed commands (missing parameters, invalid values), THEN THE ZPL_Parser SHALL either use default values or return a descriptive error string
12. WHEN a ZPL input contains field reversal (`^FR`) or label reversal (`^LR`), THE ZPL_Parser SHALL set the `reverse_print` flag on affected elements

### Requirement 2: Unit Test Module for EPL Parser

**User Story:** As a developer, I want unit tests for the EPL parser, so that I can verify EPL commands are parsed into the correct `LabelElement` variants.

#### Acceptance Criteria

1. WHEN a valid EPL input containing text commands (`A`) is provided, THE EPL_Parser SHALL produce `LabelElement::Text` with correct position, font, and content
2. WHEN a valid EPL input containing barcode commands (`B`) is provided, THE EPL_Parser SHALL produce the correct barcode `LabelElement` variant with correct parameters
3. WHEN a valid EPL input containing line commands (`LO`) is provided, THE EPL_Parser SHALL produce `LabelElement::GraphicBox` with correct position and dimensions
4. WHEN a valid EPL input contains a reference point command (`R`), THE EPL_Parser SHALL offset subsequent element positions by the reference point values
5. WHEN a valid EPL input contains a clear command (`N`), THE EPL_Parser SHALL reset the element list for the current label
6. WHEN a valid EPL input contains a print command (`P`), THE EPL_Parser SHALL finalize the current label and add it to the results
7. IF an EPL input contains malformed commands, THEN THE EPL_Parser SHALL either skip the command or return a descriptive error string

### Requirement 3: Unit Tests for Barcode Generators

**User Story:** As a developer, I want unit tests for each barcode generator module, so that I can verify barcode encoding correctness independently of the renderer.

#### Acceptance Criteria

1. WHEN valid input data is provided to the Code128 generator (`encode_auto`, `encode_no_mode`), THE Barcode_Generator SHALL produce a `BitMatrix` or `RgbaImage` with non-zero dimensions
2. WHEN valid input data is provided to the Code39 generator, THE Barcode_Generator SHALL produce a `BitMatrix` with the correct number of bar/space modules for the input length
3. WHEN valid input data is provided to the EAN-13 generator, THE Barcode_Generator SHALL produce a `BitMatrix` encoding exactly 13 digits (12 data + 1 check digit)
4. WHEN valid input data is provided to the Interleaved 2-of-5 generator, THE Barcode_Generator SHALL produce a `BitMatrix` encoding pairs of digits
5. WHEN valid input data is provided to the PDF417 generator, THE Barcode_Generator SHALL produce a 2D `BitMatrix` with valid row structure
6. WHEN valid input data is provided to the Aztec generator, THE Barcode_Generator SHALL produce a square `BitMatrix`
7. WHEN valid input data is provided to the DataMatrix generator, THE Barcode_Generator SHALL produce a `BitMatrix` with even width and height
8. WHEN valid input data is provided to the QR code generator, THE Barcode_Generator SHALL produce a square `BitMatrix`
9. WHEN valid input data is provided to the MaxiCode generator, THE Barcode_Generator SHALL produce a `BitMatrix` with the fixed MaxiCode dimensions
10. IF invalid input data is provided (empty string, invalid characters for the symbology), THEN THE Barcode_Generator SHALL return an `Err` with a descriptive message
11. THE Barcode_Generator SHALL produce a `BitMatrix` where `width()` and `height()` match the actual data dimensions for all symbologies

### Requirement 4: Unit Tests for Renderer

**User Story:** As a developer, I want unit tests for the renderer, so that I can verify individual element drawing operations produce correct pixel output.

#### Acceptance Criteria

1. WHEN a `LabelInfo` containing a single `GraphicBox` element is rendered, THE Renderer SHALL produce an `RgbaImage` with black pixels in the box region and white pixels outside
2. WHEN a `LabelInfo` containing a single `GraphicCircle` element is rendered, THE Renderer SHALL produce an `RgbaImage` with a circular outline of the specified diameter and border thickness
3. WHEN a `LabelInfo` containing a single `Text` element is rendered, THE Renderer SHALL produce an `RgbaImage` with non-white pixels in the text region
4. WHEN a `LabelInfo` containing a barcode element is rendered, THE Renderer SHALL produce an `RgbaImage` with the barcode pattern at the specified position
5. WHEN a `LabelInfo` with `inverted = true` is rendered with `enable_inverted_labels = true`, THE Renderer SHALL produce an `RgbaImage` rotated 180 degrees compared to the non-inverted rendering
6. WHEN a `LabelInfo` with `print_width` set to a value smaller than the label width, THE Renderer SHALL center the rendered content within the full label width
7. WHEN a `LabelInfo` containing a `reverse_print` element is rendered, THE Renderer SHALL invert the element pixels against the existing canvas content
8. WHEN `DrawerOptions` specifies different `dpmm` values (6, 8, 12, 24), THE Renderer SHALL scale the canvas dimensions proportionally

### Requirement 5: Unit Tests for PNG Encoder

**User Story:** As a developer, I want unit tests for the PNG encoder, so that I can verify the monochrome conversion and PNG encoding produce valid output.

#### Acceptance Criteria

1. WHEN an `RgbaImage` with known pixel values is provided, THE PNG_Encoder SHALL produce valid PNG bytes that can be decoded back to an image
2. WHEN an `RgbaImage` with pixel values above the threshold (128) is encoded, THE PNG_Encoder SHALL map those pixels to white (255) in the output
3. WHEN an `RgbaImage` with pixel values at or below the threshold (128) is encoded, THE PNG_Encoder SHALL map those pixels to black (0) in the output
4. THE PNG_Encoder SHALL preserve the width and height dimensions through the encode-decode round trip
5. FOR ALL valid `RgbaImage` inputs, encoding to PNG and decoding back SHALL produce a grayscale image with the same dimensions as the input (round-trip property)

### Requirement 6: Unit Tests for PDF Encoder

**User Story:** As a developer, I want unit tests for the PDF encoder, so that I can verify PDF output is structurally valid.

#### Acceptance Criteria

1. WHEN an `RgbaImage` and `DrawerOptions` are provided, THE PDF_Encoder SHALL produce non-empty PDF bytes
2. WHEN PDF bytes are produced, THE PDF_Encoder SHALL include a valid PDF header (`%PDF-`)
3. WHEN `DrawerOptions` specifies label dimensions, THE PDF_Encoder SHALL set the PDF `MediaBox` to the corresponding point dimensions (mm × 2.834645669)
4. THE PDF_Encoder SHALL embed the label image as a grayscale XObject within the PDF

### Requirement 7: Unit Tests for Hex and Encoding Utilities

**User Story:** As a developer, I want unit tests for the hex decoding and character encoding utilities, so that I can verify data transformation correctness.

#### Acceptance Criteria

1. WHEN a valid hex string is provided, THE Hex_Decoder SHALL decode it to the correct byte sequence
2. WHEN a hex string with odd length is provided, THE Hex_Decoder SHALL pad with a trailing zero and decode successfully
3. IF a hex string contains invalid characters, THEN THE Hex_Decoder SHALL return an error
4. WHEN compressed graphic field data (using ZPL compression characters G-Y, g-z) is provided, THE Hex_Decoder SHALL expand the compression counts correctly
5. WHEN Z64-encoded data (`:Z64:` prefix, base64+zlib) is provided, THE Hex_Decoder SHALL decode and decompress to the original bytes
6. WHEN text with a specific ZPL charset (0-13 or 27) is provided, THE Encoding_Converter SHALL produce the correct Unicode output
7. FOR ALL byte sequences, encoding to hex and decoding back SHALL produce the original bytes (round-trip property)
8. WHEN an escaped string with a hex escape character is provided, THE Hex_Decoder SHALL replace escape sequences with the corresponding byte values


### Requirement 8: Property-Based Tests with Proptest

**User Story:** As a developer, I want property-based tests using proptest, so that I can discover edge cases and invariants that example-based tests miss.

#### Acceptance Criteria

1. FOR ALL randomly generated valid ZPL label blocks (containing `^XA`, at least one command, and `^XZ`), THE ZPL_Parser SHALL either return `Ok` with a non-empty `Vec<LabelInfo>` or return a well-formed error string (no panics)
2. FOR ALL randomly generated ASCII strings, THE Code128 Barcode_Generator (`encode_auto`) SHALL either return `Ok` with a non-empty image or return an `Err` (no panics)
3. FOR ALL randomly generated digit strings of even length, THE Interleaved 2-of-5 Barcode_Generator SHALL produce a `BitMatrix` with consistent width proportional to input length
4. FOR ALL randomly generated `RgbaImage` inputs (arbitrary width 1-200, height 1-200, random pixel values), THE PNG_Encoder SHALL produce valid PNG bytes that decode without error (round-trip property)
5. FOR ALL randomly generated `DrawerOptions` with positive dimensions and valid dpmm values (6, 8, 12, 24), THE Renderer SHALL produce an `RgbaImage` with dimensions matching `(width_mm × dpmm, height_mm × dpmm)` when given an empty `LabelInfo`
6. FOR ALL randomly generated hex strings (characters 0-9, a-f, A-F), THE Hex_Decoder SHALL decode without panic and produce a byte vector of length `ceil(input_length / 2)`
7. FOR ALL randomly generated 12-digit strings, THE EAN-13 Barcode_Generator SHALL produce a `BitMatrix` with a fixed total width of 95 modules (plus quiet zones)
8. FOR ALL randomly generated QR code input strings (1-100 ASCII characters), THE QR code Barcode_Generator SHALL produce a square `BitMatrix` where `width() == height()`

### Requirement 9: E2E Labelary Comparison Tests

**User Story:** As a developer, I want E2E tests that compare labelize output against the Labelary API, so that I can measure rendering fidelity against an industry-standard reference.

#### Acceptance Criteria

1. THE Labelary_Client SHALL send a POST request to `http://api.labelary.com/v1/printers/{dpmm}/labels/{width}x{height}/0/` with ZPL body and `Accept: image/png` header
2. THE Labelary_Client SHALL respect the Labelary API rate limits (maximum 3 requests per second, 5000 requests per day) by implementing a request throttle
3. WHEN a ZPL test case is rendered by both labelize and the Labelary API, THE Pixel_Diff_Comparator SHALL compute the pixel-difference percentage between the two PNG outputs
4. THE Pixel_Diff_Comparator SHALL use a per-channel threshold of 32 (matching the existing implementation) and report the percentage of pixels exceeding this threshold
5. WHEN comparing labelize output against Labelary output for supported ZPL commands, THE Test_Harness SHALL assert that the pixel-difference percentage is below a Tolerance_Threshold of 15%
6. THE Test_Harness SHALL cache Labelary API responses in a local directory (`testdata/labelary_cache/`) to avoid redundant API calls during repeated test runs
7. WHEN the Labelary API is unreachable or returns an error, THE Test_Harness SHALL skip the comparison test with a warning rather than failing
8. THE Test_Harness SHALL include Labelary comparison tests for each supported ZPL command category: font commands, barcode commands, field commands, graphic commands, and label configuration commands
9. THE Test_Harness SHALL log the pixel-difference percentage for each comparison test to enable tracking rendering fidelity improvements over time

### Requirement 10: Improved Golden-File E2E Tests

**User Story:** As a developer, I want improved golden-file E2E tests with tighter tolerances and better diagnostics, so that I can catch rendering regressions more effectively.

#### Acceptance Criteria

1. THE Test_Harness SHALL support per-test-case Tolerance_Threshold values instead of a single global 50% tolerance
2. WHEN a golden-file test fails, THE Pixel_Diff_Comparator SHALL generate a visual diff image highlighting the differing pixels and save it to a `testdata/diffs/` directory
3. THE Test_Harness SHALL categorize golden-file tests by element type (text, barcode, graphic, mixed) and apply category-specific Tolerance_Threshold values
4. WHEN a new golden-file test case is added, THE Test_Harness SHALL provide a helper function that renders the ZPL/EPL input and writes the output PNG to the `testdata/` directory for manual review
5. THE Test_Harness SHALL compare image dimensions as a separate assertion before pixel comparison, reporting dimension mismatches explicitly
6. THE Pixel_Diff_Comparator SHALL support structural similarity (SSIM) comparison as an alternative to per-pixel difference for tests where spatial layout matters more than exact pixel values
7. THE Test_Harness SHALL set a default Tolerance_Threshold of 15% for new test cases, with a migration path to tighten existing 50%-tolerance tests incrementally

### Requirement 11: ZPL Command Coverage Matrix

**User Story:** As a developer, I want test coverage for the full set of ZPL commands that labelize supports, so that I can ensure no command is untested.

#### Acceptance Criteria

1. THE Test_Harness SHALL include at least one unit test and one E2E test for each of the following ZPL command groups: `^A`/`^CF`/`^CW` (fonts), `^BC`/`^B3`/`^B7`/`^B0`/`^BE`/`^BQ`/`^BX`/`^BD`/`^BY` (barcodes), `^FD`/`^FO`/`^FT`/`^FB`/`^FH`/`^FN`/`^FR`/`^FS`/`^FW` (fields), `^GB`/`^GC`/`^GD`/`^GE`/`^GF`/`^GS` (graphics), `^XA`/`^XZ`/`^LH`/`^LL`/`^LR`/`^LS`/`^LT`/`^PW` (label config), `^DF`/`~DG`/`~DY` (download), `^XF`/`^XG` (recall), `^PO`/`^PM`/`^PQ` (print), `~BR`/`~BI` (extensions)
2. THE Test_Harness SHALL include at least one unit test for each barcode symbology: Code128, Code39, EAN-13, Interleaved 2-of-5, PDF417, Aztec, DataMatrix, QR, MaxiCode
3. THE Test_Harness SHALL include at least one test for each `FieldOrientation` variant (Normal, Rotated90, Rotated180, Rotated270) applied to text and barcode elements
4. THE Test_Harness SHALL include at least one test for each `DrawerOptions.dpmm` value (6, 8, 12, 24)

### Requirement 12: Test Infrastructure and Helpers

**User Story:** As a developer, I want shared test infrastructure and helper functions, so that writing new tests is efficient and consistent.

#### Acceptance Criteria

1. THE Test_Harness SHALL provide a `render_zpl_to_png(zpl: &str, options: DrawerOptions) -> Vec<u8>` helper that performs the full parse-render-encode pipeline and returns PNG bytes
2. THE Test_Harness SHALL provide a `render_epl_to_png(epl: &str, options: DrawerOptions) -> Vec<u8>` helper that performs the full parse-render-encode pipeline and returns PNG bytes
3. THE Test_Harness SHALL provide a `compare_images(actual: &[u8], expected: &[u8], tolerance: f64) -> TestResult` helper that returns a structured result with diff percentage and optional diff image
4. THE Test_Harness SHALL provide a `labelary_render(zpl: &str, dpmm: u8, width: f64, height: f64) -> Option<Vec<u8>>` helper that fetches the Labelary reference image with caching and rate limiting
5. THE Test_Harness SHALL provide proptest `Arbitrary` strategy generators for `DrawerOptions`, simple ZPL label blocks, and barcode input strings
6. THE Test_Harness SHALL organize tests into separate modules: `tests/unit/` for unit tests, `tests/e2e/` for E2E tests, and `tests/common/` for shared helpers
7. WHEN the `LABELIZE_UPDATE_GOLDEN` environment variable is set, THE Test_Harness SHALL overwrite existing golden files with the current render output instead of comparing
