"""Negative controls: a searchable PDF is not necessarily a vector PDF."""
from pathlib import Path
import tempfile
import unittest

from pypdf import PdfWriter
from pypdf.generic import (
    ContentStream, DecodedStreamObject, DictionaryObject, FloatObject,
    NameObject, NumberObject,
)

from pdf_structure import FILL, STROKE, inspect_pdf, require_native_probe
from verify_pdf import OUTPUT, check_searchable


def stream(data):
    result = DecodedStreamObject()
    result.set_data(data)
    return result


def hide_text(page, reader, mode=3, hide_paths=False):
    content = ContentStream(page.get_contents(), reader)
    content.operations = [([NumberObject(mode)], b"Tr")] + [
        ([NumberObject(mode)], op) if op == b"Tr" else
        ([], b"n") if hide_paths and op in FILL | STROKE else (args, op)
        for args, op in content.operations
    ]
    page.replace_contents(content)


class PdfStructureControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = OUTPUT / "native-content.pdf"
        cls.raster = OUTPUT / "native-content-raster.pdf"
        for path in (cls.native, cls.raster):
            if not path.is_file():
                raise FileNotFoundError(f"{path}: run cargo test --features vector --test e2e_vector_visual first")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.output = Path(temporary.name) / "control.pdf"

    def save(self, writer):
        writer.write(self.output)
        return inspect_pdf(self.output)

    def test_native_text_paths_and_search_pass(self):
        require_native_probe(inspect_pdf(self.native))
        check_searchable(self.native, ["Vector 123"])

    def test_raster_only_is_rejected(self):
        record = inspect_pdf(self.raster)
        self.assertEqual(record["pages"][0]["painted_text_runs"], 0)
        with self.assertRaisesRegex(AssertionError, "raster images"):
            require_native_probe(record)

    def test_raster_with_searchable_hidden_text_is_rejected(self):
        writer = PdfWriter(clone_from=self.raster)
        overlay = PdfWriter(clone_from=self.native)
        hide_text(overlay.pages[0], overlay, hide_paths=True)
        writer.pages[0].merge_page(overlay.pages[0])
        record = self.save(writer)
        page = record["pages"][0]
        self.assertGreater(page["nonpainting_text_runs"], 0)
        self.assertEqual(page["painted_text_runs"], 0)
        self.assertEqual(page["painted_paths"], 0)
        # Search and extraction alone would accept this raster PDF.
        check_searchable(self.output, ["Vector 123"])
        with self.assertRaisesRegex(AssertionError, "raster images"):
            require_native_probe(record)

    def test_unused_font_resources_do_not_count(self):
        writer = PdfWriter(clone_from=self.native)
        page = writer.pages[0]
        self.assertIn("/Font", page["/Resources"])
        page.replace_contents(stream(b""))
        record = self.save(writer)
        self.assertEqual(record["pages"][0]["fonts"], [])
        with self.assertRaisesRegex(AssertionError, "paint native text"):
            require_native_probe(record)

    def test_invisible_and_clip_only_text_do_not_count(self):
        for mode in (3, 7):
            with self.subTest(text_rendering_mode=mode):
                writer = PdfWriter(clone_from=self.native)
                hide_text(writer.pages[0], writer, mode=mode)
                record = self.save(writer)
                self.assertGreater(record["pages"][0]["nonpainting_text_runs"], 0)
                with self.assertRaisesRegex(AssertionError, "paint native text"):
                    require_native_probe(record)

    def test_transparent_or_collapsed_content_does_not_count(self):
        for prefix in (b"/Invisible gs", b"0 0 0 0 0 0 cm", b"0 Tz"):
            with self.subTest(prefix=prefix):
                writer = PdfWriter(clone_from=self.native)
                page = writer.pages[0]
                resources = page["/Resources"]
                resources.setdefault(NameObject("/ExtGState"), DictionaryObject())[NameObject("/Invisible")] = DictionaryObject({
                    NameObject("/ca"): FloatObject(0), NameObject("/CA"): FloatObject(0),
                })
                page.replace_contents(stream(b"q " + prefix + b"\n" + page.get_contents().get_data() + b"\nQ"))
                record = self.save(writer)
                self.assertEqual(record["pages"][0]["painted_text_runs"], 0)
                with self.assertRaisesRegex(AssertionError, "paint native text"):
                    require_native_probe(record)

    def test_inline_image_cannot_bypass_image_check(self):
        writer = PdfWriter(clone_from=self.native)
        page = writer.pages[0]
        page.replace_contents(stream(page.get_contents().get_data() + b"\nq\nBI /W 1 /H 1 /BPC 8 /CS /G ID \x00 EI\nQ\n"))
        record = self.save(writer)
        self.assertEqual(record["pages"][0]["inline_image_draws"], 1)
        with self.assertRaisesRegex(AssertionError, "raster images"):
            require_native_probe(record)

    def test_forms_are_executed_with_inherited_state_each_time(self):
        writer = PdfWriter(clone_from=self.native)
        page = writer.pages[0]
        content = ContentStream(page.get_contents(), writer)
        content.operations = [(args, op) for args, op in content.operations if op != b"Tr"]
        form = stream(content.get_data())
        form.update({NameObject("/Type"): NameObject("/XObject"),
                     NameObject("/Subtype"): NameObject("/Form"),
                     NameObject("/BBox"): page.mediabox})
        # Omit Form resources deliberately: resolve its fonts from the caller.
        page["/Resources"][NameObject("/XObject")] = DictionaryObject({NameObject("/Probe"): writer._add_object(form)})
        page.replace_contents(stream(b"q 3 Tr /Probe Do Q /Probe Do"))
        record = self.save(writer)["pages"][0]
        original = inspect_pdf(self.native)["pages"][0]
        self.assertEqual(record["painted_text_runs"], original["painted_text_runs"])
        self.assertEqual(record["nonpainting_text_runs"], original["painted_text_runs"])
        self.assertEqual(record["painted_paths"], 2 * original["painted_paths"])
        # Declaring the same Form without Do must not count its contents.
        page.replace_contents(stream(b""))
        unused = self.save(writer)["pages"][0]
        self.assertEqual(unused["painted_text_runs"], 0)
        self.assertEqual(unused["painted_paths"], 0)

    def test_metadata_and_stream_compression_do_not_change_golden(self):
        writer = PdfWriter(clone_from=self.native)
        writer.add_metadata({"/Title": "Different metadata", "/CreationDate": "D:20000101000000Z"})
        writer.pages[0].compress_content_streams()
        self.assertEqual(self.save(writer), inspect_pdf(self.native))

    def test_used_fonts_require_embedding_and_unicode_mapping(self):
        for field in ("/ToUnicode", "/FontFile2"):
            with self.subTest(field=field):
                writer = PdfWriter(clone_from=self.native)
                font = next(iter(writer.pages[0]["/Resources"]["/Font"].values())).get_object()
                if field == "/ToUnicode":
                    del font[field]
                else:
                    descriptor = font["/DescendantFonts"][0].get_object()["/FontDescriptor"]
                    for key in ("/FontFile2", "/FontFile3"):
                        descriptor.pop(key, None)
                with self.assertRaisesRegex(AssertionError, "Unicode map|not embedded"):
                    self.save(writer)


if __name__ == "__main__":
    unittest.main()
