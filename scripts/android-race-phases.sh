#!/usr/bin/env bash
#
# A scripted race on an Android phone with --frame-log, its logcat saved: the
# phone's side of docs/rendering/frame-thread-cpu.md's frame-phases readings.
#
# Usage:
#   scripts/android-race-phases.sh --serial S --out FILE [--seconds N]
#       [--image NAME] [--track PATH] [--cool TENTHS] [-- EXTRA_LINE...]
#
# Writes the app's one-shot launch file (files/oag-args.txt, docs/tools/android.md)
# with a --race --autopilot --frame-log run, launches the installed app, waits,
# and saves `logcat -s oag` to FILE with the battery temperature before and
# after. --cool waits (at most 9 minutes) until the battery reads at or under
# TENTHS of a degree first, so interleaved runs start alike. EXTRA_LINE is one
# more launch-file line each: an argument, or OAG_NAME=value for a switch.
# The app is stopped at the end. Uses the installed APK; it builds nothing.

set -euo pipefail

serial="" out="" seconds=40 image="hdfury-ps3-eu-dec.iso" cool=""
track='Data\Environments\01_Vineta_K\track.vex'
while (($#)); do
    case $1 in
        --serial) serial=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        --seconds) seconds=$2; shift 2 ;;
        --image) image=$2; shift 2 ;;
        --track) track=$2; shift 2 ;;
        --cool) cool=$2; shift 2 ;;
        --) shift; break ;;
        *) echo "unknown argument $1" >&2; exit 2 ;;
    esac
done
[[ -n $serial && -n $out ]] || { echo "usage: $0 --serial S --out FILE [...]" >&2; exit 2; }

adb=(adb -s "$serial")
pkg=org.openantigrav.game
files=/sdcard/Android/data/$pkg/files
temperature() { "${adb[@]}" shell dumpsys battery | grep -m1 temperature | grep -oE '[0-9]+'; }

if [[ -n $cool ]]; then
    for _ in $(seq 54); do (( $(temperature) <= cool )) && break; sleep 10; done
fi

"${adb[@]}" shell am force-stop $pkg
args=$(mktemp)
trap 'rm -f "$args"' EXIT
printf '%s\n' "$files/data/oag/images/$image" --race --autopilot --frame-log --seed 1 \
    --track "$track" --mode single_race --script "$seconds:quit" "$@" > "$args"
"${adb[@]}" push "$args" "$files/oag-args.txt" > /dev/null
echo "temperature_start: $(temperature)" > "$out"
"${adb[@]}" logcat -c
"${adb[@]}" shell am start -n $pkg/android.app.NativeActivity > /dev/null
sleep $((seconds + 40))
"${adb[@]}" logcat -d -s oag >> "$out"
"${adb[@]}" shell am force-stop $pkg
echo "temperature_end: $(temperature)" >> "$out"
