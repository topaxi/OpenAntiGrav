#!/usr/bin/env bash
# Start a private, muted PPSSPP SDL instance under its own Xvfb.
#
#   scripts/ppsspp-start.sh <lane> <display> <port> <image>
#
# Writes data/scratch/<lane>/pp/{xvfb.pid,ppsspp.pid,env.sh}; stop with
#   scripts/ppsspp-start.sh <lane> stop
# Both are killed by the pid recorded here, never by name. The instance has its
# own HOME (so its memstick, profile and save states are private), SDL's dummy
# audio driver, and `[Sound] Enable = False`; the debugger listens on <port>.
set -euo pipefail
lane=${1:?lane}
W=/home/topaxi/projects/OpenAntiGrav/data/scratch/$lane/pp
if [ "${2:-}" = stop ]; then
  for f in ppsspp.pid xvfb.pid; do
    [ -f "$W/$f" ] && kill "$(cat "$W/$f")" 2>/dev/null || true
  done
  echo "stopped"; exit 0
fi
disp=${2:?display number}; port=${3:?debugger port}; image=${4:?image}
mkdir -p "$W/home/.config/ppsspp/PSP/SYSTEM"
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = %s\n[Sound]\nEnable = False\n' "$port" > "$W/debugger.ini"
Xvfb ":$disp" -screen 0 1280x800x24 -listen tcp -nolisten unix > "$W/xvfb.log" 2>&1 &
echo $! > "$W/xvfb.pid"; sleep 1
(cd "$W"; env -u WAYLAND_DISPLAY DISPLAY=127.0.0.1:$disp HOME="$W/home" \
  XDG_CONFIG_HOME="$W/home/.config" XDG_DATA_HOME="$W/home/.local/share" \
  XDG_CACHE_HOME="$W/home/.cache" SDL_AUDIODRIVER=dummy SDL_VIDEODRIVER=x11 \
  setsid PPSSPPSDL --appendconfig="$W/debugger.ini" --windowed "$image" \
  < /dev/null > "$W/ppsspp.log" 2>&1 & echo $! > "$W/ppsspp.pid")
cat > "$W/env.sh" <<ENV
export DISPLAY=127.0.0.1:$disp OAG_PPSSPP_PORT=$port PP_HOME=$W/home
ENV
echo "PPSSPP up on display :$disp, debugger port $port (source $W/env.sh); first boot needs: uv run --with websocket-client python3 scripts/psp-drive.py --port $port menu"
