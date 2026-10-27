#!/usr/bin/env bash
set -euo pipefail

# E2E tests for the labelize HTTP microservice installed via Homebrew.
# Expects: labelize is on PATH, e2e/testdata/sample.zpl exists.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TESTDATA_DIR="$SCRIPT_DIR/../testdata"
OUTPUT_DIR="$SCRIPT_DIR/../http-output"
WORK_DIR="$(mktemp -d)"
trap 'cleanup' EXIT

PORT=18199
SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK_DIR"
}

mkdir -p "$OUTPUT_DIR"

ZPL_DATA=$(cat "$TESTDATA_DIR/sample.zpl")

pass=0
fail=0

assert_ok() {
  local desc="$1"; shift
  if "$@"; then
    echo "  ✓ $desc"
    pass=$((pass + 1))
  else
    echo "  ✗ $desc (exit $?)"
    fail=$((fail + 1))
  fi
}

assert_eq() {
  local desc="$1" expected="$2" actual="$3"
  if [[ "$expected" == "$actual" ]]; then
    echo "  ✓ $desc"
    pass=$((pass + 1))
  else
    echo "  ✗ $desc (expected '$expected', got '$actual')"
    fail=$((fail + 1))
  fi
}

assert_file_min_size() {
  local desc="$1" file="$2" min_bytes="$3"
  local size
  size=$(wc -c < "$file" | tr -d ' ')
  if [[ "$size" -ge "$min_bytes" ]]; then
    echo "  ✓ $desc ($size bytes)"
    pass=$((pass + 1))
  else
    echo "  ✗ $desc (expected >= $min_bytes bytes, got $size)"
    fail=$((fail + 1))
  fi
}

echo "=== Labelize HTTP E2E Tests ==="
echo ""

# Start server in background
echo "[0] Starting server on port $PORT..."
labelize serve --port "$PORT" &
SERVER_PID=$!

# Wait for server to be ready (up to 10 seconds)
for i in $(seq 1 20); do
  if curl -sf "http://127.0.0.1:$PORT/health" > /dev/null 2>&1; then
    echo "  ✓ Server started (attempt $i)"
    break
  fi
  if [[ "$i" -eq 20 ]]; then
    echo "  ✗ Server failed to start within 10 seconds"
    exit 1
  fi
  sleep 0.5
done

# 1. Health check
echo "[1] Health endpoint"
HEALTH_BODY=$(curl -sf "http://127.0.0.1:$PORT/health")
assert_eq "health body" '{"status":"ok"}' "$HEALTH_BODY"

HEALTH_CT=$(curl -sf -o /dev/null -w '%{content_type}' "http://127.0.0.1:$PORT/health")
assert_eq "health content-type" "application/json" "$HEALTH_CT"

# 2. Convert ZPL → PNG
echo "[2] POST /convert ZPL → PNG"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/http_label.png" -w '%{http_code}' \
  -X POST "http://127.0.0.1:$PORT/convert" \
  -H "Content-Type: application/zpl" \
  -d "$ZPL_DATA")
assert_eq "HTTP status 200" "200" "$HTTP_CODE"
assert_file_min_size "response PNG has content" "$WORK_DIR/http_label.png" 500
cp "$WORK_DIR/http_label.png" "$OUTPUT_DIR/sample.png" 2>/dev/null || true

PNG_CT=$(curl -sf -o /dev/null -w '%{content_type}' \
  -X POST "http://127.0.0.1:$PORT/convert" \
  -H "Content-Type: application/zpl" \
  -d "$ZPL_DATA")
assert_eq "response content-type is image/png" "image/png" "$PNG_CT"

# 3. Convert ZPL → PDF
echo "[3] POST /convert ZPL → PDF"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/http_label.pdf" -w '%{http_code}' \
  -X POST "http://127.0.0.1:$PORT/convert?output=pdf" \
  -H "Content-Type: application/zpl" \
  -d "$ZPL_DATA")
assert_eq "HTTP status 200" "200" "$HTTP_CODE"
assert_file_min_size "response PDF has content" "$WORK_DIR/http_label.pdf" 500
cp "$WORK_DIR/http_label.pdf" "$OUTPUT_DIR/sample.pdf" 2>/dev/null || true

PDF_CT=$(curl -sf -o /dev/null -w '%{content_type}' \
  -X POST "http://127.0.0.1:$PORT/convert?output=pdf" \
  -H "Content-Type: application/zpl" \
  -d "$ZPL_DATA")
