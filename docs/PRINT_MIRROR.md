# Print mirror (^PM)

`^PMY` mirrors the whole label horizontally. The setting persists across
`^XA`/`^XZ` and subsequent calls to the same `ZplParser`. `^PMN` disables it;
constructing a new parser starts with mirroring disabled. Missing or invalid
parameters leave the current state unchanged. The last valid setting in a
format applies to every field and every `^PQ` copy of that label.

The completed image is mirrored after print-width clipping/centering and `^PO`
rotation. `DrawerOptions::enable_inverted_labels` controls `^PO` only; disabling
that option does not disable `^PM`.

Downloaded formats store an optional explicit mirror instruction. Defining a
format does not change the printer's mirror setting. Recalling a format replays
its instruction, if present; a format without `^PM` inherits the printer's state
at recall time. A later `^PMN` in the calling format takes precedence.

## Sources and comparison (September 10, 2026)

- The authoritative [Zebra Programming Guide, ^PM section](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/software/zpl-zbi2-pg-en.pdf)
  specifies horizontal reflection, persistence until explicit disable/power-off,
  and ignoring missing/invalid parameters.
- [Labelary lists ^PM as supported](https://labelary.com/docs.html). Live requests
  confirmed persistence, reset, ignored invalid/missing parameters, whole-label
  application, print-width/rotation composition, and stored-format behavior.
- The inspected [ZPLR interpreter at 590f03c](https://github.com/le2ni/zplr/blob/590f03c58c79663be37708cd97f8cff5eb0d797e/src/core/interpreter.ts)
  initializes mirror state per label and uses `yesNo(args[0], true)` for `^PM`.
  Its [raster transform](https://github.com/le2ni/zplr/blob/590f03c58c79663be37708cd97f8cff5eb0d797e/src/core/raster.ts)
  implements horizontal reflection composed with 180-degree rotation. Its
  parameter/state handling is not used as the authority for this implementation.
- Our JavaScript `zpl-toolkit` snapshot `51bca3ef` already interprets, generates,
  edits, and renders `^PM`. Its existing `^LS`/`^LT`/`^PM` rendering regression
  passes. However, a live `Interpreter.processJob()` probe returned mirror flags
  `[true, false]` for two labels with `^PMY` only in the first. Thus its graphical
  implementation exists, but its per-label reset does not implement Zebra's
  persistence rule. The toolkit was inspected, not modified, for this change.

The Labelary HTTP endpoint also ignored a `^PMY` placed before the first `^XA`.
That observation does not establish physical-printer behavior outside formats.
Like other existing printer settings in Labelize, `^PM` is interpreted wherever
the command parser receives it; the conformance cases below use complete formats.

## Independent geometry measurements

Requests used `POST https://api.labelary.com/v1/printers/8dpmm/labels/1x0.5/0/`
with `Accept: image/png` and the synthetic field
`^FO10,20^GB20,10,10^FS` inside `^XA`/`^XZ`. The returned canvas was 203 x 101
dots. Coordinates below are inclusive black-pixel bounds.

| Settings | Left, top, right, bottom |
|---|---|
| Default | 10, 20, 29, 29 |
| `^PMY` | 173, 20, 192, 29 |
| `^PW100` | 61, 20, 80, 29 |
| `^PW100^PMY` | 122, 20, 141, 29 |
| `^PW100^POI` | 122, 71, 141, 80 |
| `^PW100^PMY^POI` | 61, 71, 80, 80 |
| `^PW300^PMY` | 173, 20, 192, 29 |

The odd 103-dot margin for `^PW100` distinguishes mirroring the completed canvas
from mirroring the narrower printable region before centering.

## Regression fixtures

`testdata/unit/print_mirror{,_width,_inverted}.zpl` have independent PNG references
downloaded from Labelary at `8dpmm/labels/4.005x8.01/0/` (813 x 1626 pixels).
These contain only asymmetric filled graphics, including print-width clipping,
so font rasterization does not require a tolerance. `unit_print_mirror` compares
all three decoded images pixel-for-pixel; the normal golden suite also runs them.
Reference generation has no local-renderer fallback in this investigation.

Other tests cover persistent state, separate parser instances, invalid inputs,
stored formats containing only settings, rotated text, QR codes, clipping,
antialiasing, and the inversion option. No physical printer was used.

## Library data model

`LabelInfo::mirrored` is the resolved per-label snapshot. Code constructing
`LabelInfo` with a struct literal must supply `mirrored: false` for normal output.
`StoredFormat::mirrored` and `RecalledFormat::mirrored` are `Option<bool>`:
`None` inherits the current setting, while `Some(false)` explicitly disables it.
EPL labels initialize the new flag to false.
