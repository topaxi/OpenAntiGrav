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
#   scripts/deploy-to-android.sh [--serial S] [--skip-build] [--reinstall]
#                                [--dry-run]
#
#   --serial S    adb device serial (`adb devices`). Default: $ANDROID_SERIAL,
#                 the only connected device, or - with more than one - a
#                 picker asking which (scripts/adb-pick-device.sh).
#   --skip-build  Don't rebuild; install the newest APK already in target/apk/.
#   --reinstall   If the installed app was signed with another key, uninstall
#                 it and install this one without asking (see below).
#   --dry-run     Print what would be built and installed; touch nothing.
#
# An APK signed with a different key (one built on another machine, or before
# the key moved to ~/.android/oag-debug.keystore) cannot be updated in place:
# Android answers INSTALL_FAILED_UPDATE_INCOMPATIBLE. The script then offers to
# reinstall, keeping the app's data: everything the app writes (images, caches,
# settings, saves, logs) lives under its external files dir, which is moved
# aside on the phone, the old app uninstalled, the new one installed, and the
# directory moved back. Asked first in a terminal; --reinstall skips the
# question; without a terminal and without --reinstall it stops.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="org.openantigrav.game"

skip_build=0
reinstall=0
dry_run=0
adb_args=()

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die()  { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --serial)     [[ $# -ge 2 ]] || die "--serial needs a value"
                      adb_args=(-s "$2"); shift 2 ;;
        --skip-build) skip_build=1; shift ;;
        --reinstall)  reinstall=1; shift ;;
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

# Uninstalls a differently-signed app and installs $apk, keeping its data by
# moving the app's external dir aside on the device and back afterwards.
reinstall_keeping_data() {
    local app_dir="/sdcard/Android/data/$package" keep="/sdcard/oag-reinstall-keep"
    local kept=0
    if [[ -n "$(adb shell "[ -d '$app_dir' ] && echo yes" | tr -d '\r')" ]]; then
        adb shell "rm -rf '$keep' && mv '$app_dir' '$keep'" \
            || die "could not move $app_dir aside; nothing was uninstalled"
        kept=1
        echo "moved the app's data to $keep"
    fi
    adb uninstall "$package" >/dev/null || die "adb uninstall failed; the app's data is in $keep on the device"
    adb install "$apk" || die "install failed after uninstalling; the app's data is in $keep on the device"
    if (( kept )); then
        adb shell "rm -rf '$app_dir' && mv '$keep' '$app_dir'" \
            || die "installed, but could not move $keep back to $app_dir; move it by hand"
        echo "moved the app's data back"
    fi
}

step "Installing ${apk:-the APK}"
if (( dry_run )); then
    echo "would run: adb install -r ${apk:-<built apk>}"
elif ! out="$(adb install -r "$apk" 2>&1)"; then
    echo "$out" | grep -v -e 'absl::InitializeLog' -e '^I0000' >&2
    grep -q INSTALL_FAILED_UPDATE_INCOMPATIBLE <<< "$out" || die "adb install failed - see above"
    echo >&2
    echo "The app on the device was signed with another key, so it cannot be updated in place." >&2
    if (( ! reinstall )); then
        [[ -t 0 ]] || die "pass --reinstall to uninstall it and install this one (its data is kept)"
        read -r -p "Uninstall it and install this one, keeping its data? [Y/n] " answer
        [[ -z $answer || $answer == [yY]* ]] || die "left the installed app as it was"
    fi
    step "Reinstalling (signature changed), keeping the app's data"
    reinstall_keeping_data
else
    echo "$out" | grep -v -e 'absl::InitializeLog' -e '^I0000'
fi

step "Done"
echo "Game data: just push-data android"
echo "Launch:    adb shell monkey -p $package -c android.intent.category.LAUNCHER 1"
echo "Logs:      adb logcat -s oag"
