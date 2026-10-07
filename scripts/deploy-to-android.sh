#!/usr/bin/env bash
#
# Builds the Android APK and installs it over adb - the Android counterpart of
# scripts/deploy-to-deck.sh.
#
# No game data goes over here: `just push-data android`
# (scripts/push-game-data.sh) picks which disc images, DLC, 2048/Omega packages
# and key tables to copy, shows which are already on the phone, and puts them in
# the app's own files dir, /sdcard/Android/data/org.openantigrav.game/files.
# That directory belongs to the app: uninstalling it deletes the data too;
# `adb install -r` keeps it.
#
# Usage:
#   scripts/deploy-to-android.sh [--serial S] [--skip-build] [--dry-run]
#
#   --serial S    adb device serial (`adb devices`). Default: $ANDROID_SERIAL,
#                 the only connected device, or - with more than one - a
#                 picker asking which (scripts/adb-pick-device.sh).
#   --skip-build  Don't rebuild; install the newest APK already in target/apk/.
#   --dry-run     Print what would be built and installed; touch nothing.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="org.openantigrav.game"

skip_build=0
dry_run=0
adb_args=()

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die()  { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --serial)     [[ $# -ge 2 ]] || die "--serial needs a value"
                      adb_args=(-s "$2"); shift 2 ;;
        --skip-build) skip_build=1; shift ;;
        --dry-run)    dry_run=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

command -v adb >/dev/null || die "adb not found; install android-tools (see docs/tools/android.md)"
adb() { command adb "${adb_args[@]}" "$@"; }

step "Checking the device"
# shellcheck source=scripts/adb-pick-device.sh
. "$project_root/scripts/adb-pick-device.sh"
pick_adb_device
adb get-state >/dev/null 2>&1 \
    || die "the device is not ready ('adb devices'); USB debugging must be on and the host authorised."
echo "device: $(adb shell getprop ro.product.model | tr -d '\r') ($(adb get-serialno))"

apk=""
if (( skip_build )); then
    apk="$(ls -t "$project_root"/target/apk/OpenAntiGrav-*-android-arm64.apk 2>/dev/null | head -n1 || true)"
    [[ -n $apk ]] || die "no APK in target/apk/. Run 'just apk' first, or drop --skip-build."
elif (( dry_run )); then
    echo "would run: scripts/build-apk.sh"
else
    step "Building the APK"
    apk="$("$project_root/scripts/build-apk.sh" | tail -n1)"
    [[ -f $apk ]] || die "build-apk.sh did not produce an APK - see the output above."
fi

step "Installing ${apk:-the APK}"
if (( dry_run )); then
    echo "would run: adb install -r ${apk:-<built apk>}"
else
    adb install -r "$apk"
fi

step "Done"
echo "Game data: just push-data android"
echo "Launch:    adb shell monkey -p $package -c android.intent.category.LAUNCHER 1"
echo "Logs:      adb logcat -s oag"
