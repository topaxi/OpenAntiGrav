#!/usr/bin/env bash
# Take an RPCS3 save state of whatever the running `serve` shows, into
# data/saves/hd-fury/<name>.SAVESTAT.zst. The emulator EXITS when the state is written
# ("Save Emulation State And Exit"), so this is the last step of a session.
#
#   scripts/emu-save-state.sh <lane> <name> [subdir]
#
# The serve must have been started with the stub off and Compatible Savestate Mode on
# (`OAG_RPCS3_NO_GDB=1 OAG_RPCS3_COMPAT_STATE=1 OAG_RPCS3_SUSPEND_STATE=1`, no
# OAG_RPCS3_GDB); see emu-rebuild-states.sh hd-grid for why each is needed.
set -euo pipefail
lane=${1:?lane}; name=${2:?state name}; sub=${3:-hd-fury}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
unset OAG_RPCS3_GDB OAG_RPCS3_ATTACH
cd "$repo"
rm -rf "$XDG_CONFIG_HOME"/rpcs3/savestates/*
[ "${MENU_OPEN:-0}" = 1 ] || python3 scripts/rpcs3-drive.py press ps --wait 4
python3 scripts/rpcs3-drive.py press up up up --wait 1.5
python3 scripts/rpcs3-drive.py press cross --wait 4
python3 scripts/rpcs3-drive.py press cross --wait 1
for _ in $(seq 1 45); do ls "$XDG_CONFIG_HOME"/rpcs3/savestates/*/*.zst >/dev/null 2>&1 && break; sleep 2; done
ls "$XDG_CONFIG_HOME"/rpcs3/savestates/*/*.zst >/dev/null 2>&1 \
  || { echo "SaveState wrote nothing (see the RPCS3.log tail)" >&2; exit 1; }
sleep 3
mkdir -p "$main/data/saves/$sub"
cp "$XDG_CONFIG_HOME"/rpcs3/savestates/*/*.zst "$main/data/saves/$sub/$name.SAVESTAT.zst"
ls -la "$main/data/saves/$sub/$name.SAVESTAT.zst"
python3 scripts/rpcs3-drive.py stop
