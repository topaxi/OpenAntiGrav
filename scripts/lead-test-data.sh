#!/usr/bin/env bash
# The full disc-image suite as M slices, each under its own `flock -o` on the
# shared gate lock, so a member's gate waits for at most one slice instead of
# the whole run (`.claude/skills/oag-drive/SKILL.md`, "Who runs which gate").
#
#   just lead-test-data [-n M] [--full]      # M defaults to 4
#
# Never wrap this in `flock`: it takes the lock itself, once per slice, and a
# second hold of the same lock deadlocks (as for `just bench-render`).
#
# Stops at the first red slice and exits with its status. A binary is recorded
# green only when every one of its tests passed across the slices
# (`scripts/test-data.py`, "Slices"), so a red or interrupted run records only
# the binaries it finished. Extra arguments (`--full`) go to every slice.
set -uo pipefail
cd "$(dirname "$0")/.."

slices=4
extra=()
while [ $# -gt 0 ]; do
    case "$1" in
        -n) slices="${2:-}"; shift 2 || { echo "lead-test-data: -n needs a number" >&2; exit 2; } ;;
        *) extra+=("$1"); shift ;;
    esac
done
case "$slices" in
    '' | *[!0-9]* | 0) echo "lead-test-data: -n must be a positive integer, got '$slices'" >&2; exit 2 ;;
esac

lock="$HOME/.cache/oag/gate.lock"
mkdir -p "$(dirname "$lock")"
started=$SECONDS
times=()
for k in $(seq 1 "$slices"); do
    echo "lead-test-data: slice $k of $slices, waiting for the gate lock"
    waited=$SECONDS
    flock -o "$lock" bash -c '
        k=$1; m=$2; shift 2
        echo "lead-test-data: slice $k of $m holds the lock"
        start=$SECONDS
        nice -n 10 ionice -c 3 env OAG_REQUIRE_GAME_DATA=1 \
            python3 scripts/test-data.py --partition "count:$k/$m" "$@"
        rc=$?
        echo "lead-test-data: slice $k of $m ran $((SECONDS - start))s under the lock, exit $rc"
        exit $rc
    ' _ "$k" "$slices" "${extra[@]}"
    rc=$?
    times+=("$k:$((SECONDS - waited))s")
    if [ "$rc" -ne 0 ]; then
        echo "lead-test-data: slice $k of $slices is red (exit $rc); stopping. Slices so far, wait included: ${times[*]}"
        exit "$rc"
    fi
done
echo "lead-test-data: $slices slices green. Per slice, lock wait included: ${times[*]}; total $((SECONDS - started))s"
