#!/usr/bin/env bash
#
# Follows OpenAntiGrav's log on an adb device: `adb logcat -s oag` on the device
# scripts/adb-pick-device.sh picks (asking when several are connected).
#
# Usage:
#   scripts/logcat-android.sh [--serial S] [--clear] [--dump] [-- LOGCAT_ARGS...]
#
#   --serial S  adb device serial. Default: $ANDROID_SERIAL, the only device,
#               or a picker asking which.
#   --clear     Clear the device's log buffer first (only new lines follow).
#   --dump      Print what is buffered and exit instead of following.
#   -- ARGS     Replace the default filter `-s oag` with your own logcat
#               arguments, e.g. `-- '*:W'` for every app's warnings.

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

clear_first=0
dump=0
filter=(-s oag)
adb_args=()

die() { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --serial) [[ $# -ge 2 ]] || die "--serial needs a value"
                  adb_args=(-s "$2"); shift 2 ;;
        --clear)  clear_first=1; shift ;;
        --dump)   dump=1; shift ;;
        --)       shift; filter=("$@"); break ;;
        -h|--help) awk 'NR>2 && /^#/ {sub(/^# ?/,""); print; next} NR>2 {exit}' \
                       "${BASH_SOURCE[0]}"; exit 0 ;;
        *) die "unknown argument: $1 (logcat's own arguments go after --)" ;;
    esac
done

command -v adb >/dev/null || die "adb not found; install android-tools (see docs/tools/android.md)"
# shellcheck source=scripts/adb-pick-device.sh
. "$project_root/scripts/adb-pick-device.sh"
pick_adb_device
adb() { command adb "${adb_args[@]}" "$@"; }

echo "device: $(adb shell getprop ro.product.model | tr -d '\r') ($(adb get-serialno))" >&2
(( clear_first )) && adb logcat -c
if (( dump )); then
    exec "$(type -P adb)" "${adb_args[@]}" logcat -d "${filter[@]}"
fi
exec "$(type -P adb)" "${adb_args[@]}" logcat "${filter[@]}"
