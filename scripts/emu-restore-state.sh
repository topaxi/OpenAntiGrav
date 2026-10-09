#!/usr/bin/env bash
# Restore an RPCS3 save state into a long-lived, drivable `serve`: working virtual
# pad, GDB proxy and /proc memory, in about 10 s.
#
#   scripts/emu-restore-state.sh <lane> <state-file> [--restart] [--no-gdb] [--serial BCES00664]
#
# <lane> is the scratch dir of a private tree (scripts/emu-env.sh <lane> ...).
# --restart then runs the pause menu's Restart Race (another ~8 s), which prints
# the TTY lines the attached scripts read (`Play welcome`, screen `HUD`); skip it
# only for a still of the saved moment. Afterwards attach as usual:
#   OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/<capture>.py ...
# Stop with `scripts/rpcs3-drive.py stop`. The state file is copied, never consumed.
set -euo pipefail
lane=${1:?lane}; state=${2:?state file}; shift 2
restart=0; serial=BCES00664; nogdb=0
while [ $# -gt 0 ]; do
  case "$1" in
    --restart) restart=1 ;;
    --no-gdb) nogdb=1 ;;
    --serial) serial=$2; shift ;;
    *) echo "unknown $1" >&2; exit 2 ;;
  esac; shift
done
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
unset OAG_RPCS3_ATTACH
# --no-gdb: the stub off, for a run that will take the next state (a stub on makes the
# save hang, see emu-rebuild-states.sh).
[ "$nogdb" = 1 ] && { unset OAG_RPCS3_GDB; export OAG_RPCS3_NO_GDB=1; }
cd "$repo"
python3 scripts/rpcs3-drive.py stop >/dev/null 2>&1 || true
sleep 3
: > "$W/serve.log"
t0=$(date +%s.%N)
(nohup env -u WAYLAND_DISPLAY uv run --with evdev python3 -u scripts/rpcs3-drive.py \
   --image "$main/data/images/hdfury-ps3-eu-dec.iso" --log-dir "$W/serve" serve \
   --load-state "$state" --serial "$serial" > "$W/serve.log" 2>&1 &)
for _ in $(seq 1 480); do
  grep -q "^serving\|Error\|Traceback" "$W/serve.log" && break
  sleep 0.25
done
grep -q "^serving" "$W/serve.log" || { tail -5 "$W/serve.log" >&2; echo "restore failed" >&2; exit 1; }
printf 'restored in %.1f s\n' "$(echo "$(date +%s.%N)-$t0" | bc)"
if [ "$restart" = 1 ]; then
  export OAG_RPCS3_ATTACH=1
  uv run --with evdev python3 scripts/emu-run.py -- python3 -u scripts/rpcs3-drive.py \
    --log-dir "$W/restart" restart 2>&1 | tail -2
fi
