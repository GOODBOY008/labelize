"""Inspect executed PDF content independently of rendered PNG snapshots.

This is a structural regression check for Labelize's PDF output, not a general
PDF renderer. Clipping, occlusion, colour and glyph shape remain visual checks.
See ISO 32000-1 sections 8.4, 8.5, 8.10 and 9.3 for graphics/text state and Forms.
"""
from collections import Counter
import re

from pypdf import PdfReader
from pypdf.generic import ContentStream

PATH = {b"m", b"l", b"c", b"v", b"y", b"h", b"re"}
FILL = {b"f", b"F", b"f*", b"B", b"B*", b"b", b"b*"}
STROKE = {b"S", b"s", b"B", b"B*", b"b", b"b*"}
TEXT = {b"Tj", b"TJ", b"'", b'"'}


def determinant(matrix):
    a, b, c, d = map(float, matrix[:4])
    return a * d - b * c


def inspect_pdf(file):
    reader = PdfReader(file)
    pages = []
    for page in reader.pages:
        counts = Counter(painted_text_runs=0, nonpainting_text_runs=0,
                         painted_paths=0, curved_paths=0, image_draws=0,
                         inline_image_draws=0)
        segments, fonts, images = Counter(), {}, []
        active_forms = set()

        def font_record(font):
            # Inspect the font selected by an actual text-showing operator;
            # merely declaring an unused /Font resource does not count.
            key = id(font)
            if key not in fonts:
                if "/ToUnicode" not in font or font.get("/Subtype") != "/Type0":
                    raise AssertionError("used text font lacks a CID font or Unicode map")
                for descendant in font["/DescendantFonts"]:
                    descriptor = descendant.get_object()["/FontDescriptor"]
                    if not any(k in descriptor for k in ("/FontFile2", "/FontFile3")):
                        raise AssertionError("used text font is not embedded")
                name = re.sub(r"^[A-Z]{6}\+", "", str(font["/BaseFont"]).lstrip("/"))
                fonts[key] = dict(name=name, painted_runs=0, nonpainting_runs=0)
            return fonts[key]

        def walk(content, resources, inherited):
            if hasattr(resources, "get_object"):
                resources = resources.get_object()
            state = inherited.copy()
            stack, path = [], Counter()
            # Text matrices are reset by BT, and are not graphics-state entries.
            text_det = 1.0
            for args, op in ContentStream(content, reader).operations:
                if op == b"q":
                    stack.append(state.copy())
                elif op == b"Q":
                    if not stack:
                        raise AssertionError("unbalanced PDF graphics-state restore")
                    state = stack.pop()
                elif op == b"cm":
                    state["det"] *= determinant(args)
                elif op == b"gs":
                    gs = resources["/ExtGState"][args[0]].get_object()
                    for pdf_key, key in (("/ca", "fill_alpha"), ("/CA", "stroke_alpha")):
                        if pdf_key in gs:
                            state[key] = float(gs[pdf_key])
                    if "/Font" in gs:
                        state["font"] = gs["/Font"][0].get_object()
                        state["font_size"] = float(gs["/Font"][1])
                elif op == b"BT":
                    text_det = 1.0
                elif op == b"Tm":
                    text_det = determinant(args)
                elif op == b"Tf":
                    state["font"] = resources["/Font"][args[0]].get_object()
                    state["font_size"] = float(args[1])
                elif op == b"Tr":
                    state["text_mode"] = int(args[0])
                elif op == b"Tz":
                    state["text_scale"] = float(args[0])
                elif op in TEXT:
                    chunks = args[0] if op == b"TJ" else [args[-1]]
                    runs = sum(isinstance(chunk, (str, bytes)) and bool(chunk) for chunk in chunks)
                    if not runs:
                        continue
                    if state["font"] is None:
                        raise AssertionError("text drawn without a font")
                    font = font_record(state["font"])
                    mode = state["text_mode"]
                    fill = mode in (0, 2, 4, 6) and state["fill_alpha"] > 0
                    stroke = mode in (1, 2, 5, 6) and state["stroke_alpha"] > 0
                    painted = (fill or stroke) and all(v != 0 for v in (
                        state["det"], text_det, state["font_size"], state["text_scale"]))
                    kind = "painted" if painted else "nonpainting"
                    counts[f"{kind}_text_runs"] += runs
                    font[f"{kind}_runs"] += runs
                elif op in PATH:
                    path[op.decode("ascii")] += 1
                elif op in FILL | STROKE | {b"n"}:
                    painted = ((op in FILL and state["fill_alpha"] > 0)
                               or (op in STROKE and state["stroke_alpha"] > 0))
                    if path and painted and state["det"] != 0:
                        counts["painted_paths"] += 1
                        counts["curved_paths"] += bool(path.keys() & {"c", "v", "y"})
                        segments.update(path)
                    path.clear()
                elif op == b"Do":
                    obj = resources["/XObject"][args[0]].get_object()
                    if obj.get("/Subtype") == "/Image":
                        counts["image_draws"] += 1
                        images.append(dict(width=int(obj["/Width"]), height=int(obj["/Height"]), inline=False))
                    elif obj.get("/Subtype") == "/Form":
                        key = id(obj)
                        if key in active_forms:
                            raise AssertionError("recursive Form XObject")
                        active_forms.add(key)
                        child = state.copy()
                        child["det"] *= determinant(obj.get("/Matrix", [1, 0, 0, 1, 0, 0]))
                        walk(obj, obj.get("/Resources", resources), child)
                        active_forms.remove(key)
                    else:
                        raise AssertionError("unsupported XObject subtype")
                elif op == b"INLINE IMAGE":
                    settings = args["settings"]
                    counts["image_draws"] += 1
                    counts["inline_image_draws"] += 1
                    images.append(dict(width=int(settings.get("/W", settings.get("/Width"))),
                                       height=int(settings.get("/H", settings.get("/Height"))), inline=True))
            if stack:
                raise AssertionError("unbalanced PDF graphics-state save")

        walk(page.get_contents(), page.get("/Resources", {}), dict(
            font=None, font_size=0, text_mode=0, text_scale=100,
            fill_alpha=1.0, stroke_alpha=1.0, det=1.0))
        pages.append(dict(size_pt=[round(float(v), 4) for v in page.mediabox],
                          **counts, path_segments=dict(sorted(segments.items())),
                          fonts=sorted(fonts.values(), key=lambda f: (f["name"], f["painted_runs"], f["nonpainting_runs"])),
                          images=images))
    return {"pages": pages}


def require_native_probe(summary):
    """Handwritten contract for native_content.zpl, independent of snapshots."""
    if len(summary["pages"]) != 1:
        raise AssertionError("native probe must contain one page")
    page = summary["pages"][0]
    if page["image_draws"]:
        raise AssertionError("native probe must not paint raster images")
    if not page["painted_text_runs"]:
        raise AssertionError("native probe must paint native text; hidden text does not count")
    if not page["painted_paths"] or not page["curved_paths"]:
        raise AssertionError("native probe must paint paths including native curves")
