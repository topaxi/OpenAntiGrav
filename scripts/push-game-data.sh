#!/usr/bin/env bash
#
# Pick which game data goes onto a Steam Deck (or any ssh host) or an Android
# phone, and copy it there. One row per thing the player owns: a disc image,
# a DLC zip, an unpacked 2048 package, an unpacked Omega package (its .psarc
# archives only) and Pure's DLC key table. Each row says whether it is
# already on the device ("on device"), partly there or changed ("partial"), or
# not there at all; TAB selects in the fzf picker, Enter copies.
#
# The data lands where oag-game's own search path looks without any flag:
# `<XDG_DATA_HOME>/oag/{images,dlc,extracted/vita,extracted/ps4,keys}` - on the
# Deck the remote user's XDG_DATA_HOME, on Android the app's files dir
# (/sdcard/Android/data/org.openantigrav.game/files/data, which the app sets as
# its XDG_DATA_HOME). The binaries go over with `just deploy-deck` /
# `just deploy-android`, which copy no game data.
#
# Usage:
#   scripts/push-game-data.sh deck [user@host] [--dry-run] [PATTERN...]
#   scripts/push-game-data.sh android [--serial S] [--dry-run] [PATTERN...]
#
#   user@host     Deck to copy to. Default: $OAG_DECK_HOST or deck@steamdeck.
#   --serial S    adb device serial. Default: $ANDROID_SERIAL or the only device.
#   --dry-run     Show the rows and what would be copied; copy nothing.
#   PATTERN...    Skip the picker and copy every row whose name matches one of
#                 these globs, e.g. 'images/pulse-psp-*' or 'dlc/*'.
#
# **Pre-converted caches.** A phone or a Deck has no `ffmpeg`, so it has no movie
# picture and no ATRAC3+ sound unless the converted files come with it. Each
# Pulse PSP image gets a second row, `cache/<image>`: its movies (AV1) and sounds
# (PCM), listed by the manifest `oag-game --dry-run --prefetch` writes beside the
# local cache (`data/cache/manifests/<image>.txt`). Selecting a row whose manifest
# is missing runs that prefetch first (about 10 minutes and 0.7 GB the first time)
# and then copies. They land in the device's cache directory, not its data one:
# `<cache dir>/oag` on the Deck, the app's `files/cache/oag` on Android.
#
# Only files that are missing or differ in size are copied. Nothing on the
# device is deleted. Left out on purpose, as deploy-to-deck.sh always did: the
# still-encrypted HD image and its .dkey (only -dec.iso opens), every .pkg and
# .sha256 (oag-game never reads a package), and every other key file.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data="$project_root/data"
package="org.openantigrav.game"

step() { printf '\n\033[1m==> %s\033[0m\n' "$*" >&2; }
die()  { echo "error: $*" >&2; exit 1; }

target="${1:-}"
[[ $target == deck || $target == android ]] \
    || die "first argument is the target: deck or android (see --help)"
shift

host="${OAG_DECK_HOST:-deck@steamdeck}"
adb_args=()
dry_run=0
patterns=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --serial)  [[ $# -ge 2 ]] || die "--serial needs a value"
                   adb_args=(-s "$2"); shift 2 ;;
        --dry-run) dry_run=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *@*)       [[ $target == deck ]] || die "a user@host only applies to deck"
                   host="$1"; shift ;;
        -*)        die "unknown argument: $1" ;;
        *)         patterns+=("$1"); shift ;;
    esac
done

adb() { command adb "${adb_args[@]}" "$@"; }

# Runs a shell command on the device and prints its stdout.
remote() {
    if [[ $target == deck ]]; then
        ssh "$host" "$1"
    else
        adb shell "$1" | tr -d '\r'
    fi
}

step "Resolving the device's data directory"
if [[ $target == deck ]]; then
    remote_root="$(ssh "$host" 'echo "${XDG_DATA_HOME:-$HOME/.local/share}"')/oag"
else
    command -v adb >/dev/null || die "adb not found; install android-tools (see docs/tools/android.md)"
    adb get-state >/dev/null 2>&1 \
        || die "no device (or more than one: pass --serial). USB debugging must be on and the host authorised."
    remote_root="/sdcard/Android/data/$package/files/data/oag"
fi
if [[ $target == deck ]]; then
    cache_root="$(ssh "$host" 'echo "${XDG_CACHE_HOME:-$HOME/.cache}"')/oag"
else
    cache_root="/sdcard/Android/data/$package/files/cache/oag"
