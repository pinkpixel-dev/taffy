#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${ROOT_DIR}/tools"
DIST_DIR="${ROOT_DIR}/target/dist"
APPDIR_PATH="${ROOT_DIR}/target/taffy.AppDir"
APPIMAGE_OUTPUT="${ROOT_DIR}/target/appimage/taffy.AppImage"
METADATA_SOURCE="${ROOT_DIR}/packaging/appimage/usr/share/metainfo/cargo-appimage.appdata.xml"

mkdir -p "${TOOLS_DIR}" "${DIST_DIR}"
cd "${ROOT_DIR}"

APPIMAGETOOL_BIN="${APPIMAGETOOL:-}"
if [[ -z "${APPIMAGETOOL_BIN}" ]] && command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL_BIN="$(command -v appimagetool)"
fi

if [[ -z "${APPIMAGETOOL_BIN}" ]]; then
  APPIMAGETOOL_BIN="${TOOLS_DIR}/appimagetool"
fi

if [[ ! -x "${APPIMAGETOOL_BIN}" ]]; then
  ARCH="$(uname -m)"
  URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH}.AppImage"

  echo "Downloading appimagetool from ${URL}..."
  curl -L "${URL}" -o "${APPIMAGETOOL_BIN}"
  chmod +x "${APPIMAGETOOL_BIN}"
fi

echo "Building AppImage..."
PATH="$(dirname "${APPIMAGETOOL_BIN}"):${PATH}" APPIMAGE_EXTRACT_AND_RUN=1 cargo-appimage

# cargo-appimage currently drops extra asset directories into the AppDir root.
# Move AppStream metadata into usr/share/metainfo, then re-run appimagetool so
# the final AppImage is built from the corrected layout.
mkdir -p "${APPDIR_PATH}/usr/share/metainfo"
install -Dm644 "${METADATA_SOURCE}" "${APPDIR_PATH}/usr/share/metainfo/cargo-appimage.appdata.xml"
rm -rf "${APPDIR_PATH}/metainfo" "${APPDIR_PATH}/appimage"

PATH="$(dirname "${APPIMAGETOOL_BIN}"):${PATH}" \
APPIMAGE_EXTRACT_AND_RUN=1 \
ARCH="$(uname -m)" \
VERSION="$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*\"version\":\"\\([^\"]*\\)\".*/\\1/p' | head -n 1)" \
"${APPIMAGETOOL_BIN}" --no-appstream "${APPDIR_PATH}" "${APPIMAGE_OUTPUT}"

find "${ROOT_DIR}/target/appimage" -maxdepth 1 -type f -name '*.AppImage' -exec cp -f {} "${DIST_DIR}/" \;

echo
echo "AppImage artifacts:"
find "${DIST_DIR}" -maxdepth 1 -type f -name '*.AppImage' | sort
