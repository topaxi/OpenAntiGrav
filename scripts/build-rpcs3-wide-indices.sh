#!/usr/bin/env bash
#
# Builds the RPCS3 the HD references are taken with, with generated index buffers widened to 32 bits, so a
# QUADS draw past 65,535 vertices reaches its last vertex. See docs/reverse-engineering/rpcs3-index-wrap.md.
#
# The base is the commit of the packaged build the references were always taken with, so the patch is the only
# difference. Three patches apply in order: the fix, a log line per new peak draw size (the census), and the
# ffmpeg link group GNU ld needs. LLVM comes from the submodule: this commit does not compile against LLVM 23.
#
# Configure once, with the patches in place. A CMake re-run in an existing tree changes the flags of the
# vendored LLVM and its compile then fails against libstdc++ 16.
#
# Usage: scripts/build-rpcs3-wide-indices.sh [out-dir]    default data/tools/rpcs3-wide-indices
# Environment: JOBS, parallel ninja jobs, default 10.
#
# Run the result with OAG_RPCS3_BIN=<out-dir>/src/build/bin/rpcs3 in the environment of `rpcs3-drive.py serve`.

set -euo pipefail

COMMIT=028d1e8f4
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$root/data/tools/rpcs3-wide-indices}"
src="$out/src"

git clone --no-checkout https://github.com/RPCS3/rpcs3.git "$src"
git -C "$src" checkout --quiet --detach "$COMMIT"
git -C "$src" submodule update --init --recursive
for patch in wide-emulated-indices draw-peak-census gnu-ld-ffmpeg-group; do
    git -C "$src" apply --verbose "$root/scripts/patches/rpcs3-$patch.patch"
done
CC=clang CXX=clang++ cmake -B "$src/build" -S "$src" -G Ninja -DCMAKE_BUILD_TYPE=Release \
    -DUSE_LTO=OFF -DUSE_DISCORD_RPC=OFF -DBUILD_LLVM=ON
nice -n 15 ninja -C "$src/build" -j "${JOBS:-10}" rpcs3
"$src/build/bin/rpcs3" --version
