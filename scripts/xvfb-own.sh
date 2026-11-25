#!/usr/bin/env bash
# A virtual display that records who started it, so cleanup never has to guess.
#
#   scripts/xvfb-own.sh start :N <lane>   start Xvfb on :N, pid into the lane's scratch dir
#   scripts/xvfb-own.sh stop  :N <lane>   stop the Xvfb this lane recorded for :N, nothing else
#   scripts/xvfb-own.sh status            every recorded display, and every live one nobody recorded
#
# `start` refuses a display whose socket or lock file exists, even a stale one:
# removing it would be a guess about who owned it. `stop` signals only the pid
# `start` recorded, and only while that pid is still an `Xvfb` serving that same
# `:N` (a pid file outlives its process, and a pid can be reused). A display or
# emulator started by anything else is never signalled; it is listed by `status`
# as not ours.
#
# The pid file is `data/scratch/<lane>/xvfb-N.pid` under the main checkout, so
# `status` sees every lane's displays from any worktree. The environment
# overrides are for `scripts/check-xvfb-own.sh`, which runs a stand-in `Xvfb`.
set -euo pipefail

x11_dir=${XVFB_OWN_X11_DIR:-/tmp/.X11-unix}
lock_dir=${XVFB_OWN_LOCK_DIR:-/tmp}
xvfb_bin=${XVFB_OWN_BIN:-Xvfb}
screen=${XVFB_OWN_SCREEN:-1280x720x24}
start_wait=${XVFB_OWN_START_WAIT:-100}
stop_wait=${XVFB_OWN_STOP_WAIT:-50}

die() { echo "xvfb-own: $*" >&2; exit 1; }

scratch_root() {
    if [ -n "${XVFB_OWN_SCRATCH:-}" ]; then
        printf '%s\n' "$XVFB_OWN_SCRATCH"
        return
    fi
    local common
    common=$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --path-format=absolute --git-common-dir) \
        || die "not inside the repository; set XVFB_OWN_SCRATCH"
    printf '%s\n' "$(dirname "$common")/data/scratch"
}

# `:N` with N a positive number. `:0` is the maintainer's display.
display_number() {
    [[ $1 =~ ^:([1-9][0-9]*)$ ]] || die "display must look like :92 (and never :0), got '$1'"
    printf '%s\n' "${BASH_REMATCH[1]}"
}

lane_dir() {
    [[ $1 =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ && $1 != *..* ]] || die "bad lane name '$1'"
    printf '%s/%s\n' "$(scratch_root)" "$1"
}

alive() { [ -d "/proc/$1" ]; }

# True while pid $1 is an Xvfb with `:N` as one of its arguments.
is_xvfb_for() {
    local pid=$1 n=$2 comm
    comm=$(cat "/proc/$pid/comm" 2>/dev/null) || return 1
    [ "$comm" = Xvfb ] || return 1
    tr '\0' '\n' <"/proc/$pid/cmdline" 2>/dev/null | grep -qxF ":$n"
}

read_pid() {
    local pid
    pid=$(cat "$1" 2>/dev/null) || return 1
    [[ $pid =~ ^[0-9]+$ ]] || return 1
    printf '%s\n' "$pid"
}

