#!/usr/bin/env bash
#
# Starts OpenAntiGrav on an adb device and, unless told not to, follows its log.
#
# Usage:
#   scripts/launch-android.sh [--serial S] [--no-logs] [--stop]
#
#   --serial S  adb device serial. Default: $ANDROID_SERIAL, the only device,
#               or a picker asking which (scripts/adb-pick-device.sh).
#   --no-logs   Only launch; don't follow `adb logcat -s oag`.
#   --stop      Force-stop a running instance first, so the launch is a cold
#               start rather than bringing the old one to the front.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="org.openantigrav.game"

follow_logs=1
stop_first=0
adb_args=()

die() { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --serial)  [[ $# -ge 2 ]] || die "--serial needs a value"
                   adb_args=(-s "$2"); shift 2 ;;
        --no-logs) follow_logs=0; shift ;;
        --stop)    stop_first=1; shift ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

command -v adb >/dev/null || die "adb not found; install android-tools (see docs/tools/android.md)"
# shellcheck source=scripts/adb-pick-device.sh
. "$project_root/scripts/adb-pick-device.sh"
pick_adb_device
adb() { command adb "${adb_args[@]}" "$@"; }

[[ -n "$(adb shell pm list packages "$package" | tr -d '\r')" ]] \
    || die "$package is not installed on this device; run 'just deploy-android' first"

echo "device: $(adb shell getprop ro.product.model | tr -d '\r') ($(adb get-serialno))"
(( stop_first )) && adb shell am force-stop "$package"
# Cleared before the launch, so what follows is this run's log only.
(( follow_logs )) && adb logcat -c
# monkey echoes its arguments on stderr (Samsung's build does); only its
# failure matters.
adb shell monkey -p "$package" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1 \
    || die "could not launch $package"
echo "launched $package"

if (( follow_logs )); then
    echo "following 'adb logcat -s oag' (Ctrl-C stops following; the app keeps running)"
    exec adb logcat -s oag
fi
