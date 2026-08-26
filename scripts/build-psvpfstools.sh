#!/usr/bin/env bash
#
# Builds psvpfsparser, the CLI from motoharu-gosuto/psvpfstools that decrypts
# a PS Vita PKG's PFS layer (sce_pfs/{files,unicv}.db) given its zRIF or raw
# klicensee. See data/README.md#the-ps3-and-vita-images-are-encrypted-and-
# nothing-here-decrypts-them-yet for what pkg2zip does and does not decrypt.
#
# Deliberately NOT the Vita3K/psvpfsparser fork: that one is a library used
# internally by the Vita3K emulator with no standalone CLI. This project's
# original repo has one, plus a documented -k/-z/-f00d_url interface.
#
# Upstream is a dead 2018 C++11 codebase and does not build clean on a
# current toolchain. Four fixes are applied here, all upstream bugs rather
# than anything version-specific to this project:
#   1. cmake/scripts/FindLibTomCrypt.cmake is loaded by `find_package(LIBTOMCRYPT
#      REQUIRED)`, which looks for FindLIBTOMCRYPT.cmake (all caps) - only
#      resolves by accident on case-insensitive filesystems. Copied in under
#      the exact name CMake looks for.
#   2. That same file's original body set LIBTOMCRYPT_LIBRARIES from
#      LIBTOMCRYPT_LIBRARY *before* find_library() had populated it, so it
#      was always empty - configure succeeded (LIBTOMCRYPT_FOUND only checks
#      LIBTOMCRYPT_LIBRARY) but linking failed with undefined references to
#      every libtomcrypt symbol. Rewritten to compute LIBTOMCRYPT_LIBRARIES
#      after both find_path()/find_library() calls.
#   3. libb64's and libzRIF's cmake_minimum_required predates CMake 3.5,
#      which current CMake refuses outright. Built with
#      -DCMAKE_POLICY_VERSION_MINIMUM=3.5.
#   4. A handful of .cpp files use memcpy/memset without including <cstring>
#      - relied on it being pulled in transitively, which current libstdc++
#      no longer does everywhere. Forced in project-wide via CMAKE_CXX_FLAGS
#      rather than patching each file.
#
# Usage:
#   scripts/build-psvpfstools.sh [--ref <git-ref>] [--clean]
#
# Environment:
#   PSVPFSTOOLS_DIR   Checkout + build location. Default: data/tools/psvpfstools

set -euo pipefail

REPO_URL="https://github.com/motoharu-gosuto/psvpfstools.git"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="${PSVPFSTOOLS_DIR:-$project_root/data/tools/psvpfstools}"

ref="master"
clean=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ref)   ref="${2:?--ref needs a value}"; shift 2 ;;
        --clean) clean=1; shift ;;
        # Print the header comment block, however long it happens to be.
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "Checking dependencies"

for dep in cmake g++; do
    command -v "$dep" >/dev/null || die "$dep not found. Install it."
done

# boost and libtomcrypt have no `command -v` equivalent; let cmake's own
# find_package report those missing, with a clearer message than a bare
# configure failure - see the pkg-config probe below instead.
pkg-config --exists libtomcrypt 2>/dev/null || [[ -f /usr/include/tomcrypt.h ]] \
    || die "libtomcrypt headers not found. Install it (Arch: sudo pacman -S libtomcrypt)."

step "Fetching psvpfstools ($ref)"

if [[ $clean -eq 1 && -d $checkout_dir ]]; then
    echo "removing $checkout_dir"
    rm -rf "$checkout_dir"
fi

if [[ -d $checkout_dir/.git ]]; then
    git -C "$checkout_dir" fetch --tags --prune origin
else
    mkdir -p "$(dirname "$checkout_dir")"
    git clone "$REPO_URL" "$checkout_dir"
fi

if git -C "$checkout_dir" rev-parse --verify --quiet "origin/$ref" >/dev/null; then
    git -C "$checkout_dir" checkout --quiet --detach "origin/$ref"
