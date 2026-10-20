#!/usr/bin/env bash
# Cold-boot HD Fury EU and walk Racebox SINGLE RACE with WEAPONS OFF to one circuit, then leave the
# `serve` running on the grid (the race is loaded, START RACE not yet pressed).
#
#   scripts/emu-hd-race-walk.sh <lane> <circuit-index> [team-rights [classic 0|1]]
#
# The recipe of `emu-hd-timetrial-state.sh` (stub off, Compatible Savestate Mode, so a state can be
# saved later by `emu-save-state.sh`) with the Racebox page's rows for a race: RACE TYPE is Single
# Race by default, `down` x2 reaches WEAPONS and one `right` turns it Off (screenshotted 2026-10-20:
# RACE TYPE, SPEED CLASS, WEAPONS, AI DIFFICULTY, NUMBER OF PLAYERS). Needs `scripts/emu-env.sh <lane>`.
set -euo pipefail
lane=${1:?lane}; circuit=${2:?circuit index}; team=${3:-0}; classic=${4:-0}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
unset OAG_RPCS3_GDB OAG_RPCS3_ATTACH
export OAG_RPCS3_SUSPEND_STATE=1 OAG_RPCS3_COMPAT_STATE=1 OAG_RPCS3_NO_GDB=1 OAG_COMPARE_PLATFORM=hd-fury
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
export OAG_RPCS3_ATTACH=1
screen() { $drive status | sed -E 's/.*screen //'; }
press() { $drive press "$@" >/dev/null; }
for _ in 1 2 3 4; do
  press right --wait 2
  press cross --wait 3
  [ "$(screen)" = "Single Player" ] && break
  for _ in 1 2 3; do [ "$(screen)" = "Main Menu" ] && break; press circle --wait 2.5; done
done
[ "$(screen)" = "Single Player" ] || { echo "never reached Racebox setup: $(screen)" >&2; exit 3; }
press down --wait 1; press down --wait 1      # WEAPONS
press right --wait 1.5                        # On -> Off
press cross --wait 3                          # Track Select (reads `Track Creation`)
[ "$(screen)" = "Track Creation" ] || { echo "not on Track Select: $(screen)" >&2; exit 3; }
for _ in $(seq 1 "$circuit"); do press right --wait 0.8; done
press cross --wait 3                          # Ship Select (reads `Team Selection`)
[ "$(screen)" = "Team Selection" ] || { echo "not on Ship Select: $(screen)" >&2; exit 3; }
for _ in $(seq 1 "$team"); do press right --wait 0.8; done
[ "$classic" = 1 ] && press down --wait 1
press cross --wait 3                          # the race loads
for _ in $(seq 1 60); do [ "$(screen)" = HUD ] && break; sleep 2; done
[ "$(screen)" = HUD ] || { echo "race did not come up: $(screen)" >&2; exit 3; }
grep -a "Loading World" "$XDG_CACHE_HOME/rpcs3/TTY.log" | tail -1
echo "on the grid; weapons Off was set on the Racebox page (verify with a screenshot of the page or the race)"
