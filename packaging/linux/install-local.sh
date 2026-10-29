#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.Franck-Demongin.Combinator"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
BIN_SOURCE="${1:-$REPO_ROOT/target/release/combinator-gui}"
HOME_DIR="${HOME:?HOME is required}"
LOCAL_BIN="$HOME_DIR/.local/bin"
APPLICATIONS="$HOME_DIR/.local/share/applications"
ICONS="$HOME_DIR/.local/share/icons/hicolor"
DESKTOP_SOURCE="$REPO_ROOT/packaging/linux/$APP_ID.desktop"
INDEX_SOURCE="$REPO_ROOT/packaging/linux/hicolor.index.theme"
SYSTEM_INDEX="/usr/share/icons/hicolor/index.theme"

if [[ ! -x "$BIN_SOURCE" ]]; then
    echo "binary not found or not executable: $BIN_SOURCE" >&2
    exit 1
fi

mkdir -p "$LOCAL_BIN" "$APPLICATIONS" "$ICONS"/{16x16,32x32,48x48,64x64,128x128,256x256,512x512,scalable}/apps

install -m 0755 "$BIN_SOURCE" "$LOCAL_BIN/combinator-gui"
install -m 0644 "$DESKTOP_SOURCE" "$APPLICATIONS/$APP_ID.desktop"

for size in 16 32 48 64 128 256 512; do
    install -m 0644 "$REPO_ROOT/assets/icons/combinator-${size}.png" \
        "$ICONS/${size}x${size}/apps/$APP_ID.png"
done
install -m 0644 "$REPO_ROOT/assets/icons/combinator.svg" \
    "$ICONS/scalable/apps/$APP_ID.svg"

if [[ -f "$INDEX_SOURCE" ]]; then
    install -m 0644 "$INDEX_SOURCE" "$ICONS/index.theme"
elif [[ -f "$SYSTEM_INDEX" ]]; then
    install -m 0644 "$SYSTEM_INDEX" "$ICONS/index.theme"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPLICATIONS" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$ICONS" || true
fi

echo "Installed Combinator GUI to $LOCAL_BIN/combinator-gui"