else
    git -C "$checkout_dir" checkout --quiet --detach "$ref"
fi
git -C "$checkout_dir" clean -qfdx -- . ':!cmake/build'

echo "at $(git -C "$checkout_dir" rev-parse --short HEAD) ($(git -C "$checkout_dir" log -1 --format=%cs))"

step "Applying the FindLIBTOMCRYPT.cmake fix"

find_module="$checkout_dir/cmake/scripts/FindLIBTOMCRYPT.cmake"
cat > "$find_module" <<'EOF'
# - Try to find LIBTOMCRYPT
# Specify the following variables to help the search:
# LIBTOMCRYPT_INCLUDE_DIR - include directory path
# LIBTOMCRYPT_LIBRARY - library file path
# After search - these variables will be set:
# LIBTOMCRYPT_FOUND - System has LIBTOMCRYPT
# LIBTOMCRYPT_INCLUDE_DIRS - The LIBTOMCRYPT include directories
# LIBTOMCRYPT_LIBRARIES - The libraries needed to use LIBTOMCRYPT
#
# Rewritten from upstream's cmake/scripts/FindLibTomCrypt.cmake: the original
# body set LIBTOMCRYPT_LIBRARIES from LIBTOMCRYPT_LIBRARY before
# find_library() had populated it, so it was always empty and every target
# linking against ${LIBTOMCRYPT_LIBRARIES} failed at link time with
# undefined references, not configure time. Installed under this exact
# filename (all caps) because find_package(LIBTOMCRYPT) looks for it by that
# name - only resolves by accident on a case-insensitive filesystem.

find_path(LIBTOMCRYPT_INCLUDE_DIR "tomcrypt.h")
find_library(LIBTOMCRYPT_LIBRARY tomcrypt)

set(LIBTOMCRYPT_INCLUDE_DIRS ${LIBTOMCRYPT_INCLUDE_DIR})
set(LIBTOMCRYPT_LIBRARIES ${LIBTOMCRYPT_LIBRARY})

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(LIBTOMCRYPT DEFAULT_MSG LIBTOMCRYPT_LIBRARY LIBTOMCRYPT_INCLUDE_DIR)

mark_as_advanced(LIBTOMCRYPT_INCLUDE_DIR LIBTOMCRYPT_LIBRARY)
EOF
echo "$find_module"

step "Configuring"

build_dir="$checkout_dir/cmake/build"
rm -rf "$build_dir"
mkdir -p "$build_dir"

cmake -S "$checkout_dir/cmake" -B "$build_dir" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
    -DCMAKE_CXX_FLAGS="-include cstring -include cstdint -include cstdio"

step "Building"

cmake --build "$build_dir" --parallel

step "Verifying output"

bin_path="$checkout_dir/cmake/output/Release/psvpfsparser"
[[ -x $bin_path ]] || die "build reported success but $bin_path is not there"

# --help exits non-zero (it is argument-parsing failure, not a real -h flag
# upstream wired up), which under `set -o pipefail` would fail this whole
# pipeline regardless of what grep finds - capture output first instead.
help_output="$("$bin_path" --help 2>&1 || true)"
grep -q -- '--klicensee' <<<"$help_output" \
    || die "$bin_path ran but did not print the expected --klicensee option"

step "Done"

cat <<EOF
Binary: $bin_path

Decrypt a pkg2zip'd Vita PKG's PFS layer:
  1. pkg2zip -x <pkg>                    # strips the outer AES-CTR layer only
  2. $bin_path -i <app|patch|addcont dir> -o <dest> -z <zRIF> -f http://cma.henkaku.xyz

-z wants a zRIF string (from e.g. nopaystation.com); -k wants a raw 16-byte
klicensee hex string instead, if that is what you have. Either needs the
public F00D crypto service reachable at -f (see the psvpfstools README for
what that service does and does not see). A patch pkg reuses its base app's
zRIF - same content ID, same license.
EOF
