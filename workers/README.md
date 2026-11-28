# Cloudflare website and API

The public Worker serves the project website at `/` and `/zh/`, the generated
guides at `/docs/` and `/zh/docs/`, and the embedded workspace at `/playground/`.
The native HTTP server continues to serve the workspace at its root.

From the repository root, build the static website and WASM engine, then run Wrangler:

```sh
python3 -m venv workers/.venv
workers/.venv/bin/pip install -r tools/docs/requirements.txt
workers/.venv/bin/python workers/site/build.py
./wasm/build.sh
(cd workers && npx wrangler@4 dev)
```

Edit the homepage and navigation in `workers/site/` and the guide content in
`docs/`. After editing a guide, run `workers/.venv/bin/python tools/docs/build.py`
to update the embedded copy, then rebuild the website. The deploy workflow builds
the public site from source. Generated site pages live in the ignored `workers/dist/`.

The homepage samples use a 100 × 50 mm order at 8 dots/mm. To verify their
previews, run `node --test workers/demo.test.mjs` after building WASM. To
regenerate the tracked PNGs, run `node workers/demo.test.mjs --update-previews`.
