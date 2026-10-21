#!/usr/bin/env bash
# Cold-boot HD Fury EU and walk Time Trial to one circuit WITHOUT saving a state; the serve stays up on the race (a no-savestate arm of the glass study).
#   scripts/emu-hd-glass-boot.sh <lane> <circuit-index> <team-rights> <classic 0|1> <name> [class-rights]
# Needs `scripts/emu-env.sh <lane> ...` run first. The GDB stub stays ON and the savestate modes are off: the serve is then drivable by rpcs3-hd-pair.py (capture reads the camera and pose through the stub).
set -euo pipefail
lane=${1:?lane}; circuit=${2:?circuit index}; team=${3:?team rights}; classic=${4:?0|1}; name=${5:?state name}; speed=${6:-0}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
unset OAG_RPCS3_ATTACH
export OAG_RPCS3_SUSPEND_STATE=${OAG_RPCS3_SUSPEND_STATE:-0} OAG_RPCS3_COMPAT_STATE=${OAG_RPCS3_COMPAT_STATE:-0}
cd "$repo"
drive="python3 scripts/rpcs3-drive.py"
$drive stop >/dev/null 2>&1 || true
sleep 3
: > "$W/serve.log"
(nohup env -u WAYLAND_DISPLAY uv run --with evdev python3 -u scripts/rpcs3-drive.py \
   --image "$main/data/images/hdfury-ps3-eu-dec.iso" --log-dir "$W/serve" serve \
   > "$W/serve.log" 2>&1 &)
for _ in $(seq 1 240); do grep -q "^serving\|Error\|Traceback" "$W/serve.log" && break; sleep 2; done
grep -q "^serving" "$W/serve.log" || { tail -5 "$W/serve.log" >&2; echo "boot failed" >&2; exit 1; }
screen() { $drive status | sed -E 's/.*screen //'; }
press() { $drive press "$@" >/dev/null; }
# A first `right` is dropped about 1 time in 3 and the walk then enters Campaign: read the
# screen and back out until Racebox setup (`Single Player`) is up.
for _ in 1 2 3 4; do
  press right --wait 2
  press cross --wait 3
  [ "$(screen)" = "Single Player" ] && break
  for _ in 1 2 3; do [ "$(screen)" = "Main Menu" ] && break; press circle --wait 2.5; done
done
[ "$(screen)" = "Single Player" ] || { echo "never reached Racebox setup: $(screen)" >&2; exit 3; }
press right --wait 2                      # RACE TYPE: Time Trial
if [ "$speed" -gt 0 ]; then
  press down --wait 1                     # SPEED CLASS
  for _ in $(seq 1 "$speed"); do press right --wait 1; done
  import -display "127.0.0.1:${OAG_RPCS3_DISPLAY}" -window root "$W/class-$name.png"
  press up --wait 1                       # back to RACE TYPE, where `cross` was measured to go on
fi
press cross --wait 3                      # Track Select (the screen reads `Track Creation`)
[ "$(screen)" = "Track Creation" ] || { echo "not on Track Select: $(screen)" >&2; exit 3; }
for _ in $(seq 1 "$circuit"); do press right --wait 0.8; done
press cross --wait 3                      # Ship Select (reads `Team Selection`)
[ "$(screen)" = "Team Selection" ] || { echo "not on Ship Select: $(screen)" >&2; exit 3; }
for _ in $(seq 1 "$team"); do press right --wait 0.8; done
[ "$classic" = 1 ] && press down --wait 1
press cross --wait 3                      # the race loads
for _ in $(seq 1 60); do [ "$(screen)" = HUD ] && break; sleep 2; done
sleep 12
for _ in 1 2 3; do press cross --wait 4; done   # START RACE
sleep 8
grep -a "Loading World" "${XDG_CACHE_HOME:-$W/xdg/cache}/rpcs3/TTY.log" | tail -1 || { echo "no race loaded: $(screen)" >&2; exit 3; }
import -display "127.0.0.1:${OAG_RPCS3_DISPLAY}" -window root "$W/boot-$name.png"

