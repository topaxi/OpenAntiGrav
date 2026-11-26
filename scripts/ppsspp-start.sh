#!/usr/bin/env bash
# Start a private, muted PPSSPP SDL instance under its own Xvfb.
#
#   scripts/ppsspp-start.sh <lane> <display> <port> <image>
#
# Writes data/scratch/<lane>/pp/{xvfb.pid,ppsspp.pid,env.sh}; stop with
#   scripts/ppsspp-start.sh <lane> stop
# The instance has its own HOME (so its memstick, profile and save states are
# private), SDL's dummy audio driver, and `[Sound] Enable = False`; the debugger
# listens on <port>.
#
# `stop` signals only the two pids recorded here, never by name, and each only
# while it is still this lane's process (PPSSPP carrying this lane's config, an
# Xvfb on this lane's display). PPSSPP goes first and is killed if TERM does not
# end it; the display goes once it is gone. It is safe to run twice. `start`
# refuses a lane that is still running, which would overwrite the pid files and
# leave that instance running for good.
# `PPSSPP_START_SCRATCH`, `PPSSPP_STOP_WAIT` (tenths of a second) and
# `PPSSPP_XVFB_WAIT` (seconds) are for `scripts/check-ppsspp-start.sh`.
set -euo pipefail
lane=${1:?lane}
[[ $lane =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ && $lane != *..* ]] || { echo "ppsspp-start: bad lane name '$lane'" >&2; exit 1; }
W=${PPSSPP_START_SCRATCH:-/home/topaxi/projects/OpenAntiGrav/data/scratch}/$lane/pp
stop_wait=${PPSSPP_STOP_WAIT:-100}
xvfb_wait=${PPSSPP_XVFB_WAIT:-1}

# The pid a pid file records, or failure.
recorded() {
  local pid
  pid=$(cat "$W/$1.pid" 2>/dev/null) || return 1
  [[ $pid =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "$pid"
}

# A process that exists and is not a zombie.
alive() {
  local stat
  stat=$(cat "/proc/$1/stat" 2>/dev/null) || return 1
  stat=${stat##*) }
  [ "${stat%% *}" != Z ]
}

has_arg() {
  tr '\0' '\n' <"/proc/$1/cmdline" 2>/dev/null | grep -qxF -- "$2"
}

# True while pid $2 is still the process this lane recorded as $1.
ours() {
  local n
  case $1 in
    ppsspp) has_arg "$2" "--appendconfig=$W/debugger.ini" ;;
    xvfb)
      n=$(sed -n 's/^export DISPLAY=127\.0\.0\.1:\([0-9][0-9]*\) .*/\1/p' "$W/env.sh" 2>/dev/null)
      [ -n "$n" ] && [ "$(cat "/proc/$2/comm" 2>/dev/null)" = Xvfb ] && has_arg "$2" ":$n" ;;
  esac
}

# PPSSPP is started with `setsid`, so its pid leads a group that holds all it spawned.
signal() {
  local sig=$1 pid=$2 stat pgrp
  stat=$(cat "/proc/$pid/stat" 2>/dev/null) || return 0
  read -r _ _ pgrp _ <<<"${stat##*) }"
  if [ "$pgrp" = "$pid" ]; then
    kill -"$sig" -- "-$pid" 2>/dev/null || true
  else
    kill -"$sig" "$pid" 2>/dev/null || true
  fi
}

wait_gone() {
  local i
  for ((i = 0; i < stop_wait; i++)); do
    alive "$1" || return 0
    sleep 0.1
  done
  ! alive "$1"
}

# Stop what $1.pid records. Fails only if one of ours survives SIGKILL.
stop_one() {
  local what=$1 pid
  [ -e "$W/$what.pid" ] || return 0
  if ! pid=$(recorded "$what"); then
    echo "$what: unreadable pid file removed"
  elif ! alive "$pid"; then
    echo "$what: pid $pid already gone"
  elif ! ours "$what" "$pid"; then
    echo "$what: pid $pid is not ours (the number was reused); not signalled"
  else
    signal TERM "$pid"
    if wait_gone "$pid"; then
      echo "$what: stopped pid $pid"
    else
      signal KILL "$pid"
      if ! wait_gone "$pid"; then
        echo "ppsspp-start: $what pid $pid survived SIGKILL; pid file kept: $W/$what.pid" >&2
        return 1
      fi
      echo "$what: pid $pid ignored SIGTERM, stopped with SIGKILL"
    fi
  fi
  rm -f -- "$W/$what.pid"
}

if [ "${2:-}" = stop ]; then
  if [ ! -e "$W/ppsspp.pid" ] && [ ! -e "$W/xvfb.pid" ]; then
    echo "nothing running for lane $lane"; exit 0
  fi
  rc=0
  # The display stays up while PPSSPP does: it may need it to quit.
  if stop_one ppsspp; then stop_one xvfb || rc=1; else rc=1; fi
  exit "$rc"
fi
disp=${2:?display number}; port=${3:?debugger port}; image=${4:?image}
for what in ppsspp xvfb; do
  if pid=$(recorded "$what") && alive "$pid" && ours "$what" "$pid"; then
    echo "ppsspp-start: lane $lane is still running ($what pid $pid); stop it first: $0 $lane stop" >&2
    exit 1
  fi
done
mkdir -p "$W/home/.config/ppsspp/PSP/SYSTEM"
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = %s\n[Sound]\nEnable = False\n[Graphics]\nGraphicsBackend = 0 (OPENGL)\nSoftwareRenderer = True\n' "$port" > "$W/debugger.ini"
# Extra ini text (e.g. the compare profile's picture settings, scripts/compare_env.py ppsspp-ini):
# appended after the generated sections, so a later [Graphics] key overrides the default one.
if [ -n "${PPSSPP_EXTRA_INI:-}" ]; then cat "$PPSSPP_EXTRA_INI" >> "$W/debugger.ini"; fi
# Written before anything starts: `stop` reads the display number from it.
cat > "$W/env.sh" <<ENV
export DISPLAY=127.0.0.1:$disp OAG_PPSSPP_PORT=$port PP_HOME=$W/home
ENV
Xvfb ":$disp" -screen 0 1280x800x24 -listen tcp -nolisten unix > "$W/xvfb.log" 2>&1 &
echo $! > "$W/xvfb.pid"; sleep "$xvfb_wait"
(cd "$W"; env -u WAYLAND_DISPLAY DISPLAY=127.0.0.1:$disp HOME="$W/home" \
  XDG_CONFIG_HOME="$W/home/.config" XDG_DATA_HOME="$W/home/.local/share" \
  XDG_CACHE_HOME="$W/home/.cache" SDL_AUDIODRIVER=dummy SDL_VIDEODRIVER=x11 \
  setsid PPSSPPSDL --appendconfig="$W/debugger.ini" --windowed "$image" \
  < /dev/null > "$W/ppsspp.log" 2>&1 & echo $! > "$W/ppsspp.pid")
echo "PPSSPP up on display :$disp, debugger port $port (source $W/env.sh); first boot needs: uv run --with websocket-client python3 scripts/psp-drive.py --port $port menu"
