#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_NAME="labelize"
IMAGE_TAG="${PROJECT_NAME}:windows-build"
OUTPUT_DIR="${SCRIPT_DIR}/target/windows-release"

echo "==> Baue Docker-Image für Windows-Cross-Build..."
docker build -f "${SCRIPT_DIR}/Dockerfile.windows" -t "${IMAGE_TAG}" "${SCRIPT_DIR}"

echo "==> Extrahiere fertige Windows-EXE..."
mkdir -p "${OUTPUT_DIR}"
docker run --rm -v "${OUTPUT_DIR}:/out" "${IMAGE_TAG}" cp /output/labelize.exe /out/labelize.exe

echo "==> Fertig: ${OUTPUT_DIR}/labelize.exe"
ls -lh "${OUTPUT_DIR}/labelize.exe"
file "${OUTPUT_DIR}/labelize.exe" || true
