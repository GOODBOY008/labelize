# Test corpus and reference provenance

[Documentation index](README.md)

Use this ledger to distinguish captured inputs, adapted templates, synthetic
fixtures, external reference images and renderer baselines. Test execution and
reference-update rules live in [Testing](TESTING.md).

- [ZPL and EPL](#zpl-and-epl): carrier captures, production templates and demos.
- [TSPL](#tspl): real input sources and reference capture provenance.
- [CPCL and DPL](#cpcl-and-dpl): vendor examples, manual samples and payload tests.

Specialized fixtures keep their provenance beside the data:
[reverse encoding](../testdata/reverse/README.md),
[vector output](../testdata/vector/README.md), and
[Legacy DataMatrix](DATAMATRIX.md#legacy-ecc-000-140).

## ZPL and EPL


`testdata/labels/` contains captured labels, adapted production templates and
synthetic carrier demos. The tables below distinguish those origins. Current
differences are regenerated in `testdata/diffs/diff_report_labels.txt`; limits
are maintained in [Diff thresholds](DIFF_THRESHOLDS.md).

### Provenance kinds

| Kind | Meaning |
|------|---------|
| real (capture) | Real ZPL captured from a production print stream. Personal names, phone numbers, street addresses and tracking numbers are anonymized while preserving field formats, layout and graphics |
| real (template, adapted) | A real production label template; carrier-specific printer fonts (`^SEE`/`^CW` dot fonts) were normalized to scalable font 0 while keeping the authentic geometry, barcodes and graphics |

### Real corpus

#### Captured production labels

| Label | Carrier / system | Source |
|-------|------------------|--------|
| `usps`, `usps_apo`, `usps_intl` | USPS eVS production stream | captured (anonymized) |
| `ups`, `ups_surepost`, `ups_import_control` | UPS production waybills | captured (anonymized) |
| `fedex`, `fedex_express`, `fedex_ground` | FedEx production waybills | captured (anonymized) |
| `dhlpaket` | DHL Paket (DE) production | captured (anonymized) |
| `dhl_home_delivery` | DHL home delivery production | captured (anonymized) |
| `dhlparceluk` | DHL Parcel UK production | captured (anonymized) |
| `dhlparcelit` | DHL Parcel Italy production | captured (anonymized) |
| `dhlecommercetr` | DHL eCommerce Türkiye production | captured (anonymized) |
| `dpduk.epl` | DPD UK production (EPL) | captured (anonymized) |
| `pnldpd` | DPD pipeline production | captured (anonymized) |
| `dpdpl` | DPD Poland production | captured (anonymized) |
| `glscz`, `glsdk_return` | GLS CZ/DK production | captured (anonymized) |
| `swisspost` | Swiss Post production | captured (anonymized) |
| `posten` | Posten Norway production | captured (anonymized) |
| `porterbuddy` | Porterbuddy (NO) production | captured (anonymized) |
| `icapaket` | ICA Paket (SE) production | captured (anonymized) |
| `dbs` | DB Schenker (SE) production | captured (anonymized) |
| `brtit` | BRT Bartolini (IT) production | captured (anonymized) |
| `posteit` | Poste Italiane production | captured (anonymized) |
| `pocztex` | Pocztex (PL) production | captured (anonymized) |
| `bstc` | BSTC production | captured (anonymized) |
| `amazon`, `amazonshipping` | Amazon FBA / MXP5 production | captured (anonymized) |
| `jcpenney`, `kmart` | US retail routing production | captured (anonymized) |
| `kuehnenagel_intl`, `kuehnenagel_eselect` | Kuehne+Nagel (Centiro platform) | [sahild-cen/Label_EDI_Validator_V3](https://github.com/sahild-cen/Label_EDI_Validator_V3) real uploads (anonymized) |
| `bring` | Bring (SE), Centiro platform | same source as above (anonymized) |
| `labelary` | [Labelary sample label](http://labelary.com/service.html) | official demo |

#### Real production templates (adapted)

| Label | Carrier | Source |
|-------|---------|--------|
| `cjlogistics` | CJ대한통운 (KR) warehouse template | [sLogis-SLK/CS.WCS.PAS `대한통운_템플릿`](https://github.com/sLogis-SLK/CS.WCS.PAS/blob/main/pas.smp/Services/%EC%B6%9C%ED%95%98/%EC%B6%9C%EB%A0%A5%ED%85%9C%ED%94%8C%EB%A6%BF.cs) — Korean dot-font setup normalized to font 0 |
| `correiosbr` | Correios Brasil SEDEX postagem | [marcelofecchio/correios-api `postagem.zpl`](https://github.com/marcelofecchio/correios-api/blob/master/resources/templates/postagem.zpl) — includes the genuine SEDEX service logo as `^GFA` Z64 |
| `tnt_express` | TNT Express 12:00 (Centiro) | real capture from the Kuehne+Nagel/Centiro upload corpus (anonymized); replaces an earlier stylized demo |
| `dhl_express` | DHL Express Worldwide (Centiro) | same source (anonymized); replaces an earlier stylized demo |

### Synthetic fixtures

A number of older golden fixtures in `testdata/labels/` (e.g. `auspost`,
`bpost`, `royalmail`, `canadapost`, `colissimo`, `correos`, `dpdde`, `evri`,
`inpost`, `ontrac`, `postnl`, `seur`, `yodel`, `dbschenker`, `purolator`) are **stylized demos authored from public label designs**, kept
because they exercise rendering paths (QR/DataMatrix/PDF417 placement, rotated
text, `^GFA` logos) against Labelary with tight tolerances. They are *not*
real captures and are listed here so nobody mistakes them for carrier
artifacts. `DIFF_THRESHOLDS.md` marks them implicitly; this note is the
authoritative provenance statement.

## TSPL

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
| `tspl_real_te210.tspl` | [nasnek/FlowStock](https://github.com/nasnek/FlowStock) `flowstock_scanner_diagnostics_v1_TSC_TE210_100x72_working.prn` | **TSC TE210 hardware capture** (repo marks it "working") — 3 labels per stream |
| `tspl_real_sklad.tspl` | [AndreySavastyuk/AndroidSklad01](https://github.com/AndreySavastyuk/AndroidSklad01) `app/src/main/assets/tspl_label.prn` | Russian warehouse app template; `${placeholder}` tokens render as literal text (documented behavior) |

### Reference provenance

`tspl_real_*.png` references were fetched from the labelixa API on
2026-10-12 (before the anonymous daily quota reset). `auto_bootstrap_tspl`
prefers labelixa on refetch and falls back to the renderer baseline when the
quota is spent; both provenances are printed in the bootstrap log line.

## CPCL and DPL


Each `cpcl_real_*` / `dpl_real_*` fixture comes from a renderer vendor's example
corpus or a printer manual. The source tables record adaptations explicitly.

### Reference renderers

| Language | Reference | Notes |
|---|---|---|
| CPCL | `api.labelixa.com/v1/cpcl/render` | Recorded external comparison; an interpretation of the CPCL guide at 203 dpi covering a subset (text, BOX/LINE, 128/39/EAN-13/UPC-A). Same pipeline as the TSPL corpus. |
| DPL | `layoutview.evolabel.com` (Layout view) | Recorded server-side DPL comparison (Evolabel print-applicator preview, ~11.8 px/mm ≈ 300 dpi). Not printer-verified; used as the external comparison like portakal was for TSPL triangulation. |

Both clients cache under `testdata/labelixa_cache/` (gitignored) and fall
back to the renderer baseline when offline — the golden tolerances record
the honest cross-engine distance where a reference was fetched and guard
regressions where the baseline was used.

### Parser and payload regression coverage

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

### CPCL fixtures — Labelixa/thermal-printer-examples

Source: <https://github.com/Labelixa/thermal-printer-examples> (MIT,
© 2026 Newempo LLC), fetched 2026-10-25.

| Fixture | Source file | Content |
|---|---|---|
| `cpcl_real_asset_tag` | `cpcl/asset-tag-2x1.cpcl` | Asset tag 2×1": fonts 4/7, Code 39 (ratio 2, no HRI) |
| `cpcl_real_route_label` | `cpcl/route-label-2x2.cpcl` | Delivery route 2×2": BOX border, LINE rule, `TEXT270`, Code 128 |
| `cpcl_real_receipt` | `cpcl/receipt-label-2x3.cpcl` | Delivery note 2×3": text block, LINE, Code 128 |

Unmodified byte-for-byte from the corpus.

### DPL fixtures — M-Class DPL Programmer's Manual (Appendix B)

Source: *M-Class DPL Programmer's Manual*, Rev B (Datamax-O'Neil,
P/N 88-2302-01), Appendix B "Sample Programs".

| Fixture | Source | Content | Adaptation |
|---|---|---|---|
| `dpl_real_manual_component` | Figure B-1 sample (Basic/C/ASCII programs) | Electronic component label: smooth font 9 (`008` = A18), Code 39 `a` with 6:2 modules, `H07`/`D11` | none — verbatim sample |
| `dpl_real_manual_order` | "VB Application Generating DPL" sample | Order-picking label: font 9 headings/values, five Code 39 fields, box record | field values filled in with plausible data (`SO-44715`, `NORTHWIND SUPPLY`, …); the `1Y…SLANT1` stored-image record is dropped (stored images are unsupported and error loudly); the `~`-alternative STX is written as the standard 0x02 byte |

Current per-fixture tolerances are in [Diff thresholds](DIFF_THRESHOLDS.md);
regenerated measurements are in `testdata/diffs/diff_report_unit.txt`. External
font substitutions and barcode geometry can differ substantially. A renderer
baseline protects regressions but does not independently establish printer
accuracy.
