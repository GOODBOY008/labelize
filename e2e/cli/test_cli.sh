#!/usr/bin/env bash
set -euo pipefail

# E2E tests for the labelize CLI installed via Homebrew.
# Expects: labelize is on PATH, e2e/testdata/sample.zpl exists.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TESTDATA_DIR="$SCRIPT_DIR/../testdata"
OUTPUT_DIR="$SCRIPT_DIR/../cli-output"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

mkdir -p "$OUTPUT_DIR"

ZPL_FILE="$TESTDATA_DIR/sample.zpl"

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

assert_file_header() {
  local desc="$1" file="$2" expected_hex="$3"
  local actual_hex
  actual_hex=$(xxd -p -l "${#expected_hex}" "$file" | tr -d '\n' | head -c "${#expected_hex}")
  # Compare case-insensitively (compatible with Bash 3)
  if [[ "$(echo "$actual_hex" | tr '[:upper:]' '[:lower:]')" == "$(echo "$expected_hex" | tr '[:upper:]' '[:lower:]')" ]]; then
    echo "  ✓ $desc"
    pass=$((pass + 1))
  else
    echo "  ✗ $desc (expected header $expected_hex, got $actual_hex)"
    fail=$((fail + 1))
  fi
}

echo "=== Labelize CLI E2E Tests ==="
echo ""

# 1. Version check
echo "[1] Version"
assert_ok "labelize --version exits 0" labelize --version

# 2. Convert ZPL → PNG (default)
echo "[2] Convert ZPL → PNG"
cp "$ZPL_FILE" "$WORK_DIR/label.zpl"
assert_ok "convert exits 0" labelize convert "$WORK_DIR/label.zpl"
assert_file_min_size "output PNG has content" "$WORK_DIR/label.png" 500
assert_file_header "output is valid PNG" "$WORK_DIR/label.png" "89504e47"
cp "$WORK_DIR/label.png" "$OUTPUT_DIR/sample.png" 2>/dev/null || true

# 3. Convert ZPL → PNG with explicit output path
echo "[3] Convert ZPL → PNG (explicit output)"
assert_ok "convert with -o exits 0" labelize convert "$ZPL_FILE" -o "$WORK_DIR/explicit.png"
assert_file_min_size "explicit output PNG has content" "$WORK_DIR/explicit.png" 500
cp "$WORK_DIR/explicit.png" "$OUTPUT_DIR/explicit.png" 2>/dev/null || true

# 4. Convert ZPL → PDF
echo "[4] Convert ZPL → PDF"
assert_ok "convert to PDF exits 0" labelize convert "$ZPL_FILE" -t pdf -o "$WORK_DIR/label.pdf"
assert_file_min_size "output PDF has content" "$WORK_DIR/label.pdf" 500
assert_file_header "output is valid PDF" "$WORK_DIR/label.pdf" "25504446"
cp "$WORK_DIR/label.pdf" "$OUTPUT_DIR/sample.pdf" 2>/dev/null || true

# 5. Convert with custom dimensions
echo "[5] Convert with custom dimensions"
assert_ok "convert with dimensions exits 0" labelize convert "$ZPL_FILE" -o "$WORK_DIR/custom.png" --width 100 --height 62 --dpmm 12
assert_file_min_size "custom dimension PNG has content" "$WORK_DIR/custom.png" 200
cp "$WORK_DIR/custom.png" "$OUTPUT_DIR/sample-custom.png" 2>/dev/null || true

# 6. Format override
echo "[6] Format override"
cp "$ZPL_FILE" "$WORK_DIR/label.txt"
assert_ok "convert with -f zpl exits 0" labelize convert "$WORK_DIR/label.txt" -f zpl -o "$WORK_DIR/override.png"
assert_file_min_size "format-override PNG has content" "$WORK_DIR/override.png" 500
cp "$WORK_DIR/override.png" "$OUTPUT_DIR/override.png" 2>/dev/null || true

# 7. TSPL conversion (SIZE drives the canvas; no --width/--height needed)
echo "[7] TSPL conversion"
TSPL_FILE="$TESTDATA_DIR/sample.tspl"
assert_ok "convert .tspl exits 0" labelize convert "$TSPL_FILE" -o "$WORK_DIR/tspl.png"
assert_file_min_size "TSPL PNG has content" "$WORK_DIR/tspl.png" 300
cp "$WORK_DIR/tspl.png" "$OUTPUT_DIR/tspl.png" 2>/dev/null || true

# 8. TSPL format override on an unknown extension
echo "[8] TSPL format override"
cp "$TSPL_FILE" "$WORK_DIR/tspl_label.txt"
assert_ok "convert with -f tspl exits 0" labelize convert "$WORK_DIR/tspl_label.txt" -f tspl -o "$WORK_DIR/tspl_override.png"
assert_file_min_size "TSPL override PNG has content" "$WORK_DIR/tspl_override.png" 300

echo ""
echo "=== Results: $pass passed, $fail failed ==="
[[ "$fail" -eq 0 ]] || exit 1
