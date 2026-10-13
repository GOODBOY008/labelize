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

# Idempotency: a re-triggered release run (e.g. after a failed downstream
# job) must not collide with an already-published version — npm rejects
# duplicate versions with a 403. Skip quietly when the tag's version is live.
PUBLISHED="$(npm view "@goodboy008/labelize-wasm@$VERSION" version 2>/dev/null || true)"
if [[ "$PUBLISHED" == "$VERSION" ]]; then
  echo "npm: @goodboy008/labelize-wasm@$VERSION already published — skipping"
  exit 0
fi

# Sanity: the packaged artifacts must exist (build.sh stages them).
for f in labelize_wasm.js labelize_wasm_bg.js labelize_wasm_bg.wasm; do
  if [[ ! -f "$f" ]]; then
    echo "error: missing $f — run ./build.sh first" >&2
    exit 1
  fi
done

npm publish --access public