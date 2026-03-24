# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

## [Unreleased]

### Added

- Enhanced Aztec barcode error correction handling and documentation
- 16 carrier ZPL labels with side-by-side diff comparison tool

### Fixed

- Direction-specific baseline offsets for `^FT` rotated text positioning
- CI test commands updated to use wildcard patterns for better matching

### Changed

- Test directory structure flattened with removed rendered output
