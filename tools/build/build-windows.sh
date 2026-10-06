#!/bin/bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE_TAG="labelize:windows-build"
OUTPUT_DIR="${PROJECT_ROOT}/target/windows-release"

echo "==> Building Docker image for Windows cross-compilation..."
docker build -f "${PROJECT_ROOT}/tools/build/Dockerfile.windows" -t "${IMAGE_TAG}" "${PROJECT_ROOT}"

echo "==> Extracting the finished Windows executable..."
mkdir -p "${OUTPUT_DIR}"
docker run --rm -v "${OUTPUT_DIR}:/out" "${IMAGE_TAG}" cp /output/labelize.exe /out/labelize.exe

echo "==> Done: ${OUTPUT_DIR}/labelize.exe"
ls -lh "${OUTPUT_DIR}/labelize.exe"
file "${OUTPUT_DIR}/labelize.exe" || true
