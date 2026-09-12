# Golden validation and reference generation

Normal golden tests and diff reports read the committed ZPL/EPL inputs and PNG
references. Missing/unreadable inputs, missing/invalid PNGs, and unavailable
fixture directories are errors with paths. No comparison fetches or creates a
reference. Diff reports also fail on render errors; large pixel differences are
still reported, with per-fixture thresholds enforced by the golden suite.

```sh
cargo test --test e2e_golden -- --test-threads=4
cargo test --test e2e_diff_report -- --nocapture
```

These commands work without network access after Cargo dependencies are installed.
They may write diagnostic images/reports under `testdata/diffs/`, but never modify
reference PNGs. `LABELIZE_UPDATE_GOLDEN` no longer changes this behavior: a failing
comparison still fails, even when that environment variable is present.

For ZPL unit fixtures the effective tolerance is `min(requested, 8%)`; an explicit
0% or 1% limit remains 0% or 1%. Other ZPL fixtures and EPL fixtures retain their
requested limits. This percentage counts differing pixels using the existing
per-channel threshold of 32; it is not byte-for-byte PNG equality.

## Explicit ZPL reference generation

The following ignored tests must be invoked by name with `--ignored --exact`.
They use Labelary (or its response cache), validate the returned PNG before
writing, and fail on fetch/validation errors. They never call the local renderer
to substitute for a missing external reference. HTTP requests use HTTPS with a
30-second timeout. Existing cached responses are not a fresh external verification.

Generate only missing references in `testdata/`, `testdata/labels/`, and
`testdata/unit/`; existing PNGs remain untouched:

```sh
cargo test --test e2e_labelary bootstrap_golden_pngs -- --ignored --exact --nocapture
```

Explicitly replace all unit ZPL references with Labelary results:

```sh
cargo test --test e2e_labelary update_unit_golden_pngs -- --ignored --exact --nocapture
```

Both use 8 dpmm and `4.005x8.01` inches (813 x 1626 pixels). Successful downloads
are saved unchanged, without padding/cropping. Failed downloads or invalid PNGs
do not create a new reference or replace an existing one. A batch may write its
successful items before reporting failures; review its Git diff before committing.
Explicit live comparison tests also fail if Labelary is unavailable.

After adding a reference, run the golden tests and diff report, inspect the
comparison images, and commit the input, independently obtained reference,
changed diff artifacts, and source details together. Do not increase tolerances
or replace references simply to make a failing test pass.

## EPL and existing references

Labelary does not support EPL. Keep existing EPL references, or add a reviewed
PNG from a documented independent source next to the `.epl` input. There is no
automatic EPL reference generator or renderer fallback in validation.

Existing references were retained by this change. Some predate this workflow;
neither a passing test nor an image's dimensions prove its independent origin.
An audit of historical provenance is separate from enforcing the new workflow.

## Regression coverage

`unit_golden_fixtures` exercises temporary files, missing/unreadable inputs,
missing/corrupt references, directory discovery, failed/invalid fetches, and
explicit creation without overwriting existing references. Golden and diff tests
also exercise their actual failure paths. A subprocess checks that the legacy
update flag cannot overwrite ZPL or EPL references or turn a mismatch into a pass.
All negative cases use temporary directories; committed references are untouched.
