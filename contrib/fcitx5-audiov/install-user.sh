#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${BUILD_DIR:-/tmp/audiov-fcitx5-user-build}"
PREFIX="${PREFIX:-$HOME/.local}"
ADDON_CONF_DIR="$PREFIX/share/fcitx5/addon"
ADDON_LIB="$PREFIX/lib/fcitx5/libaudiovfcitx5"

cmake -S "$ROOT_DIR" -B "$BUILD_DIR" -DCMAKE_INSTALL_PREFIX="$PREFIX"
cmake --build "$BUILD_DIR"
cmake --install "$BUILD_DIR"

CONF_FILE="$ADDON_CONF_DIR/audiovfcitx5.conf"
if [[ -f "$CONF_FILE" ]]; then
  python - "$CONF_FILE" "$ADDON_LIB" <<'PY'
from pathlib import Path
import sys

conf_path = Path(sys.argv[1])
lib_path = sys.argv[2]
text = conf_path.read_text()
text = text.replace("Library=libaudiovfcitx5", f"Library={lib_path}")
conf_path.write_text(text)
PY
fi

echo "Installed fcitx5 addon to $PREFIX"
echo "Library path pinned to $ADDON_LIB"
