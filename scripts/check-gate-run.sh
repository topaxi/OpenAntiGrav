#!/usr/bin/env bash
# `just check-gate-run`: scripts/gate-run.sh against a stand-in `just` and a private
# lock, no build. A free lock runs the recipe at once with nothing prebuilt; a busy one
# starts the prebuild, which is stopped if the lock comes first and left to finish if
# it is done; every run lands one row in the times file with the right exit code; the
# build's hash is fixed; a recipe with no prebuild is left alone; an interrupt stops the
# prebuild, the wait and the recipe, and frees the lock only once the recipe is dead.
#
# No scenario waits a fixed time. A lock is held until the check releases it, the stand-in
# `just` writes a marker once its TERM trap is armed and holds a shutdown open until the
# check lets it die, and every wait is on one of those events with a two minute deadline. A fixed
# sleep decides nothing on a host under load: the process it stands for may not have started
# or died yet. `GATE_RUN_UNDER_TEST` points at another copy of gate-run.sh (a mutant, to
# prove each scenario still fails against a broken wrapper).
set -uo pipefail
cd "$(dirname "$0")/.."
root=$PWD
. scripts/oag-tmp.sh
gate_run=${GATE_RUN_UNDER_TEST:-$root/scripts/gate-run.sh}
tmp=$(mktemp -d -p "$(oag_tmp_root)")
trap 'rm -rf "$tmp"; kill $(jobs -p) 2>/dev/null' EXIT
mkdir "$tmp/bin"
cat >"$tmp/bin/just" <<'JUST'
#!/usr/bin/env bash
# Every wait here ends when its file appears, when $FAKE_DIR is gone (the check exited) or
# after two minutes, so a failed check never leaves a stand-in spinning.
wait_file() {
    local n=0
    until [ -e "$1" ] || [ ! -d "$FAKE_DIR" ] || [ "$n" -ge 1200 ]; do sleep 0.1; n=$((n + 1)); done
}
echo "$* hash=$OAG_GIT_HASH data=${OAG_REQUIRE_GAME_DATA:-unset}" >>"$FAKE_LOG"
case "$1" in
    gate-prep)
        echo $$ >"$FAKE_DIR/prep.pid"
        if [ "${PREP_MODE:-}" = grandchild ]; then
            # The leader dies at once; a child of it is slow to notice TERM.
            (trap 'echo grandchild-term >>"$FAKE_LOG"; wait_file "$FAKE_DIR/let-grandchild-die"
                   echo grandchild-dead >>"$FAKE_LOG"; exit 0' TERM
             echo grandchild-armed >>"$FAKE_LOG"
             sleep 30 & wait $!) &
            trap 'exit 143' TERM
            wait
        fi
        trap 'echo prep-stopped >>"$FAKE_LOG"; exit 143' TERM
        echo prep-armed >>"$FAKE_LOG"
        sleep "${PREP_SLEEP:-30}" &
        wait $!
        echo prep-finished >>"$FAKE_LOG"
        ;;
    fail) exit 7 ;;
    busy-code) exit 111 ;;
    hold)
        # A recipe with a child of its own that is slow to die.
        trap 'echo hold-term >>"$FAKE_LOG"; wait_file "$FAKE_DIR/let-hold-die"
              echo hold-dead >>"$FAKE_LOG"; exit 143' TERM
        echo hold-armed >>"$FAKE_LOG"
        sleep 30 &
        wait $!
        ;;
    *)
        # A recipe starts only once no prebuild process is left.
        if pgrep -f -- "$FAKE_DIR/bin/just gate-prep" >/dev/null; then echo prep-alive >>"$FAKE_LOG"; fi
        ;;
esac
JUST
chmod +x "$tmp/bin/just"
# What a caller's shell exports must not decide what is asserted.
unset OAG_GIT_HASH OAG_BUILD_DATE OAG_REQUIRE_GAME_DATA OAG_GATE_ENTERED
export PATH="$tmp/bin:$PATH" FAKE_DIR="$tmp" FAKE_LOG="$tmp/log" OAG_GATE_LOCK="$tmp/lock" \
    OAG_GATE_TIMES="$tmp/times.tsv" OAG_GATE_PREP_LOG="$tmp/prep.log"
