#!/usr/bin/env bash
#
# Adds `r2` to the unaffected-register list of Ghidra's PowerPC 64/32-addr
# compiler spec, which is what PS3 PPU code needs to decompile correctly.
#
# On the PS3 the PPU's r2 holds the TOC pointer and a call does not clobber it.
# Ghidra's stock `ppc_64_32.cspec` does not say so, so the decompiler assumes
# every call destroys r2 and invents reloads of it all over the output. That is
# worse than a hard failure: the listing looks fine and the decompilation is
# quietly wrong. Upstream Ps3GhidraScripts documents the change as required.
#
# The edit is one line, but it lives inside the Ghidra installation rather than
# in this repository, so it does not survive reinstalling or upgrading Ghidra -
# on Arch, `pacman -Syu` puts the stock file back without saying anything. Run
# `--check` after a Ghidra upgrade; re-running the script is safe at any time.
#
# Usage:
#   scripts/patch-ghidra-ppc-cspec.sh            apply the change
#   scripts/patch-ghidra-ppc-cspec.sh --check    report status, change nothing
#   scripts/patch-ghidra-ppc-cspec.sh --revert    restore the backup
#
# `--check` exits 0 if the change is present, 1 if it is not, so it can gate a
# post-upgrade script. Applying needs write access to the Ghidra installation;
# the script re-runs itself under sudo when it needs to and asks first.
#
# Environment:
#   GHIDRA_INSTALL_DIR   Ghidra root. Default: /opt/ghidra

set -euo pipefail

GHIDRA_INSTALL_DIR="${GHIDRA_INSTALL_DIR:-/opt/ghidra}"
RELATIVE_CSPEC="Ghidra/Processors/PowerPC/data/languages/ppc_64_32.cspec"
REGISTER_LINE='        <register name="r2"/>'

self="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"

mode="apply"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)  mode="check"; shift ;;
        --revert) mode="revert"; shift ;;
        # Print the header comment block, however long it happens to be.
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

cspec="$GHIDRA_INSTALL_DIR/$RELATIVE_CSPEC"
backup="$cspec.oag-backup"

[[ -d $GHIDRA_INSTALL_DIR ]] \
    || die "GHIDRA_INSTALL_DIR=$GHIDRA_INSTALL_DIR does not exist. Install Ghidra, or set GHIDRA_INSTALL_DIR."
[[ -f $cspec ]] \
    || die "$cspec not found. Is GHIDRA_INSTALL_DIR really a Ghidra root?"

# Only the one block, and only the one the PS3 language actually uses. If a
# future Ghidra restructures this file, stop rather than edit the wrong list.
is_patched() {
    sed -n '/<unaffected>/,/<\/unaffected>/p' "$cspec" | grep -q 'name="r2"'
}

check_shape() {
    local blocks
    blocks="$(grep -c '<unaffected>' "$cspec" || true)"
    [[ $blocks -eq 1 ]] || die \
        "expected exactly one <unaffected> block in $cspec, found $blocks.
   Ghidra's compiler spec has changed shape; patch it by hand and update this script."
}

# The .cspec is read when a program is opened, so an edit under a running
# Ghidra is at best ignored and at worst half-applied to an open project.
warn_if_running() {
    if pgrep -f 'ghidra.*GhidraRun|ghidra.GhidraRun' >/dev/null 2>&1; then
        echo "warning: Ghidra looks like it is running. Close it, then re-run," >&2
        echo "         or the change will not be picked up." >&2
    fi
}

# A malformed cspec makes Ghidra refuse the whole PowerPC language, so never
# leave one behind. python3 is what the rest of this repository's checks use.
validate_xml() {
    local file="$1"
    if command -v python3 >/dev/null; then
        python3 -c 'import sys,xml.etree.ElementTree as e; e.parse(sys.argv[1])' "$file" \
            || return 1
    else
        echo "warning: python3 not found, skipping the XML well-formedness check" >&2
    fi
}

# Takes the arguments to re-run with, not the script's own: the caller passes
# the flags that got it here, so a sudo re-run lands in the same mode.
need_write() {
    [[ -w $cspec ]] && return 0
    [[ $EUID -eq 0 ]] && die "$cspec is not writable even as root."
    command -v sudo >/dev/null || die "$cspec needs root to write and sudo is not installed."
    echo "$cspec belongs to root; re-running this script under sudo."
    exec sudo --preserve-env=GHIDRA_INSTALL_DIR -- "$self" "$@"
}

case "$mode" in
check)
    check_shape
    if is_patched; then
        echo "patched: r2 is in the unaffected list of $cspec"
        exit 0
    fi
    echo "NOT patched: r2 is missing from the unaffected list of $cspec"
    echo "run: scripts/patch-ghidra-ppc-cspec.sh"
    exit 1
    ;;

revert)
    [[ -f $backup ]] || die "no backup at $backup - nothing to revert to."
    need_write --revert
    warn_if_running
    cp -- "$backup" "$cspec"
    echo "restored $cspec from $backup"
    exit 0
    ;;
esac

step "Checking $cspec"

check_shape

if is_patched; then
    echo "already patched; nothing to do."
    exit 0
fi

need_write
warn_if_running

step "Backing up"

# Always taken from the unpatched file, since this point is only reached when
# the file is unpatched - so a backup written after a Ghidra upgrade is the new
# stock file, not a stale copy of the old one.
cp -- "$cspec" "$backup"
echo "$backup"

step "Adding r2 to the unaffected list"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

awk -v ins="$REGISTER_LINE" '
    /<\/unaffected>/ && !done { print ins; done = 1 }
    { print }
' "$cspec" > "$tmp"

validate_xml "$tmp" || {
    die "the edited file is not well-formed XML; $cspec is untouched."
}

# Written through the existing file rather than moved over it, so the owner and
# mode Ghidra's installer set are the ones that survive.
cat -- "$tmp" > "$cspec"

step "Verifying"

is_patched || die "the edit did not take; $cspec is unchanged in the way that matters."
validate_xml "$cspec" || die "$cspec is not well-formed after the edit. Restore it with --revert."

sed -n '/<unaffected>/,/<\/unaffected>/p' "$cspec" | grep -n 'name="r2"'
echo
echo "done. Restart Ghidra if it is open - the compiler spec is read when a"
echo "program is opened, so an already-open program keeps the old one."
