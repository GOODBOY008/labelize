# CPCL / DPL corpus provenance

Real-provenance fixtures for the CPCL and DPL frontends, mirroring
`docs/CARRIER_LABELS.md` (ZPL) and `docs/TSPL_CORPUS.md` (TSPL): every
`cpcl_real_*` / `dpl_real_*` fixture comes from an official source — a
renderer vendor's example corpus or the printer vendor's own manual —
never from synthetic self-authoring.

## Reference renderers

| Language | Reference | Notes |
|---|---|---|
| CPCL | `api.labelixa.com/v1/cpcl/render` | The only hosted CPCL renderer found; an interpretation of the CPCL guide at 203 dpi covering a subset (text, BOX/LINE, 128/39/EAN-13/UPC-A). Same pipeline as the TSPL corpus. |
| DPL | `layoutview.evolabel.com` (Layout view) | The only hosted server-side DPL renderer found (Evolabel print-applicator preview, ~11.8 px/mm ≈ 300 dpi). Not printer-verified; used as the external comparison like portakal was for TSPL triangulation. Negative results: zplpreview.com/dpl-viewer is browser-side and approximate; no other public DPL renderer exists. |

Both clients cache under `testdata/labelixa_cache/` (gitignored) and fall
back to the renderer baseline when offline — the golden tolerances record
the honest cross-engine distance where a reference was fetched and guard
regressions where the baseline was used.

## Parser and payload regression coverage

The language review also checks behavior that a visual corpus cannot reliably
detect. `tests/unit_cpcl_parser.rs`, `tests/unit_dpl_parser.rs` and
`tests/unit_cpcl_dpl_robustness.rs` cover counted payload boundaries, all byte
values, malformed records, formatter round trips and arbitrary input. The
forward-render tests in `tests/e2e_cpcl_dpl_payloads.rs` compare every CPCL
bitmap pixel with its input bit and independently decode DPL DataMatrix output
at 8 and 12 dpmm, before and after source formatting.

The command semantics come from Zebra's
[CPCL Programming Guide](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/software/cpcl-link-os-pg-en.pdf)
(sections 3.3.2, 6.7, 6.11, 6.19 and 6.34) and Honeywell's
[DPL Command Reference](https://applog.se/storage/B31B01F8DF5B7C3BA38F0A91D9493A5D9D2EE0047E4127279FEC9D22BBD08659/eafb4bac400147bc8931418ea6f25c88/pdf/media/b092b1e119d540c49e214c88e4649039/DPL%20Command%20Reference%20Guide.pdf)
(`T`, `W1C`, `Z`, `U` and scalable font records). DPL byte counts include
the barcode option fields after the count; they override record terminators.
`Tnn` takes two hexadecimal digits and changes only the next format record.

The existing `dpl_2d_graphics` fixture now specifies `200` explicitly for its
ECC 200 symbol, correcting the previous `000` command without replacing its
reference PNG. Legacy DPL DataMatrix qualities and unequal horizontal/vertical
modules remain unsupported and now fail explicitly. QR structured append and
DPL QR hex-ASCII modes also fail explicitly. PDF417 accepts UTF-8 and embedded
control characters; non-UTF-8 PDF417 payloads are rejected because its shared
encoder currently takes text. Font substitution and the previously documented
printer-only/unsupported commands remain limitations of these frontends.

The `dpl_text_styles` renderer baseline was regenerated after removing the
erroneously printed scalable-size fields (`P015P015`). Its 0.01% tolerance is
unchanged. External reference PNGs are unchanged; the scalable-text assertions
and independent box-edge pixel check cover the intended semantic corrections.

## CPCL fixtures — Labelixa/thermal-printer-examples

Source: <https://github.com/Labelixa/thermal-printer-examples> (MIT,
© 2026 Newempo LLC), fetched 2026-10-25.

| Fixture | Source file | Content |
|---|---|---|
| `cpcl_real_asset_tag` | `cpcl/asset-tag-2x1.cpcl` | Asset tag 2×1": fonts 4/7, Code 39 (ratio 2, no HRI) |
| `cpcl_real_route_label` | `cpcl/route-label-2x2.cpcl` | Delivery route 2×2": BOX border, LINE rule, `TEXT270`, Code 128 |
| `cpcl_real_receipt` | `cpcl/receipt-label-2x3.cpcl` | Delivery note 2×3": text block, LINE, Code 128 |

Unmodified byte-for-byte from the corpus.

## DPL fixtures — M-Class DPL Programmer's Manual (Appendix B)

Source: *M-Class DPL Programmer's Manual*, Rev B (Datamax-O'Neil,
P/N 88-2302-01), Appendix B "Sample Programs".

| Fixture | Source | Content | Adaptation |
|---|---|---|---|
| `dpl_real_manual_component` | Figure B-1 sample (Basic/C/ASCII programs) | Electronic component label: smooth font 9 (`008` = A18), Code 39 `a` with 6:2 modules, `H07`/`D11` | none — verbatim sample |
| `dpl_real_manual_order` | "VB Application Generating DPL" sample | Order-picking label: font 9 headings/values, five Code 39 fields, box record | field values filled in with plausible data (`SO-44715`, `NORTHWIND SUPPLY`, …); the `1Y…SLANT1` stored-image record is dropped (stored images are unsupported and error loudly); the `~`-alternative STX is written as the standard 0x02 byte |

## Distances (2026-10-25)

| Fixture | vs reference | Tolerance |
|---|---|---|
| `cpcl_real_asset_tag` | 17.76% (labelixa) | 18.5 |
| `cpcl_real_route_label` | 12.24% (labelixa) | 13.0 |
| `cpcl_real_receipt` | 9.54% (labelixa) | 10.5 |
| `dpl_real_manual_component` | 14.49% (evolabel) | 16.0 |
| `dpl_real_manual_order` | 9.74% (evolabel) | 13.0 |

The dominant diff source is the same class as ZPL-vs-Labelary font
substitution (our CondensedBold substitute renders narrower than the
reference's face, measured ~20% on the Appendix B component label) plus
barcode geometry; positions align (the DPL comparison canvas is derived
from the reference itself at dpmm 12, bottom-left anchored like DPL's
home). Tolerances tighten as calibration lands, mirroring the TSPL
corpus workflow.
