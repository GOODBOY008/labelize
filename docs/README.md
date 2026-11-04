# Labelize documentation

Start with a [platform tutorial](tutorials/README.md) to convert your first label.
Use the references below for options, renderer behavior and development work.

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
