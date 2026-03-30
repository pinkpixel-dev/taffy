#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/target/dist"

cd "${ROOT_DIR}"

mkdir -p "${DIST_DIR}"

echo "Building release binary..."
cargo build --release

echo "Building .deb package..."
cargo deb --no-build --output "${DIST_DIR}"

echo "Building .rpm package..."
cargo-generate-rpm --output "${DIST_DIR}"

echo "Building AppImage..."
"${ROOT_DIR}/scripts/build-appimage.sh"

echo
echo "Artifacts:"
find "${DIST_DIR}" -maxdepth 1 -type f | sort
