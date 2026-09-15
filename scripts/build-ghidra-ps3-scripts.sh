#!/usr/bin/env bash
#
# Builds the Ps3GhidraScripts extension for the installed Ghidra, with this
# project's PS3 language added to it: the compiler-spec fix and the Cell
# `lvlx` instruction.
#
# Ps3GhidraScripts defines the imports, exports and TOC assignment PS3 PPU
# code needs; see docs/reverse-engineering/toolchain.md#ps3. Upstream ships no
# language of its own - stock Ghidra decodes the Cell PPU already - but two
# things are wrong with the stock language for this target, and this script
# ships both fixes as a second language, `PowerPC:BE:64:A2ALT-32addr-PS3`,
# rather than as root-owned edits to the Ghidra install (a duplicate language
# id is not allowed, which is why this is not `PowerPC:BE:64:A2ALT-32addr`
# again):
#
#   1. A decompile needs `r2` added to the unaffected-register list of
#      `ppc_64_32.cspec` (otherwise scripts/patch-ghidra-ppc-cspec.sh's sudo
#      edit). `ppc_64_32_ps3.cspec` is that patched cspec.
#   2. Sleigh does not know the Cell/PPC970 `lvlx`, which Wipeout HD / Fury
#      uses 851 times, and each site holes the function it sits in. The fix is
#      a sleigh constructor (`cell_lvlx.sinc`), and a constructor can only be
#      added by compiling a whole spec - so this script copies stock Ghidra's
#      own PowerPC .sinc files and ppc_64_isa_altivec_be.slaspec out of the
#      local install at build time, generates ppc_64_isa_altivec_ps3.slaspec
#      (stock plus one `@include`), and compiles it with Ghidra's own sleigh
#      compiler. Same shape as scripts/build-ghidra-allegrex.sh's tracked
#      patch: nothing of Ghidra's is committed here.
#
# `ppc_64.pspec` is still stock Ghidra's, reused by filename - Ghidra's
# SleighLanguageProvider falls back to an application-wide search by filename
# when a referenced file is not found beside the .ldefs, as long as the
# filename is unique across the install.
#
# The added files (scripts/ghidra-ps3-language/) are this project's own and
# tracked here, not upstream's - Ps3GhidraScripts itself ships no
# data/languages/ directory. Whether to contribute them upstream is a separate,
# undecided question; ask before publishing anything to
# https://github.com/clienthax/Ps3GhidraScripts.
#
# Building from source rather than downloading a release is deliberate, same
# reasoning as scripts/build-ghidra-allegrex.sh: Ghidra matches an extension's
# declared version against its own exactly, and a source build always
# declares the right one.
#
# Usage:
#   scripts/build-ghidra-ps3-scripts.sh [--ref <git-ref>] [--clean]
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME         JDK 21 home. Default: autodetected.

set -euo pipefail

REPO_URL="https://github.com/clienthax/Ps3GhidraScripts.git"
GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checkout_dir="$project_root/data/tools/ps3-ghidra-scripts"
output_dir="$project_root/data/tools"
language_src="$project_root/scripts/ghidra-ps3-language"
stock_languages="$GHIDRA_INSTALL_DIR/Ghidra/Processors/PowerPC/data/languages"
stock_slaspec="ppc_64_isa_altivec_be.slaspec"
ps3_slaspec="ppc_64_isa_altivec_ps3.slaspec"
ps3_sla="${ps3_slaspec%.slaspec}.sla"

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

# The files this project adds. Fail early and clearly rather than let an
# empty data/languages/ ship silently.
for f in ppc_64_32_ps3.cspec ppc_ps3.ldefs cell_lvlx.sinc; do
    [[ -f $language_src/$f ]] \
        || die "$language_src is missing $f - checkout is incomplete."
done

# What the sleigh compile needs from the install: the stock spec its output is
# stock-plus-one-constructor of, and the compiler itself.
[[ -f $stock_languages/$stock_slaspec ]] \
    || die "$stock_languages/$stock_slaspec not found. Is $GHIDRA_INSTALL_DIR a Ghidra root with the PowerPC processor?"
sleigh="$GHIDRA_INSTALL_DIR/support/sleigh"
[[ -x $sleigh ]] || die "$sleigh not found or not executable."

# JDK 21 specifically. The upstream Gradle wrapper is 8.10.2, which does not
# support JDK 23 or newer.
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

step "Fetching Ps3GhidraScripts ($ref)"

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

step "Adding the PS3 language"

languages_dir="$checkout_dir/data/languages"
mkdir -p "$languages_dir"
cp -f "$language_src/ppc_64_32_ps3.cspec" "$language_src/ppc_ps3.ldefs" \
    "$language_src/cell_lvlx.sinc" "$languages_dir/"
echo "$languages_dir/ppc_64_32_ps3.cspec"
echo "$languages_dir/ppc_ps3.ldefs"
echo "$languages_dir/cell_lvlx.sinc"

step "Compiling the sleigh spec (stock $stock_slaspec plus cell_lvlx.sinc)"

