#!/usr/bin/env bash
#
# Installs a `.desktop` entry and icon for a source checkout, into the
# current user's own XDG data directories - no root, no package manager.
#
# **Why this exists at all.** On X11, `winit`'s window icon (`main/window.rs`
# -> `oag_game::icon`) is enough on its own: a taskbar can read it straight
# off the window. On Wayland there is no such protocol - a compositor's
# taskbar instead looks up the window's app_id (`main/gpu.rs` sets it to
# `oag-game`, matching this script) against an *installed* `.desktop` entry
# and takes the icon that entry names. `just appimage` already does this
# packaging step for a distributed build; running the game straight out of
# the checkout with `cargo run`/`just play` installs nothing on its own, so
# without this script a Wayland taskbar (niri included) has nothing to find
# for a dev build. See docs/tools/packaging.md.
#
# Usage:
#   scripts/install-desktop-file.sh [--release|--debug]
#
#   --release  Build (or reuse) target/release/oag-game. Default.
#   --debug    Build (or reuse) target/debug/oag-game instead - faster to
#              have ready, slower to run.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="release"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) profile="release"; shift ;;
        --debug)   profile="debug"; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "Building oag-game ($profile)"
if [[ $profile == release ]]; then
    cargo build --release -p oag-game --manifest-path "$project_root/Cargo.toml"
else
    cargo build -p oag-game --manifest-path "$project_root/Cargo.toml"
fi
binary="$project_root/target/$profile/oag-game"
[[ -x $binary ]] || { echo "error: $binary not found after building" >&2; exit 1; }

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
apps_dir="$data_home/applications"
icons_dir="$data_home/icons/hicolor"

step "Writing the icon"
# One rasterizer, two sizes: the same `oag_game::icon` call
# `scripts/build-appimage.sh` uses for the packaged build, so a source
# checkout's taskbar icon and a packaged one's are pixel-for-pixel the same
# picture, never two independently drawn ones. No 16x16 tier: `oag_game::icon`
# only ever renders `assets/icons/64x64.svg`, and downscaling that to 16px
# would silently stand in for the separately hand-tuned `16x16.svg` - see its
# doc comment. Sparse hicolor tiers are normal; a theme engine picks the
# nearest one it has.
for size in 64 256; do
    dir="$icons_dir/${size}x${size}/apps"
    mkdir -p "$dir"
    "$binary" --write-icon "$dir/oag-game.png" --icon-size "$size"
done

step "Writing the desktop entry"
mkdir -p "$apps_dir"
desktop_file="$apps_dir/oag-game.desktop"
# Exec is an absolute path, unlike packaging/appimage/oag-game.desktop's bare
# `oag-game`: that one runs inside the AppImage's own mount where AppRun puts
# the binary on `PATH`, and a checkout's build directory is on nobody's.
cat > "$desktop_file" <<DESKTOP
[Desktop Entry]
Type=Application
Name=OpenAntiGrav (dev)
GenericName=Anti-gravity racer
Comment=A clean-room reimplementation of Wipeout Pulse, played from your own disc image
Exec=$binary
Icon=oag-game
Terminal=false
Categories=Game;ActionGame;
Keywords=racing;antigravity;wipeout;
DESKTOP

if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$desktop_file"
fi

# Both are advisory: a desktop environment that polls its data directories
# (most do) picks the change up without either, and neither tool exists on
# every distribution.
command -v update-desktop-database >/dev/null 2>&1 \
    && update-desktop-database "$apps_dir" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 \
    && gtk-update-icon-cache -f -t "$icons_dir" 2>/dev/null || true

step "Done"
echo "installed: $desktop_file"
echo "icon:      $icons_dir/{64x64,256x256}/apps/oag-game.png"
echo
echo "A running taskbar/app grid may need to restart to pick this up. To"
echo "remove it again: rm '$desktop_file' and the icon files above."
