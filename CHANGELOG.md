# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **DPL support (fifth language)** — Datamax-O'Neil (Honeywell) label
  formats. `<STX>L` … `E`/`X` formats with 15-byte record headers and the
  bottom-left home position (a new language-neutral
  `LabelPosition.from_bottom` anchor the renderer resolves against the
  canvas): bitmap fonts 0-8 with base-25 multipliers, the smooth font 9
  point table, scalable `S`/`U` size fields, Code 128 / 39 (+HIBC) /
  UCC-EAN 128 / UPC-A/E / EAN-13/8 / I2of5 (+mod-10) with the
  uppercase-HRI convention, QR auto/manual payloads, PDF-417, DataMatrix,
  MaxiCode (fixed-field form), Aztec, and `X` line/box records. Metric
  mode, `C`/`R` offsets and the `T` terminator command included. Wired
  through every surface like CPCL. Golden tests use the renderer baseline
  for the synthetic fixtures plus a real-provenance corpus: the CPCL side
  adds the three MIT-licensed labels from Labelixa's example collection
  (referenced against labelixa's CPCL renderer), the DPL side adds the
  manual's Appendix B samples (referenced against the hosted Evolabel
  layout view, the only server-side DPL renderer found). Provenance and
  honest cross-engine distances: docs/CPCL_DPL_CORPUS.md.

- **CPCL support (fourth language)** — Zebra's mobile-printer language
  (QL/MZ series). `! {offset} {hdpi} {vdpi} {height} {qty}` sessions with
  `PRINT`/`END`/`ABORT` terminators; resident fonts 0-7 with `SETMAG`,
  `SCALE-TEXT` + `ROTATE`, `ML` blocks, `LEFT`/`CENTER`/`RIGHT`
  justification, 1-D barcodes (128, UCCEAN128, 39 family, UPC-A/E,
  EAN-13/8, I2of5) with `BARCODE-TEXT` HRI, QR / PDF-417 / MaxiCode /
  Aztec, `BOX` / `LINE` / `INVERSE-LINE`, and counted `EG`/`CG` graphics.
  Wired through every surface: `Language::Cpcl` + content sniffing,
  CLI `-f cpcl` / `.cpcl` files, HTTP `application/cpcl`, the wasm
  bindings, the Worker, and the playground (selector, CPCL showcase
  sample, validate/format badges). Golden tests reference labelixa's CPCL
  renderer where it covers the commands and the renderer baseline
  otherwise (EPL/TSPL convention; see docs/DIFF_THRESHOLDS.md).

- **Playground SEO landing page** — the hosted playground ships real search
  metadata: keyword-rich `<title>`/`description`, canonical URL, Open Graph /
  Twitter cards, `WebApplication` + `FAQPage` JSON-LD, a `<noscript>` pitch
  for non-JS visitors and a new collapsible **About · FAQ** panel (EN/中文)
  with the feature list, API endpoints and repository links. All wording
  covers the full five-language engine (ZPL, EPL, TSPL, CPCL, DPL). The
  server and worker now also serve `/robots.txt` and `/sitemap.xml`.
- **`LABELIZE_PUBLIC_BASE_URL`** — canonical origin of the hosted playground
  (baked into canonical/OG/JSON-LD URLs and echoed by robots/sitemap). After
  attaching a custom domain, set it (worker: `PUBLIC_BASE_URL` var, also
  uncommented in `wrangler.toml`) so search engines consolidate on the domain
  and the workers.dev HTML page 301-redirects there.

## [1.9.0] - 2026-10-21

### Added

- **`POST /validate` + `POST /format` APIs** — the HTTP server (and the
  Cloudflare Worker mirror) gain two language-aware endpoints:
  `/validate` syntax-checks label source without rendering (JSON report:
  detected `language`, `ok`, `labels`, and the exact parse `error`),
  `/format` pretty-prints it one command per line with `^XA` blocks
  indented. Both take an optional `?language=zpl|epl|tspl` and auto-detect
  otherwise; unknown values are a 400.
- **Source formatters (core + SDK)** — `labelize::format_source()` and the
  wasm exports `lz_validate()` / `lz_format()` bring the same capabilities
  to library and `@goodboy008/labelize-wasm` users, so SDK callers can
  surface the real parser error (JSON report) instead of guessing from a
  thrown string. Formatters never fail and never touch binary payloads
  (`^FD` data, `GW`/`BITMAP` graphics): formatting real corpus labels
  (dpduk.epl, dpdpl.zpl, dhl_express.zpl, tspl_graphics.tspl) renders
  pixel-identically (sha256-verified).
- **Playground: syntax badge + Format button** — the editor header shows a
  live `ZPL · syntax OK · 1 label` (or the parser error) badge driven by
  `POST /validate`, and a Format button reflows the editor via
  `POST /format`. While the user owns the content and has not pinned the
  format selector by hand, the detected language keeps the selector in
  sync so pasted EPL/TSPL renders with the right engine.
- **Playground: per-format demos** — switching the Format selector
  (ZPL/EPL/TSPL) now auto-loads a demo label for the newly selected language
  while the editor still holds content the user never typed; any manual edit
  is never replaced. The Samples menu gains an "EPL showcase" label
  demonstrating text fonts, `X` boxes, `LO` lines and Code 39 / Code 128
  barcodes with human-readable lines.
- **One-call render API** — `labelize::render_png` / `render_pdf` with
  `RenderOptions` and a `Rendered { language, labels, bytes }` result:
  language detection, first-label selection, dimension fallback
  (user size > label-declared size > 102×152 mm) and PNG/PDF encoding in a
  single call returning one `LabelizeError` (parse = user input, rest =
  engine-side). The HTTP `/convert` handler and the wasm render path now go
  through it instead of re-implementing the glue; outputs verified
  byte-identical (sha256) across ZPL/EPL/TSPL, PDF, custom sizes and
  antialias before/after.
