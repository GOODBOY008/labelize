# Labelize documentation

Start with [your first label](GETTING_STARTED.md): render a small example,
understand its coordinates, and export an 800 × 480 PNG. These guides are also
available inside the playground under **Documentation**, with search, section
links, copyable code and examples that load directly into the editor.

## Learn in the playground

| Guide | What you will learn |
| --- | --- |
| [Your first label](GETTING_STARTED.md) | Render, edit, export and call the API |
| [Playground guide](PLAYGROUND.md) | Sizing, resolution, sharing, image import and data handling |
| [Languages and examples](LANGUAGES.md) | Choose a parser and try small labels |
| [Troubleshooting](TROUBLESHOOTING.md) | Diagnose clipping, font differences, barcodes and API errors |

## Use Labelize

| Document | Purpose |
| --- | --- |
| [Tutorials](tutorials/README.md) | CLI, Docker, HTTP, JavaScript/WASM, Android and Rust walkthroughs |
| [Usage](USAGE.md) | Installation, CLI and native HTTP reference, library examples |
| [Vector output](VECTOR_OUTPUT.md) | Raster/vector modes, searchable PDF, scalable PNG and fidelity limits |
| [ZPL command matrix](ZPL_COMMANDS_REFERENCE.md) | Command support and parameter reference |

## Develop and verify

| Document | Purpose |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Parser/IR/renderer boundaries and the new-language checklist |
| [Testing](TESTING.md) | Unit/E2E commands, golden policy and independent PDF verification |
| [Test corpus](TEST_CORPUS.md) | Fixture sources, adaptations and reference provenance |
| [Diff thresholds](DIFF_THRESHOLDS.md) | Per-fixture visual tolerances and known rendering differences |
| [DataMatrix](DATAMATRIX.md) | ECC 200 and Legacy behavior, dimensions, field data and evidence |
| [Rendering notes](RENDERING_NOTES.md) | Print-mirror semantics and QR mask selection |
| [TSPL baselines](TSPL_BASELINES.md) | Calibration policy, reference coverage and unresolved differences |
| [Docker publishing](DOCKER_PUBLISHING.md) | Maintainer configuration, image tags and publishing workflow |

Release history lives in the [changelog](../CHANGELOG.md). Build and integration
details stay beside their code: [Android](../android/README.md),
[WASM package](../wasm/npm/README.md) and the
[Worker deployment workflow](../.github/workflows/deploy-worker.yml).

## Keep documentation organized

- `tutorials/` is the only subdirectory here. Keep platform walkthroughs there
  and maintained reference documents at the root, linked from this index.
- Give each topic one canonical document. Link to it from tutorials and the
  repository README instead of copying implementation or test details.
- Keep test fixtures and golden provenance under `testdata/`, runnable checks
  under `e2e/`, and generated reports under ignored `output/` or CI artifacts.
- Keep completed plans, task checklists and one-off investigation results in
  Git/PR history. Preserve lasting semantics, source attribution and evidence
  limitations in the appropriate reference document.
- Update inbound links when renaming or removing a document. Keep dated
  observations scoped to their renderer/device; do not present old test totals
  or measurements as the current build's result.

The SVG logos and preview PNGs at this root are shared documentation assets;
their paths are also used by package READMEs and playground metadata.

## Update the built-in docs

Markdown in this directory is the source for the playground documentation.
The page manifest and compiler live in `tools/docs/build.py`. To regenerate:

```sh
python3 -m venv /tmp/labelize-docs
/tmp/labelize-docs/bin/pip install -r tools/docs/requirements.txt
/tmp/labelize-docs/bin/python tools/docs/build.py
/tmp/labelize-docs/bin/python tools/docs/build.py --check
```

Commit the Markdown and generated `src/playground/docs.html` together. CI checks
for stale output and broken internal page/section links. Native, Worker and
WASM playgrounds embed the same bundle; runtime builds need neither Python
nor a documentation server. Add runnable examples with a fenced-code info
string such as `zpl playground width=100 height=60 dpmm=8`; the dimensions and
resolution must match the surrounding explanation. Only mark complete,
renderable examples this way. Other code blocks are copyable references.
