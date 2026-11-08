#!/usr/bin/env bash
#
# Fetch the PPSSPP texture packs for Wipeout Pulse and Pure into
# data/texture-packs/<name>/ at the commit scripts/texture-packs.pin records.
#
#   scripts/fetch-texture-packs.sh [--update] [--install-ppsspp MEMSTICK [--copy]] [NAME...]
#
#   NAME...                 Only these packs (pin file names). Default: all.
#   --update                Move each pack's pin to the head of its default branch,
#                           fetch that, and rewrite scripts/texture-packs.pin. Commit the pin.
#   --install-ppsspp DIR    Also put each pack where a PPSSPP install reads it:
#                           DIR/PSP/TEXTURES/<game serial>/ for every serial the pack's
#                           [games] table lists, as a symlink to the checkout (--copy:
#                           a copy without .git; about 1 GB per serial for Pulse).
#                           DIR is the memory stick folder (the one holding PSP/): on
#                           Linux ~/.config/ppsspp, on Android the PPSSPP app's folder.
#
# `OAG_TEXTURE_PACKS=DIR` puts the checkouts in DIR instead of data/texture-packs.
#
# A shallow clone of one commit; the checkout is detached at the pin. Nothing is ever
# deleted: a checkout with local edits, or an existing PPSSPP texture folder that is not
# a symlink, stops the script rather than being replaced.
#
# **The packs are not ours and are never committed.** Both repositories are unlicensed fan
# derivatives of the games' textures. They live under the gitignored data/, and the game
# reads them only when a player has put them there.
#
# The game looks for a pack at data/texture-packs/<name>/textures.ini (or textures.zip),
# which is PPSSPP's own TEXTURES/<serial>/ layout, so the same folder also works unchanged
# in a PPSSPP install. See docs/formats/ppsspp-texture-pack.md and docs/tools/texture-packs.md.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin_file="$project_root/scripts/texture-packs.pin"
dest_root="${OAG_TEXTURE_PACKS:-$project_root/data/texture-packs}"

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*" >&2; }

update=0
memstick=""
copy=0
names=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --update) update=1; shift ;;
        --install-ppsspp) [[ $# -ge 2 ]] || die "--install-ppsspp needs the memory stick folder"
                          memstick="$2"; shift 2 ;;
        --copy) copy=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' "${BASH_SOURCE[0]}"; exit 0 ;;
        -*) die "unknown argument: $1" ;;
        *) names+=("$1"); shift ;;
    esac
done
(( copy == 0 )) || [[ -n $memstick ]] || die "--copy only applies to --install-ppsspp"
command -v git >/dev/null || die "git not found"
[[ -f $pin_file ]] || die "no pin file at $pin_file"

# name url commit, one per pin row.
rows=()
while IFS= read -r line; do
    [[ -z $line || $line == \#* ]] && continue
    rows+=("$line")
done < "$pin_file"
[[ ${#rows[@]} -gt 0 ]] || die "$pin_file names no pack"

wanted() {
    [[ ${#names[@]} -eq 0 ]] && return 0
    local n
    for n in "${names[@]}"; do [[ $n == "$1" ]] && return 0; done
    return 1
}

for n in "${names[@]}"; do
    found=0
    for row in "${rows[@]}"; do [[ ${row%% *} == "$n" ]] && found=1; done
    (( found )) || die "$n is not in $pin_file"
done

# Checks out `commit` of `url` in `dir`, shallow, detached.
checkout() {
    local dir="$1" url="$2" commit="$3"
    if [[ ! -d $dir/.git ]]; then
        [[ ! -e $dir ]] || die "$dir exists and is not a git checkout; move it away first"
        mkdir -p "$dir"
        git -C "$dir" init -q
        git -C "$dir" remote add origin "$url"
    fi
    if [[ "$(git -C "$dir" rev-parse HEAD 2>/dev/null || true)" == "$commit" ]]; then
        return 0
    fi
    [[ -z "$(git -C "$dir" status --porcelain 2>/dev/null)" ]] \
        || die "$dir has local changes; they would be lost by moving to $commit"
    git -C "$dir" fetch --depth 1 --quiet origin "$commit"
    git -C "$dir" checkout --quiet --detach FETCH_HEAD
}

install_ppsspp() {
    local name="$1" dir="$2" serial target
    local serials
    serials="$(awk '/^\[/ {games = tolower($0) ~ /^\[games\]/; next}
                    games && /^[A-Z][A-Z][A-Z][A-Z][0-9][0-9][0-9][0-9][0-9][ \t]*=/ {sub(/[ \t]*=.*/, ""); print}' \
                    "$dir/textures.ini" | tr -d '\r')"
    [[ -n $serials ]] || die "$dir/textures.ini has no [games] serials"
    for serial in $serials; do
        target="$memstick/PSP/TEXTURES/$serial"
        mkdir -p "$memstick/PSP/TEXTURES"
        if [[ -L $target ]]; then
            ln -sfn "$dir" "$target"
        elif [[ -e $target ]]; then
            die "$target exists and is not a symlink; leaving it alone"
        elif (( copy )); then
            mkdir -p "$target"
            if command -v rsync >/dev/null; then
                rsync -a --exclude .git "$dir/" "$target/"
            else
                tar -C "$dir" --exclude=.git -cf - . | tar -C "$target" -xf -
            fi
        else
            ln -sn "$dir" "$target"
        fi
        echo "  $name -> $target" >&2
    done
}

mkdir -p "$dest_root"
new_pin=""
for row in "${rows[@]}"; do
    read -r name url commit <<< "$row"
    if ! wanted "$name"; then
        new_pin+="$row"$'\n'
        continue
    fi
    dir="$dest_root/$name"
    if (( update )); then
        step "$name: moving the pin to the head of the default branch"
        head="$(git ls-remote "$url" HEAD | awk 'NR==1 {print $1}')"
        [[ -n $head ]] || die "$url did not answer"
        [[ $head == "$commit" ]] && echo "  already at $commit" >&2 || echo "  $commit -> $head" >&2
        commit="$head"
    fi
    step "$name at ${commit:0:12}"
    checkout "$dir" "$url" "$commit"
    [[ -f $dir/textures.ini ]] || die "$dir has no textures.ini; is $url a texture pack?"
    new_pin+="$name $url $commit"$'\n'
    [[ -z $memstick ]] || install_ppsspp "$name" "$dir"
done

if (( update )); then
    {
        grep -E '^#' "$pin_file" || true
        printf '%s' "$new_pin"
    } > "$pin_file.new"
    mv "$pin_file.new" "$pin_file"
    echo "pin file updated; commit scripts/texture-packs.pin" >&2
fi
step "Done: $dest_root"
