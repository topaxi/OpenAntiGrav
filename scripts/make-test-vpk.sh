#!/usr/bin/env bash
#
# Builds test `.vpk` files from the extracted Wipeout 2048 package folders, with
# the system `zip` (an independent ZIP writer, so the reader is not tested against
# its own idea of the format). Writes into data/test-vpk/, which is git-ignored.
#
# A real NoNpDrm `.vpk` is a ZIP of an installed game's folder, which is what
# data/extracted/vita/<TITLE_ID>/base is: plain files, `eboot.bin` still an NpDrm
# SELF. The base and the DLC are stored (method 0, what a multi-gigabyte
# archive wants); the patch and the second DLC are deflated, so both paths of
# the reader run. `eboot.elf` is this project's own Ghidra output and is left out.
#
# Usage: scripts/make-test-vpk.sh [<TITLE_ID, default PCSF00007>]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data="$root/data"
[ -L "$data" ] && data="$(readlink -f "$data")"
title="${1:-PCSF00007}"
from="$data/extracted/vita/$title"
out="$data/test-vpk"
[ -d "$from/base" ] || { echo "error: $from/base is missing" >&2; exit 1; }
mkdir -p "$out"

pack() { # <folder> <level> <file>
    rm -f "$3"
    (cd "$1" && zip -q -r -"$2" -X "$3" . -x eboot.elf)
}
pack "$from/base" 0 "$out/2048-$title.vpk"
[ -d "$from/patch-v104" ] && pack "$from/patch-v104" 6 "$out/2048-$title-patch.vpk"
[ -d "$from/dlc1" ] && pack "$from/dlc1" 0 "$out/2048-$title-dlc1.vpk"
[ -d "$from/dlc2" ] && pack "$from/dlc2" 6 "$out/2048-$title-dlc2.vpk"
ls -l "$out"