# Sleigh resolves @include relative to the including file, so the stock
# sources have to sit beside the generated spec. Copying them from the install
# at build time is the whole reason nothing of Ghidra's needs tracking here.
cp -f "$stock_languages"/*.sinc "$stock_languages/$stock_slaspec" "$languages_dir/"
echo "copied $(find "$stock_languages" -maxdepth 1 -name '*.sinc' | wc -l) .sinc files and $stock_slaspec from $stock_languages"

# The only edit to the stock spec: one include, straight after altivec.sinc so
# the constructor sees the same vrD/RA_OR_ZERO/B/XOP_1_10 tables lvx uses.
grep -q '^@include "altivec.sinc"$' "$languages_dir/$stock_slaspec" \
    || die "$stock_slaspec has no '@include \"altivec.sinc\"' line to hook after - Ghidra's PowerPC layout changed; re-check cell_lvlx.sinc against it."
sed 's/^@include "altivec.sinc"$/@include "altivec.sinc"\n@include "cell_lvlx.sinc"/' \
    "$languages_dir/$stock_slaspec" > "$languages_dir/$ps3_slaspec"
grep -q '^@include "cell_lvlx.sinc"$' "$languages_dir/$ps3_slaspec" \
    || die "generated $ps3_slaspec does not include cell_lvlx.sinc"
# The stock spec was only the sed source; shipping a second copy of it under
# the extension would be one more file Ghidra could find by name.
rm -f "$languages_dir/$stock_slaspec"

# The warnings sleigh prints are stock Ghidra's own (unreferenced tables in
# ppc_common.sinc and the like); the constructor adds none - checked once by
# compiling the stock spec beside it and diffing, see toolchain.md#ps3.
"$sleigh" "$languages_dir/$ps3_slaspec" \
    || die "sleigh failed to compile $ps3_slaspec - see the errors above."
[[ -f $languages_dir/$ps3_sla ]] \
    || die "sleigh reported success but produced no $ps3_sla"

# Ghidra recompiles a .sla it finds older than its .slaspec, so make sure the
# one just built is the newer of the two.
touch "$languages_dir/$ps3_sla"
echo "$languages_dir/$ps3_sla"

step "Building"

env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME="$java_home" \
    GHIDRA_INSTALL_DIR="$GHIDRA_INSTALL_DIR" \
    "$checkout_dir/gradlew" \
        --project-dir "$checkout_dir" \
        --no-daemon \
        buildExtension

step "Verifying output"

zip_path="$(find "$checkout_dir/dist" -name '*Ps3GhidraScripts.zip' -newermt '-1 hour' | sort | tail -1)"
[[ -n $zip_path && -f $zip_path ]] || die "build reported success but produced no zip in $checkout_dir/dist"

declared="$(unzip -p "$zip_path" '*/extension.properties' 2>/dev/null \
    | sed -n 's/^version=//p' | head -1 | tr -d '\r[:space:]')"
[[ -n $declared ]] || die "no version found in the extension.properties inside $zip_path"

if [[ $declared != "$ghidra_version" ]]; then
    die "built extension declares version '$declared' but Ghidra is '$ghidra_version'.
   Ghidra will refuse to load it. This should not happen with a source build."
fi

# The whole point of this script over a plain buildExtension is that the
# language files actually made it in - check the zip, not just the source tree.
for entry in "data/languages/ppc_64_32_ps3.cspec" "data/languages/ppc_ps3.ldefs" \
             "data/languages/cell_lvlx.sinc" "data/languages/$ps3_sla"; do
    unzip -l "$zip_path" | grep -qF "$entry" \
        || die "$zip_path was built without $entry - check $checkout_dir/data/languages/."
done

mkdir -p "$output_dir"
final="$output_dir/$(basename "$zip_path")"
cp -f "$zip_path" "$final"

step "Done"

cat <<EOF
Extension: $final
Declares:  version=$declared  (matches Ghidra $ghidra_version)
Built from: $(git -C "$checkout_dir" rev-parse --short HEAD), plus this project's
  scripts/ghidra-ps3-language/{ppc_64_32_ps3.cspec,ppc_ps3.ldefs,cell_lvlx.sinc}
  and $ps3_sla compiled from Ghidra $ghidra_version's own PowerPC sources

Install it:
  1. Ghidra: File > Install Extensions > + > select the zip above
     (or replace the unzipped extension folder directly, if Ghidra is not running)
  2. Restart Ghidra

This adds a second language, PowerPC:BE:64:A2ALT-32addr-PS3, alongside the
scripts: the r2 cspec fix plus the Cell lvlx instruction. It does not touch
the Ghidra install itself, so scripts/patch-ghidra-ppc-cspec.sh's fix is still
separately needed for imports that use the stock PowerPC:BE:64:A2ALT-32addr
language (which never decodes lvlx) - see
docs/reverse-engineering/toolchain.md#ps3 and scripts/import-ps3-eboot.sh
--help. A program imported under the PS3 id before lvlx landed keeps its
undefined sites until it is reimported.
EOF
