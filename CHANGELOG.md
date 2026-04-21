# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-04-21

### Added

- **ZPL Diff Auto-Fix Skill** — New `src/skill/` module with data models, diff classification, diff scanning, and element-level contribution analysis for automated ZPL rendering improvement
- **UCC/GS1 Mode for `^BC`** — Code 128 barcode mode `D` now correctly prepends FNC1, strips parentheses and spaces, and converts `>8` escape sequences to embedded FNC1 separators per ZPL spec
- **QR Code UCC Mode** — Implemented UCC mode data preparation and improved error correction level handling for `^BQ`
- **USPS Priority Mail ZPL labels** — Added USPS test labels (Priority Mail and Test Merchant) to the golden test suite

### Fixed

- **Rotated Text Positioning** — Corrected `get_text_top_left_pos` for 90° and 270° rotations in the renderer
- **`^BC` GS1 Barcode Encoding** — Fixed GS1-128 (mode `D`) to handle embedded AI separators (`>8`) and strip grouping characters from the encoding string
- **QR Code Quiet Zone** — Fixed quiet zone handling for QR codes to match Labelary reference output
- **CI: Clippy warnings** — Resolved `empty-line-after-doc-comments`, `dead-code`, `ptr-arg`, and `needless-range-loop` warnings in `src/skill/`

### Changed

- **Code Refactor** — Improved readability and maintainability across renderer and parser modules
- **Diff Thresholds** — Updated per-label tolerance thresholds for DHL Parcel IT, BRT IT, and USPS labels



### Added

- **E2E Test Artifacts** — CI workflow now captures and uploads convert outputs (PNG/PDF) from CLI, HTTP, and SDK tests as GitHub Artifacts with 1-day retention
- **Consolidated E2E Workflow** — Merged CLI, HTTP, and SDK E2E jobs into single macOS job for efficiency
- **Rendering Change Workflow** — Added documentation requiring `e2e_diff_report` test runs and `testdata/diffs/` commits for rendering-related PRs
- **Performance Benchmarks** — Added performance comparison section (~5ms vs Labelary ~388ms) to README
- **Render Comparison Gallery** — Added side-by-side comparison images for 6 major carriers (Amazon, FedEx, UPS, DHL, USPS, Swiss Post) in README
- **MIT License File** — Added LICENSE file with full MIT license text

### Fixed

- **Bash 3 Compatibility** — Fixed case-insensitive comparison in CLI E2E test for macOS default Bash 3
- **License Link** — Fixed broken `../LICENSE` reference in README to point to `LICENSE`
- **Test Command Documentation** — Corrected AGENTS.md to use `cargo test --test e2e_diff_report` for diff regeneration

### Changed

- **Output Directory Naming** — Standardized E2E test output directories to `cli-output`, `http-output`, `sdk-output`
- **README Structure** — Added motivation paragraph, cost comparison row, and reorganized sections
- **Documentation** — Enhanced AGENTS.md with clear rendering change workflow and test commands

## [0.1.0] - 2026-03-24

### Added

- **ZPL Parser** with support for 30+ commands including text, barcodes, graphics, stored formats, graphic fields, and field blocks
- **EPL Parser** with support for text, barcodes, line draw, and reference points
- **10 Barcode Symbologies**: Code 128, Code 39, EAN-13, Interleaved 2-of-5, PDF417, Aztec, DataMatrix, QR Code, MaxiCode
- **PNG Output** — Monochrome 1-bit PNG encoding
- **PDF Output** — Single-page embedded PDF generation
- **CLI Tool** — Convert ZPL/EPL files with format auto-detection, multi-label support, and custom dimensions
- **HTTP Microservice** — RESTful API for label conversion with format detection via Content-Type
- **Embedded Fonts** — Zero runtime font dependencies (Helvetica Bold Condensed, DejaVu Sans Mono, ZPL GS)
- **Unit Tests** — Comprehensive test coverage for EPL, ZPL, PNG, PDF encoders, and hex encoding
- **Regression Tests** — ZPL rendering issue detection with test data files
- **Golden Tests** — 57 E2E tests comparing rendered output against Labelary reference PNGs
- **Documentation** — ZPL Commands Reference, rendering diff report, and enhanced README

### Fixed

- Guard bar extension calculation for EAN-13 barcode
- QR code rendering with proper quiet zone
- CI failures (clippy warnings, rustfmt, test target naming)
- Hex escape handling in parser

### Changed

- Default value of `enable_inverted_labels` set to `true` in `DrawerOptions`
- Enhanced `^GD` command implementation
- Improved code structure for readability and maintainability
- Upgraded GitHub Actions (checkout and upload-artifact to v5)
- Updated macOS runner to latest version in CI

### Security

- Added timeout configuration in CI workflows

## [0.2.1] - 2026-03-25

### Changed

- Excluded `testdata/`, `docs/`, `examples/`, and CI/IDE config from published crate — reduced crate size from ~18MB to ~508KB

## [0.2.0] - 2026-03-25

### Added

- Enhanced Aztec barcode error correction handling and documentation
- 16 carrier ZPL labels with side-by-side diff comparison tool

### Fixed

- Direction-specific baseline offsets for `^FT` rotated text positioning
- CI test commands updated to use wildcard patterns for better matching

### Changed

- Test directory structure flattened with removed rendered output
