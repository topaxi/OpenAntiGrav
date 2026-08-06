#!/usr/bin/env bash
#
# Builds the Emotion Engine (PS2 R5900) processor module for the installed
# Ghidra.
#
# Stock Ghidra's generic MIPS support auto-detects R5900 code as
# MIPS:LE:64:64-32R6addr - MIPS Release 6, a 2014 ISA revision that reassigns
# much of the opcode space MIPS III (what R5900 actually implements) used.
# Analysis on a wrongly-decoded binary finds almost nothing, and some MMI
# encodings alias onto real-looking but wrong MIPS DSP ASE instructions rather
# than failing loudly. See docs/reverse-engineering/toolchain.md.
#
# Building from source rather than downloading a release is deliberate, same
# reasoning as Allegrex: Ghidra matches an extension's declared version
# against its own exactly, and upstream does not publish a build for every
# point release.
#
# This project has no Gradle wrapper of its own - it borrows the Allegrex
# checkout's wrapper via --project-dir, so scripts/build-ghidra-allegrex.sh
# (or this script) must have run at least once first to populate
# data/tools/ghidra-allegrex.
#
# Usage:
#   scripts/build-ghidra-emotionengine.sh [--ref <git-ref>] [--clean] [--no-stdump]
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME         JDK 21 home. Default: autodetected.

set -euo pipefail

REPO_URL="https://github.com/chaoticgd/ghidra-emotionengine-reloaded.git"
ALLEGREX_REPO_URL="https://github.com/kotcrab/ghidra-allegrex.git"
GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="$project_root/data/tools/ghidra-emotionengine-reloaded"
allegrex_dir="$project_root/data/tools/ghidra-allegrex"
output_dir="$project_root/data/tools"

ref="main"
clean=0
fetch_stdump=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ref)        ref="${2:?--ref needs a value}"; shift 2 ;;
        --clean)      clean=1; shift ;;
        --no-stdump)  fetch_stdump=0; shift ;;
        # Print the header comment block, however long it happens to be.
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "Checking Ghidra installation"

[[ -d $GHIDRA_INSTALL_DIR ]] \
    || die "GHIDRA_INSTALL_DIR=$GHIDRA_INSTALL_DIR does not exist. Install Ghidra, or set GHIDRA_INSTALL_DIR."

build_support="$GHIDRA_INSTALL_DIR/support/buildExtension.gradle"
[[ -f $build_support ]] \
    || die "$build_support not found. Is GHIDRA_INSTALL_DIR really a Ghidra root?"

app_props="$GHIDRA_INSTALL_DIR/Ghidra/application.properties"
[[ -f $app_props ]] || die "$app_props not found."

# Properties files here use CRLF, so strip carriage returns before comparing.
ghidra_version="$(sed -n 's/^application\.version=//p' "$app_props" | tr -d '\r[:space:]')"
[[ -n $ghidra_version ]] || die "could not read application.version from $app_props"
echo "Ghidra $ghidra_version at $GHIDRA_INSTALL_DIR"

# JDK 21 specifically - same wrapper constraint as Allegrex, since this
# project borrows that wrapper.
step "Locating JDK 21"

if [[ -n ${JAVA_21_HOME:-} ]]; then
    java_home="$JAVA_21_HOME"
else
    java_home=""
    for candidate in \
        /usr/lib/jvm/java-21-openjdk \
        /usr/lib/jvm/java-21-openjdk-amd64 \
        /usr/lib/jvm/temurin-21-jdk \
        /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
    do
        [[ -x $candidate/bin/javac ]] && { java_home="$candidate"; break; }
    done
fi

[[ -n $java_home && -x $java_home/bin/javac ]] || die \
    "no JDK 21 found. Install it (Arch: sudo pacman -S jdk21-openjdk) or set JAVA_21_HOME.
   The Gradle wrapper this project borrows does not support newer JDKs."

echo "JDK 21 at $java_home"
"$java_home/bin/java" -version 2>&1 | head -1

step "Locating a Gradle wrapper (borrowed from ghidra-allegrex)"

if [[ ! -x "$allegrex_dir/gradlew" ]]; then
    echo "$allegrex_dir has no gradlew yet; fetching ghidra-allegrex to get one"
    mkdir -p "$(dirname "$allegrex_dir")"
    git clone "$ALLEGREX_REPO_URL" "$allegrex_dir"
fi
[[ -x "$allegrex_dir/gradlew" ]] || die "$allegrex_dir/gradlew still missing after clone"
echo "using $allegrex_dir/gradlew"

step "Fetching ghidra-emotionengine-reloaded ($ref)"

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

# Resolve the ref locally so a branch name and a tag both work, and so the
# build is reproducible from the printed commit.
if git -C "$checkout_dir" rev-parse --verify --quiet "origin/$ref" >/dev/null; then
    git -C "$checkout_dir" checkout --quiet --detach "origin/$ref"
else
    git -C "$checkout_dir" checkout --quiet --detach "$ref"
fi
git -C "$checkout_dir" clean -qfd

echo "at $(git -C "$checkout_dir" rev-parse --short HEAD) ($(git -C "$checkout_dir" log -1 --format=%cs))"

if [[ $fetch_stdump -eq 1 ]]; then
    step "Fetching stdump (MIPS .mdebug symbol import helper, optional)"
    bash "$checkout_dir/os/download.sh"
else
    echo "skipping (--no-stdump)"
fi

step "Building"

# GHIDRA_INSTALL_DIR is what build.gradle reads to locate buildExtension.gradle,
# which is also what stamps the extension version. --project-dir points the
# borrowed wrapper at this project instead of ghidra-allegrex.
env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME="$java_home" \
    GHIDRA_INSTALL_DIR="$GHIDRA_INSTALL_DIR" \
    "$allegrex_dir/gradlew" \
        --project-dir "$checkout_dir" \
        --no-daemon \
        buildExtension

step "Verifying output"

zip_path="$(find "$checkout_dir/dist" -name '*ghidra-emotionengine-reloaded.zip' -newermt '-1 hour' | sort | tail -1)"
[[ -n $zip_path && -f $zip_path ]] || die "build reported success but produced no zip in $checkout_dir/dist"

# The whole point of building from source is that the declared version matches.
# Check it rather than assume it: a mismatch means Ghidra will silently refuse
# to load the extension.
declared="$(unzip -p "$zip_path" '*/extension.properties' 2>/dev/null \
    | sed -n 's/^version=//p' | head -1 | tr -d '\r[:space:]')"
[[ -n $declared ]] || die "no version found in the extension.properties inside $zip_path"

if [[ $declared != "$ghidra_version" ]]; then
    die "built extension declares version '$declared' but Ghidra is '$ghidra_version'.
   Ghidra will refuse to load it. This should not happen with a source build."
fi

mkdir -p "$output_dir"
final="$output_dir/$(basename "$zip_path")"
cp -f "$zip_path" "$final"

step "Done"

cat <<EOF
Extension: $final
Declares:  version=$declared  (matches Ghidra $ghidra_version)
Built from: $(git -C "$checkout_dir" rev-parse --short HEAD)

Install it:
  1. Ghidra: File > Install Extensions > + > select the zip above
  2. Restart Ghidra
  3. Import the PS2 main executable. Unlike Allegrex, no manual language
     selection is needed: the extension ships its own .opinion file matching
     the EE's ELF e_flags, so a plain import auto-detects language
     r5900:LE:32:default. No image-base fix is needed either - the LOAD
     segment's VirtAddr already places it correctly.

See docs/reverse-engineering/toolchain.md.
EOF
