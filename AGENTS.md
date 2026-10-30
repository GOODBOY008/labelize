# Labelize — Project Guidelines

## Overview

Labelize is a Rust engine that parses ZPL (Zebra Programming Language), EPL (Eltron Programming Language), TSPL/TSC, CPCL (Zebra mobile) and DPL (Datamax-O'Neil) label data and renders it to PNG or PDF. It runs as a CLI tool, HTTP microservice, or library.

## Architecture

Full details and the new-language checklist live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

```
Input bytes + Language
  → src/lang.rs                  — the single front door: Language enum, parse_labels(),
                                    detect_language() content sniffing
  → src/parsers/zpl/             — ZPL: parser, VirtualPrinter state machine, ^CI charsets,
                                    ^MU units, Zebra font matrices, ^FN/^XF resolution
  → src/parsers/epl/             — EPL: line parser, built-in font-number metrics
  → src/parsers/tspl/            — TSPL/TSC: line + raw-data commands, resident font cells,
                                    SIZE → per-label physical dims (LabelInfo::label_*_mm)
  → src/parsers/cpcl/            — CPCL: ! header sessions, resident font cells, counted
                                    EG/CG graphics, header height/PAGE-WIDTH → label dims
  → src/parsers/dpl/             — DPL: <STX>L formats, 15/17-byte record headers, fonts 0-9,
                                    bottom-left home (LabelPosition.from_bottom)
  → src/elements/                — language-neutral IR: Text, Barcode*, Graphic*, LabelInfo
  → src/drawers/                 — language-blind renderer (CI-guarded, see
                                    tests/unit_architecture.rs)
  → src/images/                  — PNG (monochrome 1-bit) or PDF encoding
```

Two invariants hold the three-language boundary clean (a new frontend must not
break them; CI enforces both):

- **Calibration lives parser-side.** Everything that encodes "how a specific
  printer renders" — Zebra font matrices, `^FD` payload conventions, ^CI
  charset tables — stays in `parsers/zpl/` (or the language's own module).
  Parsers resolve it into the neutral IR before an element exists:
  `FontInfo { face, advance_mult, dedot_zero }`, QR raw payload +
  `error_correction`, DataMatrix literal bytes.
- **The IR and renderer are language-blind.** `elements` describes a label
  without naming any printer language; `drawers` keys off `FontFace` and
  neutral field values only. `tests/unit_architecture.rs` fails CI if either
  module references `parsers::` or carries language tokens in code.

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/lang.rs` | `Language` enum, `parse_labels()`, content sniffing — the only place that picks a parser |
| `src/parsers/zpl/` | ZPL command parser, VirtualPrinter state machine, Zebra calibration |
| `src/parsers/epl/` | EPL command parser, built-in font metrics |
| `src/parsers/tspl/` | TSPL/TSC command parser, resident font cells |
| `src/parsers/cpcl/` | CPCL command parser (Zebra mobile), Appendix-D font cells |
| `src/parsers/dpl/` | DPL command parser (Datamax-O'Neil), Table C-2 font cells |
| `src/elements/` | Language-neutral element structs (text, barcodes, graphics, stored formats) |
| `src/drawers/renderer.rs` | Image canvas creation, element drawing, compositing |
| `src/barcodes/` | Barcode encoders: Code128, Code39, EAN-13, 2of5, PDF417, Aztec, DataMatrix, QR, MaxiCode |
| `src/parsers/zpl/charsets.rs` | ^CI character-set tables (ZPL-only; `labelize::encodings` is a compat alias) |
| `src/assets/fonts/` | Embedded fonts: Roboto Condensed Bold (font 0), DejaVu Sans Mono, ZPL GS |

## ZPL Official Reference

The authoritative specification for all ZPL commands is the Zebra Programming Guide:
https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands.html

Individual command docs follow the pattern:
`https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-<cmd>.html`

**Always consult this reference when:**
- Implementing or fixing any ZPL command parser logic
- Rendered output doesn't match expected behavior
- Parameter defaults, ranges, or interactions are unclear
- Adding support for new ZPL commands

## Supported TSPL Commands

**Setup:** `SIZE` (mm/dot/inch → per-label canvas dims), `DIRECTION`, `REFERENCE`, `SHIFT`, `CLS`, `GAP`/`DENSITY`/`SPEED`/`OFFSET`/`CODEPAGE` (no-op)
**Content:** `TEXT`, `BAR`, `BOX`, `CIRCLE`, `ELLIPSE`, `ERASE`, `REVERSE`, `BARCODE` (128/128M/EAN128/EAN128M/39/39C/EAN13), `QRCODE` (ECC default M per TSC), `DMATRIX` (ECC200; `x#` = module size in dots, default 4 calibrated from the manual's result figure; width/height are an advisory area — the symbol is never scaled to fill it), `PDF417` (raw `L<len>`, incl. compound option tokens like `E30R4C4L26` — ECC is single-digit per its 0-8 domain), `BITMAP` (binary cursor or exact-length hex-text line with optional spaces/tabs; TSC polarity 0=black; ambiguous/short hex stays binary; modes 0/1/2 = overwrite/OR/XOR), `PRINT`
**Unsupported (explicit error):** `MPDF417`, `PUTPCX`, `PUTBMP`, `AZTEC`
**Semantics notes:** `DIRECTION` models 0 (inverted) / 1 (normal) per the common TSC usage; the 1-4 forms with mirror semantics are not arbitrated by any reference renderer and stay unimplemented. `REVERSE` is a print-time region inversion (white-on-black text). TSPL calibration references and open questions: [docs/TSPL_BASELINES.md](docs/TSPL_BASELINES.md); corpus provenance: [docs/TSPL_CORPUS.md](docs/TSPL_CORPUS.md)

## Supported CPCL Commands

Zebra's mobile-printer language (QL/MZ series); sessions start with `! {offset} {hdpi} {vdpi} {height} {qty}` and end at `PRINT`/`END` (both render) or `ABORT` (discards). Coordinates are dots at the header dpi using the manual's own 8-dots/mm (203 dpi) and 12-dots/mm (300 dpi) classes, so a 203-dpi file renders dot-identical at the engine's 8 dpmm; `IN-INCHES`/`IN-CENTIMETERS`/`IN-MILLIMETERS`/`IN-DOTS` switch the session's unit.

**Setup:** `PAGE-WIDTH` (label width), `UNITS` commands, `;` comments
**Content:** `TEXT`/`T` (+ `VTEXT`/`TEXT90`/`TEXT180`/`TEXT270` rotations, 90° CCW), `SCALE-TEXT`/`ST` (+ vertical; 72 pt/inch, `ROTATE` in 90° steps), `ML`/`MULTILINE` blocks (inline text command, scalable text, blank rows), resident fonts 0-7 (Appendix D cell metrics in `parsers/cpcl/fonts.rs`), `SETMAG {w} {h}`, justification (`LEFT`/`CENTER`/`RIGHT [end]` via one-line blocks), `BARCODE`/`B`/`VBARCODE` (128, UCCEAN128, 39/39C/F39/F39C, UPCA, UPCE, EAN13, EAN8, I2OF5/I2OF5C/I2OF5G; ratio codes 0-4 = 1.5-3.5:1, 20-30 = 2.0-3.0:1), `BARCODE-TEXT`/`BT` (HRI state), `B PDF-417 … ENDPDF` (XD/YD/C/S), `B QR … ENDQR` (`MA,`/`H0M,N…` payload conventions resolved parser-side), `B MAXICODE … ENDMAXICODE` (POST/CC/SC/MSG tags → mode-2 payload), `B AZTEC … ENDAZTEC` (XD/EC), `BOX`, `LINE`, `INVERSE-LINE` (region inversion of earlier objects), `EG`/`CG` and their full `EXPANDED-GRAPHICS`/`COMPRESSED-GRAPHICS` names (byte-safe counted payloads, set bits black)
**No-op (device behaviour):** `CONTRAST`, `TONE`, `JOURNAL`, `PACE`, `AUTO-PACE`, `NO-PACE`, `WAIT`, `BEEP`, `CUT`, `PARTIAL-CUT`, `FORM`, `PREFEED`, `POSTFEED`, `ENCODING`, `COUNT` (inert serial state like `^SN`)
**Unsupported (explicit error):** `CONCAT`/`VCONCAT`, `FG` font groups, `PCX`, `SCALE-TO-FIT`, `DEFINE-FORMAT`/`USE-FORMAT`, `VEG`/`VCG`, `PATTERN` ≠ 100, barcode types with addenda (`UPCA2/5`, `UPCE2/5`, `EAN132/5`, `EAN82/5`), `93`, `CODABAR`, `MSI*`, `POSTNET`, `FIM`
**Reference renderer:** labelixa.com renders a CPCL subset (`POST /v1/cpcl/render`) and is the golden reference where it covers the commands; renderer-baseline otherwise (EPL/TSPL convention). Real-provenance corpus (the MIT-licensed Labelixa example collection) and reference details: [docs/CPCL_DPL_CORPUS.md](docs/CPCL_DPL_CORPUS.md); tolerances in [docs/DIFF_THRESHOLDS.md](docs/DIFF_THRESHOLDS.md).

## Supported DPL Commands

Datamax-O'Neil (now Honeywell) label formats: `<STX>L` opens a format, `E` terminates and prints, `X` terminates without printing; records are `{rotation 1-4}{type}{c}{d}{eee}{row}{col}…{data}<CR>` with the **bottom-left home** (rows grow up — `LabelPosition.from_bottom`) and coordinates in hundredths of an inch (`m` switches to tenths of a millimetre).

**Config inside a format:** `D` (dot size), `Q` (quantity), `C`/`R` (column/row offsets), `Tnn` (two hexadecimal digits; field-data terminator for the next record only), `m`/`n` (metric/imperial), `A` (format attribute), `F` (bold/italic flags), `M` (mirror), `z`, `G`, `U`, `+`, `-`, `^`, `y`, speeds/heat/cut (no-ops where they have no drawable effect)
**Text:** bitmap fonts `0`-`8` (Table C-2 cells, base-25 `c`/`d` multipliers), smooth font `9` (CG Triumvirate, `eee` = `Ann`/`0nn` point sizes), scalable `S`/`U` IDs with `P015`-style point or dot size fields
**Barcodes:** `A` (39), `H` (HIBC 39 + mod-43), `B` (UPC-A), `C` (UPC-E), `D` (I2of5), `J`/`L` (I2of5 + mod-10), `E` (Code 128), `Q`/`R`/`S` (UCC/EAN 128), `F` (EAN-13), `G` (EAN-8) — uppercase prints the HRI line; `z`/`Z` (PDF-417 with T/F, security, aspect, rows, columns), `u`/`U` (MaxiCode fixed-field zip+4/country/class/message), `W1c`/`W1C` (ECC 200 DataMatrix, raw bytes; the uppercase form counts all following option and data bytes), `W1d`/`W1D` (QR auto/manual — manual carries the EC/mask/segment payload conventions), `W1f`/`W1F` (Aztec)
**Graphics:** `X` records with `L`/`l` lines and `B`/`b` boxes (dimensions in units)
**Unsupported (explicit error):** `Y` (stored images), polygons `P` and circles `C` in `X` records, `W1z`/`W1t`, legacy DataMatrix qualities and unequal module dimensions, QR structured append/hex-ASCII, non-UTF-8 PDF417 payloads, Codabar `I`, Plessey `K`, Code 93 `O`, Telepen `T`, Postnet `p`, FIM `v`, addenda `M`/`N`, MaxiCode structured `[)>RS…` form
**Reference renderer:** no printer-grade public renderer exists; the hosted Evolabel layout view (layoutview.evolabel.com, ~11.8 px/mm server preview) serves as the external comparison for the real-provenance corpus, renderer-baseline otherwise. Real-provenance corpus (manual Appendix B samples) and reference details: [docs/CPCL_DPL_CORPUS.md](docs/CPCL_DPL_CORPUS.md).

## Reverse Rendering (Image → Code)

`src/reverse.rs` is the inverse of the render pipeline: a raster goes in, printer-language source comes out. It emits text/bytes directly and never touches `elements`/`drawers`, so the language-blind-IR invariants don't apply to it.

- **Input limits:** at most 16 Mi pixels, 32000 dots per dimension and 128 MiB of decoded data, checked before decoding pixels. Encoded inputs honor EXIF orientation. `encode_image_bytes` and `ReverseLanguage` share language dispatch across CLI, HTTP and WASM; reject unknown targets and invalid options.
- **Binarization contract:** pixels are composited over white, converted to Rec.601 luminance, and become ink when strictly below `ReverseOptions::threshold` (default 128). One image pixel = one dot at `ReverseOptions::dpmm` (default 8).
- **ZPL** (`image_to_zpl`): `^FO0,0^GFA,total,total,row_bytes,` split on row boundaries into fields of at most 99999 bytes, with one uppercase-hex line per bitmap row (line breaks inside `^GF` data are ignored by the parser and by printers). Set bits print black. Generated ZPL leaves the canvas size unset, so callers render at `image dims / dpmm` mm.
- **TSPL** (`image_to_tspl`): `SIZE {w} mm,{h} mm` + `CLS` + `BITMAP 0,0,row_bytes,height,0,<payload>` + `PRINT 1,1`. The payload is the bitwise complement of the ZPL packing (TSC polarity: cleared bit = ink) and must start **inline right after the fifth comma** — that is where `split_tspl_commands` begins the raw-data cursor, so a line break there breaks parsing. The return value is raw bytes, not text.
- **Round-trip e2e:** `tests/e2e_reverse.rs` encodes real label fixtures,
  renders them through the ordinary forward pipeline, and requires
  pixel-identical output. Keep that property when touching this module.
- **Golden fixtures:** eight `golden_reverse_*` tests in `tests/e2e_golden.rs`
  pin the output bytes and exact rendered pixels for both languages against
  `testdata/reverse/` snapshots and externally captured PNGs. Provenance and
  explicit regeneration instructions: [testdata/reverse/README.md](testdata/reverse/README.md).
  `e2e_diff_report` writes `testdata/diffs/diff_report_reverse.txt` and the
  `reverse_*` comparison images; commit these alongside reverse changes.
  The field-block case covers the overflow header; a live render→encode→render
  regression in `e2e_reverse` also checks all orientations, `^FO`/`^FT`, and
  antialiased input against the current renderer.
- **Surfaces:** besides the library and `labelize encode`, the feature is
  exposed as `POST /encode` (`?dpmm&threshold&language=zpl|tspl`; zpl →
  `text/plain`, tspl → `application/octet-stream`) on both the axum server
  and `workers/worker.js` (wasm export `lz_encode`), and as the
  playground's **Image → Code** button (ZPL loads into the editor with the
  canvas sized to the image, TSPL downloads as a file). HTTP behaviour is
  pinned by `tests/e2e_http_encode.rs` (`--features serve`) and the
  encode→convert byte-identity cases in `e2e/http/test_http.sh`.

## Supported ZPL Commands

**Label control:** `^XA`, `^XZ`, `^LH`, `^LR`, `^LT` (label top), `^LS` (label shift), `^LL` (label length), `^PO`, `^PW`, `^CI`, `^CF`, `^PQ` (print quantity), `^FX` (comment), `^SN`/`^SF` (inert serial state)
**Text/Font:** `^A`, `^A@` (named font), `^CW` (font identifier), `^FO`, `^FT`, `^FW`, `^FD`, `^FS`, `^FB`, `^FV`, `^FN`, `^FR`, `^FH`
**Barcodes:** `^BC` (Code128), `^BE` (EAN-13), `^B8` (EAN-8), `^B9` (UPC-E), `^BU` (UPC-A), `^B2` (2of5), `^B3` (Code39), `^B7` (PDF417), `^BO` (Aztec), `^BX` (DataMatrix), `^BQ` (QR), `^BD` (MaxiCode), `^BY` (defaults)
**Graphics:** `^GB`, `^GC`, `^GD`, `^GE`, `^GF`, `^GS`
**Storage:** `~DG`, `^IL`, `^XG`, `^ID`, `^IM`, `^IS`, `~EG`, `^DF`, `^XF`

## Build & Test

```bash
# Build
cargo build

# Run all tests
cargo test

# Run all e2e tests (golden comparison against Labelary reference images)
cargo test --test 'e2e_*'

# Run all unit tests
cargo test --test 'unit_*'

# Run a specific test file
cargo test --test unit_renderer

# Run e2e golden tests with output
cargo test --test e2e_golden -- --nocapture

# Convert a label
cargo run -- convert testdata/amazon.zpl

# Bootstrap Labelary reference PNGs for any new ZPL files (requires network)
cargo test --test e2e_labelary bootstrap_golden_pngs -- --ignored --nocapture

# Regenerate all diff images and diff reports after any rendering change
cargo test --test e2e_diff_report -- --nocapture
```

## Testing Conventions

- **Golden tests** compare rendered PNGs pixel-by-pixel against reference images from Labelary
- Reference images live in `testdata/labels/*.png` and `testdata/unit/*.png`; diff outputs go to `testdata/diffs/`
- Each label has a tolerance threshold documented in `docs/DIFF_THRESHOLDS.md`
- After any rendering change, run `cargo test --test e2e_golden` to verify diff percentages stay within tolerance
- Test files follow naming convention: `tests/e2e_*.rs` for e2e tests, `tests/unit_*.rs` for unit tests
- Shared test utilities are in `tests/common/` (not compiled as tests)

### Testdata Directory Layout

| Directory | Canvas size | Labelary params | Purpose |
|-----------|-------------|------------------|---------|
| `testdata/labels/` | **813×1626 px** | 101.625mm × 203.25mm | Carrier/real-world labels |
| `testdata/unit/` | **812×1624 px** | 101.5mm × 203.0mm | Synthetic/unit fixture labels |

These sizes come from `render_helpers::default_options()` and `render_helpers::unit_options()` respectively.

### Adding a New ZPL Test File

1. Drop `<name>.zpl` into `testdata/unit/` (synthetic) or `testdata/labels/` (real-world carrier label)
2. Fetch its Labelary reference PNG (requires network):
   ```bash
   cargo test --test e2e_labelary bootstrap_golden_pngs -- --ignored --nocapture
   ```
   This scans both directories and fetches a PNG **only** for ZPL files that don't already have one. Existing PNGs are never overwritten.
3. Generate the diff comparison image:
   ```bash
   cargo test --test e2e_diff_report -- --nocapture
   ```
4. Commit both the `.zpl` and `.png` files, plus the updated files in `testdata/diffs/`.

### Rendering Change Workflow

When you modify any rendering-related logic (parsers, elements, drawers, barcodes, fonts, encoders), you **must**:

1. Run `cargo test --test e2e_diff_report -- --nocapture` to regenerate all diff images and reports:
   - `testdata/diffs/diff_report_labels.txt` — carrier/real-world labels (813×1626)
   - `testdata/diffs/diff_report_unit.txt` — unit/synthetic tests (812×1624)
   - `testdata/diffs/diff_report_reverse.txt` — image-to-code golden fixtures (exact pixel identity)
2. Run `cargo test --test e2e_golden` to verify diff percentages stay within per-label tolerance thresholds
3. Review all three reports to confirm diff percentages stay within tolerance
4. **Commit the changed files in `testdata/diffs/`** (including updated `*.png` and reports) as part of the same PR

This ensures PR reviewers can visually inspect the before/after rendering impact directly in the diff — left side is Labelary reference, right side is the current Labelize output.

**CI enforces this**: the `Test` job runs the golden tests (including `e2e_diff_report`, which regenerates `testdata/diffs/*` and auto-bootstraps any missing reference PNGs), then fails with a "Golden test artifacts are stale" error if `git status` shows any uncommitted changes under `testdata/`. It also catches a new `.zpl`/`.epl` committed without its reference `.png`.

## Rendering Reference

- **Unit canvas**: 812×1624 px (101.5mm × 203.0mm at 8 dpmm) — `render_helpers::unit_options()`
- **Label canvas**: 813×1626 px (101.625mm × 203.25mm at 8 dpmm) — `render_helpers::default_options()`
- Labelary API is the reference renderer: `http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/`
- `^FO` positions elements by top-left corner; `^FT` uses baseline positioning
- Barcode quiet zones are NOT rendered (Labelary convention)
- `^PW` constrains print width; `^PO I` inverts the label 180°

## Known Limitations

- **MaxiCode**: Standards-oriented encoder for modes 2-4 with sets A-E,
  shortest-path text compaction and numeric shift. Modes 5/6, ECI and
  Structured Append remain unsupported.
- **PDF417**: Encoding gaps vs reference (~18% diff)
- **Font metrics**: Helvetica Bold vs Zebra built-in fonts causes 2–7% typical diff
- **`^LL` command**: Not parsed (label length)

## Code Style

- Rust 2021 edition
- Use `LabelizeError` for error handling (see `src/error.rs`)
- Parser state is managed by `VirtualPrinter` struct — mutations happen during command parsing, elements are emitted on `^FS`
- Keep barcode encoders pure: input data → BitMatrix output, no rendering logic
