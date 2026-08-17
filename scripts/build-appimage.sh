#!/usr/bin/env bash
#
# Packages oag-game as a single-file x86_64 AppImage.
#
# The target is a Steam Deck: SteamOS has an immutable root filesystem, so the
# fastest possible feedback cycle is one file copied over and run, with no
# runtime to install and no package manager to unlock. See
# docs/tools/packaging.md for why AppImage rather than Flatpak, and for what is
# and is not bundled.
#
# **No game content is ever packaged.** The AppImage is the engine; the player
# supplies their own disc image, which the game looks for beside the AppImage
# among other places (crates/game/src/source.rs). This script refuses to build
# if anything that looks like game content has found its way into the AppDir.
#
# Usage:
#   scripts/build-appimage.sh [--out <file>] [--container] [--skip-build]
#                             [--binary <path>] [--no-strip]
#
#   --container  Build inside Debian bookworm (glibc 2.36) instead of natively,
#                so the AppImage also loads on a distribution older than this
#                one - which is the whole point on a Steam Deck. Needs podman or
#                docker; see docs/tools/packaging.md#glibc.
#   --binary     Package a binary built elsewhere. Implies --skip-build. Must
#                run on this host: the icon step below now invokes it
#                directly (--write-icon), so a binary for another OS/arch
#                needs its icon supplied some other way.
#
# Environment:
#   APPIMAGETOOL      appimagetool to use. Default: downloaded into data/tools/.
#   CONTAINER_ENGINE  podman or docker. Default: whichever is installed.

set -euo pipefail

APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tools_dir="$project_root/data/tools"
work_dir="$project_root/data/appimage"
app_dir="$work_dir/AppDir"
# Empty until the build mode is known: a --container build gets its own name, so
# the two never overwrite each other and the file says which it is. The whole
# point of the container build is that the native one does not start on an older
# glibc, and a directory holding one unlabelled AppImage cannot answer which one
# is sitting there.
out=""

# The glibc a --container build is expected to produce, and the version this
# script stops warning about. Debian bookworm's.
PORTABLE_FLOOR="2.36"

# Tag of the image built from packaging/appimage/Containerfile.
CONTAINER_IMAGE="oag-appimage-build:bookworm"

skip_build=0
strip_binary=1
container=0
binary=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out)        out="${2:?--out needs a value}"; shift 2 ;;
        --container)  container=1; shift ;;
        --binary)     binary="${2:?--binary needs a value}"; skip_build=1; shift 2 ;;
        --skip-build) skip_build=1; shift ;;
        --no-strip)   strip_binary=0; shift ;;
        # Print the header comment block, however long it happens to be.
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }

step "Building oag-game (release)"

if [[ -n $binary ]]; then
    echo "using $binary"
elif (( container )); then
    engine="${CONTAINER_ENGINE:-}"
    if [[ -z $engine ]]; then
        for candidate in podman docker; do
            command -v "$candidate" >/dev/null 2>&1 && { engine="$candidate"; break; }
        done
    fi
    [[ -n $engine ]] \
        || die "--container needs podman or docker, and neither is installed. \
Build natively instead and mind the glibc floor this script prints."

    if ! "$engine" image exists "$CONTAINER_IMAGE" 2>/dev/null \
        && ! "$engine" image inspect "$CONTAINER_IMAGE" >/dev/null 2>&1; then
        echo "building $CONTAINER_IMAGE (once; ~1 GB, ~2 minutes)"
        "$engine" build -t "$CONTAINER_IMAGE" \
            -f "$project_root/packaging/appimage/Containerfile" \
            "$project_root/packaging/appimage"
    fi

    # A separate CARGO_HOME and target directory, both under the gitignored
    # data/, so a container build neither shares nor invalidates the host's
    # incremental state - the two use different compilers against different
    # libcs.
    container_target="$work_dir/container/target"
    container_cargo="$work_dir/container/cargo"
    mkdir -p "$container_target" "$container_cargo"

    # Rootless podman already maps the container's root onto this user, so
    # anything written to the mounts is owned correctly. Docker does not, hence
    # --user there; without it the mounted directories come back owned by root.
    engine_args=(--rm -v "$project_root:/src" -w /src
                 -e CARGO_HOME=/src/data/appimage/container/cargo
                 -e CARGO_TARGET_DIR=/src/data/appimage/container/target)
    [[ $engine == docker ]] && engine_args+=(--user "$(id -u):$(id -g)")

    "$engine" run "${engine_args[@]}" "$CONTAINER_IMAGE" \
        bash -c 'ldd --version | head -1 && cargo build --release -p oag-game'
    binary="$container_target/release/oag-game"
elif (( skip_build )); then
    echo "skipped, using whatever is in target/release"
    binary="$project_root/target/release/oag-game"
else
    cargo build --release -p oag-game --manifest-path "$project_root/Cargo.toml"
    binary="$project_root/target/release/oag-game"
fi

[[ -x $binary ]] || die "$binary not found; run without --skip-build"

if [[ -z $out ]]; then
    (( container )) && suffix="-portable" || suffix=""
    out="$work_dir/OpenAntiGrav-x86_64${suffix}.AppImage"
fi

step "Assembling the AppDir"

