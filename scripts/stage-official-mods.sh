#!/usr/bin/env bash
#
# Copies the official mods (assets/mods/official, the OpenAntiGrav/official-mods
# submodule) into <dest>, one folder per mod, for a packaged build to ship in a
# `mods` directory that oag_mod's search path reaches (ADR-0083 decision 5,
# docs/tools/packaging.md, "Official mods").
#
# Usage: scripts/stage-official-mods.sh <dest>
#
# Fails when the submodule is not checked out or holds no mod: a release
# without its official mods is a build error, not a quiet omission.

set -euo pipefail

dest="${1:?usage: stage-official-mods.sh <dest>}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$root/assets/mods/official"

count=0
mkdir -p "$dest"
for manifest in "$src"/*/oag-mod.toml; do
    [[ -f $manifest ]] || continue
    mod_dir="$(dirname "$manifest")"
    rm -rf "${dest:?}/$(basename "$mod_dir")"
    cp -r "$mod_dir" "$dest/"
    count=$((count + 1))
done
(( count > 0 )) || {
    echo "error: no official mod in $src; run: git submodule update --init assets/mods/official" >&2
    exit 1
}
echo "staged $count official mod(s) into $dest"
