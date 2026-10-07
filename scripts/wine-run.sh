#!/usr/bin/env bash
#
# Cross-builds the Windows (x86_64-pc-windows-gnu) oag-game and runs it, or the
# determinism checks, under Wine in a prefix of its own. See docs/tools/wine.md.
#
# Usage:
#   scripts/wine-run.sh build                 oag-game.exe, debug (RELEASE=1 for release)
#   scripts/wine-run.sh run <oag-game args>   build, then run under wine
#   scripts/wine-run.sh determinism           the four determinism tests as .exe under wine
#
# Linker: x86_64-w64-mingw32-gcc if installed, else `zig cc` (zig ships the MinGW
# runtime). Env: OAG_WINEPREFIX (default data/wine/prefix, never ~/.wine),
# CARGO_TARGET_DIR, RELEASE=1.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
triple=x86_64-pc-windows-gnu
profile=debug
profile_flag=()
if [[ "${RELEASE:-0}" == 1 ]]; then profile=release; profile_flag=(--release); fi

die() { echo "error: $*" >&2; exit 1; }

command -v wine >/dev/null || die "wine not found (install wine)"
rustup target list --installed | grep -qx "$triple" || rustup target add "$triple"

tools="${OAG_WINE_TOOLS:-$root/data/wine/tools}"
mkdir -p "$tools"
if command -v x86_64-w64-mingw32-gcc >/dev/null; then
    linker=x86_64-w64-mingw32-gcc
elif command -v zig >/dev/null; then
    linker="$root/scripts/zig-cc-windows-gnu.sh"
    ln -sf "$root/scripts/zig-dlltool-windows-gnu.sh" "$tools/x86_64-w64-mingw32-dlltool"
    export PATH="$tools:$PATH"
else
    die "need MinGW-w64 (x86_64-w64-mingw32-gcc) or zig on PATH"
fi
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$linker"
export WINEPREFIX="${OAG_WINEPREFIX:-$root/data/wine/prefix}"
export WINEDEBUG="${WINEDEBUG:--all}"
mkdir -p "$WINEPREFIX"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
mkdir -p "$target_dir"

build_game() {
    cargo build -p oag-game --target "$triple" "${profile_flag[@]}"
}

determinism() {
    local rc=0 crate bin
    for crate in oag-core oag-physics oag-gameplay oag-ai; do
        cargo test -p "$crate" --target "$triple" "${profile_flag[@]}" --test determinism --no-run \
            --message-format=json >"$target_dir/wine-$crate.json"
        bin="$(jq -r 'select(.profile.test==true and .target.name=="determinism") | .executable' \
            "$target_dir/wine-$crate.json" | tail -1)"
        [[ -n "$bin" ]] || die "no determinism test binary for $crate"
        echo "== $crate: $(basename "$bin")"
        wine "$bin" --nocapture || rc=1
    done
    for ex in oag-core:core_determinism_report oag-physics:physics_determinism_report oag-ai:ai_determinism_report; do
        cargo build -p "${ex%%:*}" --example "${ex##*:}" --target "$triple" "${profile_flag[@]}"
        echo "== example ${ex##*:}"
        wine "$target_dir/$triple/$profile/examples/${ex##*:}.exe"
    done
    return $rc
}

case "${1:-}" in
    build) build_game ;;
    run) shift; build_game; wine "$target_dir/$triple/$profile/oag-game.exe" "$@" ;;
    determinism) determinism ;;
    *) die "usage: wine-run.sh build | run <args> | determinism" ;;
esac
