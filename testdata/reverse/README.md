# Image → code golden fixtures

These are persisted regression fixtures for the reverse encoder, separate from
the property/round-trip tests in `tests/e2e_reverse.rs`.

| Case | Input | Options | Coverage |
|---|---|---|---|
| `shipping` | Existing `../labels/amazon.png` (813×1626) | 8 dpmm, threshold 128 | Real text/barcodes, odd width, more than 99999 packed bytes: two ZPL graphic fields |
| `rgba` | `rgba_input.png` (205×137) | 8 dpmm, threshold 128 | Color luminance, transparent/partial-alpha pixels, strict 127/128 boundary, edge pixels and row padding |
| `threshold` | Same RGBA image | 12 dpmm, threshold 200 | Custom threshold (199/200 boundary), non-default resolution and fractional millimetre dimensions |

Each case has a committed `.zpl` and **binary** `.tspl` output snapshot and a
`.png` reference captured from Labelary rendering that ZPL. `references.json`
records the exact API URL and SHA-256 hashes of the submitted code and response.
No cropping, padding, or local-renderer fallback is used for these references.

Before a capture is accepted, every reference pixel is checked against an
independent oracle: the shipping input is grayscale thresholded at 128, and
the synthetic swatches have hand-specified black/white decisions. The colored
input has four rows of four swatches, from left to right:

1. White, black, gray 127, gray 128.
2. Gray 199, gray 200, red, green.
3. Blue, yellow, transparent black, black with alpha 127.
4. Black with alpha 128, black with alpha 56, red with alpha 128, green with alpha 128.

The TSPL tests use the same independently verified raster as their expected
image. **This is not a TSPL printer/reference-renderer capture.** It verifies
that the generated TSPL renders to the required pixels, including binary
polarity and inline payload handling; the snapshots also lock down its bytes.

Six `golden_reverse_*` tests in `tests/e2e_golden.rs` encode the input anew,
require byte-identical code snapshots, parse/render the new code, and require
identical dimensions and every RGBA pixel to match the committed reference.
They run offline and fail on missing fixtures. They do not honor the general
`LABELIZE_UPDATE_GOLDEN` shortcut because overwriting an independent reference
with the implementation under test would weaken this check.

`tests/e2e_diff_report.rs` also writes `../diffs/diff_report_reverse.txt` and
`../diffs/reverse_{case}_{language}.png` comparisons (reference left, current
Labelize output right), plus `_diff.png` pixel masks. CI's existing stale
artifact check covers this directory and those outputs.

To intentionally update fixtures, review the encoder change, then run:

```bash
cargo build --features cli
# Requires Python 3, Pillow, and access to api.labelary.com.
python3 testdata/reverse/regenerate.py
cargo test --test e2e_golden golden_reverse
cargo test --test e2e_diff_report -- --nocapture
```

The capture script stages everything in a temporary directory and replaces
fixtures only after all external responses pass the independent pixel checks.
Review and commit the source/snapshots/references and regenerated comparisons
together. `.gitattributes` preserves LF ZPL and binary TSPL on Windows.
