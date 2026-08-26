#!/usr/bin/env bash
#
# Builds VitaLoaderRedux, the Ghidra ELF/PRX loader for PS Vita binaries, for
# the installed Ghidra.
#
# Stock Ghidra loads a Vita ELF/SELF as a generic ARM/Thumb binary with none
# of the Vita-specific handling this extension adds: NID-based import/export
# resolution against known module databases, .sceModuleInfo parsing, segment
# permission fixups, and more - see its README for the full list. Needed
# before Ghidra can make sense of the eboot.bin recovered by
# scripts/build-psvpfstools.sh; see
# data/README.md#the-ps3-and-vita-images-are-encrypted-and-nothing-here-decrypts-them-yet.
#
# Unlike Allegrex or the PS2/PS3 extensions, upstream ships no Gradle
# wrapper of its own - its README says to install Gradle system-wide and run
# `gradle`. This project borrows the Allegrex checkout's wrapper instead
# (same approach as build-ghidra-emotionengine.sh), so
# scripts/build-ghidra-allegrex.sh (or this script) must have run at least
# once first to populate data/tools/ghidra-allegrex.
#
# extension.properties ships with @extname@/@extversion@ template
# placeholders committed as-is; buildExtension.gradle resolves them itself
# from the project directory name and the target Ghidra's own version, so no
# preprocessing step is needed before building (checked directly: the built
# zip's extension.properties comes out with real values, not the literal
# placeholders).
#
# Installation is manual, per the user's own preference - this script only
# builds the zip. Ghidra: File > Install Extensions > + > select the zip.
#
# Usage:
#   scripts/build-vita-loader-redux.sh [--ref <git-ref>] [--clean]
#
# Environment:
#   GHIDRA_INSTALL_DIR      Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME            JDK 21 home. Default: autodetected.
#   VITA_LOADER_REDUX_DIR   Checkout + build location. Default: data/tools/VitaLoaderRedux

set -euo pipefail

REPO_URL="https://github.com/CreepNT/VitaLoaderRedux.git"
ALLEGREX_REPO_URL="https://github.com/kotcrab/ghidra-allegrex.git"
GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="${VITA_LOADER_REDUX_DIR:-$project_root/data/tools/VitaLoaderRedux}"
allegrex_dir="$project_root/data/tools/ghidra-allegrex"
output_dir="$project_root/data/tools"

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

# Upstream has never supported below Ghidra 10.3, and 1.09+ requires 12.0+.
ghidra_major="${ghidra_version%%.*}"
if [[ $ghidra_major -lt 12 ]]; then
    echo "warning: VitaLoaderRedux requires Ghidra 12.0+; $ghidra_version may not build or load." >&2
fi

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

step "Fetching VitaLoaderRedux ($ref)"

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
git -C "$checkout_dir" clean -qfd

echo "at $(git -C "$checkout_dir" rev-parse --short HEAD) ($(git -C "$checkout_dir" log -1 --format=%cs))"

step "Building"

env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME="$java_home" \
    GHIDRA_INSTALL_DIR="$GHIDRA_INSTALL_DIR" \
    "$allegrex_dir/gradlew" \
        --project-dir "$checkout_dir" \
        --no-daemon \
        buildExtension

step "Verifying output"

zip_path="$(find "$checkout_dir/dist" -name '*VitaLoaderRedux.zip' -newermt '-1 hour' | sort | tail -1)"
[[ -n $zip_path && -f $zip_path ]] || die "build reported success but produced no zip in $checkout_dir/dist"

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
Built from: $(git -C "$checkout_dir" rev-parse --short HEAD), $(git -C "$checkout_dir" log -1 --format=%cs)

Install it (manual, per your own preference - this script only builds):
  1. Ghidra: File > Install Extensions > + > select the zip above
  2. Restart Ghidra
  3. Import a decrypted Vita eboot.bin/.suprx (see
     scripts/build-psvpfstools.sh for getting one) - VitaLoaderRedux
     registers itself as a loader for Vita ELF/SELF files automatically.

See the extension's own README (data/tools/VitaLoaderRedux/README.md) for
NID database setup and known-issue scripts like FixupVLRImportThunks.java.
EOF
