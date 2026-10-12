# TSPL Test Corpus

Real-world TSPL/TSC label data used by the golden tests (`tspl_real_*` in
`testdata/unit/`). Provenance follows the repo convention: every file has a
real source; nothing here is synthesized. Reference PNGs were rendered by
[labelixa.com](https://labelixa.com/tools/tspl-viewer) (see
[TSPL_BASELINES.md](TSPL_BASELINES.md) for the baseline strategy).

| Fixture | Source | Notes |
|---|---|---|
| `tspl_real_probe_text.tspl` | [hyiger/filament-db](https://github.com/hyiger/filament-db) `tests/fixtures/tspl/02_probe_text.prn` | Text-only capability probe from a Rust 3D-filament inventory project |
| `tspl_real_probe_full.tspl` | [hyiger/filament-db](https://github.com/hyiger/filament-db) `tests/fixtures/tspl/03_probe_full.prn` | Full element-kind probe (incl. `REVERSE`, magnified text) |
| `tspl_real_drybox.tspl` | [hyiger/filament-db](https://github.com/hyiger/filament-db) `tests/fixtures/tspl/04_drybox_label.prn` | 3D-printing drybox label |
| `tspl_real_te210.tspl` | [naskek/FlowStock](https://github.com/nasnek/FlowStock) `flowstock_scanner_diagnostics_v1_TSC_TE210_100x72_working.prn` | **TSC TE210 hardware capture** (repo marks it "working") — 3 labels per stream |
| `tspl_real_sklad.tspl` | [AndreySavastyuk/AndroidSklad01](https://github.com/AndreySavastyuk/AndroidSklad01) `app/src/main/assets/tspl_label.prn` | Russian warehouse app template; `${placeholder}` tokens render as literal text (documented behavior) |

## Screened out

- [nithin2k5/FeederCable](https://github.com/nithin2k5/FeederCable) `*.prn` — real TSPL
  (uses `DMATRIX`, `PUTBMP`, `.TTF` font names) but `PUTBMP` loads BMPs from
  printer storage, which the stream does not carry; renders as an explicit
  error. Revisit if `PUTBMP` gets a host-side form.
- [NakodaInfotech/TEXTRADE](https://github.com/NakodaInfotech/TEXTRADE) `*.PRN` —
  Seagull-driver output whose body is actually **EPL** (`A…` commands);
  belongs to (and parses with) the EPL side, not this corpus.
- [Tekvan1612/WMS-Sonata](https://github.com/Tekvan1612/WMS-Sonata) `*.prn` — Seagull
  proprietary print stream, not TSPL.
- PR #13's `testdata/tspl/**` — TSC documentation samples; `doc_*` styles are
  covered by the synthetic unit fixtures (`tspl_graphics_doc_style` etc.).

## Reference provenance

`tspl_real_*.png` references were fetched from the labelixa API on
2026-10-12 (before the anonymous daily quota reset). `auto_bootstrap_tspl`
prefers labelixa on refetch and falls back to the renderer baseline when the
quota is spent; both provenances are printed in the bootstrap log line.
