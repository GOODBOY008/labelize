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