assert_eq "response content-type is application/pdf" "application/pdf" "$PDF_CT"

# 4. Custom dimensions
echo "[4] POST /convert with custom dimensions"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/custom.png" -w '%{http_code}' \
  -X POST "http://127.0.0.1:$PORT/convert?width=100&height=62&dpmm=12" \
  -H "Content-Type: application/zpl" \
  -d "$ZPL_DATA")
assert_eq "HTTP status 200 with custom dims" "200" "$HTTP_CODE"
assert_file_min_size "custom-dim PNG has content" "$WORK_DIR/custom.png" 200
cp "$WORK_DIR/custom.png" "$OUTPUT_DIR/sample-custom.png" 2>/dev/null || true

# 5. Bad request (empty body)
echo "[5] POST /convert with empty body"
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' \
  -X POST "http://127.0.0.1:$PORT/convert" \
  -H "Content-Type: application/zpl" \
  -d "")
assert_eq "empty body returns 400" "400" "$HTTP_CODE"

# 6. Playground page
echo "[6] GET / playground page"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/playground.html" -w '%{http_code}' \
  "http://127.0.0.1:$PORT/")
assert_eq "GET / returns 200" "200" "$HTTP_CODE"

PLAYGROUND_CT=$(curl -sf -o /dev/null -w '%{content_type}' \
  "http://127.0.0.1:$PORT/")
# content_type may include charset suffix, so check with grep
echo "$PLAYGROUND_CT" | grep -qi "text/html" \
  && { echo "  ✓ GET / content-type is text/html"; pass=$((pass + 1)); } \
  || { echo "  ✗ GET / content-type is text/html (got '$PLAYGROUND_CT')"; fail=$((fail + 1)); }

# body must contain the HTML doctype / root element
grep -qi "<html" "$WORK_DIR/playground.html" \
  && { echo "  ✓ GET / body contains <html>"; pass=$((pass + 1)); } \
  || { echo "  ✗ GET / body missing <html>"; fail=$((fail + 1)); }

assert_file_min_size "GET / response is non-trivially large" "$WORK_DIR/playground.html" 2000

# 8. TSPL body (Content-Type names the language; SIZE drives the canvas)
echo "[8] TSPL conversion"
TSPL_BODY=$'SIZE 76 mm,40 mm\nDIRECTION 1\nCLS\nTEXT 20,20,"3",0,1,1,"TSPL HTTP"\nPRINT 1'
HTTP_CODE=$(curl -sf -o "$WORK_DIR/tspl.png" -w "%{http_code}" -X POST \
  -H "Content-Type: text/tspl" \
  --data-binary "$TSPL_BODY" \
  "http://127.0.0.1:$PORT/convert?dpmm=8")
assert_eq "TSPL POST status 200" "200" "$HTTP_CODE"
assert_file_min_size "TSPL response PNG has content" "$WORK_DIR/tspl.png" 300

# 9. CPCL body (Content-Type names the language)
echo "[9] CPCL conversion"
CPCL_BODY=$'! 0 203 203 200 1\r\nPAGE-WIDTH 400\r\nTEXT 4 0 20 30 CPCL HTTP\r\nBARCODE 128 2 1 60 30 100 8412345678\r\nFORM\r\nPRINT\r\n'
HTTP_CODE=$(curl -sf -o "$WORK_DIR/cpcl.png" -w "%{http_code}" -X POST \
  -H "Content-Type: application/cpcl" \
  --data-binary "$CPCL_BODY" \
  "http://127.0.0.1:$PORT/convert?dpmm=8")
assert_eq "CPCL POST status 200" "200" "$HTTP_CODE"
assert_file_min_size "CPCL response PNG has content" "$WORK_DIR/cpcl.png" 300

# 10. DPL body (Content-Type names the language)
echo "[10] DPL conversion"
printf '\x02L\r191100801200100DPL HTTP\r1E21040015002401234567890\rQ0001\rE\r' > "$WORK_DIR/label.dpl"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/dpl.png" -w "%{http_code}" -X POST \
  -H "Content-Type: application/dpl" \
  --data-binary "@$WORK_DIR/label.dpl" \
  "http://127.0.0.1:$PORT/convert?dpmm=8")
assert_eq "DPL POST status 200" "200" "$HTTP_CODE"
assert_file_min_size "DPL response PNG has content" "$WORK_DIR/dpl.png" 300

