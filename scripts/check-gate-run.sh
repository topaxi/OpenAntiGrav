#!/usr/bin/env bash
# `just check-gate-run`: scripts/gate-run.sh against a stand-in `just` and a private
# lock, no build. A free lock runs the recipe at once with nothing prebuilt; a busy one
# starts the prebuild, which is stopped if the lock comes first and left to finish if
# it is done; every run lands one row in the times file with the right exit code; the
# build's hash is fixed; a recipe with no prebuild is left alone.
set -uo pipefail
cd "$(dirname "$0")/.."
root=$PWD
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
cat >"$tmp/bin/just" <<'JUST'
#!/usr/bin/env bash
echo "$* hash=$OAG_GIT_HASH data=${OAG_REQUIRE_GAME_DATA:-unset}" >>"$FAKE_LOG"
case "$1" in
    gate-prep)
        if [ "${PREP_MODE:-}" = grandchild ]; then
            # The leader dies at once; a child of it takes a second to notice TERM.
            (trap 'sleep 1; echo grandchild-dead >>"$FAKE_LOG"; exit 0' TERM
             sleep 30 & wait $!) &
            trap 'exit 143' TERM
            wait
        fi
        trap 'echo prep-stopped >>"$FAKE_LOG"; exit 143' TERM
        sleep "${PREP_SLEEP:-30}" &
        wait $!
        echo prep-finished >>"$FAKE_LOG"
        ;;
    fail) exit 7 ;;
    busy-code) exit 111 ;;
    hold)
        # A recipe with a child of its own that is slow to die.
        trap 'sleep 1.5; echo hold-dead >>"$FAKE_LOG"; exit 143' TERM
        sleep 30 &
        wait $!
        ;;
esac
JUST
chmod +x "$tmp/bin/just"
# What a caller's shell exports must not decide what is asserted.
unset OAG_GIT_HASH OAG_BUILD_DATE OAG_REQUIRE_GAME_DATA OAG_GATE_ENTERED
export PATH="$tmp/bin:$PATH" FAKE_LOG="$tmp/log" OAG_GATE_LOCK="$tmp/lock" OAG_GATE_TIMES="$tmp/times.tsv" \
    OAG_GATE_PREP_LOG="$tmp/prep.log"
: >"$FAKE_LOG"
fail() { echo "check-gate-run: $*" >&2; cat "$FAKE_LOG" "$OAG_GATE_TIMES" >&2; exit 1; }
busy() { (flock -o "$OAG_GATE_LOCK" sleep "$1" &) ; sleep 0.3; }
run() { "$root/scripts/gate-run.sh" "$@" >"$tmp/out" 2>&1; }
col() { awk -F'\t' -v n="$1" -v c="$2" 'NR == n { print $c }' "$OAG_GATE_TIMES"; }

run gate-affected || fail "a free lock failed"
grep -q '^gate-affected hash=test-data data=unset$' "$FAKE_LOG" || fail "free lock: wrong environment"
grep -q gate-prep "$FAKE_LOG" && fail "free lock: prebuilt"
[ "$(col 1 6)" = none ] || fail "free lock: prebuild state $(col 1 6)"

busy 3; PREP_SLEEP=60 run gate-affected || fail "busy lock, long prebuild failed"
grep -q prep-stopped "$FAKE_LOG" || fail "the unfinished prebuild was not stopped"
case "$(col 2 6)" in stopped@*) ;; *) fail "busy lock: prebuild state $(col 2 6)" ;; esac

busy 3; PREP_SLEEP=1 run test-data-affected || fail "busy lock, short prebuild failed"
grep -q prep-finished "$FAKE_LOG" || fail "the short prebuild did not finish"
grep -q '^test-data-affected hash=test-data data=1$' "$FAKE_LOG" || fail "test-data-affected: wrong environment"
[ "$(col 3 6)" = done ] || fail "short prebuild state $(col 3 6)"

busy 2; run check-docs || fail "busy lock, no prebuild failed"
[ "$(col 4 6)" = none ] || fail "a recipe with no prebuild was prebuilt: $(col 4 6)"

run fail; [ $? -eq 0 ] && fail "a failing recipe passed"
[ "$(col 5 7)" = 7 ] || fail "exit code not recorded: $(col 5 7)"
[ "$(wc -l <"$OAG_GATE_TIMES")" -eq 5 ] || fail "expected five rows"
run busy-code; [ $? -eq 111 ] || fail "a recipe's own 111 was swallowed"
[ "$(grep -c '^busy-code' "$FAKE_LOG")" -eq 1 ] || fail "a recipe exiting 111 ran twice"
[ "$(wc -l <"$OAG_GATE_TIMES")" -eq 6 ] || fail "expected six rows"

: >"$FAKE_LOG"
busy 4
PREP_SLEEP=60 "$root/scripts/gate-run.sh" gate-affected >"$tmp/out" 2>&1 &
waiter=$!
sleep 1
kill -TERM "$waiter"
wait "$waiter" 2>/dev/null
sleep 0.5
grep -q prep-stopped "$FAKE_LOG" || fail "an interrupted gate left its prebuild running"
sleep 4
grep -q '^gate-affected' "$FAKE_LOG" && fail "an interrupted gate still ran"
: >"$FAKE_LOG"
# An interrupt while the recipe holds the lock stops the recipe, and the lock is free only
# once the recipe is dead.
"$root/scripts/gate-run.sh" hold >"$tmp/out" 2>&1 &
holder=$!
sleep 1
flock -n -o "$OAG_GATE_LOCK" true && fail "the lock was free while a recipe held it"
kill -TERM "$holder"
sleep 0.5
grep -q hold-dead "$FAKE_LOG" && fail "the recipe died before it had to"
flock -n -o "$OAG_GATE_LOCK" true && fail "an interrupted gate freed the lock before its recipe was dead"
wait "$holder"; [ $? -eq 130 ] || fail "an interrupted gate did not exit 130"
grep -q hold-dead "$FAKE_LOG" || fail "an interrupted gate left its recipe running"
flock -n -o "$OAG_GATE_LOCK" true || fail "the lock is still held after the interrupted gate"

: >"$FAKE_LOG"
# A prebuild whose child outlives its leader is gone before the recipe starts.
busy 3; PREP_MODE=grandchild run gate-affected || fail "grandchild prebuild: the gate failed"
awk '/grandchild-dead/ { dead = NR } /^gate-affected/ { ran = NR } END { exit !(dead && ran && dead < ran) }' "$FAKE_LOG" \
    || fail "the recipe started before the prebuild's child was dead"

: >"$FAKE_LOG"
# A lock that frees at once: the prebuild may have barely started, and is still stopped.
(flock -o "$OAG_GATE_LOCK" sleep 0.2 &) ; sleep 0.05
PREP_SLEEP=5 run gate-affected || fail "quick lock: the gate failed"
sleep 0.5
grep -q prep-finished "$FAKE_LOG" && fail "the prebuild ran on beside the recipe"
echo "check-gate-run: ok"
