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
#   at idle priority, so it only takes cores nothing else wants) and no engine test runs.
#   The lock is then held for the checks and the tests, not for the build. If the lock
#   is free at once nothing is prebuilt, and when it is taken before the prebuild is
#   done the prebuild is stopped (a half-done cargo build is safe to stop) so the gate
#   does the rest at its own priority.
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

# The locked half marks that it ran, so a recipe that itself exits 111 is not read as a
# busy lock, and looks for the cancel file an interrupted gate leaves before it starts.
entered="${OAG_GATE_ENTERED:-}"
cancel="${OAG_GATE_CANCEL:-}"
prep_log="${OAG_GATE_PREP_LOG:-target/gate-prep.log}"

# Process groups. A job started under `set -m` is its own group from the fork on, so a
# group kill never races the child's own `setsid`. A group is gone when no member is
# left but zombies; the leader alone says nothing about its compilers.
group_alive() {
    ps -eo pgid=,stat= | awk -v g="$1" '$1 == g && $2 !~ /^Z/ { f = 1 } END { exit !f }'
}
# TERM once, then wait for the group to empty: a member that is shutting down is not
# interrupted again. KILL after thirty seconds.
stop_group() {
    local i=0
    kill -TERM -- "-$1" 2>/dev/null
    while group_alive "$1"; do
        if [ "$i" -ge 150 ]; then kill -KILL -- "-$1" 2>/dev/null; fi
        sleep 0.2
        i=$((i + 1))
    done
}

if [ "${1:-}" = "--locked" ]; then
    # Re-entered through flock: this process holds the lock, and flock keeps holding it
    # until this process has exited, so nothing it starts outlives the lock.
    [ -n "$cancel" ] && [ -e "$cancel" ] && exit 130
    requested=$2 prep_pid=$3 prep_started=$4
    shift 4
    : >"$entered"
    state=none
    if [ "$prep_pid" != 0 ]; then
        if group_alive "$prep_pid"; then
            stop_group "$prep_pid"
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
    job=0
    # An interrupt stops the recipe's whole group and waits for it before this process
    # exits, which is what releases the lock.
    trap '[ "$job" != 0 ] && stop_group "$job"; cut_short=1' INT TERM HUP
    cut_short=0
    set -m
    just "$@" &
    job=$!
    set +m
    wait "$job"
    rc=$?
    while group_alive "$job"; do sleep 0.2; done
    [ "$cut_short" = 1 ] && rc=130
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

. "$(dirname "$self")/oag-tmp.sh"
entered=$(mktemp -u -p "$(oag_tmp_root)")
cancel=$(mktemp -u -p "$(oag_tmp_root)")
export OAG_GATE_ENTERED=$entered OAG_GATE_CANCEL=$cancel
prep_pid=0
waiter=0

# An interrupted gate stops its prebuild and whatever it waits on. A locked recipe is
# signalled through its own process, never by killing `flock`, which would free the lock
# while the recipe still runs; the cancel file turns away one that is only starting.
interrupted() {
    : >"$cancel"
    [ "$prep_pid" != 0 ] && stop_group "$prep_pid"
    if [ "$waiter" != 0 ]; then
        kids=$(pgrep -P "$waiter")
        if [ -n "$kids" ]; then kill -TERM $kids 2>/dev/null; else kill -TERM "$waiter" 2>/dev/null; fi
        wait "$waiter"
    fi
    exit 130
}
trap interrupted INT TERM HUP
trap '[ "$prep_pid" != 0 ] && stop_group "$prep_pid"; rm -f "$entered" "$cancel"' EXIT

# A free lock is taken at once, with nothing prebuilt (flock exits 111 when it is busy).
flock -E 111 -n -o "$lock" "$self" --locked 0 0 0 "$@" &
waiter=$!
wait "$waiter"
rc=$?
if [ "$rc" -ne 111 ] || [ -e "$entered" ]; then exit "$rc"; fi

prep_started=$(date +%s)
if [ -n "$scope" ]; then
    mkdir -p "$(dirname "$prep_log")"
    echo "gate-run: the lock is busy; prebuilding (just gate-prep $scope, log $prep_log)" >&2
    set -m
    chrt -i 0 ionice -c 3 nice -n 19 just gate-prep "$scope" >"$prep_log" 2>&1 </dev/null &
    prep_pid=$!
    set +m
fi
flock -o "$lock" "$self" --locked "$(date +%s)" "$prep_pid" "$prep_started" "$@" &
waiter=$!
wait "$waiter"
exit $?
