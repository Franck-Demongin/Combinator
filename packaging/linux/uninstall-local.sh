#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.Franck-Demongin.Combinator"
HOME_DIR="${HOME:?HOME is required}"
LOCAL_BIN="$HOME_DIR/.local/bin"
APPLICATIONS="$HOME_DIR/.local/share/applications"
ICONS="$HOME_DIR/.local/share/icons/hicolor"

rm -f "$LOCAL_BIN/combinator-gui"
rm -f "$APPLICATIONS/$APP_ID.desktop"
for size in 16 32 48 64 128 256 512; do
    rm -f "$ICONS/${size}x${size}/apps/$APP_ID.png"
done
rm -f "$ICONS/scalable/apps/$APP_ID.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPLICATIONS" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$ICONS" || true
fi

echo "Uninstalled Combinator GUI"
EOF