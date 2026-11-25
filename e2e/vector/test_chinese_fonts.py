"""Search a mixed-script field as one line, including supplementary glyphs."""
import subprocess
import unittest
from pathlib import Path


class ChineseFontPdf(unittest.TestCase):
    def test_mixed_script_line(self):
        directory = Path(__file__).resolve().parents[2] / "output/vector-golden"
        for font in ("GBUNSG16.CPF", "GBUNSG24.CPF", "TSS16.BF2", "TSS24.BF2"):
            for dpmm in (8, 12):
                for mode in ("-raw", "-layout"):
                    with self.subTest(font=font, dpmm=dpmm, mode=mode):
                        pdf = directory / f"{font}-{dpmm}.pdf"
                        text = subprocess.check_output(["pdftotext", mode, pdf, "-"], encoding="utf-8")
                        self.assertEqual(text.rstrip("\n\f"), "A保温杯B𝚉CΩβЖāǎě€")
