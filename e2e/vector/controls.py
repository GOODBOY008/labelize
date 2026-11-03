#!/usr/bin/env python3
"""Isolate PDF viewer effects with legacy-PDF and same-layout outline controls."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image
from pypdf import PdfReader
import pypdfium2 as pdfium

from verify_corpus import barcodes, gray, measure


def pdfium_image(file, dimensions, scale):
    doc = pdfium.PdfDocument(str(file))
    page = doc[0]
    bitmap = page.render(scale=scale * 8 * 25.4 / 72)
    image = Image.new("L", tuple(v * scale for v in dimensions), 255)
    image.paste(bitmap.to_pil().convert("L"), (0, 0))
    bitmap.close()
    page.close()
    doc.close()
    return image


def write(file, data):
    file.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "output/pdf/vector-audit")
    manifest = json.loads((root / "manifest.json").read_text())
    pages = {p["name"]: p for p in manifest["pages"]}
    report = json.loads((root / "verification.json").read_text())
    controls = []
    with tempfile.TemporaryDirectory(prefix="labelize-pdf-control-") as tmp:
        for record in report["pages"]:
            failed = [f.removeprefix("lost barcode in ") for f in record["failures"]
                      if f.startswith("lost barcode in ")]
            if not failed:
                continue
            name = record["name"]
            dimensions = pages[name]["dimensions"]
            pdf = root / f"{name}-legacy.pdf"
            decoded = {}
            for key in failed:
                scale = int(key[-1])
                if key.startswith("pdfium"):
                    image = pdfium_image(pdf, dimensions, scale)
                elif key.startswith("poppler"):
                    stem = Path(tmp) / key
                    subprocess.run(["pdftoppm", "-singlefile", "-png",
                                    "-scale-to-x", str(dimensions[0] * scale),
                                    "-scale-to-y", str(dimensions[1] * scale),
                                    str(pdf), str(stem)], check=True, capture_output=True)
                    image = Image.open(stem.with_suffix(".png"))
                else:
                    raise ValueError(f"Cannot use a PDF viewer control for {key}")
                decoded[key] = barcodes(image)
            controls.append({"name": name, "expected": record["decoded"]["legacy"],
                             "vector": {k: record["decoded"][k] for k in failed},
                             "legacy_pdf": decoded})
    write(root / "barcode-controls.json", controls)

    outline_root = root.parent / "vector-outline-control"
    outlines = []
    for file in sorted(outline_root.glob("*.pdf")):
        name = file.stem
        image = pdfium_image(file, pages[name]["dimensions"], 3)
        outlines.append({"name": name, "pdfium_outlines":
                         measure(gray(root / f"{name}-3x.png"), np.asarray(image), 3)})
    if len(outlines) != 8:
        raise RuntimeError("Run export_pdf_outline_controls first (expected eight PDFs)")
    write(outline_root / "verification.json", outlines)

    file = root.parent / "vector-text.pdf"
    doc = pdfium.PdfDocument(str(file))
    page = doc[0]
    textpage = page.get_textpage()
    texts = {"pypdf": PdfReader(file).pages[0].extract_text(),
             "pdfium": textpage.get_text_range(),
             "poppler": subprocess.check_output(["pdftotext", "-layout", str(file), "-"], text=True)}
    textpage.close()
    page.close()
    doc.close()
    write(root.parent / "vector-text-extraction.json", texts)
    for engine in ["pdfium", "poppler"]:
        for expected in ["Café Straße 123", "100200", "One two three",
                         "four five six", "ROTATED", "Registered® and copyright©"]:
            if expected not in " ".join(texts[engine].split()):
                raise AssertionError(f"{engine} lost {expected!r}")
    print(f"Recorded {len(controls)} barcode controls, {len(outlines)} outline controls, and three text extractors")


if __name__ == "__main__":
    main()
