# Reproduce the Legacy printer probes

These are native ZPL jobs, not Labelize-generated graphics or golden references.
Use 50x25 mm media on a 300 dpi printer. Jobs set SD15, MD0 and PR2 and reset
CV to N after testing; they do not calibrate the printer or save settings.
Generation scripts write files only and never connect to a printer.

## Literal fields and FH

Print `legacy-escapes-300dpi.zpl` (CI27, L01/L02) and
`legacy-escapes-ci13-300dpi.zpl` (CI13, L03/L04), or their individual label files.
Each job has two labels, one copy each; fields are ECC000, format6, 23x23 at
5 dots/module. Barcode tops are at dot 78 and bottoms at 193. CI13 jobs restore
CI27 after the test fields. Parameter g is omitted.

Regenerate in order:

```
node examples/legacy-printer-study/generate.mjs
node examples/legacy-printer-study/generate-ci13.mjs
```

Manifests identify exact fields and LF-normalized job hashes. Their original
hypotheses describe the comparison design, not the measured outcome.

Recorded device: ZD421-300dpi, firmware V93.21.17Z.

| CI27 / CI13 position | Matching bytes (hex) | Observed meaning |
| --- | --- | --- |
| L01 / L03 left | 41 5C 26 42 | Literal backslash and ampersand |
| L01 / L03 middle | 41 0D 0A 42 | FH-created CR/LF |
| L01 / L03 right | 41 5C 26 42 | FH-created backslash and ampersand |
| L02 / L04 left | 41 5C 5C 42 | Two literal backslashes |
| L02 / L04 middle | 41 5C 42 | One FH-created backslash |
| L02 / L04 right | 41 7C 7C 42 | Two literal pipes |

All twelve symbols were complete; no INVALID diagnostic appeared. Each sampled
23x23 matrix matches its known-byte candidate at all 529 modules. CI27 matches
persisted across common thresholds 130..150 in steps of 5; CI13 used threshold
100 and equals CI27 field by field. Observation JSON preserves sampling geometry,
thresholds, photo hashes and measured rows. Photos are not bundled in the repository.
Candidates came from the same author's Toolkit encoder: this is not independent
decoding. Evidence covers ECC000/format6 and these settings, not every firmware.

`extract-photo-grids.mjs` copies the recorded rows into the fixed Rust fixture
without invoking an encoder. It is an optional maintenance tool, not a build step.
Tests read the fixed fixture and never replace reference data.

## Numeric length

See [LENGTHS.md](LENGTHS.md) for the six AUTO/fixed49 comparisons. Under the tested
settings 500 prints; tested lengths above 500 print no symbol, and CV reports
INVALID-L. This does not settle the historical nine-bit/596-character discrepancy.

References:

- [Zebra BX](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html)
- [Zebra B7 field rules](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-b7.html)
