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
# aside on the phone, the old app uninstalled, the new one installed and
# started once, and the kept files copied back in (a moved-back directory stays
# unreadable to the new install). Asked first in a terminal; --reinstall skips the
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
    # The x86_64 library is only for Waydroid; a phone never loads it, so
    # building it for one doubles the build for nothing.
    apk_args=()
    abi="$(adb shell getprop ro.product.cpu.abi | tr -d '\r')"
    [[ $abi == arm64-v8a ]] && apk_args=(--arm64-only)
    step "Building the APK (${abi:-unknown ABI}${apk_args[*]:+, arm64 only})"
    apk="$("$project_root/scripts/build-apk.sh" "${apk_args[@]}" | tail -n1)"
    [[ -f $apk ]] || die "build-apk.sh did not produce an APK - see the output above."
fi

app_dir="/sdcard/Android/data/$package"
keep="/sdcard/oag-reinstall-keep"

# Copies a kept data dir back into the freshly installed app's own dir.
#
# Copied, never moved: measured on the S24, a directory carried over from the
# previous install (moved aside and back) stays unreadable to the new one -
# `settings.toml: Permission denied`, no image found - while files copied into
# directories the new install created itself read and write fine. So the app is
# started once to create its dir, stopped, and the kept files are copied in on
# top; the keep dir is deleted only after the copy succeeded.
restore_kept_data() {
    [[ -n "$(adb shell "[ -d '$keep/files' ] && echo yes" | tr -d '\r')" ]] || return 0
    step "Restoring the app's data kept from the reinstall"
    if [[ -z "$(adb shell "[ -d '$app_dir/files' ] && echo yes" | tr -d '\r')" ]]; then
        adb shell monkey -p "$package" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1 || true
        local i
        for i in 1 2 3 4 5 6 7 8 9 10; do
            [[ -n "$(adb shell "[ -d '$app_dir/files' ] && echo yes" | tr -d '\r')" ]] && break
            sleep 1
        done
        adb shell am force-stop "$package"
    fi
    [[ -n "$(adb shell "[ -d '$app_dir/files' ] && echo yes" | tr -d '\r')" ]] \
        || die "the app did not create $app_dir/files; the kept data is still in $keep"
    adb shell "cp -r '$keep/files/.' '$app_dir/files/'" \
        || die "copying $keep back failed; the kept data is still in $keep"
    adb shell "rm -rf '$keep'"
    echo "copied the app's kept data back from $keep"
}

# Uninstalls a differently-signed app and installs $apk, moving the app's
# external dir aside first; restore_kept_data copies it back afterwards.
reinstall_keeping_data() {
    if [[ -n "$(adb shell "[ -d '$app_dir' ] && echo yes" | tr -d '\r')" ]]; then
        [[ -z "$(adb shell "[ -e '$keep' ] && echo yes" | tr -d '\r')" ]] \
            || die "$keep already exists on the device (an earlier reinstall's data); move or delete it first"
        adb shell "mv '$app_dir' '$keep'" \
            || die "could not move $app_dir aside; nothing was uninstalled"
        echo "moved the app's data to $keep"
    fi
    adb uninstall "$package" >/dev/null || die "adb uninstall failed; the app's data is in $keep on the device"
    adb install "$apk" || die "install failed after uninstalling; the app's data is in $keep on the device - re-run 'just deploy-android --skip-build' and it is copied back"
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

# A reinstall interrupted after the uninstall (the phone dropped off USB, say)
# leaves the app's data in the keep dir; finish it now that an install worked.
if (( ! dry_run )); then
    restore_kept_data
fi

if (( ! dry_run )); then
    # Read back off the device: what is actually installed there now.
    installed="$(adb shell dumpsys package "$package" | tr -d '\r' | sed -n 's/^ *versionName=//p' | head -n1)"
    echo "installed: oag-game ${installed:-?} on $(adb shell getprop ro.product.model | tr -d '\r')"
fi

step "Done"
echo "Game data: just push-data android"
echo "Launch:    just launch-android   (follows the log too; --stop for a cold start)"
