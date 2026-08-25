#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-unknown-unknown
rustup target add $TARGET

# labelize-wasm is its own crate root (no workspace), so build from inside it.
cd labelize-wasm
cargo build --release --target $TARGET

# Generate the JS glue for the worker; --out-dir .. places both
# labelize_wasm.js and labelize_wasm_bg.wasm next to worker.js.
wasm-bindgen --target bundler --out-dir .. --out-name labelize_wasm \
  target/$TARGET/release/labelize_wasm.wasm

cd ..

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz labelize_wasm_bg.wasm -o labelize_wasm_bg.wasm
fi

# Stage the npm package artifacts next to the static package files.
mkdir -p npm
cp labelize_wasm.js labelize_wasm_bg.js labelize_wasm_bg.wasm \
   labelize_wasm.d.ts labelize_wasm_bg.wasm.d.ts npm/

SIZE=$(stat -f%z labelize_wasm_bg.wasm 2>/dev/null || stat -c%s labelize_wasm_bg.wasm)
echo "labelize_wasm_bg.wasm: $((SIZE / 1024)) KB"
if [ "$SIZE" -gt 10485760 ]; then
  echo "WARNING: wasm exceeds 10 MB — Cloudflare may reject it."
  echo "Remedies: image with default-features=false,features=[\"png\"] in labelize-wasm/Cargo.toml, or drop PDF."
  exit 1
fi