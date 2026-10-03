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
