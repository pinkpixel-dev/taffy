#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${ROOT_DIR}/target/debug/taffy"
APP_DIR="${HOME}/.local/share/applications"
ICON_BASE="${HOME}/.local/share/icons/hicolor"
DESKTOP_FILE="${APP_DIR}/taffy.desktop"

if [[ ! -x "${BIN_PATH}" ]]; then
  echo "Building Taffy first..."
  cargo build --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

mkdir -p "${APP_DIR}"
mkdir -p "${ICON_BASE}/256x256/apps"
mkdir -p "${ICON_BASE}/512x512/apps"

if command -v magick >/dev/null 2>&1; then
  magick "${ROOT_DIR}/icon.png" -resize 256x256 "${ICON_BASE}/256x256/apps/taffy.png"
  magick "${ROOT_DIR}/icon.png" -resize 512x512 "${ICON_BASE}/512x512/apps/taffy.png"
else
  install -Dm644 "${ROOT_DIR}/icon.png" "${ICON_BASE}/512x512/apps/taffy.png"
  install -Dm644 "${ROOT_DIR}/icon.png" "${ICON_BASE}/256x256/apps/taffy.png"
fi

cat > "${DESKTOP_FILE}" <<EOF
[Desktop Entry]
Type=Application
Name=Taffy
Comment=Simple screen capture for Wayland and COSMIC
Exec=${BIN_PATH}
Terminal=false
Icon=taffy
Categories=Utility;Graphics;
Keywords=screenshot;screen recording;gif;wayland;cosmic;
StartupWMClass=taffy
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed:"
echo "  Desktop entry: ${DESKTOP_FILE}"
echo "  Icon: ${ICON_BASE}/256x256/apps/taffy.png"
echo "  Icon: ${ICON_BASE}/512x512/apps/taffy.png"
