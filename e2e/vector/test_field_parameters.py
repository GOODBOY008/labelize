"""Native text and independent viewer geometry for each field direction.

The external Labelary tests establish the source semantics. These checks
additionally reject a PDF-only regression that PNG comparison cannot detect.
"""
import json
import subprocess
import unittest

import numpy as np
from PIL import Image
import pypdfium2 as pdfium

from pdf_structure import inspect_pdf
from verify_corpus import measure
from verify_pdf import OUTPUT, check_searchable


CASES = {
    "h": "FIELDGAP", "r": "REVERSE", "v": "VERTICAL",
    "r-block": "REVERSE", "r-wrapped": "REVERSE",
    "r-words": "FIRST SECOND THIRD",
}


class FieldParameterPdf(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.results = {}
        for name in CASES:
            for suffix in (".pdf", "-3x.png"):
                path = OUTPUT / f"field-parameter-{name}{suffix}"
                if not path.is_file():
                    raise FileNotFoundError(
                        f"{path}: run cargo test --features vector "
                        "--test e2e_field_parameter_pdf first"
                    )

    @classmethod
    def tearDownClass(cls):
        report = {
            "passed": sum("failure" not in result for result in cls.results.values()),
            "total": len(CASES),
            "cases": cls.results,
        }
        (OUTPUT / "field-parameter-pdf-report.json").write_text(
            json.dumps(report, indent=2) + "\n", encoding="utf-8"
        )

    def verify_flow(self, name):
        text = CASES[name]
        stem = f"field-parameter-{name}"
        path = OUTPUT / f"{stem}.pdf"
        result = {"source_text": text}
        self.results[name] = result
        try:
            summary = inspect_pdf(path)
            self.assertEqual(len(summary["pages"]), 1)
            page = summary["pages"][0]
            self.assertGreater(page["painted_text_runs"], 0, "native painting text required")
            self.assertEqual(page["nonpainting_text_runs"], 0, "hidden text is not a substitute")
            self.assertEqual(page["image_draws"], 0, "text-only probe must not rasterize")
            self.assertTrue(page["fonts"], "used native fonts required")
            # inspect_pdf validates embedding and ToUnicode for every USED font,
            # following Form XObjects and rejecting unused-resource shortcuts.
            for value in (page["size_pt"][2] - page["size_pt"][0],
                          page["size_pt"][3] - page["size_pt"][1]):
                self.assertAlmostEqual(value, 80 * 72 / 25.4, delta=0.001)
            check_searchable(path, [text])
            result["structure"] = page
            result["searchable_in_pdfium_and_poppler"] = True
            with Image.open(OUTPUT / f"{stem}-3x.png") as scene:
                self.assertEqual(scene.size, (1920, 1920))
                expected = np.asarray(scene.convert("L"))
            with pdfium.PdfDocument(str(path)) as document:
                pdf_page = document[0]
                bitmap = pdf_page.render(scale=3 * 8 * 25.4 / 72)
                try:
                    viewer = bitmap.to_pil().convert("L")
                    raw_dimensions = viewer.size
                    # Match the corpus verifier: PDF float points can create an
                    # extra edge pixel. Use the declared source-dot canvas only.
                    declared = Image.new("L", (1920, 1920), 255)
                    declared.paste(viewer, (0, 0))
                    declared.save(OUTPUT / f"{stem}-pdfium-3x.png")
                    spatial = {"pdfium": measure(expected, np.asarray(declared), 3)}
                    spatial["pdfium"]["raw_dimensions"] = raw_dimensions
                finally:
                    bitmap.close()
                    pdf_page.close()
            poppler_stem = OUTPUT / f"{stem}-poppler-3x"
            subprocess.run(
                ["pdftoppm", "-singlefile", "-png", "-scale-to-x", "1920",
                 "-scale-to-y", "1920", str(path), str(poppler_stem)],
                check=True, capture_output=True,
            )
            with Image.open(poppler_stem.with_suffix(".png")) as viewer:
                self.assertEqual(viewer.size, (1920, 1920))
                spatial["poppler"] = measure(expected, np.asarray(viewer.convert("L")), 3)
            result["spatial"] = spatial
            for engine, metrics in spatial.items():
                self.assertEqual(metrics["outside_tolerance_pixels"], 0,
                                 f"{name}: {engine} ink outside one source dot: {metrics}")
        except Exception as error:
            result["failure"] = str(error)
            raise

    def test_horizontal_gap_has_native_searchable_text(self):
        self.verify_flow("h")

    def test_reverse_flow_retains_logical_search_text(self):
        self.verify_flow("r")

    def test_vertical_flow_retains_logical_search_text(self):
        self.verify_flow("v")

    def test_reverse_block_retains_logical_search_text(self):
        self.verify_flow("r-block")

    def test_reverse_wrapped_word_remains_searchable_across_lines(self):
        self.verify_flow("r-wrapped")

    def test_reverse_wrapped_words_retain_logical_order(self):
        self.verify_flow("r-words")


if __name__ == "__main__":
    unittest.main()
