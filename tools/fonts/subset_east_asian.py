"""Build the embedded dual-width font from GNU Unifont 16.0.04 (see THIRD_PARTY_LICENSES.md)."""

import hashlib
import io
import sys
import zlib
from pathlib import Path

from fontTools import subset
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import TTFont

def pinned_font(path, sha256):
    source = Path(path).read_bytes()
    if hashlib.sha256(source).hexdigest() != sha256:
        sys.exit(f"Unexpected SHA-256 for {path}")
    return TTFont(io.BytesIO(source), recalcTimestamp=False)


root = Path(__file__).resolve().parents[2]
font = pinned_font(sys.argv[1], "0e3981ab552231b5a2a870f2b61741903a4bf25c23ef5aeb05fdced1b3c7af4d")
upper = pinned_font(sys.argv[2], "fc8c6e9c4958243cef651c87cfbd916916f3545215d3ebcae92366b6edbb2f05")
options = subset.Options()
options.name_IDs = ["*"]  # Preserve copyright and license metadata.
subsetter = subset.Subsetter(options)
# Latin, punctuation, radicals, ideographic descriptions, kana, Bopomofo, Han
# (including Extension A), vertical/compatibility and half/full-width forms.
codepoints = set(subset.parse_unicodes(
    "0020-00FF,2000-206F,2E80-2FDF,2FF0-2FFF,3000-312F,31A0-31BF,31F0-31FF,3400-4DBF,4E00-9FFF,F900-FAFF,FE10-FE1F,FF01-FFEF"
))
# Include the CP936 symbols as well as Han. The single-byte euro sign is
# specific to CP936, while Python's gbk codec implements the double-byte map.
codepoints.add(0x20AC)
for lead in range(0x81, 0xFF):
    for trail in range(0x40, 0xFF):
        if trail != 0x7F:
            codepoints.update(map(ord, bytes((lead, trail)).decode("gbk", errors="ignore")))
# The previous substitute covered Greek, Cyrillic, extended Latin and other
# scripts. Keep every character shared with the upstream font.
with TTFont(root / "src/assets/fonts/DejaVuSansMono.ttf") as previous:
    previous_chars = set(previous.getBestCmap())
    codepoints.update(previous_chars)
subsetter.populate(unicodes=codepoints)
subsetter.subset(font)

# Keep all glyphs in one face so mixed-script fields retain a single PDF
# baseline and text run. Both upstream fonts use identical coordinate units.
cmap = dict(font.getBestCmap())
upper_cmap = upper.getBestCmap()
top = font["CFF "].cff.topDictIndex[0]
private = top.FDArray[0].Private
upper_glyphs = upper.getGlyphSet()
base_glyphs = font.getGlyphSet()
for codepoint in sorted(previous_chars - cmap.keys()):
    if codepoint == 0xF6C5:
        # Adobe's afii10064 is a private-use variant of Cyrillic small Be.
        # Give the substitute its own glyph ID to preserve the original Unicode.
        glyph = base_glyphs[cmap[ord("б")]]
    else:
        glyph = upper_glyphs[upper_cmap[codepoint]]
    name = f"cid{top.CIDCount:05d}"
    top.CIDCount += 1
    advance = glyph.width
    width_delta = None if advance == private.defaultWidthX else advance - private.nominalWidthX
    pen = T2CharStringPen(width_delta, None)
    glyph.draw(pen)
    strings = top.CharStrings
    strings.charStrings[name] = len(top.charset)
    strings.charStringsIndex.append(pen.getCharString(private, top.GlobalSubrs))
    top.charset.append(name)
    top.FDSelect.append(0)
    font["hmtx"][name] = (advance, glyph.lsb)
    cmap[codepoint] = name
font.setGlyphOrder(top.charset)
FontBuilder(font=font).setupCharacterMap(cmap)
font["OS/2"].recalcUnicodeRanges(font)

family = "Labelize East Asian Bitmap"
postscript = "LabelizeEastAsianBitmap"
license_text = (root / "licenses/Unifont-OFL.txt").read_text(encoding="utf-8")
names = {1: family, 3: postscript + "-16.0.04", 4: family, 6: postscript,
         13: license_text, 14: "https://openfontlicense.org/open-font-license-official-text/"}
for record in font["name"].names:
    if record.nameID in names:
        font["name"].setName(names[record.nameID], record.nameID, record.platformID,
                             record.platEncID, record.langID)
cff = font["CFF "].cff
cff.fontNames = [postscript]
top = cff.topDictIndex[0]
top.FamilyName = top.FullName = family
for dictionary in top.FDArray:
    dictionary.FontName = postscript

output = io.BytesIO()
font.save(output)
target = root / "src/assets/fonts/EastAsianBitmap.otf.zlib"
target.write_bytes(zlib.compress(output.getvalue(), level=9))
