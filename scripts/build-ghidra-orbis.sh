#!/usr/bin/env bash
#
# Builds GhidraOrbis, the Ghidra loader/analyzer extension for PS4 (Orbis OS)
# binaries, for the installed Ghidra.
#
# Stock Ghidra already decodes the PS4's CPU correctly - it is an ordinary
# AMD64 core, unlike the PSP's Allegrex/VFPU or the PS2's Emotion Engine,
# neither of which stock Ghidra handles without a processor module. What is
# missing is everything Orbis-specific layered on top of a standard ELF:
# `orbis.elf.OrbisElfExtension` teaches Ghidra the SCE-specific program
# header types and dynamic tags (`PT_SCE_DYNLIBDATA`, `DT_SCE_STRTAB_TAG`,
# and friends) that the stock ELF loader has no entry for and would
# otherwise skip silently, and `orbis.loader.GhidraOrbisSelfLoader` parses
# the SELF (Signed ELF) wrapper every PS4 executable ships in.
#
# The SELF loader only handles a SELF whose segments are already plaintext -
# it reads each segment's own ENCRYPTED bit and throws EncryptedSelfException
# if any segment is still set, rather than attempting a decrypt itself. That
# matters directly for this project: an `eboot.bin` recovered via
# `data/README.md`'s Omega Collection section (LibOrbisPkg's `pkg_extract`)
# is still SELF-wrapped, but for that scene fPKG build all 10 of its segments
# were checked directly (a small Python parse of the SELF/extended headers,
# not a guess) and every one has its ENCRYPTED bit clear - so this loader
# should be able to import it with no separate decrypt step, unlike a retail
# SELF or Vita's own eboot.bin (see the Vita section above, which does need
# `vita-self-decrypt.py` first). Not yet confirmed inside Ghidra itself.
#
# Ghidra-Cpp-Class-Analyzer (also astrelsky) is listed upstream as needed to
# build a couple of optional features, but nothing under GhidraOrbis's own
# `src/` imports it (checked directly - not present in this build) and it is
# not required to use the loader. Not built by this script.
#
# Same shape as build-vita-loader-redux.sh: upstream ships no Gradle wrapper
# of its own, so this borrows the one from the Allegrex checkout, meaning
# scripts/build-ghidra-allegrex.sh (or this script) must have run at least
# once first to populate data/tools/ghidra-allegrex. GhidraOrbis's own
# build.gradle declares `sourceCompatibility`/`targetCompatibility` 17, but
# that is a lower bytecode target than the JDK 21 the Allegrex wrapper (and
# this Ghidra install) requires to run at all - javac happily compiles
# down-level, so JDK 21 is used throughout rather than hunting for a second
# JDK.
#
# extension.properties ships with @extname@/@extversion@ template
# placeholders, resolved automatically by buildExtension.gradle from the
# project directory name and the target Ghidra's own version - same as
# VitaLoaderRedux, no preprocessing needed.
#
# Installation is manual, per the user's own preference - this script only
# builds the zip. Ghidra: File > Install Extensions > + > select the zip.
#
# Default ref is the last tagged release, not master: master's HEAD as of
# 2026-06-24 (PR #29, "fix/28-null-function-npe") introduced a genuine
# compile error, confirmed directly by building both - StackChkFailAnalyzer
# .java's `defineFunction`, a `private static` method, calls the non-static
# `getName()` for a log message, which javac rejects regardless of Ghidra
# version (`non-static method getName() cannot be referenced from a static
# context`). Tag 1.0144 (the release before that merge) does not have the
# call and builds clean. Pass `--ref master` to try again once upstream
# fixes it - check first, this note will go stale.
#
# Usage:
#   scripts/build-ghidra-orbis.sh [--ref <git-ref>] [--clean]
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME         JDK 21 home. Default: autodetected.
#   GHIDRA_ORBIS_DIR      Checkout + build location. Default: data/tools/GhidraOrbis

set -euo pipefail

REPO_URL="https://github.com/astrelsky/GhidraOrbis.git"
ALLEGREX_REPO_URL="https://github.com/kotcrab/ghidra-allegrex.git"
GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="${GHIDRA_ORBIS_DIR:-$project_root/data/tools/GhidraOrbis}"
allegrex_dir="$project_root/data/tools/ghidra-allegrex"
output_dir="$project_root/data/tools"

ref="1.0144"
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

# README states 10.0+; the loader/extension source has no upper-bound checks
# of its own, so this is a floor, not a ceiling.
ghidra_major="${ghidra_version%%.*}"
if [[ $ghidra_major -lt 10 ]]; then
    echo "warning: GhidraOrbis requires Ghidra 10.0+; $ghidra_version may not build or load." >&2
fi

# JDK 21 - same wrapper constraint as Allegrex/VitaLoaderRedux, since this
# project borrows that wrapper. GhidraOrbis's own declared source/target of
# 17 still compiles fine under it.
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

step "Fetching GhidraOrbis ($ref)"

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

zip_path="$(find "$checkout_dir/dist" -name '*GhidraOrbis.zip' -newermt '-1 hour' | sort | tail -1)"
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
  3. Import a PS4 eboot.bin/SELF whose segments are already plaintext (see
     data/README.md's Omega Collection section for getting one via
     LibOrbisPkg's pkg_extract) - GhidraOrbisSelfLoader/GhidraOrbisElfLoader
     register themselves for Orbis SELF/ELF files automatically. A SELF with
     any segment still flagged encrypted will fail to import
     (EncryptedSelfException); no decrypt step for that case is wired up
     here yet.

See the extension's own README (data/tools/GhidraOrbis/README.md) and
docs/reverse-engineering/toolchain.md#ps4 for more.
EOF