rm -rf "$app_dir"
mkdir -p "$app_dir/usr/bin" \
         "$app_dir/usr/share/applications" \
         "$app_dir/usr/share/icons/hicolor/256x256/apps"

install -m 755 "$binary" "$app_dir/usr/bin/oag-game"
if (( strip_binary )); then
    # `[profile.release] debug = 1` is there for backtraces during development
    # and is about 60% of the binary. The profile stays as it is; the copy that
    # goes into the package loses its symbols.
    strip "$app_dir/usr/bin/oag-game"
fi

install -m 755 "$project_root/packaging/appimage/AppRun" "$app_dir/AppRun"
install -m 644 "$project_root/packaging/appimage/oag-game.desktop" \
    "$app_dir/oag-game.desktop"
install -m 644 "$app_dir/oag-game.desktop" \
    "$app_dir/usr/share/applications/oag-game.desktop"

# The same rasterizer `main/window.rs` uses for the live window icon, so the
# taskbar icon a packaged build installs and the one the window shows at
# startup can never drift apart the way the old hand-drawn chevron did. See
# `oag_game::icon` and docs/tools/packaging.md.
"$binary" --write-icon "$app_dir/oag-game.png" --icon-size 256
install -m 644 "$app_dir/oag-game.png" \
    "$app_dir/usr/share/icons/hicolor/256x256/apps/oag-game.png"
# appimagetool takes .DirIcon as the thumbnail; a copy rather than a symlink so
# the squashfs holds the bytes whatever the reader does with links.
install -m 644 "$app_dir/oag-game.png" "$app_dir/.DirIcon"

if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$app_dir/oag-game.desktop" \
        || die "the desktop entry is not valid"
fi

step "Checking for leaked game content"

# The same extensions .gitignore and `just audit-leakage` guard, applied to what
# is about to be packaged. A package must never carry game content.
leaked="$(find "$app_dir" -type f \
    \( -iname '*.chd' -o -iname '*.iso' -o -iname '*.cso' -o -iname '*.pkg' \
       -o -iname '*.pbp' -o -iname '*.wad' -o -iname '*.elf' -o -iname '*.prx' \
       -o -iname '*.self' -o -iname '*.bin' -o -iname '*.img' \) -print)"
[[ -z $leaked ]] || die "game content in the AppDir:"$'\n'"$leaked"
echo "OK: the AppDir is the engine and nothing else"

step "Runtime libraries"

# Nothing is bundled on purpose. What the binary links is libc, libm, libgcc_s,
# libudev and libasound, every one of which is either part of the base system
# or, in libudev's and libasound's case, host-owned - the device manager and
# the ALSA stack (mixer settings, PulseAudio/PipeWire's own ALSA plugin) -
# and the AppImage excludelist names all of them for the same reason. Vulkan
# and the windowing libraries are dlopen'd by wgpu and winit at runtime and
# must come from the host, because a bundled loader cannot talk to the host's
# graphics driver. See docs/tools/packaging.md.
ldd "$app_dir/usr/bin/oag-game" | sed 's/^/  /'

floor="$(objdump -T "$app_dir/usr/bin/oag-game" \
    | grep -o 'GLIBC_[0-9.]*' | sort -u -V | tail -1)"
echo
echo "glibc floor: $floor"

# Only the *newest* symbol version referenced matters, and only if it is newer
# than what a portable build produces. Below that, saying nothing beats crying
# wolf on every build.
newest="$(printf '%s\n%s\n' "$PORTABLE_FLOOR" "${floor#GLIBC_}" | sort -V | tail -1)"
if [[ $newest == "$PORTABLE_FLOOR" ]]; then
    echo "  at or below the portable baseline ($PORTABLE_FLOOR, Debian bookworm):"
    echo "  this runs on anything that new or newer, the Deck included."
else
    warn "this build only runs on a machine whose glibc is at least ${floor#GLIBC_}.
  It is the glibc it was *linked against* that sets this, not anything in the
  code: the linker binds each libm symbol to the newest version the build host
  offers. A target with an older glibc fails at load with
  \"version \`$floor' not found\". Rebuild with --container to bring the floor
  down to $PORTABLE_FLOOR; docs/tools/packaging.md#glibc has the detail, and how
  to check the Deck's own version."
fi

step "Packing"

appimagetool="${APPIMAGETOOL:-$tools_dir/appimagetool-x86_64.AppImage}"
if [[ ! -x $appimagetool ]]; then
    echo "downloading appimagetool into ${appimagetool%/*}"
    mkdir -p "${appimagetool%/*}"
    curl -fsSL -o "$appimagetool" "$APPIMAGETOOL_URL"
    chmod +x "$appimagetool"
fi

mkdir -p "$(dirname "$out")"
rm -f "$out"
# APPIMAGE_EXTRACT_AND_RUN because appimagetool is itself an AppImage and FUSE
# is not always available (a container, a sandbox, a machine without
# /dev/fuse). It costs one extraction and needs no kernel support.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$appimagetool" "$app_dir" "$out"

[[ -f $out ]] || die "appimagetool produced no file"
chmod +x "$out"

step "Done"

printf '%s\n' "$out"
du -h "$out" | cut -f1 | sed 's/^/  size: /'
echo "  glibc floor: $floor"
echo "  run it from anywhere; put a disc image beside it, or name one:"
echo "    $out path/to/pulse-psp-usa.chd"
