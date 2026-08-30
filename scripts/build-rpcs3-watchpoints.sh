#!/usr/bin/env bash
#
# Builds a patched RPCS3 with working GDB write watchpoints (Z2/z2).
#
# Stock RPCS3 (including the rpcs3-bin/rpcs3-git AUR packages) never
# implemented Z2 - confirmed against upstream master's own rpcs3/Emu/GDB.cpp,
# see docs/reverse-engineering/rpcs3-debugger.md ("A patched build exists").
# The patch this applies (scripts/patches/rpcs3-gdb-write-watchpoints.patch)
# adds a small range-checked watch registry reusing the same stop/notify path
# Z0 software breakpoints already use, verified end to end against a live
# Wipeout HD/Fury session.
#
# Needs -DHAS_MEMORY_BREAKPOINTS=ON, a genuine upstream CMake option that
# defaults off and gates the whole memory-write-hook code path in vm.h
# (including RPCS3's own pre-existing Qt memory-breakpoint feature, which
# this build also switches on as a side effect). -DUSE_LTO=OFF and the
# ffmpeg link-group hunk in the patch are local workarounds for GNU ld
# rather than lld - both harmless on a system that already links with lld.
#
# Only fires under `Core > PPU Decoder: Interpreter (static)` in RPCS3's own
# config.yml - the LLVM recompiler emits stores as generated machine code
# that never calls vm::write(), the same reason Z0 itself refuses to arm
# under it. Switching the decoder is a config change, not something this
# script does - see docs/reverse-engineering/rpcs3-debugger.md for the rest
# of the practical usage (one-shot GDB sessions, the multi-thread stuck-flag
# caveat, preferring a narrow watch).
#
# Usage:
#   scripts/build-rpcs3-watchpoints.sh [--clean] [--ref <commit>]
#
# --clean removes the checkout and starts fresh. --ref overrides the pinned
# base commit the patch is known to apply to (only useful if rebasing the
# patch onto a newer RPCS3, which needs re-verifying the patch applies and
# re-running the end-to-end test in the doc above - not just a rebuild).
#
# Environment:
#   JOBS   parallel ninja jobs. Default: nproc.

set -euo pipefail

REPO_URL="https://github.com/RPCS3/rpcs3.git"
# The exact commit the patch in scripts/patches/ was built and verified
# against - see that file's own header for the full RPCS3_GIT_VERSION string.
PINNED_COMMIT="3ef20ebb0"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="$project_root/data/tools/rpcs3-watchpoints"
patch_path="$project_root/scripts/patches/rpcs3-gdb-write-watchpoints.patch"

ref="$PINNED_COMMIT"
clean=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ref)   ref="${2:?--ref needs a value}"; shift 2 ;;
        --clean) clean=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

[[ -f $patch_path ]] || die "$patch_path not found - is this really the OpenAntiGrav repo?"

step "Checking toolchain"

for tool in git cmake ninja clang clang++; do
    command -v "$tool" >/dev/null || die "$tool not found. Install it before building RPCS3."
done
echo "git, cmake, ninja, clang, clang++ all present"

step "Fetching RPCS3 ($ref)"

if [[ $clean -eq 1 && -d $checkout_dir ]]; then
    echo "removing $checkout_dir"
    rm -rf "$checkout_dir"
fi

if [[ -d $checkout_dir/.git ]]; then
    git -C "$checkout_dir" fetch origin
else
    mkdir -p "$(dirname "$checkout_dir")"
    # Recursive: RPCS3 vendors LLVM headers, ffmpeg-core and other submodules
    # it needs at build time. This is a large checkout (several GiB).
    git clone --recursive "$REPO_URL" "$checkout_dir"
fi

git -C "$checkout_dir" checkout --quiet --detach "$ref"
git -C "$checkout_dir" submodule update --init --recursive
# Reset any tracked-file changes from a previous run of this script (the
# patch, applied below) so re-running without --clean is idempotent. Leaves
# build/ alone - a rebuild after this reapplies the patch and reuses ninja's
# object cache rather than starting over.
git -C "$checkout_dir" clean -qfdx -e build
git -C "$checkout_dir" checkout --quiet -- .

echo "at $(git -C "$checkout_dir" rev-parse --short HEAD) ($(git -C "$checkout_dir" log -1 --format=%cs))"
if [[ $(git -C "$checkout_dir" rev-parse --short HEAD) != "$PINNED_COMMIT"* && $ref == "$PINNED_COMMIT" ]]; then
    die "checked out commit does not match the pinned one - has $PINNED_COMMIT been garbage-collected upstream?"
fi

step "Applying the write-watchpoint patch"

git -C "$checkout_dir" apply --verbose "$patch_path"

step "Configuring (CMake, Ninja)"

# USE_LTO=OFF and the ffmpeg link-group hunk in the patch are both there for
# GNU ld rather than lld - see the patch file's own header. HAS_MEMORY_BREAKPOINTS
# is the real prerequisite: off by default upstream, and without it Z2/z2
# still parse and answer OK but silently never fire (no ppu_thread* reaches
# the write path to check against).
CC=clang CXX=clang++ cmake -B "$checkout_dir/build" -S "$checkout_dir" -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DUSE_LTO=OFF \
    -DHAS_MEMORY_BREAKPOINTS=ON

step "Building (this is a large C++ codebase - expect this to take a while)"

ninja -C "$checkout_dir/build" -j "${JOBS:-$(nproc)}" rpcs3

step "Verifying output"

binary="$checkout_dir/build/bin/rpcs3"
[[ -x $binary ]] || die "build reported success but $binary is missing or not executable"

step "Done"

cat <<EOF
Binary: $binary

To use it:
  1. In ~/.config/rpcs3/config.yml, set Core > PPU Decoder to
     "Interpreter (static)" - Z2 (and Z0) only fire under it, matching the
     LLVM recompiler's inability to hook vm::write(). This file is shared
     with any other RPCS3 install on this machine; switch it back afterward
     if something else needs the Recompiler.
  2. Launch: $binary --no-gui <disc image>
  3. Connect a GDB client to 127.0.0.1:2345 and use Z2,<addr>,<len> / z2,<addr>,<len>.

See docs/reverse-engineering/rpcs3-debugger.md ("A patched build exists") for
the full protocol, known traps (one-shot GDB sessions, preferring a narrow
watch over a broad one, RPCS3's own single-instance behaviour) and the
end-to-end test transcript this build was verified against.
EOF
