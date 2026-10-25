# Labelize Architecture

Labelize turns label-printer data (ZPL, EPL, TSPL, CPCL) into pixels. The design goal is
**N languages, one renderer**: each language gets a self-contained frontend
that compiles its commands into a shared, language-neutral IR; a single
language-blind renderer draws whatever the IR describes.

```
Input bytes + Language
  │
  ▼
src/lang.rs ── the only place that picks a parser
  │   Language::{Zpl, Epl, …}          enum of supported languages
  │   parse_labels(lang, bytes, dpmm)  front door every entry point calls
  │   detect_language(bytes)           content sniffing for extension-less input
  │
  ▼
src/parsers/<lang>/ ── per-language frontends (self-contained module trees)
  │   zpl/  parser.rs, virtual_printer.rs (state machine), command_utils.rs,
  │         fs.rs, charsets.rs (^CI), measurement_unit.rs (^MU),
  │         zebra_fonts.rs (font matrices + designator resolution),
  │         qr_payload.rs (^FD format conventions), stored_formats.rs (^FN/^XF)
  │   epl/  parser.rs, fonts.rs (font-number metrics)
  │   tspl/ parser.rs, fonts.rs (resident font cells; SIZE → LabelInfo dims)
  │   cpcl/ parser.rs, fonts.rs (Appendix-D font cells), format.rs,
  │         qr_payload.rs (B QR data conventions); header height /
  │         PAGE-WIDTH → LabelInfo dims
  │
  ▼  compiles commands into
src/elements/ ── language-neutral IR
  │   Text, Barcode*WithData, Graphic*, StoredFormat, LabelInfo {…}
  │   CONTRACT: an element must fully describe its rendering without
  │   naming any printer language. Per-language calibration is resolved
  │   BEFORE the element exists (see "Calibration lives parser-side").
  │
  ▼
src/drawers/ ── language-blind renderer
  │   dispatches per element type; keys text off FontFace (asset +
  │   substitute calibration), never off a printer's font names.
  │   CI-guarded by tests/unit_architecture.rs.
  │
  ▼
src/images/ ── PNG (monochrome 1-bit) or PDF encoding
```

## The two invariants

### 1. Calibration lives parser-side

Everything that encodes "how a specific printer family renders" belongs to
the language's parser module, and is resolved into final semantic values
before an element is built:

| Printer-language knowledge | Resolved into the IR as |
|---|---|
| Zebra font designator (A–H, P–V, 0, 1, GS), ^CW remap, magnification matrices | `FontInfo { face: FontFace, advance_mult, dedot_zero, width, height }` |
| ZPL ^BQ ^FD payload conventions (`QA,…`, `QM,B<len>…`, `\|` separator) | `BarcodeQrWithData { data: raw payload, error_correction }` |
| ^CI charset tables | final Unicode text in `TextField` |
| ^MU unit conversion | dot-unit positions/sizes |
| ^FN/^XF template field merging | fully resolved drawable elements |

`parsers/zpl/zebra_fonts.rs` owns the Zebra matrices and the
designator→`FontFace` mapping; other languages map their own font identifiers
onto `FontFace` values (see EPL) and never call into ZPL modules.

### 2. The IR and the renderer are language-blind

`elements` and `drawers` never mention a printer language. `FontFace`
(`elements/font_face.rs`) is the rendering class — which embedded TTF to use
plus the substitute's measured calibration (cap scale, advance ratio,
line-height factor). The renderer's `draw_text` branches on the face, not on
font names. `tests/unit_architecture.rs` enforces both directions in CI:

- shared modules (`drawers`, `elements`, `barcodes`, `images`) must not
  reference `parsers::`;
- `drawers` code (comments stripped) must carry no `zpl`/`epl`/`tspl` tokens.

Barcode encoders (`src/barcodes/`) are deliberately exempt from the token
scan: symbology-level stream semantics (Code 128 invocation codes, ECC 200
DataMatrix escapes) are encoder concerns — the language flavor is only which
parameters a command exposes.

## Public API surface

```rust
// Explicit language
let labels = labelize::parse_labels(Language::Zpl, &bytes, dpmm)?;

// Sniffed
let lang = labelize::detect_language(&bytes);

// Or the classic per-parser API (unchanged, still public)
let labels = labelize::ZplParser::with_dpmm(8).parse(&bytes)?;
let labels = labelize::EplParser::new().parse(&bytes)?;
```

Entry points (CLI, HTTP `/convert`, wasm bindings) all delegate to
`parse_labels`/`detect_language`; none of them name parser types.

### Compatibility aliases

The 2026-10 module reorganization moved files into per-language directories.
Old paths remain importable as re-export shims: `parsers::zpl_parser`,
`parsers::epl_parser`, `parsers::virtual_printer`,
`parsers::command_utils`, `parsers::fs`, `encodings`. New code should use
`parsers::zpl::…` / `parsers::epl::…` directly.

## Adding a new language (checklist)

Example: TSPL (shipped — see `parsers/tspl/`).

1. **Frontend module** — `src/parsers/tspl/` (`parser.rs` at minimum, plus
   `fonts.rs` etc. as needed). Map commands onto the shared `LabelElement`
   IR; resolve font identifiers into `FontFace` values yourself (do not
   import another language's font tables); keep your payload conventions
   parser-side so elements carry raw/resolved data. Prefer explicit
   `Err` over silently wrong rendering for unknown or ambiguous syntax.
2. **Front door** — add a `Language::Tspl` variant and one `match` arm in
   `src/lang.rs` (both in `Language::parse` and, if needed, a sniffing rule
   in `detect_language`).
3. **Entry points** — one `match` arm each: CLI `InputFormat` + file
   extension, HTTP Content-Type, wasm parameter. None of these need parser
   types.
4. **Tests** —
   - `tests/unit_tspl_parser.rs` (inline strings, no fixtures required);
   - `tests/common/render_helpers.rs`: `parse_tspl` / `render_tspl_to_png`;
   - golden tests in `tests/e2e_golden.rs` with a renderer-baseline
     bootstrap (no public TSPL reference renderer exists, mirroring the EPL
     convention) — tolerance guards regressions only;
   - `tests/e2e_diff_report.rs` `scan_dirs`: accept the `.tspl` extension;
   - a row in `docs/DIFF_THRESHOLDS.md`.
5. **Docs** — a command table in `AGENTS.md` and a CHANGELOG entry.

Do **not** touch `drawers/`, `elements/` semantics, or another language's
module. If the renderer lacks something your language needs, add a neutral
IR field + parser-side resolution and keep the renderer branch
language-blind (`tests/unit_architecture.rs` will hold you to it).
