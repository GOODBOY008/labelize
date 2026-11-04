# Public HTTP API

Use the [API console](#api) to test requests on the server hosting the
playground, or call the public service at
<https://labelize.764629910.workers.dev> from your own application.
No signup or API key is required. Each request returns its result directly.

## Test and inspect a request

Open **API** beside Playground and Documentation. Its default source,
language, dimensions, resolution and render settings match the playground,
including settings restored from a shared link. Untouched requests follow
the playground when you open the API tab. Once you edit an API request, its
draft is preserved instead. **Use playground source** copies the editor's
current source and settings again without changing the editor or your
chosen API output format. Select an endpoint, adjust its parameters, and
enter source text or choose a file to customize the request.

**Send request** shows the URL, HTTP status, elapsed time, size and response
headers. PNG and PDF responses have inline previews; JSON and text are
readable inline. The PDF preview uses your browser's PDF viewer, with its
page navigation and zoom controls. **Open PDF in new tab** gives the viewer
more space and also works when embedded viewing is unavailable. Downloads
preserve the original PDF and binary bytes. Binary printer data is shown as
hexadecimal. **Cancel** stops waiting for the response; a server conversion
that has already started may still finish.

Switching between API, Documentation and Playground preserves drafts within
the page. Reloading clears unsaved drafts and selected files. Request bodies
are never put into API navigation links.

The cURL, JavaScript and Python examples follow the current request. File
examples read the chosen filename from the script's working directory.
JavaScript examples use Node.js 20+; Python examples use only the standard
library. Copying an example does not send a request.

## Endpoints and OpenAPI

Download the [OpenAPI 3.0.3 specification](/openapi.json), also available from
the console's **OpenAPI JSON** link. Import it into an API client for testing.
Native and Worker deployments serve the same specification, with relative
server URLs that use the hosting origin.

| Method and path | Request body | Response |
| --- | --- | --- |
| `GET /health` | None | JSON `{"status":"ok"}` |
| `POST /convert` | Raw ZPL, EPL, TSPL, CPCL or DPL bytes | PNG or PDF bytes |
| `POST /validate` | Raw label bytes | JSON report; check **`ok`** even with HTTP 200 |
| `POST /format` | Raw label bytes | Formatted source bytes; binary graphics are preserved |
| `POST /encode` | Encoded image bytes | ZPL text or binary TSPL bytes |

Send raw bytes, without JSON or multipart wrapping. `/convert` selects the
parser using `Content-Type: application/zpl` (or `epl`, `tspl`, `cpcl`, `dpl`).
`/validate` and `/format` use `?language=…` or detect the source, independently
of Content-Type. `/encode?language=zpl|tspl` selects the output language.

Errors may be plain text or JSON. Check the HTTP status before saving output;
for validation, also reject reports with `ok: false`. Keep file content and
TSPL output as bytes throughout. See the [HTTP service guide](tutorials/http-service.md)
for all parameters, defaults, image limits and vector/PDF options.

## Convert a file from a terminal

```sh
curl --fail-with-body --max-time 30 \
  'https://labelize.764629910.workers.dev/convert?width=100&height=60&dpmm=8' \
  -H 'Content-Type: application/zpl' \
  --data-binary @label.zpl -o label.png
```

This returns an 800 × 480 PNG. Use `output=pdf` and save as `label.pdf` for
raster PDF. Explicit language, dimensions and dpmm avoid relying on different
defaults between Worker raster, Worker vector and native conversion.

The public Worker supports browser calls from other origins using CORS,
including preflight requests with Content-Type. No cookies or credentials
are needed. This does not configure CORS on a self-hosted native deployment.

## Run a temporary directory conversion

Use the runnable [Python example](../examples/http/convert-directory.py) or
[Node.js example](../examples/http/convert-directory.mjs) from a checkout:

```sh
python3 examples/http/convert-directory.py ./labels ./converted \
  --width 100 --height 60 --dpmm 8

node examples/http/convert-directory.mjs ./labels ./converted \
  --width 100 --height 60 --dpmm 8 --format pdf
```

Both scripts scan the input directory without recursion, choose the language
from each file's extension, and process one request at a time. They preserve
input bytes and write successful responses as `label.zpl.png` or
`label.zpl.pdf`, retaining the source extension to avoid filename collisions.
Existing output files with those names are overwritten after successful
responses. Failed requests are reported individually; the scripts continue
through the remaining files and exit nonzero if any fail.

Use `--base-url` or `LABELIZE_BASE_URL` to choose another deployment:

```sh
python3 examples/http/convert-directory.py ./labels ./converted \
  --base-url http://127.0.0.1:8080 --timeout 60

node examples/http/convert-directory.mjs ./labels ./converted \
  --base-url http://127.0.0.1:8080 --timeout 60
```

The default request timeout is 30 seconds (Python's timeout applies to socket
operations; Node's timeout bounds the complete request). Conversion returns
the first label in each file. For multipage input, use a vector PDF request
with `pages=all` as described in the HTTP guide.

These jobs run in your process and stop when it stops. The API has no job
queue, scheduler or stored-result URLs. Public service availability and
capacity depend on the deployment's [Cloudflare limits](https://developers.cloudflare.com/workers/platform/limits/);
the scripts report failures without automatically retrying them. Run a local
service when you need control over capacity or where your label data is sent.
