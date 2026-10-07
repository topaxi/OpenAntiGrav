#!/usr/bin/env bash
#
# Builds the portable AppImage and copies it onto a Steam Deck (or any Linux
# box reachable over ssh). It lands on the remote user's Desktop, ready to
# double-click or run from a terminal.
#
# No game data goes over here: `just push-data deck` (scripts/push-game-data.sh)
# picks which disc images, DLC, 2048/Omega packages and key tables to copy, and
# shows which are already there. It puts them where `oag-game`'s own search
# path looks without any flag (`<XDG_DATA_HOME>/oag/...`; see
# crates/source/src/source.rs and docs/tools/packaging.md#where-the-disc-image-comes-from),
# so a fresh Deck needs nothing set to find them.
#
# Usage:
#   scripts/deploy-to-deck.sh [host] [--skip-build] [--dry-run] [--scp]
#
#   host          user@host to deploy to. Default: $OAG_DECK_HOST or
#                 deck@steamdeck.
#   --skip-build  Don't rebuild; sync whatever is already in data/appimage/
#                 OpenAntiGrav-x86_64-portable.AppImage.
#   --dry-run     Pass --dry-run to rsync, or print what scp would copy.
#                 Touches nothing, locally or on the remote, beyond the ssh
#                 probes needed to resolve paths.
#   --scp         Force scp instead of probing the remote for rsync.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
appimage="$project_root/data/appimage/OpenAntiGrav-x86_64-portable.AppImage"

host="${OAG_DECK_HOST:-deck@steamdeck}"
skip_build=0
dry_run=0
force_scp=0
host_given=0

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die()  { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build) skip_build=1; shift ;;
        --dry-run)    dry_run=1; shift ;;
        --scp)        force_scp=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        -*) die "unknown argument: $1" ;;
        *)
            (( host_given )) && die "unexpected extra argument: $1"
            host="$1"; host_given=1; shift ;;
    esac
done

if (( skip_build )); then
    [[ -f $appimage ]] || die "$appimage not found. Run 'just appimage-portable' first, or drop --skip-build."
else
    step "Building the portable AppImage"
    "$project_root/scripts/build-appimage.sh" --container
    [[ -f $appimage ]] || die "$appimage still missing after the build - see the output above."
fi

step "Resolving paths on $host"
remote_home="$(ssh "$host" 'echo "$HOME"')"
[[ -n $remote_home ]] || die "could not read \$HOME on $host"
desktop_dir="$remote_home/Desktop"
echo "AppImage -> $desktop_dir/"

if (( dry_run )); then
    echo "(--dry-run: not creating remote directories or transferring anything)"
else
    ssh "$host" mkdir -p "$desktop_dir"
fi

use_rsync=0
if (( ! force_scp )) && ssh "$host" 'command -v rsync' >/dev/null 2>&1; then
    use_rsync=1
fi

step "Copying the AppImage ($( (( use_rsync )) && echo rsync || echo scp ))"
if (( use_rsync )); then
    dry_flag=()
    (( dry_run )) && dry_flag=(--dry-run)
    rsync -avzc "${dry_flag[@]}" "$appimage" "$host:$desktop_dir/"
elif (( dry_run )); then
    echo "would run: scp '$appimage' '$host:$desktop_dir/'"
else
    scp "$appimage" "$host:$desktop_dir/"
fi

if ! (( dry_run )); then
    # Not relied on above: plain scp does not preserve the executable bit, and
    # this is cheap insurance even when rsync -a already carried it over.
    ssh "$host" chmod +x "$desktop_dir/$(basename "$appimage")"
fi

step "Done"
echo "AppImage: $desktop_dir/$(basename "$appimage")"
echo "Game data: just push-data deck $host"
echo
echo "Run it: ssh $host '$desktop_dir/$(basename "$appimage")'"
