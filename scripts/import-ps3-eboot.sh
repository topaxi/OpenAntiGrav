#!/usr/bin/env bash
#
# Imports a PS3 `EBOOT.elf` into the Ghidra project, headless and in the one
# order that works.
#
# Four things make a PS3 import easy to get subtly wrong, and this script
# exists so none of them has to be remembered:
#
#   1. The language must be `PowerPC:BE:64:A2ALT-32addr`. Ghidra's ELF loader
#      auto-detects `PowerPC:BE:64:A2ALT` - the 64-bit-address variant - which
#      imports cleanly and is not what Ps3GhidraScripts expects. Headless takes
#      a loader and a processor together, which the GUI's "recommended"
#      checkbox and GhidraMCP's `import_file` both make awkward. `--ps3-cspec`
#      imports under `PowerPC:BE:64:A2ALT-32addr-PS3` instead - the same
#      pspec, but a compiler spec and, since 2026-09-15, a sleigh spec that
#      also decodes the Cell `lvlx` (851 sites in Wipeout HD / Fury, each
#      holing the function it sits in under the stock language), both shipped
#      by the locally built Ps3GhidraScripts extension rather than as
#      root-owned edits to the Ghidra install. It is not the default flag
#      because the live `OpenAntiGrav.gpr` project was first imported under
#      the stock id, and switching meant a fresh reimport - safe per ADR-0005
#      (`just apply-names` reproduces the recovered names), but slow enough on
#      a 26,100-function binary that it should be a deliberate choice, not a
#      script default. The live project has been on --ps3-cspec since
#      2026-08-28. See docs/reverse-engineering/toolchain.md#ps3.
#   2. `AnalyzePs3Binary.java` must run *before* auto-analysis, not after. It
#      defines the imports, exports and TOC that analysis then works from.
#      `-preScript` is exactly that hook.
#   3. `AssignPs3R2FromOpd.java` must run *after* analysis, because it walks
#      functions that analysis creates and gives each the `r2` value its own OPD
#      entry declares. **Not optional.** Each function has its own TOC, and
#      without this Ghidra uses the entry point's for all of them, so every
#      TOC-relative load in every function with a different TOC resolves to the
#      wrong slot - yielding a real string at a real address that is not the one
#      the code loads. Upstream's README does not mention this script, and the
#      first sweep of this binary was made without it; see
#      docs/ghidra/functions/ps3-hdfury-eu/memory.md for the worked example.
#   4. `DefinePS3Syscalls.java` runs after that.
#
# The executable on the disc is a SELF and cannot be imported at all; if the
# decrypted ELF is missing this decrypts it first with RPCS3, which needs no
# firmware install.
#
# Usage:
#   scripts/import-ps3-eboot.sh [--eboot <path>] [--folder <name>] [--no-analysis] [--ps3-cspec]
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra
#   JAVA_21_HOME         JDK 21 home. Default: whatever Ghidra picks.
#
# **Close Ghidra first.** A running Ghidra holds the project lock and headless
# cannot open the same project; the script checks and stops rather than failing
# halfway through an import.

set -euo pipefail

GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"
LANGUAGE="PowerPC:BE:64:A2ALT-32addr"
CSPEC="default"
ps3_cspec=0

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project_name="OpenAntiGrav"

eboot="$project_root/data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"
folder="ps3-hdfury-eu"
analysis=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --eboot)  eboot="${2:?--eboot needs a path}"; shift 2 ;;
        --folder) folder="${2:?--folder needs a name}"; shift 2 ;;
        --no-analysis) analysis=0; shift ;;
        --ps3-cspec) ps3_cspec=1; shift ;;
        # Print the header comment block, however long it happens to be.
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

if [[ $ps3_cspec -eq 1 ]]; then
    LANGUAGE="PowerPC:BE:64:A2ALT-32addr-PS3"
fi

step "Checking prerequisites"

headless="$GHIDRA_INSTALL_DIR/support/analyzeHeadless"
[[ -x $headless ]] || die "$headless not found. Set GHIDRA_INSTALL_DIR."

