#!/usr/bin/env bash
# Install a desktop entry for sml2 so Wayland shells show the coin icon.
#
# Wayland has no per-window icon: GNOME, KDE and WSLg look up the window's
# app ID (`sml2`) in a .desktop file instead. WSLg only reads system folders,
# so there use: sudo tools/install-desktop-entry.sh BINARY /usr/local
#
# Usage: tools/install-desktop-entry.sh [BINARY] [PREFIX]
#   BINARY defaults to target/release/sml2, PREFIX to ~/.local.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
binary=$(realpath "${1:-$root/target/release/sml2}")
prefix=${2:-$HOME/.local}
icon=$prefix/share/icons/hicolor/256x256/apps/sml2.png
entry=$prefix/share/applications/sml2.desktop

[[ -x $binary ]] || { echo "not an executable: $binary" >&2; exit 1; }

install -Dm644 "$root/assets/icons/sml2-256.png" "$icon"
mkdir -p "$(dirname "$entry")"
cat >"$entry" <<EOF
[Desktop Entry]
Type=Application
Name=Super Mario Land 2
Comment=Native port of Super Mario Land 2
Exec=$binary
Icon=$icon
Terminal=false
Categories=Game;
StartupWMClass=sml2
EOF
chmod 644 "$entry"
echo "installed $entry"
