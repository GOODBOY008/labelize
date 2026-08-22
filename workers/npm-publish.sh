#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Version comes from the release tag (vX.Y.Z); GITHUB_REF_NAME is set by Actions.
REF_NAME="${GITHUB_REF_NAME:-}"
if [[ "$REF_NAME" != v* ]]; then
  echo "error: expected a vX.Y.Z tag in GITHUB_REF_NAME, got '$REF_NAME'" >&2
  exit 1
fi
VERSION="${REF_NAME#v}"

cd npm

node -e "
  const fs = require('fs');
  const p = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  p.version = '$VERSION';
  fs.writeFileSync('package.json', JSON.stringify(p, null, 2) + '\n');
"

# Sanity: the packaged artifacts must exist (build.sh stages them).
for f in labelize_wasm.js labelize_wasm_bg.js labelize_wasm_bg.wasm; do
  if [[ ! -f "$f" ]]; then
    echo "error: missing $f — run ./build.sh first" >&2
    exit 1
  fi
done

npm publish --access public