# Installed extensions put their ghidra_scripts on the default script path, so
# finding one is how we know the extension is installed and enabled. Prune
# `*.oag-backup` dirs - this repo's own naming for a backup taken before
# reinstalling an extension or patching Ghidra in place - or a stale backup
# copy wins the search over the live one.
analyze_script="$(find "$HOME/.config/ghidra" "$GHIDRA_INSTALL_DIR/Ghidra/Extensions" \
    \( -name '*.oag-backup' -prune \) -o \
    -name AnalyzePs3Binary.java -print -quit 2>/dev/null || true)"
[[ -n $analyze_script ]] || die \
    "AnalyzePs3Binary.java not found. Install the Ps3GhidraScripts extension:
   see docs/reverse-engineering/toolchain.md#ps3"
echo "scripts: $(dirname "$analyze_script")"

if [[ $ps3_cspec -eq 1 ]]; then
    # The fix ships with the extension's own language now (ppc_64_32_ps3.cspec,
    # data/languages/ next to ghidra_scripts/ in the same install), so there is
    # nothing to check on the Ghidra install itself.
    ldefs="$(dirname "$(dirname "$analyze_script")")/data/languages/ppc_ps3.ldefs"
    [[ -f $ldefs ]] || die \
        "$ldefs not found. The installed Ps3GhidraScripts extension predates the
   PS3 language addition - run: just build-ps3-scripts, then reinstall it. See
   docs/reverse-engineering/toolchain.md#ps3."
    echo "language: $LANGUAGE ships from $ldefs"
else
    # The compiler-spec fix is not optional: without it the decompiler treats r2
    # as call-clobbered and invents TOC reloads, which reads as plausible output.
    if ! "$project_root/scripts/patch-ghidra-ppc-cspec.sh" --check >/dev/null 2>&1; then
        die "ppc_64_32.cspec is missing the r2 fix. Run: just patch-ppc-cspec"
    fi
    echo "cspec: r2 is in the unaffected list"
fi

if [[ -f "$project_root/$project_name.lock" ]]; then
    die "$project_name.lock exists - Ghidra has the project open. Close it and re-run."
fi

step "Locating the decrypted executable"

if [[ ! -f $eboot ]]; then
    self="${eboot%.elf}.BIN"
    [[ -f $self ]] || die "neither $eboot nor $self exists. Extract the disc first."
    command -v rpcs3 >/dev/null || die "$eboot is missing and rpcs3 is not installed to decrypt $self."
    echo "decrypting $self"
    rpcs3 --decrypt "$self"
    [[ -f $eboot ]] || die "rpcs3 ran but produced no $eboot."
fi

# A SELF that slipped through would import as garbage rather than fail, so
# check the magic rather than the extension.
magic="$(head -c 4 "$eboot" | xxd -p)"
[[ $magic == "7f454c46" ]] || die \
    "$eboot does not start with the ELF magic (got $magic). If it starts with
   53434500 it is still an encrypted SELF - run rpcs3 --decrypt on it."
echo "$eboot"

step "Importing into $project_name/$folder as $LANGUAGE"

args=(
    "$project_root" "$project_name/$folder"
    -import "$eboot"
    -loader ElfLoader
    -processor "$LANGUAGE"
    -cspec "$CSPEC"
    -overwrite
    -preScript AnalyzePs3Binary.java
)

if [[ $analysis -eq 1 ]]; then
    # Order matters between these two as well: syscall definitions are read
    # through the TOC, so r2 has to be assigned first.
    args+=(-postScript AssignPs3R2FromOpd.java -postScript DefinePS3Syscalls.java)
else
    # -noanalysis also skips the post-script's reason to exist: syscalls are
    # resolved against functions analysis has not created yet.
    args+=(-noanalysis)
fi

if [[ -n ${JAVA_21_HOME:-} ]]; then
    export JAVA_HOME="$JAVA_21_HOME"
fi

# Analysis of a 9.6 MB PPC64 executable is not quick; let it run rather than
# capping it, and let the caller background the whole script instead.
"$headless" "${args[@]}"

step "Done"

cat <<EOF
Reopen Ghidra and check, in this order:

  1. the language reads $LANGUAGE, not the -32addr-less variant
  2. the function count is in the thousands, not single digits
  3. imports carry real names (cellGcmSys, sceNp...) rather than FUN_ addresses

This is a separate project from any existing $project_name/$folder import under
the other language id - Ghidra does not migrate a program between languages.

Then apply this project's own names:

  scripts/apply-ghidra-names.py
EOF