fi
echo "$target: $remote_root (caches: $cache_root)" >&2

# Every file already on the device, as "<size> <path relative to remote_root>".
declare -A remote_size=()
while IFS=' ' read -r size path; do
    [[ -n $path ]] && remote_size["${path#./}"]="$size"
done < <(remote "cd '$remote_root' 2>/dev/null && find . -type f -exec stat -c '%s %n' {} + 2>/dev/null" || true)
# The same for the cache directory, keyed "@cache/<path>" like a cache row's files.
while IFS=' ' read -r size path; do
    [[ -n $path ]] && remote_size["@cache/${path#./}"]="$size"
done < <(remote "cd '$cache_root' 2>/dev/null && find . -type f -exec stat -c '%s %n' {} + 2>/dev/null" || true)

# A row's file as it is named locally and on the device. Plain names are relative
# to data/ and remote_root; "@cache/..." ones to data/cache/ and cache_root.
local_of()  { case "$1" in @cache/*) echo "$data/cache/${1#@cache/}" ;; *) echo "$data/$1" ;; esac; }
remote_of() { case "$1" in @cache/*) echo "$cache_root/${1#@cache/}" ;; *) echo "$remote_root/$1" ;; esac; }

# The files a cache row names, from the manifest of one image. Empty when there
# is no manifest yet.
cache_manifest_files() {
    local manifest="$data/cache/manifests/$1.txt" line
    [[ -f $manifest ]] || return 0
    while IFS= read -r line; do
        [[ -z $line || $line == \#* ]] && continue
        [[ -f $data/cache/$line ]] && echo "@cache/$line"
    done < "$manifest"
}

# Rows: a name, and the local files that make it up, relative to data/ -
# remote_root has data/'s own layout, so the same path is the remote one.
declare -a row_names=()
declare -A row_files=()

add_row() {
    local name="$1"; shift
    [[ $# -gt 0 ]] || return 0
    row_names+=("$name")
    row_files["$name"]="$(printf '%s\n' "$@")"
}

excluded_image() {
    case "$1" in
        hdfury-ps3-eu.iso|*.dkey|*.pkg|*.sha256|README.md) return 0 ;;
    esac
    return 1
}

shopt -s nullglob
for f in "$data"/images/*; do
    [[ -f $f ]] || continue
    name="$(basename "$f")"
    excluded_image "$name" && continue
    add_row "images/$name" "images/$name"
    # Only Pulse PSP images have a prefetch: see `oag_game::prefetch`.
    case "$name" in
        pulse-psp-*.chd|pulse-psp-*.iso)
            mapfile -t cache_files < <(cache_manifest_files "$name")
            if [[ ${#cache_files[@]} -gt 0 ]]; then
                add_row "cache/$name" "${cache_files[@]}"
            else
                # Listed anyway: selecting it runs the prefetch that makes the manifest.
                add_row "cache/$name" "@nocache/$name"
            fi ;;
    esac
done
for f in "$data"/dlc/*; do
    [[ -f $f ]] || continue
    name="$(basename "$f")"
    case "$name" in *.pkg|*.sha256|README.md) continue ;; esac
    add_row "dlc/$name" "dlc/$name"
done
for d in "$data"/extracted/vita/*/; do
    d="${d%/}"
    mapfile -t files < <(cd "$data" && find "extracted/vita/$(basename "$d")" -type f | sort)
    add_row "extracted/vita/$(basename "$d")" "${files[@]}"
done
for d in "$data"/extracted/ps4/*/; do
    d="${d%/}"
    mapfile -t files < <(cd "$data" && find "extracted/ps4/$(basename "$d")" -type f -name '*.psarc' | sort)
    add_row "extracted/ps4/$(basename "$d")" "${files[@]}"
done
[[ -f $data/keys/pure-dlc-keys.txt ]] && add_row "keys/pure-dlc-keys.txt" "keys/pure-dlc-keys.txt"
shopt -u nullglob

[[ ${#row_names[@]} -gt 0 ]] || die "nothing under $data to copy (see data/README.md)"

# Prints "<bytes> <status>" for a row.
row_state() {
    local name="$1" file total=0 present=0 count=0 local_size
    if [[ ${row_files[$name]} == @nocache/* ]]; then
        echo "0 no local cache"
        return
    fi
    while IFS= read -r file; do
        local_size="$(stat -c %s "$(local_of "$file")")"
        total=$((total + local_size))
        count=$((count + 1))
        [[ ${remote_size["$file"]:-} == "$local_size" ]] && present=$((present + 1))
    done <<< "${row_files[$name]}"
    if (( present == count )); then
        echo "$total on device"
    elif (( present > 0 )) || [[ -n ${remote_size["$(head -n1 <<< "${row_files[$name]}")"]:-} ]]; then
        echo "$total partial"
    else
        echo "$total -"
    fi
}

table=""
for name in "${row_names[@]}"; do
    read -r bytes status < <(row_state "$name")
    status="${status/no local cache/no cache}"
    table+="$(printf '%s\t%-10s %8s  %s' "$name" "$status" "$(numfmt --to=iec "$bytes")" "$name")"$'\n'
done

if [[ ${#patterns[@]} -gt 0 ]]; then
    selected=()
    for name in "${row_names[@]}"; do
        for p in "${patterns[@]}"; do
            # Unquoted on purpose: $p is a glob.
            # shellcheck disable=SC2053
            [[ $name == $p ]] && { selected+=("$name"); break; }
        done
    done
    printf '%s' "$table" | cut -f2- >&2
else
    command -v fzf >/dev/null || die "fzf not found; install it, or name the rows as arguments"
    mapfile -t selected < <(printf '%s' "$table" | fzf --multi --delimiter '\t' --with-nth 2.. \
        --header "TAB select, Enter copy to $target ($remote_root), Esc cancel" \
        --header-first --layout reverse | cut -f1)
fi

[[ ${#selected[@]} -gt 0 ]] || { echo "nothing selected" >&2; exit 0; }

# A cache row with no manifest yet: convert the disc's movies and sounds here
# first, which also writes the manifest. `--dry-run --prefetch` is the headless
# run that does only that.
prefetch_for() {
    local image="$1" game=""
    for candidate in "$project_root/target/release/oag-game" "$project_root/target/debug/oag-game"; do
        [[ -x $candidate ]] && { game="$candidate"; break; }
    done
    step "Converting $image's movies and sounds (about 10 minutes and 0.7 GB the first time)"
    if (( dry_run )); then
        echo "(--dry-run: would run ${game:-cargo run -p oag-game --} --no-audio --dry-run --prefetch data/images/$image)" >&2
        return 0
    fi
    if [[ -n $game ]]; then
        (cd "$project_root" && "$game" --no-audio --dry-run --prefetch "data/images/$image")
    else
        (cd "$project_root" && cargo run -q --release -p oag-game -- --no-audio --dry-run --prefetch "data/images/$image")
    fi
}

# The files of the selected rows that are missing or differ.
to_copy=()
for name in "${selected[@]}"; do
    if [[ ${row_files[$name]} == @nocache/* ]]; then
        image="${name#cache/}"
        prefetch_for "$image"
        mapfile -t made < <(cache_manifest_files "$image")
        (( dry_run )) && continue
        [[ ${#made[@]} -gt 0 ]] || { echo "no cache made for $image; skipping" >&2; continue; }
        row_files["$name"]="$(printf '%s\n' "${made[@]}")"
    fi
    while IFS= read -r file; do
        [[ ${remote_size["$file"]:-} == "$(stat -c %s "$(local_of "$file")")" ]] && continue
        to_copy+=("$file")
    done <<< "${row_files[$name]}"
done

if [[ ${#to_copy[@]} -eq 0 ]]; then
    step "Everything selected is already on the device"
    exit 0
fi

bytes=0
for file in "${to_copy[@]}"; do bytes=$((bytes + $(stat -c %s "$(local_of "$file")"))); done
step "Copying ${#to_copy[@]} file(s), $(numfmt --to=iec "$bytes"), to $remote_root (caches to $cache_root)"

if (( dry_run )); then
    printf '  %s\n' "${to_copy[@]}" | head -40 >&2
    echo "(--dry-run: nothing copied)" >&2
    exit 0
fi

if [[ $target == deck ]]; then
    for file in "${to_copy[@]}"; do
        dest="$(remote_of "$file")"
        ssh "$host" mkdir -p "'$(dirname "$dest")'"
        if ssh "$host" 'command -v rsync' >/dev/null 2>&1; then
            rsync -t --progress "$(local_of "$file")" "$host:$dest"
        else
            scp "$(local_of "$file")" "$host:$dest"
        fi
    done
else
    for file in "${to_copy[@]}"; do
        dest="$(remote_of "$file")"
        adb shell mkdir -p "'$(dirname "$dest")'"
        adb push "$(local_of "$file")" "$dest"
    done
fi

step "Done"
