#!/usr/bin/env bash
# A gate recipe under the shared gate lock, built while it waits.
#
#   scripts/gate-run.sh gate-affected
#   scripts/gate-run.sh test-data-affected
#   scripts/gate-run.sh check                 # the lead's `just`
#
# What `flock -o "$HOME/.cache/oag/gate.lock" just <recipe>` did, plus two things
# (`docs/architecture/gate-wall-time.md`):
#
# - While the lock is busy it compiles what the recipe will compile (`just gate-prep`,
#   at nice 5) and no test runs. The lock is then held for the checks and the tests,
#   not for the build. If the lock is free at once nothing is prebuilt, and when it
#   is taken before the prebuild is done the prebuild is stopped (a half-done cargo
#   build is safe to stop) so the gate does the rest at its own priority.
# - Every run appends a row to `$HOME/.cache/oag/gate-times.tsv`: when, who (the
#   worktree), the recipe, seconds waited, seconds held, prebuild state, exit code and
#   the load average at the start of the hold.
#
# `OAG_GIT_HASH` and `OAG_BUILD_DATE` are fixed for the run (as `just test-data` fixes
# them): `oag-net`'s build script reruns on every new commit otherwise and rebuilds
# `oag-game`'s whole dependency cone once per profile. Set them yourself to keep yours.
#
# Never wrap this in `flock`, and never call it from inside a gate: it takes the lock
# itself (the same rule as `just bench-render` and `just lead-test-data`).
set -uo pipefail
cd "$(dirname "$0")/.."

lock="${OAG_GATE_LOCK:-$HOME/.cache/oag/gate.lock}"
times="${OAG_GATE_TIMES:-$HOME/.cache/oag/gate-times.tsv}"
mkdir -p "$(dirname "$lock")"

export OAG_GIT_HASH="${OAG_GIT_HASH:-test-data}"
export OAG_BUILD_DATE="${OAG_BUILD_DATE-}"

# A finished prebuild is a zombie of this process, which `kill -0` still reaches.
running() {
    local stat
    stat=$(ps -o stat= -p "$1" 2>/dev/null) || return 1
    [ -n "$stat" ] && [ "${stat#Z}" = "$stat" ]
}

if [ "${1:-}" = "--locked" ]; then
    # Re-entered through flock: this process holds the lock.
    requested=$2 prep_pid=$3 prep_started=$4
    shift 4
    state=none
    if [ "$prep_pid" != 0 ]; then
        if running "$prep_pid"; then
            kill -TERM -- "-$prep_pid" 2>/dev/null
            while running "$prep_pid"; do sleep 0.2; done
            state="stopped@$(($(date +%s) - prep_started))s"
        else
            state=done
        fi
    fi
    waited=$((requested == 0 ? 0 : $(date +%s) - requested))
    load=$(cut -d' ' -f1 /proc/loadavg)
    echo "gate-run: lock held after ${waited}s (prebuild: $state), load $load: just $*" >&2
    case "$1" in test-data-affected) export OAG_REQUIRE_GAME_DATA="${OAG_REQUIRE_GAME_DATA:-1}" ;; esac
    start=$SECONDS
    just "$@"
    rc=$?
    ran=$((SECONDS - start))
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$(date -Is)" "$(basename "$PWD")" "$1" \
        "$waited" "$ran" "$state" "$rc" "$load" >>"$times"
    echo "gate-run: ran ${ran}s under the lock, exit $rc" >&2
    exit "$rc"
fi

[ $# -ge 1 ] || { echo "usage: scripts/gate-run.sh RECIPE [ARGS...]" >&2; exit 2; }
self="$PWD/scripts/gate-run.sh"
case "$1" in
    gate-affected | test-data-affected) scope=affected ;;
    check) scope=full ;;
    *) scope= ;;
esac

# A free lock is taken at once, with nothing prebuilt (exit 111 is flock's "busy").
flock -E 111 -n -o "$lock" "$self" --locked 0 0 0 "$@"
rc=$?
[ "$rc" -ne 111 ] && exit "$rc"

prep_pid=0
prep_started=$(date +%s)
if [ -n "$scope" ]; then
    mkdir -p target
    echo "gate-run: the lock is busy; prebuilding (just gate-prep $scope, log target/gate-prep.log)" >&2
    # Its own session, so stopping it stops cargo and every compiler under it.
    setsid nice -n 5 just gate-prep "$scope" >target/gate-prep.log 2>&1 </dev/null &
    prep_pid=$!
fi
exec flock -o "$lock" "$self" --locked "$(date +%s)" "$prep_pid" "$prep_started" "$@"
