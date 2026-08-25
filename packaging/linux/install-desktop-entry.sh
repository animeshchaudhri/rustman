#!/usr/bin/env sh
# Installs Rustman's desktop entry and icons for the current user.
#
# Why this exists: the Linux release ships a bare `rustman` binary in a
# tarball. A binary on its own has no icon anywhere in the desktop environment
# — Wayland compositors look the icon up by matching the window's app id
# against an installed `.desktop` file, and X11 desktops need one for the
# launcher/dock entry. Without these files Rustman shows the generic
# placeholder icon in the dock, task switcher and application menu.
#
# Usage (from the extracted tarball, alongside the `rustman` binary):
#   ./install-desktop-entry.sh
#
# Removal:
#   ./install-desktop-entry.sh --uninstall

set -eu

APP_ID="io.github.animeshchaudhri.rustman"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_HOME="$HOME/.local/bin"
DESKTOP_DIR="$DATA_HOME/applications"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$DESKTOP_DIR/$APP_ID.desktop"
    # Remove the icon at every size it may have been installed at.
    find "$DATA_HOME/icons/hicolor" -name "$APP_ID.png" -delete 2>/dev/null || true
    command -v update-desktop-database >/dev/null 2>&1 &&
        update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 &&
        gtk-update-icon-cache -f -t "$DATA_HOME/icons/hicolor" 2>/dev/null || true
    echo "Removed Rustman desktop entry and icons."
    exit 0
fi

# 1. The binary must be on PATH for Exec=/TryExec= to resolve.
if [ -f "$SCRIPT_DIR/rustman" ]; then
    mkdir -p "$BIN_HOME"
    install -m 0755 "$SCRIPT_DIR/rustman" "$BIN_HOME/rustman"
    echo "Installed binary  -> $BIN_HOME/rustman"
    case ":$PATH:" in
        *":$BIN_HOME:"*) ;;
        *) echo "NOTE: $BIN_HOME is not on your PATH; add it to use 'rustman' from a shell." ;;
    esac
fi

# 2. Icons, at each size the hicolor theme expects. The icon *basename must
#    equal the app id*, which is what lets a Wayland compositor connect the
#    running window to this icon.
ICON_SRC="$SCRIPT_DIR/icon.png"
if [ -f "$ICON_SRC" ]; then
    installed_any=0
    for size in 16 24 32 48 64 128 256; do
        dir="$DATA_HOME/icons/hicolor/${size}x${size}/apps"
        mkdir -p "$dir"
        if command -v magick >/dev/null 2>&1; then
            magick "$ICON_SRC" -resize "${size}x${size}" "$dir/$APP_ID.png" && installed_any=1
        elif command -v convert >/dev/null 2>&1; then
            convert "$ICON_SRC" -resize "${size}x${size}" "$dir/$APP_ID.png" && installed_any=1
        fi
    done
    if [ "$installed_any" -eq 0 ]; then
        # No ImageMagick: install the source PNG unscaled. Desktops downscale
        # as needed, so this still beats having no icon.
        dir="$DATA_HOME/icons/hicolor/256x256/apps"
        mkdir -p "$dir"
        install -m 0644 "$ICON_SRC" "$dir/$APP_ID.png"
    fi
    echo "Installed icons   -> $DATA_HOME/icons/hicolor/*/apps/$APP_ID.png"
else
    echo "WARNING: icon.png not found next to this script; skipping icons." >&2
fi

# 3. The desktop entry itself.
mkdir -p "$DESKTOP_DIR"
if [ -f "$SCRIPT_DIR/$APP_ID.desktop" ]; then
    install -m 0644 "$SCRIPT_DIR/$APP_ID.desktop" "$DESKTOP_DIR/$APP_ID.desktop"
else
    cat > "$DESKTOP_DIR/$APP_ID.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Rustman
GenericName=API Client
Comment=Native API testing tool built with Rust and iced
Exec=rustman %U
TryExec=rustman
Icon=$APP_ID
Terminal=false
Categories=Development;
Keywords=API;REST;HTTP;WebSocket;testing;client;curl;
StartupNotify=true
StartupWMClass=$APP_ID
DESKTOP
fi
echo "Installed launcher -> $DESKTOP_DIR/$APP_ID.desktop"

command -v update-desktop-database >/dev/null 2>&1 &&
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 &&
    gtk-update-icon-cache -f -t "$DATA_HOME/icons/hicolor" 2>/dev/null || true

echo
echo "Done. Rustman should now appear in your application menu with its icon."
echo "If the icon does not update immediately, log out and back in."
