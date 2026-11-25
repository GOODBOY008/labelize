# Third-party licenses

## libzint MaxiCode implementation

Parts of `src/barcodes/maxicode.rs`, including character tables, Reed-Solomon
encoding and text-compaction logic, are adapted from libzint.

Copyright (C) 2010-2026 Robin Stuart <rstuart114@gmail.com>

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.
3. Neither the name of the project nor the names of its contributors may be
   used to endorse or promote products derived from this software without
   specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.

SPDX-License-Identifier: BSD-3-Clause

## Roboto Condensed (font 0 substitute)

`src/assets/fonts/RobotoCondensedBoldFont0.ttf` is Roboto Condensed Bold,
Copyright 2015 Google Inc. All Rights Reserved., licensed under the Apache
License 2.0 (full text in `licenses/Roboto-Condensed-Apache2.txt`). The file
was subset with fontTools to the font-0 character set plus the Latin
Extended-A glyphs Labelary renders (issue #65) and is embedded via
`include_bytes!`.

It replaces the previous `HelveticaBoldCondensedCustom.ttf`, which traced back
to Adobe's proprietary Helvetica Condensed Bold (vendor `ADBE` copyright
string preserved in its name table) and had no redistributable license.

## DejaVu Sans Mono / DejaVu Sans Mono Bold (barcode interpretation lines, bitmap font substitutes)

`src/assets/fonts/DejaVuSansMono.ttf` and `src/assets/fonts/DejaVuSansMonoBold.ttf`
are unmodified DejaVu fonts v2.37. The DejaVu changes are in public domain; the
underlying Bitstream Vera portions are (c) 2003 Bitstream, Inc. and may be
bundled with software free of charge provided the copyright, trademark and
permission notices accompany the copies — see the full license text embedded
in the font files, reproduced in `licenses/DejaVu-BitstreamVera.txt`. The fonts
are shipped unmodified and are not sold standalone, satisfying the license's
renaming and bundling conditions.

## ZPL GS font (`^GS` graphic symbol)

`src/assets/fonts/ZplGSCustom.ttf` is a 9-glyph FontForge-generated TrueType
font (only space and `E` are cmap-mapped) received from the ingridhq/zebrash
project (MIT) and carried over when this engine was extracted. Its name table
carries no copyright string and its exact upstream origin is not documented by
zebrash; the FontForge `FFTM` table and empty metadata indicate a generated
font rather than an extraction from a commercial typeface (extraction keeps
vendor metadata, as the removed Adobe Helvetica file demonstrated). Treated as
distributable via zebrash's MIT license; if provenance concerns arise, the
glyphs can be redrawn from scratch against the Labelary reference without
affecting the renderer.

## East Asian bitmap font substitute

`src/assets/fonts/EastAsianBitmap.otf.zlib` contains a renamed subset of GNU
Unifont 16.0.04. Labelize distributes it under the SIL Open Font License 1.1.
The subset includes the upstream copyright and the full OFL in its OpenType
metadata. The release archives and npm package contain `Unifont-OFL.txt`.
In the Android AAR the notice is at `assets/labelize/licenses/Unifont-OFL.txt`,
and in the Docker image at `/usr/share/doc/labelize/Unifont-OFL.txt`.
The playground serves the same notice under About Labelize → GNU Unifont
(`/#unifont-license`). All copies come from
[licenses/Unifont-OFL.txt](licenses/Unifont-OFL.txt).

The same outlines are embedded on all targets. Named Chinese bitmap fonts
use a full cell for Han and a half cell for ASCII. The subset covers the BMP
unified ideographs, Extension A, compatibility ideographs, Latin-1, common
punctuation, kana, Bopomofo, and half/full-width forms. It also includes
CP936 symbols and the previous DejaVu Sans Mono repertoire. The supplementary
mathematical letters and digits come from Unifont Upper. The private-use
Cyrillic Be variant at U+F6C5 uses the standard Be outline. Glyph shapes follow Unifont's 16-dot design, scaled
for 24-dot cells. Individual device fonts can have different outlines,
regional glyph forms, and character coverage. See Zebra's
[ZQ310 Plus / ZQ320 Plus User Guide, “Printing in Chinese Fonts”](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/product/zq310plus-zq320plus-ug-en.pdf).

To reproduce the subset, run the following from the repository root. The
script verifies each upstream SHA-256 before subsetting.

```sh
curl -fLO https://unifoundry.com/pub/unifont/unifont-16.0.04/font-builds/unifont-16.0.04.otf
curl -fLO https://unifoundry.com/pub/unifont/unifont-16.0.04/font-builds/unifont_upper-16.0.04.otf
uv run --with fonttools==4.68.0 python tools/fonts/subset_east_asian.py unifont-16.0.04.otf unifont_upper-16.0.04.otf
```