- **`serde` feature** — opt-in `Serialize` for `Language` and
  `ValidationReport` (lowercase wire names, `error` omitted when `None`),
  so HTTP layers and SDK consumers share one wire shape; the server drops
  its mirror response struct.

### Changed

- **Dependency maintenance** — Docker base image `rust` 1.99.0-bookworm →
  1.99.1-bookworm; `tokio` 1.53.2 → 1.53.3 and `sha2` minor/patch bump
  (dependabot group).

### Docs

- **README: icon-only logo** — the wordmark above the `# Labelize` heading
  duplicated the brand name; README (and the npm package README) now show
  the new icon-only `docs/logo-icon.svg` crop instead.
- **Rust tutorial rewritten** (`docs/tutorials/rust-library.md`) — front-door
  API (`detect_language`/`parse_labels`), new sections for `validate()` and
  `format_source()`, canvas fallback pattern, multi-label loops, and a
  stage/error-type table; every snippet compile- and run-verified. The
  `docs/USAGE.md` library quickstart now uses the same front door and links
  to the tutorial.

### Fixed

- **TSPL: `DMATRIX` rendered a giant black slab** — the `x#` module-size
  option was parsed but discarded, and the symbol-area `height` argument
  (e.g. the `180` in `DMATRIX 560,540,180,180,x5,"…"`) leaked into the
  module scale, drawing 180-px modules that covered the label as a solid
  black block. `x#` now sets the module size in dots per the TSC TSPL
  manual (omitted → 4, calibrated from the manual's result figure), the
  advisory area no longer feeds geometry, and the previously unused `c#`
  escape char, `a#` rectangle shape, and manual `row,col` ordering are
  wired through. Verified round-trip: the rendered symbols decode back to
  the exact content with rxing (22×22 and 32×32 ECC200 cases from the
  TE210 capture). `labelixa` returns blank for every `DMATRIX`, so the
  calibration rests on the manual + its figure alone (documented in
  `docs/TSPL_BASELINES.md`).
- **ZPL: lowercase single-letter parameters (issue #104)** — real-world
  labels using lowercase orientation values (`^FWr`, `^BCr`, `^A0r`) were
  silently rendered as normal orientation. Single-letter parameter parsing
  (orientation letters in `^FW`/`^BC`/`^A`/etc., Y/N booleans, `^FB`
  justification, `^BC` mode) is now case-insensitive, matching Zebra
  printers and Labelary (verified pixel-identical against Labelary
  references).
- **wasm crate test module** — `wasm/src/lib.rs` tests had not compiled
  since the crate moved into `wasm/` (stale `include_str!` path) and an
  earlier refactor removed the `render_payload` helper the tests called;
  both restored, 12 tests green again.
- **EPL `GW` boundary logic deduplicated** — the graphic-data span scan
  lives in one place (`gw_span`) shared by the parser and the new
  formatter, so the two can never disagree about where binary data ends.

## [1.8.1] - 2026-10-16

### Changed

- **CI maintenance** — bumped GitHub Actions workflows (actions/checkout 5→7,
  docker/build-push-action 6→7, docker/metadata-action 5→6,
  peter-evans/repository-dispatch 3→4, softprops/action-gh-release 2→3) and
  the `proptest` dev-dependency.

### Fixed

- **Font-0 symbol ink calibration (issue #78)** — since the Roboto Condensed
  substitution (v1.7.0), font-0 punctuation and operators (`-` `–` `—` `+` `=`
  `<` `>` `%` `{` `|` `}` `~` `±` `×` `÷` `…` `‰` `°` `_` …) rendered visibly
  narrower than Labelary while keeping the calibrated advance, leaving a gap
  after every symbol (`EMB- 812`). The renderer now stretches (or shrinks)
  each calibrated glyph horizontally and shifts it — including a vertical
  correction for glyphs like `_` and the dashes that sat too high — so the
  pen-relative ink matches probe-measured Labelary extents
  (`tuning::FONT0_INK_TARGETS`; the embedded font files are untouched).
  `©` now renders as the same small superscript Labelary draws for `®`, and
  ASCII quotes `'` `` ` `` `"` draw as the curly `’` `‘` `“` shapes Labelary
  substitutes (`tuning::font0_shape_glyph`). The en/em dash advances were
  re-probed directly (Labelary advances the whole dash family uniformly at
  0.9 × cell height) — the old model-derived deltas were off by +0.28/−0.08 em,
  shifting every character after an en dash. Corpus effect vs Labelary:
  34 labels improved (best: correiosbr 3.67% → 3.47%, dpdpl 3.83% → 3.76%),
  none regressed beyond quantisation noise.

## [1.8.0] - 2026-10-13

### Changed

- **Per-language parser architecture (TSPL-ready)** — `src/parsers/` is now
  organized as self-contained per-language module trees (`parsers/zpl/`,
  `parsers/epl/`) instead of flat `*_parser.rs` files. ZPL-only knowledge
  moved out of the shared pipeline: ^CI charset tables
  (`parsers/zpl/charsets.rs`; `labelize::encodings` remains as a compat
  alias), ^MU measurement units, Zebra font matrices and designator
  resolution (`parsers/zpl/zebra_fonts.rs`), ^BQ ^FD payload conventions
  (`parsers/zpl/qr_payload.rs`), and ^FN/^XF stored-format resolution
  (`parsers/zpl/stored_formats.rs`; `elements/stored_format.rs` keeps the IR
  structs only). Old import paths (`parsers::zpl_parser`,
  `parsers::epl_parser`, `parsers::virtual_printer`,
  `parsers::command_utils`, `parsers::fs`) remain available as compat
  re-export shims.
- **`FontInfo` resolves rendering class at parse time** — new fields `face`
  (`FontFace`: which embedded TTF plus the substitute's calibration),
  `advance_mult`, `dedot_zero`, all set by the producing parser. The
  renderer no longer matches on ZPL font designator strings; Zebra font
  metrics live exclusively in `parsers/zpl/zebra_fonts.rs`. `FontInfo`
  itself no longer carries the Zebra size-derivation methods
  (`with_adjusted_sizes` etc. moved parser-side — the v1.7.x inherent-method
  forms are kept as compat delegates on `FontInfo`, so existing calls
  compile unchanged). *(semver-strict: breaking
  for consumers constructing `FontInfo` directly; shipping in the next
  minor per project convention.)*
- **QR elements carry raw payloads** — `BarcodeQrWithData.data` is now the
  raw QR content and a new `error_correction` field carries the resolved
  ECC level; `get_input_data()` (which parsed ZPL ^FD format prefixes like
  `QA,`/`QM,B<len>` at render time) was removed from the shared element and
  the ZPL ^FD parsing moved to `parsers/zpl/qr_payload.rs`. EPL no longer
  synthesizes ZPL-style QR payloads. `get_input_data()` remains as a
  `#[deprecated]` shim reporting the parser-resolved fields. *(semver-strict:
  breaking for consumers reading `data` as a ^FD-formatted string; shipping
  in the next minor.)*
- **Neutral DataMatrix entry-point names** —
  `barcodes::datamatrix::render_zpl_field` → `render_field`,
  `barcodes::datamatrix_legacy::zpl_symbol_size` → `legacy_symbol_size`.
- **Architecture guards in CI** — `tests/unit_architecture.rs` enforces that
  shared modules (`drawers`/`elements`/`barcodes`/`images`) never depend on
  `parsers::` and that the renderer carries no language tokens in code.
- Architecture documentation: `docs/ARCHITECTURE.md` (pipeline, invariants,
  new-language checklist); `AGENTS.md` architecture section updated.

### Added

- **TSPL/TSC support (#13)** — `parsers/tspl/` frontend: `SIZE` (mm/dot/inch,
  drives the canvas per label via `LabelInfo::label_*_mm`), `DIRECTION`,
  `REFERENCE`, `SHIFT`, `CLS`, `TEXT` (resident font cells 1-8 and `.BF2`
  bitmaps, magnification, alignment incl. new Center), `BAR`/`BOX`/`CIRCLE`/
  `ELLIPSE`/`ERASE`/`REVERSE`, `BARCODE` (128/128M/EAN128/EAN128M/39/39C/
  EAN13), `QRCODE` (raw payload + explicit ECC), `PDF417` (incl. raw
  `L<len>` byte-cursor form), `BITMAP` (binary cursor), `PRINT`; device
  setup commands (`GAP`, `DENSITY`, `SPEED`, …) are accepted no-ops;
  unsupported commands error explicitly. CLI (`-f tspl`, `.tspl`),
  HTTP (`text/tspl`) and wasm (`lz_render_lang`) entry points; content
  sniffing for extension-less input. By littlecxm, rewritten onto the
  per-language architecture. *(New `Language::Tspl` variant.)*
- **Unified language front door** — `labelize::Language` (Zpl, Epl),
  `labelize::parse_labels(lang, bytes, dpmm)` and
  `labelize::detect_language(bytes)` content sniffing. CLI, HTTP `/convert`
  and wasm entry points delegate to it; the CLI additionally sniffs
  extension-less/unknown-extension files instead of always assuming ZPL.

### Fixed

- **TSPL BITMAP polarity** — TSC `BITMAP` prints *cleared* bits black (the
  inverse of ZPL ^GF/EPL GW); fields rendered with inverted ink before.
  Confirmed against the labelixa.com TSPL renderer (BITMAP regions now
  pixel-match within 1.6%).
- **TSPL BITMAP OR/XOR modes** — modes 1 (OR) and 2 (XOR) are now applied at
  render time via the new neutral `GraphicField::mode` field instead of
  erroring (XOR inverts overlapped ink; OR equals overwrite on a 1-bit
  canvas).
- **TSPL PDF417 compound option tokens** — TSC's documented sample packs
  options into one token (`PDF417 …,E30R4C4L26`); the parser now splits
  letter-runs (with the ECC level's single-digit 0-8 domain respected: that
  sample reads as E3 + stray 0 + R4 + C4 + L26), recognizes a trailing
  `L<len>` raw-data marker inside a compound token, and fixes an off-by-one
  that dropped the last option token for comma-separated options.
- **TSPL QRCODE ECC default** — an omitted error-correction level now falls
  back to `M` per the TSC manual (was `H` from #13; the invalid-character
  path already used `M`).

### Fixed

- **TSPL `REVERSE` prints a region inversion** (manual semantics:
  white-on-black text in the area) instead of a filled black box that hid
  the text. New neutral `InvertRegion` IR element applied by the renderer
  as a post-pass in label coordinate space; ZPL/EPL output unchanged.

### Added

- **TSPL calibration infrastructure**: `docs/TSPL_BASELINES.md` (two-baseline
  strategy — labelixa.com API primary, portakal secondary — with a
  triangulation policy and a measured engine-disagreement ledger incl. a
  font-cell probe battery), `docs/TSPL_CORPUS.md` (real-capture provenance),
  a cached/rate-limited labelixa reference client, and five real-world
  corpus goldens (TSC TE210 hardware capture, filament-db probes/drybox, RU
  warehouse template) with labelixa references and honest tolerances
  (0.54%–10.88%).

### Added

- **v1.7.x public-API compatibility shims** — every removed public item from
  the reorganization is importable again: `elements::measurement_unit`
  (canonical neutral definition; `parsers::zpl::measurement_unit` re-exports
  it), `elements::font::is_pv_font`, the `FontInfo` inherent methods
  (`is_standard_font` / `is_bitmap_font` / `with_adjusted_sizes`, thin
  delegates written in `parsers/zpl/zebra_fonts.rs` so `elements` stays
  parser-free), `RecalledFormat::resolve_elements`, and a deprecated
  `BarcodeQrWithData::get_input_data` reporting the parser-resolved fields.
  `cargo semver-checks` against v1.7.1: all removal-type lints cleared.
- **TSPL on every user-facing surface** — playground language selector with
  a built-in TSPL showcase sample (en/zh i18n, `.tspl` file input, TSPL
  Content-Type), npm `lz_render_lang` documented in `init.d.ts`/READMEs,
  Android `renderLang` JNI entry plus `renderTsplToPng`/`renderTsplToPdf`
  Kotlin wrappers (with a TSPL render test), README/Cargo.toml
  description+keywords/tutorials/USAGE TSPL coverage, Cloudflare Worker
  `application/tspl` dispatch.

### Migration notes (1.7.x → next)

All removals/renames are covered by the compatibility shims above; the
remaining upgrade frictions are additive one-liners:

- Exhaustive `match` on `FieldAlignment` gains a `Center` arm; on
  `LabelElement` an `InvertRegion` arm (TSPL features).
- Struct literals gain fields: `BarcodeQr { orientation }`,
  `BarcodeQrWithData { error_correction }`, `GraphicField { mode,
  zero_bits_black }`, `Barcode{128,39,2of5}WithData { line_alignment }`,
  `FontInfo { face, advance_mult, dedot_zero }`, `LabelInfo {
  label_width_mm, label_height_mm }`.
- JS/wasm consumers are unaffected (`lz_render` signature unchanged;
  `lz_render_lang` is a pure addition).

## [1.7.1] - 2026-10-11

### Added

- **Playground opt-out env vars for self-hosted deployments** —
  `LABELIZE_PLAYGROUND_LABELARY_COMPARE=false` removes the "Compare with
  Labelary" button (the only feature that sends ZPL to the external
  api.labelary.com service), and `LABELIZE_PLAYGROUND_ENABLED=false` turns
  `serve`/Docker into an API-only container whose `/` answers with an
  endpoint listing. Both default to `true`; unparseable values warn on
  stderr and fall back to the default (#72).
- **Windows cross-compilation build script (#15)** — `tools/build/build-windows.sh`
  cross-compiles a Windows `labelize.exe` (x86_64-pc-windows-gnu) inside Docker
  with mingw-w64, runnable from any machine with Docker; the binary lands in
  `target/windows-release/`. Complements the official MSVC binaries attached to
  each GitHub release.
- **Unmapped ZPL font warning (#20)** — font names with no built-in mapping
  (e.g. user-installed numeric fonts or `^CW`-mapped names) now log a one-time
  notice on stderr instead of silently substituting DejaVu Sans Mono;
  rendering output is unchanged.
- **Real-provenance carrier label corpus (#68)** — golden-test labels now come
  only from official carrier demos or real-world captures, each documented in
  `docs/CARRIER_LABELS.md`; 80 synthetic fixtures were retired.
- **Comprehensive character-display test suite** —
  `tests/unit_text_character_display.rs` sweeps ~960 characters (ASCII,
  Latin-1/Extended-A/B/Additional, Greek, Cyrillic, punctuation, CJK) across
  all 14 substitute-font calibration classes and all four field orientations,
  asserting that covered characters produce ink, that rotation preserves ink
  pixel-exactly, and that uncovered characters render blank (Labelary parity),
  plus the same invariants for multiline ^FB blocks.

### Fixed

- **Resident bitmap fonts (A-H) advance by their real Labelary cell step** —
  the DejaVu substitute advanced by the coded cell width (C 10, E 15, F 13,
  G 40, H 13 dots), while Labelary measures C 12, E 20, F 16, G 48 and
  H 19 dots per character at 1x. The shortfall accumulated leftward drift
  inside every bitmap-font field and made word gaps inconsistent between
  fonts (e.g. `C2 01` vs `F0 01` vs `E2 01`). A per-font advance multiplier
  now rescales the pen walk only — glyph rasterization is unchanged
  (`bitmap_font_advance` golden + unit test pin each font's step).
- **Bitmap-font `0` dot policy now matches Labelary per font** — the DejaVu
  substitute ships a dotted zero, but Labelary's reference zeros are not
  uniform: measured over probe rows at 1x and 2x, fonts E and H have empty
  zero interiors at every size (a magnified font-E `E2 01` showed stray dots
  Labelary never drew), while A/B/C/D/F/G reference zeros carry a dot. The
  rasterizer now drops border-unconnected ink islands from `0` glyphs for E
  and H only, keeping the dot where Labelary has one (`bitmap_font_advance`
  golden + a two-way unit test pin E clean and C dotted).
- **`^FB` blocks in bitmap fonts advance by the font cell height** — fonts
  A-H and P-V scale their em by a cap-height correction so glyphs match
  Labelary, but that enlarged em was also used as the ^FB line pitch. An
  18-dot font C block advanced 21 dots per line instead of 18, so long
  blocks (e.g. delivery-note item tables) overflowed into the fields below.
  The pitch is now cell height + ^FB line spacing, matching Labelary and
  Zebra printers (new unit test + `fb_bitmap_line_pitch` golden).
- **DataMatrix honors `^BX` column/row constraints and aspect ratio** — the
  encoder no longer falls back to an unrestricted square symbol when a
  requested ECC200 size cannot fit: both dimensions pin the symbol shape, a
  fixed-row rectangular request selects the smallest matching standard symbol
  without widening, and an over-capacity field is skipped in exactly that
  row-only rectangular mode (ZD421- and Labelary-verified at r8/r12/r16,
  golden at 0.0% tolerance, #48).
- **QR mask selection follows the standard's five-percentage-point steps** —
  the QR dependency's linear N4 balance approximation could pick a
  non-minimal mask; Labelize now evaluates all eight candidate matrices with
  the exact ISO/IEC 18004 penalties and ties to the lowest mask index. Five
  verified ECC-H cases now render pixel-identical to Labelary (#53).
- **Rotated text fields no longer clip characters to the advance box** —
  rotated fields (^A…R/I/B, ^FW) are rasterised into a buffer whose width now
  covers the laid-out ink, with a left margin for marks that overhang the pen
  origin and a descent pad for glyphs extending below the buffer. Previously
  178 (font, character) combinations lost ink when rotated — catastrophically
  for font-0 characters whose calibrated advance deltas undercut their glyph
  (Ā, Ŝ, ƀ, ℀ rendered as a few-px sliver), by 5–12 % for the DejaVu Greek
  tonos capitals and Vietnamese horn glyphs, and 1–3 px on the DejaVu mono
  trailing edge. Per-orientation anchor rebasing keeps every previously
  rendered pixel in place: golden output is byte-identical (128/128 pass,
  zero testdata changes) — only previously clipped ink appears.
- **Rotated ^FB blocks anchor like Labelary** — Labelary reserves the full
  block box (max_lines × line pitch) for rotated blocks and stacks lines from
  its top edge; for 90° fields that places line k at the plain-rotated
  position plus (max_lines − k)·pitch regardless of how many lines render
  (probed across FB,1/FB,2/FB,4 with 1- and 2-line content). The renderer now
  matches, fixing a 40–56 px column offset on 90° font-0 blocks (e.g. the
  bottom-right rotated block of the `rotated_char_display` fixture) and
  improving `dpdpl` 4.07 % → 3.91 %.
- **Negative `^FB` line spacing and padded-pen snapping in rotated blocks** —
  `^FB`'s add-spacing parameter may be negative (later lines then sit above
  the first, and the deepest line need not be the last): the rotated buffer
  now grows a top margin for raised lines and sizes its bottom margin from
  the deepest line, instead of clipping raised lines whole
  (`^A1R ^FB20,2,-80 "j j"` lost one of two lines). The buffer's left margin
  is applied after the per-line pen snap, so padding is always a pure
  translation. The 90° block anchor generalises to the sign-aware rule
  `line k = field_x + (max_lines − k + 1) × pitch`, verified against
  Labelary to ≤1 px at +40/−80 dot spacing with single- and two-line content.
- **Characters outside a substitute font's coverage render blank everywhere**
  — the DejaVu faces (fonts 1, A–Z) used to draw .notdef boxes for uncovered
  characters (CJK, Latin Ext-B digraphs, parts of Latin Extended Additional);
  Labelary renders these blank while the pen still advances one cell, and the
  renderer now matches (font 0 already did).

## [1.7.0] - 2026-10-04

### Added

- **`^PM` persistent label mirroring (#49)** — the mirror-image print mode now
  persists across `^XA…^XZ` boundaries like Zebra firmware, with
  `print_mirror` / `print_mirror_inverted` / `print_mirror_width` golden
  fixtures rendering at 0.00 % diff.
- **DataMatrix ECC 000-140 (Legacy) encoding (#57)** — all five Legacy
  qualities, six formats, CRC, convolutional protection, randomization and
  generated/cached placement for the 21 supported sizes; an omitted quality
  remains ECC 000, explicit ECC 200 and EPL keep their own encoder paths.
  Original and `^FH` field bytes are preserved through parsing, resets and
  stored-format recalls; Legacy backslashes and pipes stay literal. Ported
  from QR Atelier (MIT, attribution in `licenses/`), with fixed norm examples
  and printer-grid test evidence.

### Fixed

- **Unlicensed font-0 substitute replaced with open-source Roboto Condensed (#65)** —
  the embedded Helvetica Bold Condensed traced back to Adobe's proprietary face
  (ADBE vendor ID, byte-identical glyph metrics after a FontForge rename), so
  every distributed package carried an unlicensed font. Font 0 is now a 40 KB
  Apache-2.0 Roboto Condensed Bold subset, re-calibrated to Labelary
  (`FONT0_CAP_SCALE` 1.3913, advance table refit) and extended with the six
  Latin Extended-A glyphs Labelary renders (`Ă ă Đ đ Ţ ţ`) — fixing the blank
  gaps in Croatian/Serbian/Romanian surnames (#65).
- **PDF417 compaction now matches the reference renderer** — the high-level
  encoder is rewritten (`src/barcodes/pdf417_encoding.rs`) from the ZXing
  heuristic to an ISO/IEC 15438 pipeline that reproduces the reference
  segment-for-segment: block-smoothing Text/Byte/Numeric segmentation (numeric
  compaction for medium digit runs, byte absorption of short blocks, leading
  text blocks never demoted), text sub-mode latching, 901/913/924 byte rules,
  base-900 numeric groups, and the descriptor/padding/row-indicator/cluster
  assembly. `^B7` security level 0 is used verbatim; EPL `b … P` without `s`
  auto-selects the EC level per the EPL2 manual. The isolated fedex
  secondary-message symbol went from 36,500 diff px to 0 and `pdf417_basic`
  is byte-identical to Labelary; fedex_express 6.24 → 3.31 %,
  fedex_ground 5.45 → 2.53 %, fedex 4.85 → 2.06 %, dpdpl 5.60 → 4.07 % and
  nine further labels improve with no tolerance regressions.
- **Diacritics no longer clipped in rotated text fields (#63)** — marks that rise
  above the font ascent (the dots on `Ä`/`Ö`/`Ü`) now render in `^A0R`/`^A0I`/
  `^A0B` fields just as they do in `^A0N`. The off-screen buffer used for
  rotated text gains one em of headroom above the ascent (with the overlay
  shifted back for 270° fields), so `HÄM+ÖÜ` no longer renders as `HAM+OU` in
  rotated orientations. Uncovering the previously clipped cap tops also reveals
  a pre-existing 1–2 px vertical offset of rotated fields vs Labelary; golden
  diffs worsen by at most 0.14 pp per label and stay within tolerance.
- **Rotated font-0 glyphs re-anchored to Labelary** — probe renders at 12–90 pt
  measured rotated font-0 plain text sitting 1–4 px toward its cap side
  (`^A0R` +3..4 px along +x, `^A0I` 3 px along +y, `^A0B` 1..2 px along −x);
  the debt predates the headroom fix but its cost was masked by the clipping.
  A per-orientation overlay correction on plain text removes it: usps_apo
  4.37 → 2.85 %, dhlparcelit 3.18 → 1.93 %, dhlecommercetr 2.91 → 1.91 %,
  swisspost 1.21 → 0.71 %, and nine further labels improve with no label
  regressing beyond 0.05 pp.
- **Font-0 advance deltas now scale with the cell width ratio** — the additive
  per-character deltas (and the missing-glyph advance) were calibrated in
  y-scaled em at cells where `^A0` height equals width, but were applied
  unscaled on the horizontal axis while the glyph's own hmtx advance scales
  with `scale.x ∝ w/h`. On cells whose width differs from height
  (`^A0,60,35` …) every character then drifted against Labelary by a
  fraction of the delta, accumulating over long lines. Scaling the deltas by
  `w/h` (a no-op when width == height) fixes the drift: dpdpl 4.99 → 4.07 %,
  kmart 3.82 → 3.40 %, ups_import_control 3.89 → 3.44 %, ups_surepost
  4.00 → 3.60 %, jcpenney −0.27 pp and 20 further labels improve; no label
  regresses beyond +0.05 pp.
- **Cloudflare Worker after wasm-bindgen 0.2.129** — the glue rename
  (`__wbindgen_cast_*` → `__wbindgen_generic_*`) is tracked in the worker's
  hand-written shim list so deployments stay green across the dependency bump.

### Changed

- **Dependencies bumped (#62)** — imageproc 0.26 → 0.27, lopdf 0.40 → 0.45,
  base64 0.23, rxing 0.9 (barcode decoders are now a feature-gated dev
  dependency; adding new symbologies requires explicit features), sha2 0.11.
  Rendered testdata is byte-identical.
- **Font license attribution completed** — `THIRD_PARTY_LICENSES.md` now covers
  DejaVu Sans Mono (full Bitstream Vera permission text reproduced in
  `licenses/DejaVu-BitstreamVera.txt` from the embedded name table) and the
  provenance of the FontForge-generated ZPL GS font; all four embedded fonts
  are fsType-0 installable-embedding faces with redistributable licenses.

## [1.6.0] - 2026-09-21

### Added

- **Android support** — new `android/` module: a JNI binding crate
  (`labelize-android`, mirroring the wasm API) plus a Gradle library module
  that packages the Kotlin API (`com.goodboy008.labelize.Labelize`,
  `renderZplToPng` / `renderZplToPdf` / `renderEplToPng` / `renderEplToPdf`)
  and native libraries for `arm64-v8a`, `armeabi-v7a`, `x86_64` and `x86`
  (minSdk 24) into a single AAR. Output is byte-for-byte identical to the
  desktop builds — verified on an API 36 arm64 emulator where the demo app's
  PNG and PDF matched a desktop render exactly. A prebuilt
  `labelize-android-aar.zip` is attached to every GitHub Release; CI builds
  the AAR on every push.

### Fixed

- **Empty 2D barcode fields no longer fail the whole label** — an empty `^FD` on
  `^BQ` (QR), `^BX` (DataMatrix) or `^BD` (MaxiCode) now draws nothing (matching
  Labelary) while the rest of the label renders; previously any of them aborted
  rendering with an error like `invalid qr barcode data`. A prefix-only QR field
  (`^FDQA,`) that parses to an empty payload is skipped the same way. An empty
  `^BO` (Aztec) draws the fixed 11×11 bullseye core Labelary renders (2-module
  margin, 69 dark modules), verified pixel-identical in the new
  `empty_barcodes` golden fixture (0.00 % diff).
- **Rotated text: pen-origin anchoring and stroke weight** — rotated `I`/`B`
  font 0 text anchors at the pen origin like Labelary (#45), and rotated
  overlays composite through alpha blending instead of a raw copy, so rotated
  text no longer renders with double-weight strokes under the default 1-bit
  output.
- **Scalable font 1 (`^A1`) and `^FB` justification (#46)** — font 1 is modeled
  on Labelary's monospaced substitute (including its empty-height → doubled-em
  width quirk, 0.73·h caps and 1.17·w advance), and `^FB` with `J`
  justification spreads words across the full block width.
- **Malformed QR data fails cleanly (#47)** — non-UTF-8 bytes in a `^BQ` field
  now surface a parse error instead of panicking.
- **`^CI28` font 0 missing glyphs (#56)** — characters missing from font 0
  render as blank space using Labelary's 0.2976 em advance instead of
  missing-glyph boxes.

### Changed

- **Barcode encoder empty-input contract** — `datamatrix::encode(b"")` now encodes
  naturally to the minimal 10×10 ECC 200 symbol instead of erroring, and
  `aztec::encode(b"")` returns the Labelary minimal symbol instead of erroring
  (both `Ok`). `qrcode::encode` and `maxicode::encode` still reject empty input;
  the skip-or-draw decision lives in the renderer, where Labelary's behavior
  diverges per symbology.

## [1.5.0] - 2026-09-08

### Added

- **`^LT` (Label Top) / `^LS` (Label Shift)** — Global content offsets applied at label
  emission (retroactive within a format, persisting across formats; `^LT` caps at ±120
  dots per Zebra, `^LS` clamps element x at 0). Pixel-identical to Labelary.
- **`^B9` (UPC-E)** — Full encoder: parity table verified against Labelary for all 20
  (number system, check digit) pairs, standard zero-suppression for 11/12-digit input,
  guard extension and interpretation line (NS digit, module-centered digits, check
  digit toggle via parameter e) calibrated against Labelary (bars pixel-perfect).
- **`^A@` (named font) / `^CW` (font identifier)** — Built-in font names resolve;
  downloadable names fall back to the default font (Labelary's `^A@` is nonstandard,
  so we follow the Zebra spec).
- **Stored-object management** — `^ID` (delete graphic or format), `^IM` (move),
  `^IS` (copy), `~EG` (erase graphics; blank name erases all).
- **`^PQ` (print quantity)** — Emits quantity × copies labels (`^PQa,b,c,d`; scoped
  to the format). `^SN`/`^SF` record serial state; `#` serial markers in `^FD` render
  literally, matching Labelary.
- **`^FX` (comment)** — Explicitly parsed and ignored.
- **`^GE` (Graphic Ellipse)** — Ring or filled ellipse (`^GEw,h,t,c`), per-pixel
  elliptical distance; thickness >= minor axis fills. Fixture diff 0.22%.
- **`^B8` (EAN-8)** — 67-module encoder with computed check digit and
  interpretation line; bars pixel-perfect, fixture diff 0.89% total.
- **`^BU` (UPC-A)** — 95-module EAN-13-style symbol with Labelary's 6/5 digit
  split (first six digits L-parity, remaining five + check R-parity), check
  digit from the 11-digit string (verified: 01234567890 -> 5).
- **`^LL` (Label Length)** — Parsed and recorded; no rendering effect (canvas
  size comes from draw options), matching Labelary.
- **EPL2 `B` bar code types** — The bar code selection parameter now follows Table 1 of the EPL Programming Guide (14245L-003 Rev A) instead of silently defaulting unknown types to Code 128: `3`/`3C` (Code 39, optional check digit), `0`/`1`/`1A`/`1B`/`1C`/`1E` (Code 128 UCC/auto/subsets/UCC-EAN), `2`/`2C`/`2D` (Interleaved 2 of 5, optional mod-10 check digit), `E30` (EAN-13), `E80` (EAN-8), `UA0` (UPC-A), and `UE0` (UPC-E). Previously valid files could render the wrong symbology (e.g. type `0` rendered Code 39 instead of Code 128 UCC, `E30` fell through to Code 128). Symbologies without an encoder (Code 93, Codabar, Postnet/Planet, Plessey/MSI, German Post, add-on variants) now fail with an explicit error naming the symbology.
- **EPL2 2-D bar codes (`b`)** — New command with per-symbology options per the EPL Programming Guide: Aztec (`A`; `d` scaling, `e` EC%/layers), Data Matrix (`D`; `c` columns, `r` rows, `h` module size), MaxiCode (`M`; `m` mode with the documented numeric-postal auto-selection between Modes 2/3, otherwise Mode 4), PDF417 (`P`; `s` EC level, `x` module width, `y` per-row bar height, `r`/`l` row/column limits, `t` truncated, `o` rotation), and QR Code (`Q`; `s` scale, `e` EC level). Unsupported options (`f`/`m`/`r` inverse/format flags, structured append, code model 1) are ignored; unknown symbology letters fail with an explicit error.
- **EPL2 graphics & line commands** — `GW` (Direct Graphic Write) now decodes raw binary bitmap data (width in bytes, height in lines) directly from the byte stream — binary payloads containing newline bytes are consumed correctly; `LW` (Line Draw White) draws erasing white rectangles; `LS` (Line Draw Diagonal, `LS,x1,y1,thickness,x2,y2`) draws diagonal lines between two points; `X` (Box Draw, `X,x1,y1,thickness,x2,y2`) draws bordered boxes from two corners. `LE` (exclusive-OR line) remains unsupported. Note `LO`/`LW`/`LE` are four-parameter solid rectangles per the manual — the parser previously handled `LO` correctly.
- **Playground Redesign (v2.0)** — The playground page at `GET /` is restyled with a light/dark theme system and internationalization, plus several new tools. Still a single self-contained HTML page with no external dependencies; served unchanged by both the local HTTP service and the Cloudflare Worker
- **Light/Dark Theme** — Follows `prefers-color-scheme` by default with a header toggle and `localStorage` persistence; applied before first paint to avoid a flash of the wrong theme
- **i18n (English / 简体中文)** — Auto-detects the browser language, with a header selector and persistence; every string including dynamic errors, statuses, and Labelary-compare verdict notes is localized
- **Live Auto-Render** — Optionally re-renders the label automatically ~600 ms after typing stops (toggleable, on by default); keeps the last successful preview and reports background failures only in the status line
- **Share Permalink** — Encodes the label code and render settings into the URL hash and copies the link to the clipboard; opening the link restores everything and renders immediately. Ideal for bug reports
- **Sample Labels** — Built-in shipping, barcodes & 2D, and shapes & graphics examples reachable from the header
- **Preview Zoom** — Zoom in/out (25–400 %), fit-to-panel, and double-click toggle on the preview image
- **Copy PNG to Clipboard** — One-click copy of the current render, alongside the existing PNG/PDF downloads
- **Editor & A11y Polish** — Caret Ln/Col indicator, `Ctrl/Cmd+S` to download the PNG, toast notifications, inline SVG favicon, focus-visible outlines, `prefers-reduced-motion` support, and a stacked responsive layout for narrow screens
- **CLI `--antialias` and playground toggle** — `labelize convert --antialias` emits 8-bit grayscale PNG output preserving the renderer's antialiased greys (default remains 1-bit, the faithful thermal-printer output); the playground gains an Antialias checkbox, persisted locally and carried in share permalinks

### Fixed
- **Resident bitmap fonts P–V calibration** — Font cell metrics now follow the Zebra Font Matrices (P 20×18 through V 80×71) with independent height/width stepping, fixing scaled `^A` output for fonts P/Q/R/T/U/V against Labelary.
- **`^PO I` inverted label compositing** — Inverted labels are now composited via alpha-aware src-over rotation instead of a raw pixel copy, which turned semi-transparent pixels black and doubled text-stroke weight on 1-bit output.

## [1.4.1] - 2026-08-23

### Changed
- **Docker publish tag scheme** — dropped per-commit `sha-<commit>` and bare-major tags; `main`/`edge` for branch pushes, `X.Y.Z`/`X.Y`/`latest` for releases.

## [1.4.0] - 2026-08-20

### Added

- **Antialiased PNG Output (opt-in)** — `POST /convert?antialias=true` (and `DrawerOptions::antialias`) preserves the renderer's coverage-blended greys instead of thresholding to pure black and white, matching what Labelary's PNG preview produces. The default stays 1-bit, which is what a thermal printer actually prints
- **Docker Publishing CI** — New `Docker` workflow builds `linux/amd64` and `linux/arm64` images on native runners and publishes multi-arch manifests to Docker Hub and GHCR on pushes to `main` and on semver `v[0-9]*` tags; pull requests build and smoke-test the image without publishing
- **Image Build Provenance** — Published GHCR manifests carry a signed build provenance attestation, verifiable with `gh attestation verify`
- **`^MU` Units of Measurement** — Support the `^MU` command to set the unit of measurement used by subsequent positioning commands (#26)

### Fixed

- **Font 0 Metrics** — Recalibrated the scalable font 0 against Labelary: a 107-character per-glyph advance table, a width ratio re-fitted to 0.95 now that it only has to describe glyph shape rather than absorb spacing error, and a vertical text origin corrected by -0.02 em -0.8 px. Mean pixel difference across the 50 carrier labels in the golden suite drops from 4.47% to 3.24%, with 49 of 50 improving and none regressing
- **`^FT` Baseline** — Lowered the proportional font ascent used for `^FT` baseline positioning from 0.78 to 0.76 of the cell height, which places `^FT` text closer to where Zebra puts it
- **MaxiCode Encoding** — Replaced the encoder with a standards-oriented implementation for modes 2–4 with data sets A–E, shortest-path text compaction and numeric shift (#32)
- **Code 128 Text Rendering** — Use the condensed bold font for the mode D interpretation line and stop rendering explicit FNC1 invocations in the human-readable text to match Labelary
- **Docker Build** — Dropped the unsupported `--features` flag from `cargo chef prepare`, which caused `docker build` to fail with `error: unexpected argument '--features' found`

## [1.3.0] - 2026-07-20

### Added

- **Docker Support** — Dockerfile and docker-compose for containerized builds and HTTP serving (#14)
- **Golden Tests** — New golden tests for font and coordinate edge cases

### Fixed

- **`^FO`/`^FT` Coordinate Parsing** — Tolerate leading garbage characters in coordinate values (#19)
- **Resident Font B Metrics** — Match Zebra/Labelary width and height metrics for built-in font B (#21)
- **`^CF` Font Designator Parsing** — Parse font designator as a single character, tolerating glued height digits (#18)

## [1.2.0] - 2026-06-29

### Added

- **Windows E2E Test Job** — CLI and HTTP shell tests now run via Git Bash on Windows

### Changed

- **CI Release Workflow** — Improved packaging and checksum generation; added Windows build and packaging support
- **CI Workflows** — All features enabled in cargo commands, serve feature added to build steps

## [1.1.0] - 2026-06-05

### Added

- **Optional Features** — CLI (`cli`) and HTTP server (`serve`) are now optional Cargo features, enabling lightweight library builds without clap/axum/tokio/serde dependencies
- **ZPL Diff Auto-Fix Skill** — New `.claude/skills/zpl-diff-auto-fix/SKILL.md` for automated rendering diff reduction
- **Font Q and Font S Support** — Added ZPL test fixtures for Font Q and Font S with normal and rotated styles

### Fixed

- **CP850 Encoding** — Corrected byte 0xA9 mapping to ® and added mappings for 0xA6–0xAF range

### Changed

- **Dependency Reduction** — clap, axum, tokio, and serde are now optional, feature-gated behind `cli`/`serve` features
- **Conditional Compilation** — `src/main.rs` and `src/lib.rs` refactored with `#[cfg(feature = "...")]` attributes

## [1.0.0] - 2026-05-21

### Added

- **Web Playground** — Built-in browser UI served at `GET /` with ZPL/EPL editor, Labelary-style inch-based label size presets (4×6, 4×4, 4×3, 2×4, 2×2, 3.5×1.5, Custom), inline PNG preview, and one-click PNG/PDF download buttons
- **Open File Support** — Folder icon button in the playground opens a native file picker for `.zpl`/`.epl` files; format selector auto-detects from file extension
- **`labels_dir()` / `unit_dir()` Test Helpers** — Added render helper functions for consistent test data directory resolution across all test suites
- **Release Version Skill** — Reusable `.github/skills/release-version/SKILL.md` skill for cutting reproducible releases

### Fixed

- **`debug_usps_text` Path** — Updated hardcoded test path after testdata reorganization

### Changed

- **Test Data Reorganization** — Split ZPL test fixtures into `testdata/labels/` (carrier/real-world) and `testdata/unit/` (synthetic) subdirectories; flattened snippets into `unit/`; renamed `_ref.png` files to golden PNG convention
- **Dual Diff Reports** — Split the single diff report into two: `testdata/diffs/diff_report_labels.txt` and `testdata/diffs/diff_report_unit.txt` with separate canvas dimensions
- **README Render Comparison** — Updated section with current diff statistics and Labelary comparison gallery
- **Documentation** — Improved clarity of rendering instructions, test assertions, and AGENTS.md workflow documentation

## [0.5.0] - 2026-05-01

### Added

- **Commit Message Guidance** — Added commit message guidelines for the ZPL diff auto-fix workflow

### Fixed

- **MaxiCode ECC Pipeline** — Rewrote MaxiCode encoding with proper GF(64) Reed-Solomon ECC handling
- **MaxiCode Encode Call Site** — Updated encoder call signatures to pass the required mode parameter
- **QR Payload Parsing (`^BQ`)** — Strips `|` separators in QR payloads to match Zebra and Labelary behavior
- **Code 128 Mode N (`^BC`)** — Corrected mode-N data handling and display text behavior to better align with ZPL expectations
- **Text Glyph Rendering** — Render `®` (U+00AE) as superscript in text fields for closer visual parity

### Changed

- **Renderer Cleanup** — Improved readability in renderer formatting paths
- **Golden Calibration** — Tuned bitmap font sizes and golden-test tolerance values to improve comparison accuracy

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
