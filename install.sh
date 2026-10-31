#!/usr/bin/env bash
# Installs Torosu on Linux for the current user, ready to open from the app
# launcher: the latest AppImage from GitHub, a launcher entry, its icon and a
# `torosu` command. The AppImage updates itself from the app.
#
#   curl -fsSL https://raw.githubusercontent.com/Manolo-dl/Torosu/main/install.sh | bash
#   bash install.sh --uninstall
#
# Nothing goes outside your home folder, except what it asks for first: fuse2
# on Arch, and the permission for the Trainer to read osu!'s memory.

set -euo pipefail

REPO="Manolo-dl/Torosu"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/torosu"
APPIMAGE="$APP_DIR/torosu.AppImage"
DESKTOP_FILE="${XDG_DATA_HOME:-$HOME/.local/share}/applications/torosu.desktop"
ICON_FILE="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps/torosu.png"
BIN_LINK="$HOME/.local/bin/torosu"
PTRACE_CONF="/etc/sysctl.d/60-torosu-ptrace.conf"
# Where Torosu keeps its settings (see `app_config_dir` in src-tauri/src/lib.rs).
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/com.manolo.torosu"

info() { printf '\033[1;35m::\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31mxx\033[0m %s\n' "$*" >&2; exit 1; }

# Questions read the terminal, also when the script comes through a pipe.
ask() {
  local answer
  if [ -r /dev/tty ]; then
    printf '%s [y/N] ' "$1" > /dev/tty
    read -r answer < /dev/tty || answer=""
  else
    answer=""
  fi
  [[ "$answer" =~ ^[Yy]$ ]]
}

uninstall() {
  info "Removing Torosu"
  rm -f "$APPIMAGE" "$DESKTOP_FILE" "$ICON_FILE"
  [ -L "$BIN_LINK" ] && rm -f "$BIN_LINK"
  command -v update-desktop-database > /dev/null && update-desktop-database -q "$(dirname "$DESKTOP_FILE")" || true
  if [ -f "$PTRACE_CONF" ] && ask "Also remove the permission to read osu!'s memory ($PTRACE_CONF)?"; then
    sudo rm -f "$PTRACE_CONF"
  fi
  # The settings stay, in case of reinstalling.
  info "Done. Your settings are still in $CONFIG_DIR; delete that folder to remove them too."
  exit 0
}

[ "${1:-}" = "--uninstall" ] && uninstall

[ "$(uname -s)" = "Linux" ] || fail "This script is for Linux. On Windows, use the installer from the releases page."
[ "$(uname -m)" = "x86_64" ] || fail "Torosu is only built for x86_64."
command -v curl > /dev/null || fail "curl is needed."

# AppImages need FUSE 2.
if ! ldconfig -p 2> /dev/null | grep -q 'libfuse\.so\.2'; then
  if command -v pacman > /dev/null; then
    if ask "AppImages need fuse2, which is not installed. Install it with pacman?"; then
      sudo pacman -S --needed --noconfirm fuse2
    else
      warn "Without fuse2 Torosu won't start. Install it with: sudo pacman -S fuse2"
    fi
  else
    warn "AppImages need FUSE 2 (libfuse2), which seems to be missing. Install it with your package manager."
  fi
fi

info "Looking for the latest release"
release=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest") || fail "Could not reach GitHub."
url=$(printf '%s' "$release" | grep -o '"browser_download_url": *"[^"]*_amd64\.AppImage"' | head -n 1 | sed 's/.*"\(https[^"]*\)"/\1/')
version=$(printf '%s' "$release" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | sed 's/.*"\([^"]*\)"$/\1/')
[ -n "$url" ] || fail "The latest release has no AppImage."

info "Downloading Torosu $version"
mkdir -p "$APP_DIR"
curl -fL --progress-bar "$url" -o "$APPIMAGE.part"
chmod +x "$APPIMAGE.part"
mv "$APPIMAGE.part" "$APPIMAGE"

info "Adding it to the app launcher"
mkdir -p "$(dirname "$ICON_FILE")" "$(dirname "$DESKTOP_FILE")" "$(dirname "$BIN_LINK")"
curl -fsSL "https://raw.githubusercontent.com/$REPO/main/src-tauri/icons/128x128@2x.png" -o "$ICON_FILE" \
  || warn "Could not download the icon; the launcher entry will have a generic one."
cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Type=Application
Name=Torosu
Comment=osu! stable tools in one app
Exec="$APPIMAGE"
Icon=torosu
Terminal=false
Categories=Game;Utility;
Keywords=osu;beatmap;mania;
StartupWMClass=torosu
EOF
ln -sf "$APPIMAGE" "$BIN_LINK"
command -v update-desktop-database > /dev/null && update-desktop-database -q "$(dirname "$DESKTOP_FILE")" || true
command -v gtk-update-icon-cache > /dev/null && gtk-update-icon-cache -q -t "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" 2> /dev/null || true

# The Trainer reads the selected map from osu!'s memory. With the usual
# ptrace_scope of 1, only a program with the permission can, and an AppImage
# can't keep one; ptrace_scope 0 allows it for every program.
scope=$(cat /proc/sys/kernel/yama/ptrace_scope 2> /dev/null || echo 0)
if [ "$scope" != "0" ] && [ ! -f "$PTRACE_CONF" ]; then
  echo
  info "The Trainer reads the selected map from osu!'s memory, which your system only lets programs with a permission do."
  if ask "Allow it from now on? (sets kernel.yama.ptrace_scope=0 in $PTRACE_CONF, for every program)"; then
    echo "kernel.yama.ptrace_scope = 0" | sudo tee "$PTRACE_CONF" > /dev/null
    sudo sysctl -q --system
  else
    warn "Skipped. If the Trainer can't read osu!, run: echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope"
  fi
fi

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *) warn "$HOME/.local/bin is not in your PATH: the torosu command won't work, the launcher entry will." ;;
esac

echo
info "Torosu $version is installed. Open it from the app launcher, or run: torosu"
info "It updates itself from the app. To remove it: bash install.sh --uninstall"
