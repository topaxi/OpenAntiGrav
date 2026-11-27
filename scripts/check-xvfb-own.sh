#!/usr/bin/env bash
# `just check-xvfb-own`: scripts/xvfb-own.sh against a stand-in `Xvfb` and private
# socket, lock and scratch directories. `start` records the pid and refuses a taken
# display (a socket, a lock, or its own live one) and `:0`; `stop` signals only the
# recorded pid, and only while it is an `Xvfb` serving that `:N`, so a reused pid
# (a bystander process, or an Xvfb on another display) and another lane's display
# survive; `status` lists ours and the unrecorded ones. A real display is never
# started or touched.
#
# Every wait is bounded. `XVFB_OWN_UNDER_TEST` points at another copy of xvfb-own.sh
# (a mutant, to prove each scenario still fails against a broken script).
set -uo pipefail
[ -d /proc/self ] || { echo "check-xvfb-own: skipped (no /proc)"; exit 0; }
cd "$(dirname "$0")/.." || exit 1
own=${XVFB_OWN_UNDER_TEST:-$PWD/scripts/xvfb-own.sh}
. scripts/oag-tmp.sh
tmp=$(mktemp -d -p "$(oag_tmp_root)")
bystanders=()
cleanup() {
    local p
    for p in "${bystanders[@]}"; do kill "$p" 2>/dev/null; done
    # A stand-in that outlived a failed scenario is stopped by the pid file it wrote.
    for p in "$tmp"/scratch/*/xvfb-*.pid; do
        [ -e "$p" ] && kill "$(cat "$p")" 2>/dev/null
    done
    rm -rf -- "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/bin" "$tmp/x11" "$tmp/lock" "$tmp/scratch"
cat >"$tmp/bin/Xvfb" <<'XVFB'
#!/bin/bash
# Stand-in: what a display leaves behind (socket, lock) and a TERM handler, no server.
n=${1#:}
touch "$XVFB_OWN_X11_DIR/X$n" "$XVFB_OWN_LOCK_DIR/.X$n-lock"
trap 'rm -f "$XVFB_OWN_X11_DIR/X$n" "$XVFB_OWN_LOCK_DIR/.X$n-lock"; exit 0' TERM
while :; do sleep 0.1; done
XVFB
chmod +x "$tmp/bin/Xvfb"
export XVFB_OWN_X11_DIR=$tmp/x11 XVFB_OWN_LOCK_DIR=$tmp/lock XVFB_OWN_SCRATCH=$tmp/scratch
export XVFB_OWN_BIN=$tmp/bin/Xvfb XVFB_OWN_START_WAIT=50 XVFB_OWN_STOP_WAIT=30

fail() { echo "check-xvfb-own: $*" >&2; [ -z "${out:-}" ] || echo "last output: $out" >&2; exit 1; }
run() { out=$(bash "$own" "$@" 2>&1); rc=$?; }
ok() { run "$@"; [ "$rc" -eq 0 ] || fail "'$*' failed (rc=$rc)"; }
refuses() { local what=$1; shift; run "$@"; [ "$rc" -ne 0 ] || fail "$what: '$*' succeeded"; }
live() { [ -d "/proc/$1" ]; }
pid_of() { cat "$tmp/scratch/$1/xvfb-$2.pid"; }
gone() { local i; for ((i = 0; i < 30; i++)); do live "$1" || return 0; sleep 0.1; done; return 1; }

# start records the pid of a live Xvfb and its socket exists
ok start :41 laneA
a=$(pid_of laneA 41)
live "$a" || fail "start: recorded pid $a is not alive"
[ "$(cat "/proc/$a/comm")" = Xvfb ] || fail "start: recorded pid is not the Xvfb"
[ -e "$tmp/x11/X41" ] || fail "start: no socket"

# a second start on a live display, a lock only, a socket only and :0 are refused
refuses "live display" start :41 laneB
[ ! -e "$tmp/scratch/laneB/xvfb-41.pid" ] || fail "refused start left a pid file"
touch "$tmp/lock/.X42-lock"
refuses "lock only" start :42 laneA
touch "$tmp/x11/X43"
refuses "socket only" start :43 laneA
refuses "display 0" start :0 laneA
refuses "bad display" start 41 laneA
refuses "bad lane" start :44 ../escape
live "$a" || fail "a refused start touched the live display"

# status lists ours and the unrecorded ones
ok status
case "$out" in *"ours  :41 lane laneA pid $a live Xvfb"*) ;; *) fail "status misses our display" ;; esac
case "$out" in *"other :43 socket"*) ;; *) fail "status misses the unrecorded socket" ;; esac
case "$out" in *"other :42 lock file only"*) ;; *) fail "status misses the lock-only display" ;; esac

# a second lane's display survives the first lane's stop
ok start :45 laneB
b=$(pid_of laneB 45)
refuses "another lane's display" stop :45 laneA
live "$b" || fail "laneA's refused stop killed laneB's display"

# a pid file naming a bystander (a reused pid) is refused and the bystander lives
# Its arguments carry `:46` too, so only the process name tells it from an Xvfb.
bash -c 'while :; do sleep 0.2; done' :46 &
by=$!
bystanders+=("$by")
mkdir -p "$tmp/scratch/laneC"
echo "$by" >"$tmp/scratch/laneC/xvfb-46.pid"
refuses "reused pid" stop :46 laneC
live "$by" || fail "stop killed a process that is not an Xvfb"
[ -e "$tmp/scratch/laneC/xvfb-46.pid" ] || fail "stop dropped the pid file of a live non-Xvfb"
ok status
case "$out" in *"pid reused by something else"*) ;; *) fail "status does not flag the reused pid" ;; esac

# a pid file naming an Xvfb of another display is refused too
echo "$b" >"$tmp/scratch/laneC/xvfb-47.pid"
refuses "Xvfb of another display" stop :47 laneC
live "$b" || fail "stop killed an Xvfb that serves another display"
rm -f "$tmp/scratch/laneC/xvfb-47.pid"

# stop with no record signals nothing
refuses "no pid file" stop :48 laneA

# stop kills the recorded pid, frees the display, and leaves the others
ok stop :41 laneA
gone "$a" || fail "stop left the recorded Xvfb running"
[ ! -e "$tmp/scratch/laneA/xvfb-41.pid" ] || fail "stop left the pid file"
[ ! -e "$tmp/x11/X41" ] || fail "display :41 still has a socket after stop"
live "$b" || fail "stopping laneA killed laneB's display"
live "$by" || fail "stopping laneA killed a bystander"

# a pid file whose process is gone is cleared without a signal
echo 999999 >"$tmp/scratch/laneC/xvfb-49.pid"
ok stop :49 laneC
[ ! -e "$tmp/scratch/laneC/xvfb-49.pid" ] || fail "a dead pid's file was kept"

# the display is free again, and start works a second time
ok start :41 laneA
ok stop :41 laneA
ok stop :45 laneB
gone "$b" || fail "stop left laneB's Xvfb running"
echo "check-xvfb-own: ok"