: >"$FAKE_LOG"
fail() { echo "check-gate-run: $*" >&2; cat "$tmp/out" "$FAKE_LOG" "$OAG_GATE_TIMES" >&2 2>/dev/null; exit 1; }
col() { awk -F'\t' -v n="$1" -v c="$2" 'NR == n { print $c }' "$OAG_GATE_TIMES"; }
logged() { grep -q -- "$1" "$FAKE_LOG"; }
lock_free() { flock -n -o "$OAG_GATE_LOCK" true; }
# wait_for WHAT CONDITION: the condition is shell text, polled for two minutes of wall
# clock (a poll that forks takes longer than its sleep on a loaded host, and a prebuild at
# idle priority can wait whole seconds for a core).
wait_for() {
    local start=$SECONDS
    until eval "$2"; do
        [ $((SECONDS - start)) -le 120 ] || fail "timed out waiting for $1"
        sleep 0.1
    done
}

# The lock is held until `release`, however long the scenario takes to get there.
hold_lock() {
    rm -f "$tmp/held"
    : >"$tmp/hold"
    flock -o "$OAG_GATE_LOCK" bash -c 'n=0; : >"$1/held"
        while [ -e "$1/hold" ] && [ "$n" -lt 1200 ]; do sleep 0.1; n=$((n + 1)); done' _ "$tmp" &
    holder=$!
    wait_for "the lock holder to start" '[ -e "$tmp/held" ]'
}
release() { rm -f "$tmp/hold"; wait "$holder"; }

# The gate in the background. `waiting` is true once it sits on the blocking flock, which
# is after the prebuild was started.
start_run() { "$gate_run" "$@" >"$tmp/out" 2>&1 & runpid=$!; }
waiting() { pgrep -P "$runpid" -f '^flock -o ' >/dev/null; }
finish_run() {
    wait_for "the gate to finish" '! kill -0 "$runpid" 2>/dev/null'
    wait "$runpid"
}
run() { "$gate_run" "$@" >"$tmp/out" 2>&1; }
no_prep_left() { ! pgrep -f -- "$tmp/bin/just gate-prep" >/dev/null; }
# What gate-run.sh calls a live group: a member that is not a zombie. A process that has
# logged its last line is still alive until the kernel has torn it down, which a prebuild
# at idle priority on a loaded host can take a while to get to.
group_alive() {
    ps -eo pgid=,stat= | awk -v g="$1" '$1 == g && $2 !~ /^Z/ { f = 1 } END { exit !f }'
}
prep_group_gone() { ! group_alive "$(cat "$tmp/prep.pid")"; }
gone() { local p; for p in "$@"; do ! kill -0 "$p" 2>/dev/null || return 1; done; }

run gate-affected || fail "a free lock failed"
grep -q '^gate-affected hash=test-data data=unset$' "$FAKE_LOG" || fail "free lock: wrong environment"
grep -q gate-prep "$FAKE_LOG" && fail "free lock: prebuilt"
[ "$(col 1 6)" = none ] || fail "free lock: prebuild state $(col 1 6)"

# A busy lock with a long prebuild: the prebuild is stopped when the lock is taken.
hold_lock
PREP_SLEEP=60 start_run gate-affected
wait_for "the prebuild to arm" 'logged prep-armed'
release
finish_run || fail "busy lock, long prebuild failed"
logged prep-stopped || fail "the unfinished prebuild was not stopped"
logged prep-alive && fail "the recipe started beside a live prebuild"
case "$(col 2 6)" in stopped@*) ;; *) fail "busy lock: prebuild state $(col 2 6)" ;; esac

# A busy lock with a prebuild that is done before the lock comes: it is left alone.
hold_lock
PREP_SLEEP=0 start_run test-data-affected
wait_for "the short prebuild to finish" 'logged prep-finished && prep_group_gone'
release
finish_run || fail "busy lock, short prebuild failed"
grep -q '^test-data-affected hash=test-data data=1$' "$FAKE_LOG" || fail "test-data-affected: wrong environment"
[ "$(col 3 6)" = done ] || fail "short prebuild state $(col 3 6)"

# A recipe with no prebuild waits on a busy lock and prebuilds nothing.
: >"$FAKE_LOG"
hold_lock
start_run check-docs
wait_for "the gate to wait on the lock" 'waiting'
release
finish_run || fail "busy lock, no prebuild failed"
[ "$(col 4 6)" = none ] || fail "a recipe with no prebuild was prebuilt: $(col 4 6)"
logged gate-prep && fail "a recipe with no prebuild started one"

