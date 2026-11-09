#!/usr/bin/env bash
# Install a Wipeout HD update chain with RPCS3's own installer into a private
# profile and copy the result where the engine and the ground-truth tests read it.
#
#   scripts/install-hd-updates.sh data/images/hdfury-updates BCES00664
#   scripts/install-hd-updates.sh data/images/hd-psn-updates NPEA00057
#
# Writes data/extracted/ps3-update-chain/<serial>/installed/ (dev_hdd0/game/<serial>
# after the whole chain) and <serial>/<version>/ (each package's own archives), which
# the chain's ground-truth test reads. A player copies the installed folder instead,
# see docs/overview/installing.md. RPCS3 aborts on exit after an install; the files land.
# Muted and windowless: --headless opens no window and installing plays nothing.
set -euo pipefail

packages=$1
serial=$2
root=${OAG_DATA:-data}
profile=$root/scratch/hd-update-install/$serial

rm -rf "$profile"
mkdir -p "$profile/rpcs3"
game=$PWD/$profile/rpcs3/dev_hdd0/game/$serial

for package in $(find "$packages" -name 'update-*.pkg' | sort -V); do
  version=$(basename "$package" .pkg)
  version=${version#update-}
  before=$PWD/$profile/before
  rm -rf "$before"
  mkdir -p "$before"
  if [ -d "$game/USRDIR" ]; then
    find "$game/USRDIR" -maxdepth 1 -name '*.psarc' -exec sh -c \
      'for f; do cp "$f" "$0/"; done' "$before" {} +
  fi
  XDG_CONFIG_HOME=$PWD/$profile XDG_CACHE_HOME=$PWD/$profile/cache XDG_DATA_HOME=$PWD/$profile/share \
    rpcs3 --headless --installpkg "$package" >"$profile/$version.log" 2>&1 || true
  [ -f "$game/PARAM.SFO" ] || { echo "$version: nothing installed, see $profile/$version.log" >&2; exit 1; }

  chain=$root/extracted/ps3-update-chain/$serial/$version
  mkdir -p "$chain"
  cp "$game/PARAM.SFO" "$chain/"
  for archive in "$game"/USRDIR/*.psarc; do
    [ -e "$archive" ] || continue
    name=$(basename "$archive")
    cmp -s "$archive" "$before/$name" || cp "$archive" "$chain/"
  done
  echo "$version: $(cd "$chain" && echo *)"
done

# The chain as it stands installed: a disc's update folder, or a PSN install with
# its base archives linked in so the base folder itself stays unpatched.
target=$root/extracted/ps3-update-chain/$serial/installed
rm -rf "$target"
cp -r "$game" "$target"
if [ "$serial" = NPEA00057 ]; then
  base=$(readlink -f "$root/extracted/ps3/hd-psn-eu/USRDIR")
  for archive in data01 data02 data03; do
    ln -s "$base/$archive.psarc" "$target/USRDIR/$archive.psarc"
  done
fi
rm -rf "$profile"
