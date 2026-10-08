# Carrier Label Corpus — Provenance

Every label in `testdata/labels/` is a **real shipping label with a traceable
source**. This document records where each one came from, and what we found
(and did not find) while hunting for real carrier ZPL across every region.

Rendering numbers for each label live in
[DIFF_THRESHOLDS.md](DIFF_THRESHOLDS.md); the live values are regenerated into
`testdata/diffs/diff_report_labels.txt` by `cargo test --test e2e_diff_report`.

## Provenance kinds

| Kind | Meaning |
|------|---------|
| real (capture) | Real ZPL captured from a production print stream. Personal names, phone numbers, street addresses and tracking numbers are anonymized while preserving field formats, layout and graphics |
| real (template, adapted) | A real production label template; carrier-specific printer fonts (`^SEE`/`^CW` dot fonts) were normalized to scalable font 0 while keeping the authentic geometry, barcodes and graphics |

## Real corpus

### Captured production labels

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

### Real production templates (adapted)

| Label | Carrier | Source |
|-------|---------|--------|
| `cjlogistics` | CJ대한통운 (KR) warehouse template | [sLogis-SLK/CS.WCS.PAS `대한통운_템플릿`](https://github.com/sLogis-SLK/CS.WCS.PAS/blob/main/pas.smp/Services/%EC%B6%9C%ED%95%98/%EC%B6%9C%EB%A0%A5%ED%85%9C%ED%94%8C%EB%A6%BF.cs) — Korean dot-font setup normalized to font 0 |
| `correiosbr` | Correios Brasil SEDEX postagem | [marcelofecchio/correios-api `postagem.zpl`](https://github.com/marcelofecchio/correios-api/blob/master/resources/templates/postagem.zpl) — includes the genuine SEDEX service logo as `^GFA` Z64 |
| `tnt_express` | TNT Express 12:00 (Centiro) | real capture from the Kuehne+Nagel/Centiro upload corpus (anonymized); replaces an earlier stylized demo |
| `dhl_express` | DHL Express Worldwide (Centiro) | same source (anonymized); replaces an earlier stylized demo |

## What we searched for and did not find

A systematic hunt for additional real carrier ZPL (GitHub code search across
`^XA` + carrier-name patterns, carrier developer portals, marketplace/3PL
integration repos, 352-file real-upload corpora) found **no public real ZPL**
for the following carriers, because their ecosystems print through formats we
cannot reproduce faithfully:

- **China**: SF Express, Cainiao, JD Logistics, ZTO/YTO/STO/Yunda — all use
  cloud-print JSON templates rendered server-side (丰桥 `COM_RECE_CLOUD_PRINT_COMMAND`,
  菜鸟打印组件, JD open platform). No static ZPL exists.
- **Japan/Korea**: Japan Post, Yamato, Sagawa, Korea Post emit PDF/multi-part
  forms only. (CJ Logistics is the exception, sourced above.)
- **Southeast Asia / India / Middle East / CIS / Africa**: J&T, Ninja Van,
  SingPost, Kerry, Flash, SPX, JNE, SiCepat, Pos Indonesia, LBC, Pos Laju,
  GHN, GHTK, VNPost, Delhivery, Blue Dart, DTDC, Ecom Express, India Post,
  XpressBees, Aramex, Emirates Post, SMSA*, Naqel, Fetchr, iMile, CDEK,
  Russian Post, Boxberry, PEK, Nova Poshta, Ukrposhta, Kazpost, SAPO, PostNet,
  Courier Guy, Pudo, GIG, Sendy, Jumia — carrier APIs return PDF (SMSA has a
  partner-gated `WaybillType=ZPL` option but publishes no sample).
- **Latin America (beyond Correios)**: Mercado Envios exposes
  `response_type=zpl` behind seller auth; Estafeta, OCA, Andreani, Chilexpress,
  Correos de Chile, Servientrega, Deprisa return PDF.
- **Oceania / remaining EU/NA**: NZ Post, StarTrack, Toll, Sendle,
  CouriersPlease, An Post, PostNord, Posti, Österreichische Post, CTT, MRW,
  Magyar Posta, Česká pošta, LaserShip, Canpar, Intelcom — no public ZPL.

If a real label for any of these surfaces (a captured print stream, an
official demo), it is a welcome PR: drop the `.zpl` in, bootstrap the
Labelary reference, add the golden test + tolerance, and add a row here with
its provenance.

## Synthetic fixtures

A number of older golden fixtures in `testdata/labels/` (e.g. `auspost`,
`bpost`, `royalmail`, `canadapost`, `colissimo`, `correos`, `dpdde`, `evri`,
`inpost`, `ontrac`, `postnl`, `seur`, `yodel`, `dbschenker`, `purolator`,
`royalmail`) are **stylized demos authored from public label designs**, kept
because they exercise rendering paths (QR/DataMatrix/PDF417 placement, rotated
text, `^GFA` logos) against Labelary with tight tolerances. They are *not*
real captures and are listed here so nobody mistakes them for carrier
artifacts. `DIFF_THRESHOLDS.md` marks them implicitly; this note is the
authoritative provenance statement.