run fail; [ $? -eq 0 ] && fail "a failing recipe passed"
[ "$(col 5 7)" = 7 ] || fail "exit code not recorded: $(col 5 7)"
[ "$(wc -l <"$OAG_GATE_TIMES")" -eq 5 ] || fail "expected five rows"
run busy-code; [ $? -eq 111 ] || fail "a recipe's own 111 was swallowed"
[ "$(grep -c '^busy-code' "$FAKE_LOG")" -eq 1 ] || fail "a recipe exiting 111 ran twice"
[ "$(wc -l <"$OAG_GATE_TIMES")" -eq 6 ] || fail "expected six rows"

# An interrupt while the gate waits stops the prebuild and the wait; nothing runs later.
: >"$FAKE_LOG"
hold_lock
PREP_SLEEP=60 start_run gate-affected
wait_for "the prebuild to arm" 'logged prep-armed'
wait_for "the gate to wait on the lock" 'waiting'
waiters=$(pgrep -P "$runpid" -f '^flock -o ')
kill -TERM "$runpid"
finish_run; [ $? -eq 130 ] || fail "an interrupted waiting gate did not exit 130"
logged prep-stopped || fail "an interrupted gate left its prebuild running"
no_prep_left || fail "an interrupted gate left prebuild processes"
release
wait_for "the interrupted gate's wait to be gone" 'gone $waiters'
lock_free || fail "the lock is held after the interrupted gate"
logged '^gate-affected' && fail "an interrupted gate still ran"

# An interrupt while the recipe holds the lock stops the recipe, and the lock is free only
# once the recipe is dead.
: >"$FAKE_LOG"
start_run hold
wait_for "the recipe to arm" 'logged hold-armed'
lock_free && fail "the lock was free while a recipe held it"
kill -TERM "$runpid"
wait_for "the recipe to get the TERM" 'logged hold-term'
logged hold-dead && fail "the recipe died before it had to"
lock_free && fail "an interrupted gate freed the lock before its recipe was dead"
: >"$tmp/let-hold-die"
finish_run; [ $? -eq 130 ] || fail "an interrupted gate did not exit 130"
logged hold-dead || fail "an interrupted gate left its recipe running"
[ "$(tail -n 1 "$OAG_GATE_TIMES" | cut -f7)" = 130 ] || fail "an interrupted recipe's row does not say 130"
lock_free || fail "the lock is still held after the interrupted gate"

# A prebuild whose child outlives its leader is gone before the recipe starts.
: >"$FAKE_LOG"
hold_lock
PREP_MODE=grandchild start_run gate-affected
wait_for "the prebuild's child to arm" 'logged grandchild-armed'
release
wait_for "the prebuild's child to get the TERM" 'logged grandchild-term'
# The child holds on; a wrapper that only watches the leader starts the recipe now.
n=0
while [ "$n" -lt 30 ]; do
    logged '^gate-affected' && fail "the recipe started before the prebuild's child was dead"
    sleep 0.1; n=$((n + 1))
done
: >"$tmp/let-grandchild-die"
finish_run || fail "grandchild prebuild: the gate failed"
awk '/grandchild-dead/ { dead = NR } /^gate-affected/ { ran = NR } END { exit !(dead && ran && dead < ran) }' "$FAKE_LOG" \
    || fail "the recipe started before the prebuild's child was dead"
logged prep-alive && fail "the recipe started beside a live prebuild child"

# A lock that frees as soon as the gate waits for it: the prebuild may have barely
# started, and is still stopped.
: >"$FAKE_LOG"
hold_lock
PREP_SLEEP=60 start_run gate-affected
wait_for "the gate to wait on the lock" 'waiting'
release
finish_run || fail "quick lock: the gate failed"
logged prep-finished && fail "the prebuild ran to its end beside the recipe"
logged prep-alive && fail "the recipe started beside a live prebuild"
no_prep_left || fail "the prebuild ran on beside the recipe"
case "$(tail -n 1 "$OAG_GATE_TIMES" | cut -f6)" in stopped@*) ;; *) fail "quick lock: prebuild state" ;; esac
echo "check-gate-run: ok"
