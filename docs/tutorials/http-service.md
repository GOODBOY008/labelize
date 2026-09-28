# Tutorial: HTTP Service

Run Labelize as a REST microservice with a built-in web playground. Two ways
to run the same thing:

- **Native binary**: `labelize serve` (requires the `cli`/`serve` features)
- **Container**: see the [Docker tutorial](docker.md)

A free public instance (built from `main`) is hosted at
<https://labelize.764629910.workers.dev> — you can try every endpoint below
against it without installing anything.

## 1. Start the server

```bash
labelize serve --port 8080
# or bind a specific interface:
labelize serve --host 127.0.0.1 --port 8080
```

## 2. Check health

```bash
curl http://localhost:8080/health
# {"status":"ok"}
```

## 3. Use the web playground

Open <http://localhost:8080/> — no API key, no setup:

1. Paste ZPL or EPL into the editor (or pick a built-in sample)
2. Choose a label size (4×6, 4×4, …) — the preview renders live
3. Download **PNG** or **PDF**, copy the PNG to the clipboard, or share a
   permalink to the exact label + settings
4. ZPL only: click **Compare with Labelary** to fetch the Labelary reference
   render and get a diff score on the same scale as the project's CI tests

## 4. The API

### `POST /convert`

The parser is selected by the `Content-Type` header:

| Content-Type | Parser |
|--------------|--------|
| `application/zpl` | ZPL |
| `application/epl` | EPL |

Query parameters:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `width`   | 102     | Label width in mm |
| `height`  | 152     | Label height in mm |
| `dpmm`    | 8       | Dots per mm (6, 8, 12, 24) |
| `output`  | png     | Output format: `png` or `pdf` |
| `antialias` | false | Preserve antialiased greys instead of 1-bit black/white |

PNG example:

```bash
curl -X POST "http://localhost:8080/convert?width=101.6&height=203.2" \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello World^FS^XZ' \
  -o label.png
```

PDF example:

```bash
curl -X POST "http://localhost:8080/convert?output=pdf" \
  -H "Content-Type: application/zpl" \
  --data-binary @label.zpl \
  -o label.pdf
```

EPL example:

```bash
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/epl" \
  --data-binary @label.epl \
  -o label.png
```

Responses:

| Status | Meaning |
|--------|---------|
| `200`  | Success — body is PNG (`image/png`) or PDF (`application/pdf`) bytes |
| `400`  | Parse error — the label data is invalid; body is a plain-text message |
| `500`  | Render/encode error — the input parsed but rendering failed |

Note: only the **first** label in the body is rendered. If a file contains
multiple labels, split it and make one request per label (or use the CLI,
which writes one output per label).

### `GET /health`

Returns `{"status":"ok"}` — use it for load-balancer probes.

### `GET /`

Returns the playground HTML page.

## 5. Integrate from any language

Anything that can POST bytes works. Python:

```python
import requests

zpl = open("label.zpl", "rb").read()
r = requests.post(
    "http://localhost:8080/convert",
    data=zpl,
    headers={"Content-Type": "application/zpl"},
    params={"width": 101.6, "height": 203.2},
)
r.raise_for_status()
open("label.png", "wb").write(r.content)
```

## 6. What's next

- [Docker tutorial](docker.md) — containerized deployment
- [JavaScript/WASM tutorial](javascript-wasm.md) — render client-side with no
  server at all
