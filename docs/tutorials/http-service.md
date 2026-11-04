# HTTP service

Run a local rendering service with the playground and documentation at `/`.
The same endpoints are available in the Cloudflare Worker, with the default
handling differences described below. For container setup, see [Docker](docker.md).

## Start locally

From a checkout with a Rust toolchain:

```sh
cargo run --features serve,vector -- serve --host 127.0.0.1 --port 8080
```

Open <http://localhost:8080/>. Check the service from another terminal:

```sh
curl --fail-with-body http://localhost:8080/health
# {"status":"ok"}
```

You can also use the public playground's origin in place of localhost.
That sends your label data to the public service; use a local deployment for
private data. Examples below use explicit dimensions and language for
consistent behavior across native and Worker deployments.

## Convert label bytes

`POST /convert` accepts the raw source as its request body. Use the matching
content type: `application/zpl`, `application/epl`, `application/tspl`,
`application/cpcl` or `application/dpl`. Do not wrap the source in JSON.

```sh
curl --fail-with-body 'http://localhost:8080/convert?width=100&height=60&dpmm=8' \
  -H 'Content-Type: application/zpl' \
  --data-binary '^XA^FO40,40^A0N,40,30^FDHello, HTTP^FS^XZ' \
  -o label.png
```

Expect an **800 × 480 PNG**. For a file, replace the inline data with
`--data-binary @label.zpl`. This also preserves control bytes and binary
graphics in the other languages.

| Parameter | Default | Meaning |
| --- | --- | --- |
| `width`, `height` | See below | Positive physical dimensions in mm |
| `dpmm` | `8` | Positive integer source dots per mm; typical choices 6, 8, 12, 24 |
| `output` | `png` | `png` or `pdf` |
| `antialias` | `false` | `true` preserves grayscale edges in PNG |
| `mode` | `raster` | `vector` requires a build with vector support |
| `scale` | `1` | Positive PNG scale; values other than 1 require vector PNG |
| `pages` | `first` | `all` requires vector PDF |

Native conversion resolves each dimension from the explicit query value,
then a supported dimension in the label, then **102 × 152 mm**. Without a
language content type it detects the language from the source.

Worker raster conversion defaults omitted dimensions to **102 × 152 mm**
and an unspecified language to **ZPL**. Worker vector conversion uses declared
dimensions and source detection before falling back. Send all three values
explicitly to avoid relying on these differences.

## PDF and scaled PNG

```sh
# Raster PDF, with an explicit physical page size
curl --fail-with-body 'http://localhost:8080/convert?width=100&height=60&output=pdf' \
  -H 'Content-Type: application/zpl' --data-binary @label.zpl -o label.pdf

# Searchable hybrid PDF; all labels, with their declared sizes where available
curl --fail-with-body 'http://localhost:8080/convert?mode=vector&output=pdf&pages=all' \
  -H 'Content-Type: application/tspl' --data-binary @batch.tspl -o batch.pdf

# 3 output pixels per source dot; source resolution remains 8 dpmm
curl --fail-with-body 'http://localhost:8080/convert?width=100&height=60&mode=vector&scale=3' \
  -H 'Content-Type: application/zpl' --data-binary @label.zpl -o large.png
```

The default returns the first label. Use vector PDF with `pages=all` for a
multipage document, or split input labels into separate requests. See
[vector output](../VECTOR_OUTPUT.md) for bitmap and compositing limits.

## Validate and format

`POST /validate?language=zpl&dpmm=8` returns a JSON report without rendering.
Language defaults to detection; `dpmm` defaults to 8.

```sh
curl --fail-with-body 'http://localhost:8080/validate?language=zpl' \
  --data-binary '^XA^FO40,40^A0N,40,30^FDHello^FS^XZ'
# {"language":"zpl","ok":true,"labels":1}
```

Inspect **`ok`**, even when HTTP status is 200. Parse failures appear in the
report's `error` field. An unknown explicit language returns HTTP 400.

`POST /format?language=zpl` returns formatted source as `text/plain`.
Formatting is not validation: unknown commands can be reflowed without
being supported by the renderer. Keep binary payloads as bytes.

```sh
curl --fail-with-body 'http://localhost:8080/format?language=zpl' \
  --data-binary @label.zpl -o formatted.zpl
```

## Image to code

`POST /encode` takes encoded image bytes (for example PNG or JPEG) and emits
ZPL or TSPL. It embeds a monochrome bitmap, without OCR or barcode recognition.

```sh
curl --fail-with-body 'http://localhost:8080/encode?language=zpl&dpmm=8&threshold=128' \
  -H 'Content-Type: application/octet-stream' --data-binary @logo.png -o logo.zpl

curl --fail-with-body 'http://localhost:8080/encode?language=tspl&dpmm=8' \
  -H 'Content-Type: application/octet-stream' --data-binary @logo.png -o logo.tspl
```

| Parameter | Default | Meaning |
| --- | --- | --- |
| `language` | `zpl` | `zpl` (text/plain) or `tspl` (application/octet-stream) |
| `dpmm` | `8` | Positive integer; one source pixel becomes one printer dot |
| `threshold` | `128` | Integer 0–255; pixels darker than this become ink |

An 800 × 480 image at 8 dpmm occupies 100 × 60 mm. Generated ZPL does not
set the canvas: supply those dimensions when rendering it. TSPL includes
`SIZE` and binary `BITMAP` data. Never decode that output as UTF-8.

## Handle responses

| Endpoint | Successful response | Failure handling |
| --- | --- | --- |
| `/convert` | 200, image/png or application/pdf | Invalid options/parse errors: 400; render/encode failures: 500 |
| `/validate` | 200, application/json | Check `ok`; invalid language/query: 400 |
| `/format` | 200, text/plain | Invalid explicit language/query: 400 |
| `/encode` | 200, text/plain or application/octet-stream | Invalid image/options: 400; internal failures may return 500 |
| `/health` | 200, application/json | Connection failures indicate an unavailable service |

Error bodies can be plain text. Reject non-success responses before saving
output as an image. Native request extraction can also reject malformed
queries or oversized bodies before the handler runs. Production deployments
should define their own authentication, request limits and network access.

## Integrate with Python

Install `requests` in your application's environment, then keep files as bytes:

```python
from pathlib import Path
import requests

response = requests.post(
    "http://localhost:8080/convert",
    data=Path("label.zpl").read_bytes(),
    headers={"Content-Type": "application/zpl"},
    params={"width": 100, "height": 60, "dpmm": 8},
    timeout=30,
)
response.raise_for_status()
Path("label.png").write_bytes(response.content)
```

See [Usage](../USAGE.md) for CLI and Rust options, or
[JavaScript/WASM](javascript-wasm.md) to render within your own application.
