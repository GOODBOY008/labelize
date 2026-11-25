"""Build the embedded dual-width font from GNU Unifont 16.0.04 (see THIRD_PARTY_LICENSES.md)."""

import hashlib
import io
import sys
import zlib
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

source = Path(sys.argv[1]).read_bytes()
if hashlib.sha256(source).hexdigest() != "0e3981ab552231b5a2a870f2b61741903a4bf25c23ef5aeb05fdced1b3c7af4d":
    sys.exit("Expected the original unifont-16.0.04.otf")

root = Path(__file__).resolve().parents[2]
font = TTFont(io.BytesIO(source), recalcTimestamp=False)
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
    codepoints.update(previous.getBestCmap())
subsetter.populate(unicodes=codepoints)
subsetter.subset(font)

family = "Labelize East Asian Bitmap"
postscript = "LabelizeEastAsianBitmap"
names = {1: family, 3: postscript + "-16.0.04", 4: family, 6: postscript}
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
