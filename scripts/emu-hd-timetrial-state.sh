#!/usr/bin/env bash
# Cold-boot HD Fury EU, walk Racebox Time Trial to one circuit and craft, and save the
# race as a state under data/saves/hd-fury/. The emulator exits when the state is written.
#
#   scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0|1> <state-name>
#
# <circuit-index> is the number of `right`s from the first Track Select circuit (Vineta K
# is 0, Anulpha Pass 1, Talon's Junction 8); <team-rights> the number from Ship Select's
# first row (Feisar): Assegai 7, Harimau 9; <classic> adds the `down` that picks the
# classic hull over the Fury `_c1` one (docs/reverse-engineering/emulator-recipes.md).
# Needs `scripts/emu-env.sh <lane> ...` run first. Stub off, Compatible Savestate Mode on,
# the recipe that writes a state. Read the circuit off the TTY log afterwards
# (`Loading World`), not off the press count: a dropped press picks the neighbour.
set -euo pipefail
lane=${1:?lane}; circuit=${2:?circuit index}; team=${3:?team rights}; classic=${4:?0|1}; name=${5:?state name}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
unset OAG_RPCS3_GDB OAG_RPCS3_ATTACH
export OAG_RPCS3_SUSPEND_STATE=1 OAG_RPCS3_COMPAT_STATE=1 OAG_RPCS3_NO_GDB=1
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
grep -a "Loading World" "$W/xdg/cache/rpcs3/TTY.log" | tail -1 || { echo "no race loaded: $(screen)" >&2; exit 3; }
import -display "127.0.0.1:${OAG_RPCS3_DISPLAY}" -window root "$W/state-$name.png"
MENU_OPEN=0 "$repo/scripts/emu-save-state.sh" "$lane" "$name"
