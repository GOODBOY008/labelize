# Independent output verification

The `Vector PDF structure and visual regression` CI job runs this complete
corpus audit on every PR, after checking PDF structure, searchability, negative
controls and fixed visual snapshots. The committed baseline fixtures and real
CLI/HTTP e2e checks are documented in
[`testdata/vector/README.md`](../../testdata/vector/README.md).

Run the commands below from the repository root. Export every label in the
five-language fixture corpus (including quantity copies and multiple formats)
plus the focused fixtures in `testdata/vector/fixtures/regression/`,
preserving the same canvas sizes as golden tests:

```sh
cargo run --release --locked --features vector,serde --example vector_audit
python e2e/vector/verify_corpus.py output/pdf/vector-audit
```

Install the pinned Python dependencies with
`python -m pip install -r e2e/vector/requirements.txt`.
`pdftotext` and `pdftoppm` must be on PATH. Each page receives legacy PNG,
scene PNG at 1x and 3x, vector PDF, and renderings from two independent PDF
engines. The JSON manifest records source file, every label index, page size,
scene composition, fallback reasons, encoded sizes and timings. CLP is listed
as excluded because it is not one of the engine's supported input languages.

`verify_corpus.py` intentionally retains strict gates, including the original
whole-page decoder's payload set at each resolution and zero ink outside one
source dot at 3x. A nonzero exit does not hide those differences: inspect
`verification.json` and the documented viewer/source limitations in
[Vector output](../../docs/VECTOR_OUTPUT.md#hybrid-output-and-fidelity-boundaries).
The script does not silently enlarge tolerances or skip failed pages.

Run controls after the strict verifier, even if it exits with status 1:

```sh
cargo test --locked --features vector --lib export_pdf_outline_controls -- --ignored
cargo run --locked --features cli,vector -- convert testdata/vector/fixtures/text.zpl \
  --mode vector --type pdf --width 80 --height 80 -o output/pdf/vector-text.pdf
python e2e/vector/controls.py output/pdf/vector-audit
```

The control script sends the original raster PDFs through the same viewers
for failed barcode cases, compares eight PDFs with identical glyph positions
but outlined text, and checks the logical strings in a dedicated Unicode/text
fixture. It records pypdf's inferred-spacing differences without asserting
that extractor honors ActualText. PDFium and Poppler must recover the fixture's
logical strings. The Unicode fixture in `testdata/vector/fixtures/text.zpl`
is used by these controls and the fixed goldens; it is separate from the
full-corpus regression directory.

PDFs, PNGs and measurement reports are regenerated under ignored `output/`.
Use each run's `manifest.json` and `verification.json` for page totals, gates,
decoded payloads, spatial measurements, encoded sizes and timings. Historical
investigation reports are available in Git history; they are not current CI
results or golden inputs.

PDF image-resource counts and scene image-operation counts are reported
separately: bounded opaque binary regions can be emitted as pixel paths in the
PDF. For the overall test policy, see [Testing](../../docs/TESTING.md).

The Kotlin/JNI host smoke check is maintained separately in
[`e2e/android/`](../android/README.md).
