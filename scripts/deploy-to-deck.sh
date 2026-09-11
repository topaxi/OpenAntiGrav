#!/usr/bin/env bash
#
# Builds the portable AppImage and copies it, plus whatever's under
# data/images/, data/dlc/, data/extracted/vita/ and Pure's DLC key table,
# onto a Steam Deck (or any Linux box reachable over ssh).
#
# The AppImage lands on the remote user's Desktop, ready to double-click or run
# from a terminal. The data lands where `oag-game`'s own search path already
# looks without any flag: `<XDG_DATA_HOME>/oag/images`, `<XDG_DATA_HOME>/oag/dlc`,
# `<XDG_DATA_HOME>/oag/extracted/vita` and `<XDG_DATA_HOME>/oag/keys/pure-dlc-keys.txt`
# (see crates/game/src/source.rs, crates/game/src/dlc.rs's
# `default_pure_dlc_keys_path` and docs/tools/packaging.md#where-the-disc-image-comes-from)
# - so a fresh Deck needs nothing set to find them.
#
# Usage:
#   scripts/deploy-to-deck.sh [host] [--skip-build] [--no-data] [--dry-run] [--scp]
#
#   host          user@host to deploy to. Default: $OAG_DECK_HOST or
#                 deck@steamdeck.
#   --skip-build  Don't rebuild; sync whatever is already in data/appimage/
#                 OpenAntiGrav-x86_64-portable.AppImage.
#   --no-data     Only sync the AppImage; skip data/images, data/dlc,
#                 data/extracted/vita and the DLC key table.
#   --dry-run     Pass --dry-run to rsync, or print what scp would copy.
#                 Touches nothing, locally or on the remote, beyond the ssh
#                 probes needed to resolve paths.
#   --scp         Force scp instead of probing the remote for rsync.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
appimage="$project_root/data/appimage/OpenAntiGrav-x86_64-portable.AppImage"

host="${OAG_DECK_HOST:-deck@steamdeck}"
skip_build=0
sync_data=1
dry_run=0
force_scp=0
host_given=0

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die()  { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build) skip_build=1; shift ;;
        --no-data)    sync_data=0; shift ;;
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
remote_data_home="$(ssh "$host" 'echo "${XDG_DATA_HOME:-$HOME/.local/share}"')"
desktop_dir="$remote_home/Desktop"
images_dir="$remote_data_home/oag/images"
dlc_dir="$remote_data_home/oag/dlc"
vita_dir="$remote_data_home/oag/extracted/vita"
keys_dir="$remote_data_home/oag/keys"
echo "AppImage -> $desktop_dir/"
echo "images   -> $images_dir/"
echo "dlc      -> $dlc_dir/"
echo "2048     -> $vita_dir/"
echo "keys     -> $keys_dir/"

if (( dry_run )); then
    echo "(--dry-run: not creating remote directories or transferring anything)"
else
    ssh "$host" mkdir -p "$desktop_dir" "$images_dir" "$dlc_dir" "$vita_dir" "$keys_dir"
fi

use_rsync=0
if (( ! force_scp )) && ssh "$host" 'command -v rsync' >/dev/null 2>&1; then
    use_rsync=1
fi

dry_flag=()
(( dry_run )) && dry_flag=(--dry-run)

sync_file() {
    local src="$1" dest_dir="$2"
    if (( use_rsync )); then
        rsync -avzc "${dry_flag[@]}" "$src" "$host:$dest_dir/"
    elif (( dry_run )); then
        echo "would run: scp '$src' '$host:$dest_dir/'"
    else
        scp "$src" "$host:$dest_dir/"
    fi
}

# $1 src dir, $2 remote dest dir, then any number of basenames to leave behind
# (see the hdfury exclusion below for why).
sync_dir() {
    local src="$1" dest_dir="$2"
    shift 2
    local excludes=("$@")

    if [[ ! -d $src ]] || [[ -z "$(ls -A "$src" 2>/dev/null)" ]]; then
        echo "skipping $src: nothing there yet"
        return
    fi

    if (( use_rsync )); then
        local exclude_args=() e
        for e in "${excludes[@]}"; do exclude_args+=(--exclude "$e"); done
        rsync -avz --progress "${dry_flag[@]}" "${exclude_args[@]}" "$src/" "$host:$dest_dir/"
    else
        # scp has no exclude flag, so walk the directory ourselves.
        local entry name skip e
        for entry in "$src"/*; do
            name="$(basename "$entry")"
            skip=0
            for e in "${excludes[@]}"; do
                [[ $name == "$e" ]] && { skip=1; break; }
            done
            if (( skip )); then
                echo "skipping $name (excluded)"
                continue
            fi
            if (( dry_run )); then
                echo "would run: scp -r '$entry' '$host:$dest_dir/'"
            else
                scp -r "$entry" "$host:$dest_dir/"
            fi
        done
    fi
}

step "Transport: $( (( use_rsync )) && echo rsync || echo scp )"

step "Copying the AppImage"
sync_file "$appimage" "$desktop_dir"

if ! (( dry_run )); then
    # Not relied on above: plain scp does not preserve the executable bit, and
    # this is cheap insurance even when rsync -a already carried it over.
    ssh "$host" chmod +x "$desktop_dir/$(basename "$appimage")"
fi

if (( sync_data )); then
    step "Copying data/images"
    # hdfury-ps3-eu.iso is the still-encrypted disc image and hdfury-ps3-eu.dkey
    # its decryption key - dead weight here: only hdfury-ps3-eu-dec.iso is
    # openable, and HD/Fury isn't played past its menus yet either way (see
    # data/README.md and docs/formats/ps3-disc.md). The key also has no reason
    # to leave this machine.
    sync_dir "$project_root/data/images" "$images_dir" \
        "hdfury-ps3-eu.iso" "hdfury-ps3-eu.dkey"
    step "Copying data/dlc"
    sync_dir "$project_root/data/dlc" "$dlc_dir"
    step "Copying data/extracted/vita (Wipeout 2048)"
    # Not a disc image, so it lives outside data/images/ - an extracted PKG
    # directory, one subdirectory per package (see data/README.md). Synced
    # whole, same as the other two: oag-game's package_search_path() looks
    # for it at $vita_dir on a machine with no OAG_IMAGE and no data/ beside
    # the AppImage, which is exactly the Deck's own layout.
    sync_dir "$project_root/data/extracted/vita" "$vita_dir"

    step "Copying data/keys/pure-dlc-keys.txt"
    # Only this one file, not the whole data/keys/ directory: Wipeout Pure's
    # DLC packs need it to decrypt (see
    # docs/architecture/adr/0033-external-key-material-for-decryption.md),
    # so it has to reach the Deck the same way the packs themselves do -
    # unlike hdfury-ps3-eu.dkey above, or data/keys/vita-zrif.tsv, which are
    # either not needed to play anything yet or already spent producing
    # data/extracted/vita and have no reason to leave this machine.
    pure_dlc_keys="$project_root/data/keys/pure-dlc-keys.txt"
    if [[ -f $pure_dlc_keys ]]; then
        sync_file "$pure_dlc_keys" "$keys_dir"
    else
        echo "skipping $pure_dlc_keys: not present, Pure's DLC packs (if any) won't decrypt on $host"
    fi
else
    step "Skipping data/ (--no-data)"
fi

step "Done"
echo "AppImage: $desktop_dir/$(basename "$appimage")"
echo "images:   $images_dir"
echo "dlc:      $dlc_dir"
echo "2048:     $vita_dir"
echo "keys:     $keys_dir"
echo
echo "Run it: ssh $host '$desktop_dir/$(basename "$appimage")'"
