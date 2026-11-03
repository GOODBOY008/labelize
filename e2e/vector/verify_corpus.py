#!/usr/bin/env python3
"""Independent corpus checks (Pillow, numpy, pypdf, pypdfium2, zxing-cpp, Poppler).

Retains strict measurements; a low-resolution ink IoU is not a text-layout oracle.
PDF/PNG agreement is checked at 3x, with a declared one-source-dot tolerance.
"""
import concurrent.futures
import json
from pathlib import Path
import subprocess
import sys
import numpy as np
from PIL import Image, ImageFilter
from pypdf import PdfReader
import pypdfium2 as pdfium
import zxingcpp


def gray(path):
    return np.asarray(Image.open(path).convert("L"))


def measure(a, b, radius=1):
    a, b = a < 128, b < 128
    union = np.count_nonzero(a | b)
    def bounds(mask):
        y, x = np.where(mask)
        return [int(x.min()), int(y.min()), int(x.max()), int(y.max())] if len(x) else None
    ab, bb = bounds(a), bounds(b)
    da = np.array(Image.fromarray((a * 255).astype("uint8")).filter(ImageFilter.MaxFilter(2 * radius + 1))) > 0
    db = np.array(Image.fromarray((b * 255).astype("uint8")).filter(ImageFilter.MaxFilter(2 * radius + 1))) > 0
    return {"ink_iou": float(np.count_nonzero(a & b) / union) if union else 1,
            "page_difference_percent": float(100 * np.count_nonzero(a ^ b) / a.size),
            "outside_tolerance_pixels": int(np.count_nonzero(a & ~db) + np.count_nonzero(b & ~da)),
            "bbox_delta": max(abs(x-y) for x, y in zip(ab, bb)) if ab and bb else (0 if ab == bb else None)}


def barcodes(image):
    return sorted({f"{b.format}:{b.text}" for b in zxingcpp.read_barcodes(image)})


def check(args):
    root, page = args
    name = page["name"]
    file = root / f"{name}.pdf"
    pdf = PdfReader(file)
    failures = []
    def gate(ok, message):
        if not ok: failures.append(message)
    gate(len(pdf.pages) == 1, "page count")
    p = pdf.pages[0]
    for actual, mm in zip([p.mediabox.width, p.mediabox.height], page["size_mm"]):
        gate(abs(float(actual) - mm * 72 / 25.4) < 0.001, "physical page dimensions")
    fonts, images = [], []
    def resources(res):
        for font in res.get("/Font", {}).values(): fonts.append(font.get_object())
        for ref in res.get("/XObject", {}).values():
            obj = ref.get_object()
            if obj.get("/Subtype") == "/Image": images.append(obj)
            elif obj.get("/Subtype") == "/Form": resources(obj.get("/Resources", {}))
    resources(p["/Resources"])
    gate(not page["stats"]["glyphs"] or bool(fonts), "missing native font objects")
    for font in fonts:
        gate("/ToUnicode" in font, "missing Unicode map")
        descriptor = font["/DescendantFonts"][0].get_object()["/FontDescriptor"]
        gate("/FontFile2" in descriptor or "/FontFile3" in descriptor, "font not embedded")
    # Image resources can be deduplicated by the PDF writer.
    gate(len(images) <= page["stats"]["images"], "undeclared raster images")
    doc = pdfium.PdfDocument(str(file))
    fp = doc[0]
    textpage = fp.get_textpage()
    texts = {"pypdf":p.extract_text(), "pdfium":textpage.get_text_range(),
             "poppler":subprocess.check_output(["pdftotext", "-layout", str(file), "-"], text=True)}
    textpage.close()
    legacy = gray(root / f"{name}-legacy.png")
    scene = gray(root / f"{name}.png")
    expected = barcodes(Image.open(root / f"{name}-legacy.png"))
    decoded = {"legacy":expected, "png1":barcodes(Image.open(root / f"{name}.png")),
               "png3":barcodes(Image.open(root / f"{name}-3x.png"))}
    w, h = page["dimensions"]
    spatial = {}
    for scale in [1, 3]:
        bitmap = fp.render(scale=scale * 8 * 25.4 / 72)
        # MediaBox uses float points: PDFium can ceil to an extra edge pixel.
        # Record that rounding; crop only to the declared source-dot canvas.
        img = bitmap.to_pil().convert("L")
        image_size = img.size
        padded = Image.new("L", (w * scale, h * scale), 255)
        padded.paste(img, (0, 0))
        img = padded
        img.save(root / f"{name}-pdfium-{scale}x.png")
        decoded[f"pdfium{scale}"] = barcodes(img)
        if scale == 3:
            spatial["pdfium"] = measure(gray(root / f"{name}-3x.png"), np.asarray(img), 3)
            spatial["pdfium"]["raw_dimensions"] = image_size
        bitmap.close()
        stem = root / f"{name}-poppler-{scale}x"
        subprocess.run(["pdftoppm", "-singlefile", "-png", "-scale-to-x", str(w * scale),
                        "-scale-to-y", str(h * scale), str(file), str(stem)], check=True, capture_output=True)
        pi = Image.open(stem.with_suffix(".png"))
        decoded[f"poppler{scale}"] = barcodes(pi)
        if scale == 3:
            spatial["poppler"] = measure(gray(root / f"{name}-3x.png"), np.array(pi.convert("L")), 3)
    fp.close()
    doc.close()
    for key, result in decoded.items():
        gate(set(expected).issubset(result), f"lost barcode in {key}")
    for key, metrics in spatial.items():
        gate(metrics["outside_tolerance_pixels"] == 0, f"{key}: ink outside one source dot")
    return {"name":name, "failures":failures,"fonts":len(fonts), "images":len(images),
            "texts":texts,"decoded":decoded,"legacy_scene":measure(legacy, scene),"scene_pdf_3x":spatial}


if __name__ == "__main__":
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "output/pdf/vector-audit")
    manifest = json.loads((root / "manifest.json").read_text())
    with concurrent.futures.ProcessPoolExecutor(max_workers=4) as pool:
        result = list(pool.map(check, [(root, p) for p in manifest["pages"]]))
    report = {"pages":result,"passed":sum(not p["failures"] for p in result),"total":len(result)}
    (root / "verification.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    for p in result:
        if p["failures"]: print(p["name"], p["failures"], flush=True)
    print(f'{report["passed"]}/{report["total"]} strict functional gates', flush=True)
    sys.exit(0 if report["passed"] == report["total"] else 1)