# Remove one pid file, only if it is a `xvfb-N.pid` inside the scratch root.
remove_pidfile() {
    case "$1" in
        "$(scratch_root)"/*/xvfb-*.pid) rm -f -- "$1" ;;
        *) die "refusing to remove '$1'" ;;
    esac
}

cmd_start() {
    [ $# -eq 2 ] || die "usage: start :N <lane>"
    local n dir pidfile log pid i
    n=$(display_number "$1")
    dir=$(lane_dir "$2")
    pidfile=$dir/xvfb-$n.pid
    log=$dir/xvfb-$n.log
    if [ -e "$x11_dir/X$n" ] || [ -e "$lock_dir/.X$n-lock" ]; then
        die ":$n is taken ($x11_dir/X$n or $lock_dir/.X$n-lock exists); pick the next free number and tell the lead. Never remove another display's lock."
    fi
    mkdir -p -- "$dir"
    [ ! -e "$pidfile" ] || remove_pidfile "$pidfile"
    # setsid keeps the display alive past the caller's shell; the inner `exec`
    # makes the recorded pid the Xvfb itself.
    # shellcheck disable=SC2016 # the inner shell expands its own $$ and $1
    setsid bash -c 'echo "$$" >"$1"; shift; exec "$@"' _ "$pidfile" \
        "$xvfb_bin" ":$n" -screen 0 "$screen" -nolisten tcp </dev/null >"$log" 2>&1 &
    for ((i = 0; i < start_wait; i++)); do
        if [ -e "$x11_dir/X$n" ]; then
            pid=$(read_pid "$pidfile") || die "Xvfb came up on :$n but wrote no pid"
            is_xvfb_for "$pid" "$n" || die "pid $pid is not an Xvfb for :$n; see $log"
            echo "started :$n pid $pid lane $2 (stop: scripts/xvfb-own.sh stop :$n $2)"
            return 0
        fi
        if pid=$(read_pid "$pidfile") && ! alive "$pid"; then
            remove_pidfile "$pidfile"
            die "Xvfb exited while starting :$n; see $log"
        fi
        sleep 0.1
    done
    if pid=$(read_pid "$pidfile") && is_xvfb_for "$pid" "$n"; then
        kill -TERM "$pid" 2>/dev/null || true
    fi
    [ ! -e "$pidfile" ] || remove_pidfile "$pidfile"
    die "no socket at $x11_dir/X$n after $((start_wait / 10)) s; see $log"
}

cmd_stop() {
    [ $# -eq 2 ] || die "usage: stop :N <lane>"
    local n dir pidfile pid i
    n=$(display_number "$1")
    dir=$(lane_dir "$2")
    pidfile=$dir/xvfb-$n.pid
    [ -e "$pidfile" ] || die "lane $2 recorded no Xvfb for :$n; not ours to stop"
    pid=$(read_pid "$pidfile") || die "unreadable pid file $pidfile; not signalling anything"
    if ! alive "$pid"; then
        remove_pidfile "$pidfile"
        echo ":$n pid $pid already gone; pid file removed"
        return 0
    fi
    is_xvfb_for "$pid" "$n" \
        || die "pid $pid is not an Xvfb for :$n (reused pid, or a different display); not signalling it. Pid file kept: $pidfile"
    kill -TERM "$pid"
    for ((i = 0; i < stop_wait; i++)); do
        alive "$pid" || break
        sleep 0.1
    done
    if alive "$pid"; then
        is_xvfb_for "$pid" "$n" && kill -KILL "$pid" 2>/dev/null || true
        echo "xvfb-own: :$n needed SIGKILL; $lock_dir/.X$n-lock may be left behind and is not removed" >&2
    fi
    remove_pidfile "$pidfile"
    echo "stopped :$n pid $pid lane $2"
}

cmd_status() {
    [ $# -eq 0 ] || die "usage: status"
    shopt -s nullglob
    local root f lane n pid state recorded=" " sock lock
    root=$(scratch_root)
    for f in "$root"/*/xvfb-*.pid; do
        lane=$(basename "$(dirname "$f")")
        n=$(basename "$f" .pid)
        n=${n#xvfb-}
        recorded="$recorded$n "
        if ! pid=$(read_pid "$f"); then
            state="unreadable pid file"
        elif ! alive "$pid"; then
            state="dead (stale pid file)"
        elif is_xvfb_for "$pid" "$n"; then
            state="live Xvfb"
        else
            state="pid reused by something else (do not signal)"
        fi
        printf 'ours  :%s lane %s pid %s %s\n' "$n" "$lane" "${pid:-?}" "$state"
    done
    for sock in "$x11_dir"/X[0-9]*; do
        n=${sock##*/X}
        case "$recorded" in *" $n "*) continue ;; esac
        printf 'other :%s socket, no pid file here (not ours; never kill)\n' "$n"
    done
    for lock in "$lock_dir"/.X[0-9]*-lock; do
        n=${lock##*/.X}
        n=${n%-lock}
        case "$recorded" in *" $n "*) continue ;; esac
        [ -e "$x11_dir/X$n" ] && continue
        printf 'other :%s lock file only (stale or a TCP-only display; not ours)\n' "$n"
    done
}

case "${1:-}" in
    start) shift; cmd_start "$@" ;;
    stop) shift; cmd_stop "$@" ;;
    status) shift; cmd_status "$@" ;;
    *) die "usage: $0 start :N <lane> | stop :N <lane> | status" ;;
esac
