#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-unknown-unknown
rustup target add $TARGET

cargo build --release --target $TARGET

# Generate the JS glue directly into the npm package directory. Consumers
# (the Cloudflare Worker, bundlers, Node.js) reference wasm/npm/.
mkdir -p npm
wasm-bindgen --target bundler --out-dir npm --out-name labelize_wasm \
  target/$TARGET/release/labelize_wasm.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz npm/labelize_wasm_bg.wasm -o npm/labelize_wasm_bg.wasm
fi

SIZE=$(stat -f%z npm/labelize_wasm_bg.wasm 2>/dev/null || stat -c%s npm/labelize_wasm_bg.wasm)
echo "labelize_wasm_bg.wasm: $((SIZE / 1024)) KB"
if [ "$SIZE" -gt 10485760 ]; then
  echo "WARNING: wasm exceeds 10 MB — Cloudflare may reject it."
  echo "Remedies: image with default-features=false,features=[\"png\"] in wasm/Cargo.toml, or drop PDF."
  exit 1
fi
