# Vector PDF structure and visual regression fixtures

These are fixed regression baselines for the opt-in vector backend, separate
from the existing Labelary/reference-renderer raster goldens. The original ten
visual cases were captured from `d136aa222bc7afab503f98da4e1db753306bf070`, after
the 173-page independent PDFium/Poppler corpus check passed. The native-content
probe and PDF structural baselines extend that coverage without changing the
original images. They preserve the reviewed implementation; they are not new
printer-vendor reference images.

`cases.json` selects twelve cases spanning all five languages: native glyph
placement, source bitmap QR/barcodes, shipping labels, crossing EPL lines,
TSPL physical dimensions and compositing, CPCL barcodes, DPL bottom-origin
graphics, one-dot and shallow multi-dot diagonals, Unicode/rotated text, and native
text/curves/QR. The multi-dot fixture covers widths 2, 3 and 8 in both
directions; unit tests independently require exact raster pixels at 1x/3x
and preserve later overpainting in the affected regions.

The migrated source fixtures live in `fixtures/`: `text.zpl` covers Unicode
and rotated text, while `regression/` holds the additional samples included in
the complete 174-page corpus. Corpus verification commands are maintained in
[`e2e/vector/README.md`](../../e2e/vector/README.md).

## What each check proves

- **PDF structure:** `pdf-structure.json` records executed text, paths, curves
  and image draws, page dimensions, and the embedded fonts actually used.
  The inspector follows invoked Form XObjects with inherited graphics state;
  unused resources do not count. Invisible/clip-only text and zero-opacity or
  collapsed text are recorded separately from painting text. Both image
  XObjects and inline images count as raster content. Object numbers, font
  subset prefixes, compression and metadata are excluded from the baseline.
- **Native-content requirement:** `native_content.zpl` must produce a PDF with
  painting text, native curves and **zero raster image draws**. This handwritten
  requirement is enforced even during baseline updates. A raster PDF, including
  one with searchable hidden text, must fail. Other cases can intentionally
  contain source bitmaps or hybrid output; they have their own fixed contracts.
- **Searchable text:** expected phrases in `cases.json` must be found using
  PDFium's actual text-search API and recovered by both PDFium and Poppler text
  extraction. This covers Unicode accents, rotated and wrapped text, and the
  searchable layer accompanying corrected resident-font glyph outlines.
  Searchability alone does not prove native painting text. Source bitmap text
  is not OCR'd and has no searchability guarantee.
- **Visual appearance:** every case has monochrome 1x and antialiased 3x scene
  PNGs, plus independent PDFium renderings at 1x and 3x. The native probe also
  has an 8x PDF rendering: **49 committed PNG files**. Comparison is pixel-exact.
  These images catch layout, clipping and glyph-shape regressions; they do not
  establish vector structure. `golden/pdf-barcodes.json` separately freezes the
  decoded payloads so a shared raster/vector regression cannot redefine them.

The structural inspector is a regression check for Labelize's output, not a
complete PDF renderer: clipping, occlusion, colour and actual glyph appearance
remain the responsibility of the visual checks. Raw PDF bytes are not golden
snapshots because serialization changes need not affect document semantics.

`mixed_sizes.tspl` is a process-test fixture: a 40 x 30 mm page followed by a
50 x 20 mm page. `quantity.zpl` uses `^PQ` to produce two copies of the first
format and one of the second. CLI and HTTP tests check page selection, order,
quantity, physical dimensions, overrides, and separate numbered CLI files.

## Run the checks

```sh
cargo test --locked --features serve,vector --test e2e_vector_visual --test e2e_vector_output --test e2e_field_parameter_pdf
python -m pip install -r e2e/vector/requirements.txt
# Install Poppler's pdftotext/pdftoppm with the platform package manager.
python -m unittest discover -s e2e/vector -p 'test_*.py' -v
python e2e/vector/verify_pdf.py --check structure
python e2e/vector/verify_pdf.py --check visual
# Omitting --check runs both layers.
```

The Python PDFium package/bundled engine is pinned in `requirements.txt`.
The field-parameter export target additionally writes isolated horizontal,
reverse and vertical text cases. Python tests check their visible native text,
searchability and geometry in PDFium and Poppler without replacing references.
Reference capture used macOS arm64, PDFium via pypdfium2 5.14.0, Poppler 26.10.0,
and the embedded repository fonts. Generated PNGs/PDFs and failure diagnostics
are written under ignored `output/vector-golden/`, with separate structure and
visual JSON reports. Missing references fail; normal tests never bootstrap or
replace them.

Linux and Windows CI run the Rust visual goldens and real CLI/HTTP tests. A
separate Linux job runs the negative controls, PDF structure/search checks and
PDF visual snapshots as distinct steps, then exports and checks all 175 corpus
pages in **both PDFium and Poppler**, retaining the strict barcode and spatial
gates in `e2e/vector/verify_corpus.py`. It uploads actual outputs and
reports; a failed corpus also uploads all viewer images.

## Explicitly update a reviewed baseline

```sh
cargo test --locked --features vector --test e2e_vector_visual export_vector_golden_candidates -- --ignored
cargo test --locked --features vector --test e2e_field_parameter_pdf
python -m unittest discover -s e2e/vector -p 'test_*.py' -v
python e2e/vector/verify_pdf.py --update
cargo test --locked --features serve,vector --test e2e_vector_visual --test e2e_vector_output --test e2e_field_parameter_pdf
python e2e/vector/verify_pdf.py
```

Inspect changed structural contracts, images and payloads, rerun the complete
independent corpus check after renderer changes, and commit the references with
the intentional change. Updates are forbidden when `CI` is set. Do not update
snapshots merely to make a failing test pass; establish the intended change first.
