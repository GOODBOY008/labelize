"""Homepage examples from the Studio 100 × 50 mm order template."""

from pathlib import Path

from links import playground_url

FIXTURES = Path(__file__).resolve().parents[2] / "testdata" / "templates" / "printers"

# The Studio DPL fixture also downloads a binary logo. These records print
# the same order, Code 128 and QR in a form that can be edited in a textarea.
DPL_ORDER = """\x02L
m
D11
122200004150040Order: 000123
122200003150040Thermal labels
1e2216000700065000123
1W1D5500002150730MA,000123
E"""


def load_demos():
    demos = {}
    for language in ("zpl", "epl", "tspl", "cpcl", "dpl"):
        if language == "dpl":
            source = DPL_ORDER
        else:
            # read_text normalizes the CRLF used by printers to textarea LFs.
            source = (FIXTURES / f"first.{language}").read_text(encoding="ascii").strip()
        demos[language] = {
            "source": source,
            "url": playground_url(source, language, 100, 50, 8),
            "image": f"/assets/demo/{language}.png",
        }
    return demos
