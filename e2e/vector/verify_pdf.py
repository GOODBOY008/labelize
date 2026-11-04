#!/usr/bin/env python3
"""Check PDF structure/searchability separately from rasterized visual goldens.

Run the Rust e2e_vector_visual target first. --update is an explicit maintainer
operation; normal execution never bootstraps or replaces a missing baseline.
"""
import argparse
import difflib
import json
import os
from pathlib import Path
import shutil
import subprocess

import numpy as np
from PIL import Image
import pypdfium2 as pdfium
import zxingcpp

from pdf_structure import inspect_pdf, require_native_probe

ROOT = Path(__file__).resolve().parents[2]
REFERENCES = ROOT / "testdata/vector/golden"
OUTPUT = ROOT / "output/vector-golden"


def json_text(value):
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def compare(actual, reference, name):
    expected = np.asarray(Image.open(reference).convert("L"))
    actual = np.asarray(actual.convert("L"))
    if actual.shape != expected.shape:
        raise AssertionError(f"{name}: dimensions {actual.shape} != {expected.shape}")
    changed = int(np.count_nonzero(actual != expected))
    if changed:
        Image.fromarray(np.where(actual != expected, 0, 255).astype("uint8")).save(OUTPUT / f"{name}-diff.png")
        raise AssertionError(f"{name}: {changed} pixels differ from the fixed golden")


def check_searchable(path, expected_text):
    """Use actual viewer search, plus two independent Unicode text extractors."""
    if not expected_text:
        return
    with pdfium.PdfDocument(str(path)) as document:
        page = document[0]
        textpage = page.get_textpage()
        texts = {
            "pdfium": textpage.get_text_range(),
            "poppler": subprocess.check_output(["pdftotext", "-layout", str(path), "-"], text=True),
        }
        for expected in expected_text:
            for engine, text in texts.items():
                assert expected in " ".join(text.split()), f"{engine} lost {expected!r}"
            search = textpage.search(expected, match_case=True)
            try:
                assert search.get_next() is not None, f"PDFium cannot find {expected!r}"
            finally:
                search.close()
        textpage.close()
        page.close()


def check_structure(case, _update):
    path = OUTPUT / f"{case['name']}.pdf"
    record = inspect_pdf(path)
    assert len(record["pages"]) == 1, "page count"
    bounds = record["pages"][0]["size_pt"]
    for value, mm in zip((bounds[2] - bounds[0], bounds[3] - bounds[1]), case["size_mm"]):
        assert abs(value - mm * 72 / 25.4) < 0.001, "page size"
    if case.get("native_probe"):
        require_native_probe(record)
    check_searchable(path, case["text"])
    return record


def check_visual(case, update):
    name = case["name"]
    for scale in (1, 3):
        png_name = f"{name}-{scale}x.png"
        if update:
            shutil.copyfile(OUTPUT / png_name, REFERENCES / png_name)
        else:
            compare(Image.open(OUTPUT / png_name), REFERENCES / png_name, f"{name}-png-{scale}x")
    barcodes = {}
    with pdfium.PdfDocument(str(OUTPUT / f"{name}.pdf")) as document:
        page = document[0]
        for scale in case.get("pdf_visual_scales", (1, 3)):
            bitmap = page.render(scale=scale * 8 * 25.4 / 72)
            image = bitmap.to_pil().convert("L")
            assert image.size == tuple(n * scale for n in case["dimensions"]), "extra PDF device pixel"
            filename = f"{name}-pdfium-{scale}x.png"
            image.save(OUTPUT / filename)
            if update:
                image.save(REFERENCES / filename)
            else:
                compare(image, REFERENCES / filename, f"{name}-pdfium-{scale}x")
            # Fixed payloads cannot be redefined by a shared raster/vector regression.
            barcodes[str(scale)] = sorted({f"{b.format}:{b.text}" for b in zxingcpp.read_barcodes(image)})
            bitmap.close()
        page.close()
    return barcodes


def run_checks(kind, cases, update):
    reference = (ROOT / "testdata/vector/pdf-structure.json" if kind == "structure"
                 else REFERENCES / "pdf-barcodes.json")
    expected = None if update else json.loads(reference.read_text())
    records, failures = {}, []
    if expected is not None and set(expected) != {c["name"] for c in cases}:
        failures.append(f"{reference.name}: case names differ from the manifest")
    check = check_structure if kind == "structure" else check_visual
    for case in cases:
        name = case["name"]
        try:
            records[name] = check(case, update)
            if expected is not None and records[name] != expected.get(name):
                diff = "".join(difflib.unified_diff(
                    json_text(expected.get(name)).splitlines(True),
                    json_text(records[name]).splitlines(True), fromfile="expected", tofile="actual"))
                (OUTPUT / f"{name}-{kind}-diff.txt").write_text(diff)
                raise AssertionError(f"{kind} contract changed; see {name}-{kind}-diff.txt")
            print(f"{'CAPTURE' if update else 'PASS'} {kind}: {name}", flush=True)
        except (AssertionError, FileNotFoundError) as error:
            failures.append(f"{name}: {error}")
            print(f"FAIL {name}: {error}", flush=True)
    (OUTPUT / f"pdf-{kind}-report.json").write_text(json_text({"cases": records, "failures": failures}))
    if update and not failures:
        reference.write_text(json_text(records))
    return failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", choices=("structure", "visual", "all"), default="all")
    parser.add_argument("--update", action="store_true", help="explicitly replace reviewed baselines (never in CI)")
    args = parser.parse_args()
    if args.update and os.environ.get("CI"):
        parser.error("golden updates are forbidden in CI")
    cases = json.loads((ROOT / "testdata/vector/cases.json").read_text())
    assert cases and len({c["name"] for c in cases}) == len(cases), "duplicate or empty cases"
    assert sum(bool(c.get("native_probe")) for c in cases) == 1, "native probe is required"
    if args.update:
        REFERENCES.mkdir(parents=True, exist_ok=True)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    kinds = ("structure", "visual") if args.check == "all" else (args.check,)
    failures = []
    for kind in kinds:
        failures.extend(run_checks(kind, cases, args.update))
    if failures:
        raise SystemExit(1)
    action = "captured" if args.update else "passed"
    if "structure" in kinds:
        print(f"{len(cases)} PDF structural goldens and viewer text search checks {action}")
    if "visual" in kinds:
        pdf_images = sum(len(c.get("pdf_visual_scales", (1, 3))) for c in cases)
        print(f"{2 * len(cases)} PNG and {pdf_images} independent PDF visual goldens {action}")


if __name__ == "__main__":
    main()
