# Playground guide

The playground combines an editor, label preview and documentation in one
page. Rendering and validation use the server hosting that page. The docs
are bundled with the application and need no separate documentation service.

## Edit, preview and learn

Choose a sample or open a `.zpl`, `.epl`, `.tspl`, `.cpcl` or `.dpl` file.
Check **Format** after loading your own source. **Live preview** updates as
you type; **Render** or **Ctrl/Cmd + Enter** renders immediately.
**Format code** changes whitespace without acting as a syntax validator.
The editor's syntax status reports parse results separately.

Open **Documentation** whenever you need a guide. Search across page titles
and content, follow the section links, or copy a code block. **Try in
playground** loads an example with its language, dimensions and resolution.
Navigation between docs and the editor preserves your work within the page;
reloading the page does not save an unsaved draft. A **Share** link restores
source and render settings.

## Size and resolution

Width and height describe the physical canvas. Enter millimetres or inches;
switching **Unit** preserves the size. Editing either dimension selects
**Custom…**. The playground sends both dimensions explicitly, so these
controls override dimensions declared in the label source.

| Setting | Example | Effect |
| --- | --- | --- |
| Canvas | 100 × 60 mm | Space available to draw the label |
| Resolution | 8 dpmm | 8 source dots per mm; 800 × 480 pixels at scale 1 |
| Preview zoom | Fit, 100%, etc. | Browser display only; no change to downloads |
| PNG scale | 3×, in vector mode | 2400 × 1440 pixels; source coordinates still use 8 dpmm |

Common printer classes are 6, 8, 12 and 24 dpmm (approximately 152, 203,
305 and 610 dpi). Printer marketing often calls the latter two 300 and 600 dpi.
Use the resolution your label was authored for. Raising dpmm changes the
physical meaning of source dots; use PNG scale to enlarge an export instead.

## PNG and PDF

Raster is the default and PDF embeds the rendered image. Builds with vector
support also offer **Vector PDF / scalable PNG**, **PNG scale** and **All PDF
pages**. PNG previews show the first label. **All PDF pages** combines all
labels into one vector PDF; the current width/height apply to every page.

Vector PDF preserves ordinary text as searchable text and shapes as geometry.
Images and some compositing operations retain local raster content. See
[vector output](VECTOR_OUTPUT.md) for fidelity limits and integration options.

## Image to printer code

Select **ZPL** or **TSPL**, set the resolution, then choose **Image → Code**.
The image is flattened onto white and converted to black and white at the
server's default threshold of 128.

- ZPL output loads into the editor and sets the canvas to the image dimensions.
- TSPL output downloads as a binary file. Keep it as bytes; decoding and
  re-encoding it as UTF-8 can corrupt the bitmap payload.

One image pixel becomes one printer dot. An 800 × 480 image at 8 dpmm is
100 × 60 mm. This embeds a bitmap; it does not recognize text or reconstruct
editable barcode commands. The API's `/encode` endpoint also lets you choose
the threshold. See [the HTTP guide](tutorials/http-service.md#image-to-code).

## Compare with Labelary

For ZPL, **Compare with Labelary** sends the current source to the external
Labelary service and shows its image, Labelize's image and a pixel difference.
It requires PNG scale 1. It is unavailable for EPL, TSPL, CPCL and DPL.
Font differences can produce visible pixel differences even when a label is
usable; a low difference score is not a barcode scan guarantee.

Self-hosted native servers can remove this action with
`LABELIZE_PLAYGROUND_LABELARY_COMPARE=false`. An API-only native deployment
can set `LABELIZE_PLAYGROUND_ENABLED=false`.

## Data and sharing

The hosted playground sends source and uploaded images to its rendering
server. Comparison additionally sends source to Labelary. Share URLs contain
the complete label source; treat them like the original file. Theme and UI
language preferences are stored locally. For confidential labels, use a
local server or [WASM in your own application](tutorials/javascript-wasm.md).
