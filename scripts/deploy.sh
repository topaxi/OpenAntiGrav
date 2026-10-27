#!/usr/bin/env bash
#
# One entry point for putting a build on a device: `just deploy <target> [what] [args]`.
#
# Usage:
#   scripts/deploy.sh android|deck [REV | FILE] [args for the target's script]
#
#   (nothing)   Build the current checkout and deploy it.
#   REV         Any git rev (hash, tag, branch). Built in a worktree at
#               data/deploy-wt/<hash>, removed again afterwards (also when the
#               build fails). The artifact is kept in data/deploy-builds/, so
#               it can be redeployed as a FILE without a rebuild.
#   FILE        An existing file (an .apk for android; for deck an AppImage or
#               any other ELF / shebang executable): checked, then deployed as
#               is, nothing is built.
#
# Everything else (--serial S, --reinstall, --dry-run, user@host, ...) goes to
# scripts/deploy-to-android.sh or scripts/deploy-to-deck.sh unchanged. The
# current checkout's deploy script always does the deploying, so an old rev
# needs nothing but its build scripts.
#
# An older APK than the installed one is a downgrade; deploy-to-android.sh then
# does the uninstall-and-restore dance that keeps the app's data.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
die() { echo "error: $*" >&2; exit 1; }

target="${1:-}"
[[ $target == android || $target == deck ]] || die "usage: just deploy android|deck [REV | FILE] [args]"
shift

# A real APK is a zip holding an AndroidManifest.xml; a deck payload must be
# runnable: an ELF (an AppImage is one) or a script with a shebang.
validate_artifact() {
    local f="$1"
    [[ -r $f && -s $f ]] || die "$f is empty or unreadable"
    if [[ $target == android ]]; then
        [[ $(head -c2 "$f") == PK ]] || die "$f is not a zip, so not an APK"
        unzip -l "$f" 2>/dev/null | grep -q ' AndroidManifest.xml$' \
            || die "$f has no AndroidManifest.xml, so not an APK"
        if command -v aapt2 >/dev/null; then
            aapt2 dump badging "$f" 2>/dev/null | sed -n "s/^package: name='\([^']*\)'.*versionName='\([^']*\)'.*/\1 \2/p" | head -n1 | sed 's/^/apk: /'
        fi
    else
        local magic
        magic="$(head -c4 "$f" | od -An -tx1 | tr -d ' \n')"
        [[ $magic == 7f454c46 || ${magic:0:4} == 2321 ]] \
            || die "$f is neither an ELF binary nor a #! script, so it will not run"
        [[ -x $f ]] || echo "note: $f is not executable here; the deck script chmods the copy"
    fi
}

rev=""
artifact=""
pass=()
for arg in "$@"; do
    if [[ -f $arg ]]; then
        artifact="$(realpath "$arg")"
    elif [[ $arg != -* && $arg != *@* ]] && git -C "$project_root" rev-parse --verify -q "$arg^{commit}" >/dev/null; then
        rev="$arg"
    else
        pass+=("$arg")
    fi
done

script="$project_root/scripts/deploy-to-$target.sh"

if [[ -n $artifact ]]; then
    validate_artifact "$artifact"
    exec "$script" "${pass[@]}" "--$([[ $target == android ]] && echo apk || echo appimage)" "$artifact"
fi

if [[ -z $rev ]]; then
    exec "$script" "${pass[@]}"
fi

hash="$(git -C "$project_root" rev-parse --short=9 "$rev^{commit}")"
wt="$project_root/data/deploy-wt/$hash"
for a in "${pass[@]}"; do
    [[ $a == --dry-run ]] && { echo "would build $rev ($hash) in $wt and deploy it to $target"; exit 0; }
done
cleanup() { git -C "$project_root" worktree remove --force "$wt" 2>/dev/null || true; }
trap cleanup EXIT
if [[ -d $wt ]]; then cleanup; fi
mkdir -p "$project_root/data/deploy-wt" "$project_root/data/deploy-builds"
git -C "$project_root" worktree add --detach "$wt" "$hash"
echo "building $rev ($hash) in $wt (removed afterwards)"

if [[ $target == android ]]; then
    built="$("$wt/scripts/build-apk.sh" --arm64-only | tail -n1)"
    [[ -f $built ]] || die "build-apk.sh did not produce an APK for $hash"
    apk="$project_root/data/deploy-builds/OpenAntiGrav-$hash-android-arm64.apk"
    cp "$built" "$apk"
    cleanup
    "$script" "${pass[@]}" --apk "$apk"
else
    out="$project_root/data/deploy-builds/OpenAntiGrav-x86_64-portable-$hash.AppImage"
    "$wt/scripts/build-appimage.sh" --container --out "$out"
    cleanup
    "$script" "${pass[@]}" --appimage "$out"
fi
