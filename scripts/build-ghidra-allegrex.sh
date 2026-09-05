#!/usr/bin/env bash
#
# Builds the Allegrex processor module for the installed Ghidra.
#
# Stock Ghidra has no PSP VFPU support and silently mis-decodes vector
# instructions as nonexistent 64-bit MIPS III ones. See
# docs/psp/allegrex-vfpu.md for why that is worse than it sounds.
#
# Building from source rather than downloading a release is deliberate:
# Ghidra matches an extension's declared version against its own exactly, and
# upstream publishes no build for every point release. Ghidra's own
# buildExtension.gradle stamps the version from the installation being built
# against, so a source build always declares the right one.
#
# One local patch is applied before building:
# scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch. Without it
# a PSP import silently loads with **no relocations applied at all** whenever
# ghidra-emotionengine-reloaded is installed alongside - that module's
# EE_ElfExtension declares a higher extension-point priority and inherits stock
# MIPS_ElfExtension's "any EM_MIPS" claim, so it wins adapter selection for PSP
# files and Allegrex's own relocation pass never runs. The patch file's header
# has the full mechanism; docs/ghidra/workflow.md has the measurements.
#
# Because a patch has to apply, --ref defaults to the exact commit it was
# written against rather than to master. Moving to a newer upstream means
# re-verifying the patch applies and re-measuring the import.
#
# Usage:
#   scripts/build-ghidra-allegrex.sh [--ref <git-ref>] [--clean] [--no-patch]
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME         JDK 21 home. Default: autodetected.

set -euo pipefail

REPO_URL="https://github.com/kotcrab/ghidra-allegrex.git"
GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"

# The exact commit the patch in scripts/patches/ was written and verified
# against.
BASE_COMMIT="aec4265"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="$project_root/data/tools/ghidra-allegrex"
output_dir="$project_root/data/tools"

patch_path="$project_root/scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch"

ref="$BASE_COMMIT"
clean=0
apply_patch=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ref)   ref="${2:?--ref needs a value}"; shift 2 ;;
        --clean) clean=1; shift ;;
        --no-patch) apply_patch=0; shift ;;
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

# JDK 21 specifically. The upstream Gradle wrapper is 8.10.2, which does not
# support JDK 23 or newer, and Kotlin here declares jvmToolchain(21). Building
# with a newer default JDK fails with an unhelpful Gradle error.
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
   The Gradle wrapper this project uses does not support newer JDKs."

echo "JDK 21 at $java_home"
"$java_home/bin/java" -version 2>&1 | head -1

step "Fetching ghidra-allegrex ($ref)"

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

if [[ $apply_patch -eq 1 ]]; then
    step "Applying $(basename "$patch_path")"

    [[ -f $patch_path ]] || die "$patch_path not found."

    # `git clean -qfd` above resets the tree, so this always applies to a
    # pristine checkout. A failure here means the ref moved away from the
    # commit the patch was written against - see the patch file's header.
    if ! patch -d "$checkout_dir" -p1 --dry-run -i "$patch_path" >/dev/null 2>&1; then
        die "$(basename "$patch_path") does not apply to $ref.
   It was written against $BASE_COMMIT. Either build that ref, rebase the patch
   and re-measure a PSP import, or pass --no-patch to build without the fix
   (which leaves PSP imports with no relocations applied - see the patch header)."
    fi
    patch -d "$checkout_dir" -p1 -i "$patch_path"
else
    step "Skipping the local patch (--no-patch)"
fi

step "Building"

# GHIDRA_INSTALL_DIR is what build.gradle reads to locate buildExtension.gradle,
# which is also what stamps the extension version.
env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME="$java_home" \
    GHIDRA_INSTALL_DIR="$GHIDRA_INSTALL_DIR" \
    "$checkout_dir/gradlew" \
        --project-dir "$checkout_dir" \
        --no-daemon \
        buildExtension

step "Verifying output"

zip_path="$(find "$checkout_dir/dist" -name '*ghidra-allegrex.zip' -newermt '-1 hour' | sort | tail -1)"
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

if [[ $apply_patch -eq 1 ]]; then
    patch_state="with $(basename "$patch_path")"
else
    patch_state="WITHOUT the local patch - PSP imports will load with no
            relocations applied if ghidra-emotionengine-reloaded is also
            installed. See docs/ghidra/workflow.md."
fi

cat <<EOF
Extension: $final
Declares:  version=$declared  (matches Ghidra $ghidra_version)
Built from: $(git -C "$checkout_dir" rev-parse --short HEAD)
Patched:   $patch_state

Install it:
  1. Ghidra: File > Install Extensions > + > select the zip above
  2. Restart Ghidra
  3. Re-import BOOT.BIN. The processor language is fixed at import time, so
     re-analysing the existing program will not pick up Allegrex.
       - choose the Allegrex language, or the PSP loader if offered
       - set the image base to the module's runtime address (read it from
         PPSSPP rather than assuming 0x08804000)

See docs/psp/allegrex-vfpu.md.
EOF
