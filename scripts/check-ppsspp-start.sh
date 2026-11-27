#!/usr/bin/env bash
# `just check-ppsspp-start`: scripts/ppsspp-start.sh against stand-in `PPSSPPSDL`
# and `Xvfb` and a private scratch directory. `stop` signals only the processes
# `start` recorded (a reused pid or another lane's emulator survives), waits for
# PPSSPP before it takes the display away, kills a PPSSPP that ignores TERM, says
# what it did, and can be run twice. `start` refuses a lane that is still running.
# No emulator or display is started. Every wait is bounded.
# `PPSSPP_START_UNDER_TEST` points at another copy of ppsspp-start.sh (a mutant).
set -uo pipefail
[ -d /proc/self ] || { echo "check-ppsspp-start: skipped (no /proc)"; exit 0; }
cd "$(dirname "$0")/.." || exit 1
start=${PPSSPP_START_UNDER_TEST:-$PWD/scripts/ppsspp-start.sh}
. scripts/oag-tmp.sh
tmp=$(mktemp -d -p "$(oag_tmp_root)")
bystanders=()
cleanup() {
    local p f
    for p in "${bystanders[@]}"; do kill -KILL "$p" 2>/dev/null; done
    # Stand-ins a failed scenario left, by the pids they wrote themselves.
    for f in "$tmp"/ran/*; do
        [ -e "$f" ] && kill -KILL "${f##*/}" 2>/dev/null
    done
    rm -rf -- "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/bin" "$tmp/ran" "$tmp/scratch"

# Stand-ins: each notes its pid under `ran/`. PPSSPP ignores TERM when the lane's
# `deaf` file exists; a polite one takes half a second to leave, as the real one
# takes a while to save and quit.
cat >"$tmp/bin/PPSSPPSDL" <<'PPSSPP'
#!/bin/bash
: >"$STAND_IN_RAN/$$"
if [ -e "$STAND_IN_RAN/../deaf" ]; then
    trap '' TERM
else
    trap 'sleep 0.5; exit 0' TERM
fi
while :; do sleep 0.1; done
PPSSPP
# The display notes whether the emulator was still alive when it was told to go.
cat >"$tmp/bin/Xvfb" <<'XVFB'
#!/bin/bash
: >"$STAND_IN_RAN/$$"
trap 'p=$(cat "$STAND_IN_RAN/../watch" 2>/dev/null); [ -n "$p" ] && [ -d "/proc/$p" ] && : >"$STAND_IN_RAN/../display-went-first"; exit 0' TERM
while :; do sleep 0.1; done
XVFB
chmod +x "$tmp/bin/PPSSPPSDL" "$tmp/bin/Xvfb"
export PATH="$tmp/bin:$PATH" STAND_IN_RAN=$tmp/ran PPSSPP_START_SCRATCH=$tmp/scratch PPSSPP_STOP_WAIT=20
export PPSSPP_XVFB_WAIT=0.1
unset PPSSPP_EXTRA_INI

fail() { echo "check-ppsspp-start: $*" >&2; [ -z "${out:-}" ] || echo "last output: $out" >&2; exit 1; }
run() { out=$(bash "$start" "$@" 2>&1); rc=$?; }
up() { run "$1" "$2" "$3" "$tmp/image"; [ "$rc" -eq 0 ] || fail "start $1 failed (rc=$rc)"; }
down() { run "$1" stop; [ "$rc" -eq 0 ] || fail "stop $1 failed (rc=$rc)"; }
says() { case "$out" in *"$1"*) ;; *) fail "output does not say '$1'" ;; esac; }
live() { [ -d "/proc/$1" ] && [ "$(awk '{ sub(/^.*\) /, ""); print $1 }' "/proc/$1/stat" 2>/dev/null)" != Z ]; }
gone() { local i; for ((i = 0; i < 40; i++)); do live "$1" || return 0; sleep 0.1; done; return 1; }
pid_of() { cat "$tmp/scratch/$1/pp/$2.pid"; }
: >"$tmp/image"

# A polite emulator: both go, the display after it, the pid files with them.
up polite 71 45071
e=$(pid_of polite ppsspp) x=$(pid_of polite xvfb)
sleep 0.3
live "$e" && live "$x" || fail "start left nothing running"
echo "$e" >"$tmp/watch"
down polite
gone "$e" && gone "$x" || fail "stop left a process of its own"
[ ! -e "$tmp/display-went-first" ] || fail "the display was stopped while PPSSPP was still running"
[ ! -e "$tmp/scratch/polite/pp/ppsspp.pid" ] && [ ! -e "$tmp/scratch/polite/pp/xvfb.pid" ] \
    || fail "a pid file survived stop"
says "stopped"
# Twice, and for a lane that never started.
down polite
says "nothing running"
down never-started
says "nothing running"

# One that ignores TERM is killed, with its children.
: >"$tmp/deaf"
up deaf 72 45072
e=$(pid_of deaf ppsspp) x=$(pid_of deaf xvfb)
sleep 0.3
echo "$e" >"$tmp/watch"
down deaf
gone "$e" && gone "$x" || fail "a PPSSPP ignoring TERM survived stop"
says "SIGKILL"
[ ! -e "$tmp/display-went-first" ] || fail "the display was stopped before the deaf PPSSPP was gone"
rm -f "$tmp/deaf" "$tmp/watch"

# A second `start` on a live lane would orphan the first instance: refused.
up twice 73 45073
e=$(pid_of twice ppsspp) x=$(pid_of twice xvfb)
sleep 0.3
run twice 73 45073 "$tmp/image"
[ "$rc" -ne 0 ] || fail "a second start on a running lane succeeded"
[ "$(pid_of twice ppsspp)" = "$e" ] && [ "$(pid_of twice xvfb)" = "$x" ] || fail "the second start replaced the pid files"
# Only the display left behind still counts as running.
kill -KILL "$e"
gone "$e" || fail "could not kill the stand-in"
run twice 73 45073 "$tmp/image"
[ "$rc" -ne 0 ] || fail "a start over a live display succeeded"
down twice
gone "$x" || fail "the display outlived stop"
up twice 73 45073
down twice

# A pid file naming someone else's process is never signalled: a bystander, and
# another lane's PPSSPP and display.
up other 74 45074
oe=$(pid_of other ppsspp) ox=$(pid_of other xvfb)
sleep 0.3
setsid sleep 60 &
b=$!
bystanders+=("$b")
up stale 75 45075
se=$(pid_of stale ppsspp) sx=$(pid_of stale xvfb)
sleep 0.3
echo "$b" >"$tmp/scratch/stale/pp/ppsspp.pid"
echo "$b" >"$tmp/scratch/stale/pp/xvfb.pid"
down stale
live "$b" || fail "stop signalled a bystander holding a recorded pid"
says "not ours"
echo "$oe" >"$tmp/scratch/stale/pp/ppsspp.pid"
echo "$ox" >"$tmp/scratch/stale/pp/xvfb.pid"
down stale
sleep 0.3
live "$oe" && live "$ox" || fail "stop signalled another lane's instance"
kill -KILL "$se" "$sx" 2>/dev/null
down other
gone "$oe" && gone "$ox" || fail "the other lane's own stop failed"

echo "check-ppsspp-start: ok"
