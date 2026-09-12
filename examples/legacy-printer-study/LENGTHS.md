# Numeric record-length investigation

Print `legacy-lengths-300dpi.zpl` as raw ZPL on the 300 dpi printer with
50x25 mm media. It contains six labels, one copy each, ordered 500, 501,
511, 512, 596 and 597 digits. Individual `n*-300dpi.zpl` files allow a smaller
follow-up. Each label compares AUTO (left) with explicitly requested 49x49
(right). The exact payload is repeated ASCII `1`, ECC000, format 1, CI27.
The fixed symbol is 147 dots high and ends at dot 235, inside the 300-dot label.
CV is enabled for each barcode and disabled afterwards. These jobs also set
SD15, MD0 and PR2; they do not calibrate the printer or save settings.

Generate with `node examples/legacy-printer-study/generate-lengths.mjs`.
The script validates the actual transmitted payload lengths, field counts and
symbol bounds. The manifest records payload and job hashes, not expected images.
It does not send any data to a printer. Do not render these through Labelize:
the purpose is to observe the native printer, including fields it may reject.

## Record the result

For each label, record both sides as `symbol`, `blank`, or the exact printed
error. Photograph every printed matrix straight-on at module-resolving quality,
including its label ID. Record model, firmware and any changed print settings.
Put observations in a separate file; regeneration overwrites the manifest.

| Digits | AUTO | Fixed 49x49 |
| ---: | --- | --- |
| 500 | symbol (photo) | symbol (photo) |
| 501 | INVALID-L (photo) | INVALID-L (photo) |
| 511 | INVALID-L (user report) | INVALID-L (user report) |
| 512 | INVALID-L (user report) | INVALID-L (user report) |
| 596 | INVALID-L (user report) | INVALID-L (user report) |
| 597 | INVALID-L (user report) | INVALID-L (user report) |

The 2026-09-11 follow-up reproduced the prior AUTO 500/501 boundary and
extended it to the explicit size request. The contributor supplied
a photo of N500/N501 (not bundled in the repository) and reported that all tested lengths above 500
returned INVALID-L, including the fixed-size column. The photo shows only
N500 and N501; the remaining rows rely on that accompanying report.
The two N500 matrices visibly differ and the contributor reports a size change;
their exact module dimensions and payloads have not been recovered from the photo.
Model/firmware are carried over from the prior ZD421 experiment, not re-queried.

This printer experiment is complete: requesting 49x49 does not bypass the
observed 500-character boundary. No >511 record was produced, so it cannot
resolve the historical extended-length convention. No further length bisection
on this device/settings is needed. This is a scoped compatibility observation,
not a universal Legacy capacity or an independently decoded encoder vector.

## Interpretation criteria

- AUTO rejects but fixed49 prints: size selection/request handling contributes
  to the observed restriction. Count the actual modules; a request alone does
  not prove the device used 49x49.
- Both reject: no long-record encoding can be inferred from that case. In
  particular, rejection of 501 cannot be caused by overflowing a nine-bit count.
- A symbol with at least 512 digits prints: recover its module grid, undo
  placement/randomization, inspect the length bits and verify the complete
  candidate payload against CRC and content. A printed symbol or scanner display
  alone does not establish its full encoded length. Our own decoder currently
  assumes nine bits, so its output is not independent evidence for this question.
- 597 is a negative control against Zebra's documented 596 maximum. If it prints,
  record the discrepancy; do not silently adopt a larger global maximum.

The FCD nine-bit description and its 596-character table remain inconsistent.
No production wrapping rule, ten-bit extension or universal 500-character cap
is justified by the current evidence. Keep the explicit >511 unsupported error
until actual record semantics are established. Matrix fixtures must come from
the printer observation, never from our encoder as a substitute reference.
