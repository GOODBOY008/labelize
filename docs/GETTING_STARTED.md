# Your first label

Go from printer code to a preview, then use the same label in your application.
Labelize reads ZPL, EPL, TSPL/TSC, CPCL and DPL and produces PNG or PDF.
You do not need a printer to start.

## Render a small ZPL label

Open **Playground** and paste the example below, or use **Try in playground**
in the built-in docs. Select **ZPL**, **100 × 60 mm**, and **8 dpmm**.
Press **Render**. You should see “Hello, Labelize” above a Code 128 barcode
on an **800 × 480 pixel** label.

```zpl playground width=100 height=60 dpmm=8
^XA
^FO40,40^A0N,40,30^FDHello, Labelize^FS
^FO40,110^BY2^BCN,80,Y,N,N^FD12345678^FS
^XZ
```

The example button replaces the editor source and settings. Opening docs
and returning to the playground keeps your current work; copy or download
your source before replacing it with an example.

## Understand what you changed

| Part | What it does in this example |
| --- | --- |
| `^XA` / `^XZ` | Start and finish the label |
| `^FO40,40` | Position the next field 40 dots from the left and top |
| `^A0N,40,30` | Select font 0, normal orientation, height 40 and width 30 dots |
| `^FD…^FS` | Supply the field data and finish the field |
| `^BY2` / `^BCN,80,Y,N,N` | Use a 2-dot module and an 80-dot Code 128 barcode with readable text |

At 8 dpmm, 40 dots are 5 mm. Change the first `^FO40,40` to `^FO80,40`
and watch the text move right. Changing the canvas width adds space;
it does not stretch the source. See [sizing and resolution](PLAYGROUND.md#size-and-resolution).

## Export the result

- **PNG** downloads the preview. Default output is black and white; enable
  **Antialias** for grayscale edges.
- **PDF** downloads a document at the selected physical size. Raster is the
  default. Choose **Vector PDF / scalable PNG** when available for searchable
  text and scalable geometry; bitmap content remains resolution-limited.
- **Share** copies a URL containing your source and settings. Anyone with
  that link can read the label data. Avoid sharing addresses or other secrets.

Print at **actual size / 100%**, with “fit to page” disabled. Confirm barcode
readability and placement on your target printer before using a label in production.

## Use the same label through HTTP

Start a local service from a checkout:

```sh
cargo run --features serve,vector -- serve --host 127.0.0.1 --port 8080
```

In another terminal, create the same 800 × 480 PNG:

```sh
curl --fail-with-body 'http://localhost:8080/convert?width=100&height=60&dpmm=8' \
  -H 'Content-Type: application/zpl' \
  --data-binary '^XA^FO40,40^A0N,40,30^FDHello, Labelize^FS^FO40,110^BY2^BCN,80,Y,N,N^FD12345678^FS^XZ' \
  -o label.png
```

Always send the language and size explicitly when you need matching output
across deployments. The [HTTP guide](tutorials/http-service.md) explains
binary uploads, defaults, validation and error responses.

## Choose your next step

| Goal | Guide |
| --- | --- |
| Learn the editor, sizing, image import or comparison | [Playground guide](PLAYGROUND.md) |
| Start with a different printer language | [Languages and examples](LANGUAGES.md) |
| Convert files from a terminal | [CLI tutorial](tutorials/cli.md) |
| Deploy a service | [Docker](tutorials/docker.md) or [HTTP](tutorials/http-service.md) |
| Embed the engine | [Rust](tutorials/rust-library.md), [JavaScript/WASM](tutorials/javascript-wasm.md), [Android](tutorials/android.md) |
| Fix cropped output, fonts or barcodes | [Troubleshooting](TROUBLESHOOTING.md) |
