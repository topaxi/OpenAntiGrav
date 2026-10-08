#!/usr/bin/env bash
# Rebuild a named save state from the user's own disc image, into data/saves/.
#
#   scripts/emu-rebuild-states.sh <lane> pulse-grid     # PPSSPP, Pulse USA, Time Trial Talons VENOM grid
#   scripts/emu-rebuild-states.sh <lane> hd-grid        # RPCS3, HD Fury EU, Talons grid fly-over
#
# <lane> names the scratch dir and its env.sh (scripts/emu-env.sh for RPCS3,
# scripts/ppsspp-start.sh for PPSSPP: run those first and source the env.sh).
# States are game-derived data: they go ONLY under data/saves/<title>/ (gitignored).
# See docs/reverse-engineering/emulator-recipes.md for what each restores to.
set -euo pipefail
lane=${1:?lane}; point=${2:?pulse-grid|hd-grid}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
case "$point" in
  pulse-grid)
    : "${OAG_PPSSPP_PORT:?source scratch/<lane>/pp/env.sh first}"; : "${DISPLAY:?}"; : "${PP_HOME:?}"
    cd "$repo"
    uv run --with websocket-client python3 scripts/psp-drive.py --port "$OAG_PPSSPP_PORT" menu
    uv run --with websocket-client python3 scripts/psp-state.py --home "$PP_HOME" save 1 \
      --keep "$main/data/saves/pulse-psp/time-trial-talons-venom-grid.ppst"
    ;;
  hd-grid)
    # The stub must be OFF while the state is taken (a listening GDB stub makes the
    # overlay's SaveState a silent no-op) and the state restores with it off.
    : "${XDG_CONFIG_HOME:?source scratch/<lane>/env.sh first}"
    unset OAG_RPCS3_GDB
    export OAG_RPCS3_SUSPEND_STATE=1
    sed -i 's/GDB Server: .*/GDB Server: ""/' "$XDG_CONFIG_HOME/rpcs3/config.yml"
    rm -rf "$XDG_CONFIG_HOME"/rpcs3/savestates/*
    cd "$repo"
    (nohup uv run --with evdev python3 -u scripts/rpcs3-drive.py \
       --image "$main/data/images/hdfury-ps3-eu-dec.iso" --log-dir "$W/serve" serve \
       > "$W/serve.log" 2>&1 &)
    for _ in $(seq 1 120); do grep -q "^serving\|Error" "$W/serve.log" && break; sleep 2; done
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 -u scripts/rpcs3-drive.py \
      --log-dir "$W/race" race --load 8
    python3 scripts/rpcs3-drive.py press ps --wait 4
    python3 scripts/rpcs3-drive.py press up up up cross cross --wait 1.5
    for _ in $(seq 1 30); do ls "$XDG_CONFIG_HOME"/rpcs3/savestates/*/*.zst >/dev/null 2>&1 && break; sleep 2; done
    mkdir -p "$main/data/saves/hd-fury"
    cp "$XDG_CONFIG_HOME"/rpcs3/savestates/*/*.zst "$main/data/saves/hd-fury/grid-flyover-talons.SAVESTAT.zst"
    python3 scripts/rpcs3-drive.py stop
    ;;
  *) echo "unknown point $point" >&2; exit 2 ;;
esac
ls -la "$main"/data/saves/*/
