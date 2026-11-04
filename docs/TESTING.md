# Testing and golden references

[Documentation index](README.md)

Run commands from the repository root. Use the checks relevant to the change;
the complete CI configuration is in [ci.yml](../.github/workflows/ci.yml).

## Rust and process tests

```sh
cargo test --locked --lib --test 'unit_*' --features skill,vector
cargo test --locked --test 'e2e_*' --features vector
cargo test --locked --features serve,vector \
  --test e2e_cli_encode --test e2e_http_encode --test e2e_vector_output
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
```

Unit tests cover parsers, neutral IR, layout, encoders and architecture
invariants. E2E tests exercise complete conversion and image-to-code round
trips. The `serve` feature enables tests that launch the real HTTP server;
`vector` enables scene/PDF coverage. Ignored bootstrap and diagnostic tests
are explicit operations, not part of the ordinary passing-test count.

## Raster goldens

| Fixture directory | Canvas at 8 dpmm | Purpose |
| --- | --- | --- |
| `testdata/labels/` | 813 × 1626 px (101.625 × 203.25 mm) | Carrier captures, templates and synthetic carrier demos |
| `testdata/unit/` | 812 × 1624 px (101.5 × 203 mm) | Focused fixtures; language-declared dimensions can override the canvas |
| `testdata/reverse/` | Fixture-specific | Frozen generated ZPL/TSPL bytes and exact decoded pixels |

[Test corpus](TEST_CORPUS.md) records input and reference provenance.
ZPL references come from Labelary; other languages use documented external
renderers or explicitly identified renderer baselines. A local baseline
detects regressions but cannot independently prove printer accuracy.
[Diff thresholds](DIFF_THRESHOLDS.md) explains the per-fixture tolerances.

After changing rendering logic:

```sh
cargo test --test e2e_diff_report -- --nocapture
cargo test --test e2e_golden
```

Review all three reports in `testdata/diffs/`: `diff_report_labels.txt`,
`diff_report_unit.txt` and `diff_report_reverse.txt`. Commit changed comparison
PNGs and reports with the rendering change. CI regenerates these artifacts and
rejects uncommitted differences under `testdata/`.

For a new ZPL fixture, add its source to the appropriate directory, then fetch
the missing reference:

```sh
cargo test --test e2e_labelary bootstrap_golden_pngs -- --ignored --nocapture
```

This requires network access and does not overwrite existing references.
Add the golden case and documented tolerance, record provenance, and run the
diff/golden commands above. Do not regenerate a reference or relax a tolerance
only to make a failing test pass. Reverse snapshots have their own
[explicit update procedure](../testdata/reverse/README.md).

## Vector PDF and PNG

Vector output needs several independent checks:

| Check | What it establishes |
| --- | --- |
| PDF structural goldens | Executed painting text, paths, curves, images, used embedded fonts and physical page sizes |
| Search and extraction | Expected logical strings found through PDFium search and recovered by PDFium/Poppler |
| Negative controls | Raster PDFs, including raster pages with hidden searchable text, cannot satisfy the native-content probe |
| PNG visual goldens | Stable layout, glyph shapes and clipping in scene PNGs and independently rendered PDFs |
| CLI/HTTP E2E | Options, errors, PNG resolution, page order, quantities and mixed-size multipage output |
| Complete corpus audit | Barcode payload preservation and spatial agreement with both PDFium and Poppler |

PNG snapshots are the appearance check. They do not prove that a PDF contains
native vector content, and searchability alone does not prove visible native
text. Normal tests never create or overwrite vector references.

Run the fixed goldens and process checks using
[the vector fixture guide](../testdata/vector/README.md#run-the-checks).
Run the full corpus and diagnostic controls using
[the independent verification guide](../e2e/vector/README.md).
That verifier preserves the baseline whole-page barcode payload set at each
tested resolution and requires zero ink outside one source dot at 3x;
failed pages are not skipped. Generated reports and images belong under
`output/` and CI artifacts, not `docs/`.

For intentional renderer changes, inspect structural contracts, visual diffs
and decoded payloads before following the fixture guide's
[baseline update procedure](../testdata/vector/README.md#explicitly-update-a-reviewed-baseline).
Keep source bitmap, font coverage and PDF-reader limitations from
[Vector output](VECTOR_OUTPUT.md#hybrid-output-and-fidelity-boundaries) in view.
Software comparisons do not establish physical printer or device behavior.

## Platform checks

- The [WASM and Worker CI job](../.github/workflows/ci.yml) builds the
  [WASM package](../wasm/npm/README.md), runs binding unit tests and executes
  the Worker/playground JavaScript tests.
- [Android build and tests](../android/README.md) and the
  [Kotlin/JNI host smoke check](../e2e/android/README.md) cover the mobile wrapper.
  Host JNI checks are separate from tests on an Android device.
- [HTTP E2E script](../e2e/http/test_http.sh) exercises a running service.
  [Docker publishing](DOCKER_PUBLISHING.md) describes container smoke checks.

## Playground documentation

The Markdown guides are compiled into the self-contained playground. After
editing docs, [regenerate the bundle](README.md#update-the-built-in-docs), then run:

```sh
python tools/docs/build.py --check
node --test workers/playground.test.mjs workers/api.test.mjs
cargo test --lib --features playground playground::
```

The compiler verifies internal page and section links and detects stale
bundles. JavaScript checks cover the embedded scripts and example metadata;
the Rust suite renders every runnable documentation example and checks its
pixel dimensions and nonempty output. CI enables `playground` alongside `vector` for these tests. The command
above also checks a raster-only build locally.

For navigation changes, serve locally and check docs search, empty results,
deep-link reloads, browser back, copying code and loading examples. Confirm
that switching views preserves the editor draft and that share-link reloads
restore source and settings. Check both themes, English/Chinese navigation,
PDF downloads and widths from 320 to 1440 pixels. Documentation content is
English; the surrounding controls follow the playground's language setting.

Also check `#api` deep links, draft preservation across all three views,
request cancellation, image previews, binary file downloads and copied
examples. The API tests exercise the shipped request/response code and run
the generated cURL, Node.js and Python examples against an HTTP byte echo.
Worker tests use the actual WASM for conversions and separately inject an
initialization failure to prove preflights/schema work without the engine.