# 11. Reverse rendering: image → ZPL, then ZPL → PNG (closed loop through
# the render produced by step 2, so no extra fixture is needed)
echo "[11] POST /encode image → ZPL"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/roundtrip.zpl" -w '%{http_code}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@$WORK_DIR/http_label.png" \
  "http://127.0.0.1:$PORT/encode?dpmm=8&language=zpl")
assert_eq "encode ZPL status 200" "200" "$HTTP_CODE"
grep -q '\^GFA' "$WORK_DIR/roundtrip.zpl" \
  && { echo "  ✓ encode output is a ^GFA graphic field"; pass=$((pass + 1)); } \
  || { echo "  ✗ encode output is a ^GFA graphic field"; fail=$((fail + 1)); }

ZPL_CT=$(curl -sf -o /dev/null -w '%{content_type}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@$WORK_DIR/http_label.png" \
  "http://127.0.0.1:$PORT/encode?dpmm=8&language=zpl")
echo "$ZPL_CT" | grep -qi "text/plain" \
  && { echo "  ✓ encode ZPL content-type is text/plain"; pass=$((pass + 1)); } \
  || { echo "  ✗ encode ZPL content-type is text/plain (got '$ZPL_CT')"; fail=$((fail + 1)); }

# Generated ZPL leaves the size unset: render at the source image's dims (816×1216
# @ 8 dpmm = the 102×152 default used in step 2).
HTTP_CODE=$(curl -sf -o "$WORK_DIR/roundtrip.png" -w '%{http_code}' -X POST \
  -H "Content-Type: application/zpl" \
  --data-binary "@$WORK_DIR/roundtrip.zpl" \
  "http://127.0.0.1:$PORT/convert?width=102&height=152&dpmm=8")
assert_eq "encode→convert round trip status 200" "200" "$HTTP_CODE"
assert_file_min_size "round-trip PNG has content" "$WORK_DIR/roundtrip.png" 500
cmp -s "$WORK_DIR/http_label.png" "$WORK_DIR/roundtrip.png" \
  && { echo "  ✓ round trip is byte-identical"; pass=$((pass + 1)); } \
  || { echo "  ✗ round trip is byte-identical"; fail=$((fail + 1)); }

# 12. Reverse rendering: image → TSPL (binary octet-stream + declared SIZE)
echo "[12] POST /encode image → TSPL"
HTTP_CODE=$(curl -sf -o "$WORK_DIR/roundtrip.tspl" -w '%{http_code}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@$WORK_DIR/http_label.png" \
  "http://127.0.0.1:$PORT/encode?language=tspl")
assert_eq "encode TSPL status 200" "200" "$HTTP_CODE"

TSPL_CT=$(curl -sf -o /dev/null -w '%{content_type}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@$WORK_DIR/http_label.png" \
  "http://127.0.0.1:$PORT/encode?language=tspl")
assert_eq "encode TSPL content-type is octet-stream" "application/octet-stream" "$TSPL_CT"

head -c 5 "$WORK_DIR/roundtrip.tspl" | grep -q '^SIZE' \
  && { echo "  ✓ TSPL output starts with SIZE"; pass=$((pass + 1)); } \
  || { echo "  ✗ TSPL output starts with SIZE"; fail=$((fail + 1)); }

# The generated SIZE declares the canvas, so convert needs no dimensions.
HTTP_CODE=$(curl -sf -o "$WORK_DIR/roundtrip_tspl.png" -w '%{http_code}' -X POST \
  -H "Content-Type: text/tspl" \
  --data-binary "@$WORK_DIR/roundtrip.tspl" \
  "http://127.0.0.1:$PORT/convert?dpmm=8")
assert_eq "TSPL encode→convert status 200" "200" "$HTTP_CODE"
cmp -s "$WORK_DIR/http_label.png" "$WORK_DIR/roundtrip_tspl.png" \
  && { echo "  ✓ TSPL round trip is byte-identical"; pass=$((pass + 1)); } \
  || { echo "  ✗ TSPL round trip is byte-identical"; fail=$((fail + 1)); }

# 13. /encode rejects bad input
echo "[13] POST /encode rejects bad input"
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "definitely not an image" \
  "http://127.0.0.1:$PORT/encode")
assert_eq "garbage image returns 400" "400" "$HTTP_CODE"

HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@$WORK_DIR/http_label.png" \
  "http://127.0.0.1:$PORT/encode?language=epl")
assert_eq "unknown language returns 400" "400" "$HTTP_CODE"

echo ""
echo "=== Results: $pass passed, $fail failed ==="
[[ "$fail" -eq 0 ]] || exit 1
