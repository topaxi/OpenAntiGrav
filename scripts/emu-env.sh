#!/usr/bin/env bash
# Build a member's private emulator environment in one command.
#
#   scripts/emu-env.sh <lane> <display> <gdb-port> [ppsspp-port]
#
# Writes data/scratch/<lane>/env.sh (source it) and the private XDG tree
# RPCS3 and PPSSPP need: a copy of ~/.config/rpcs3 whose heavy read-only parts
# (dev_flash*) are symlinks, so it costs ~2 GB instead of copying a firmware
# tree, a virtual pad named for the lane, and the input profile that names it.
# Nothing under it is committed; it lives in gitignored data/.
set -euo pipefail
lane=${1:?lane} ; disp=${2:?display number, e.g. 94} ; gdb=${3:?gdb port} ; pport=${4:-}
main=/home/topaxi/projects/OpenAntiGrav
W=$main/data/scratch/$lane
X=$W/xdg
mkdir -p "$X"/{cache,config,data,state} "$W/logs"
src=$HOME/.config/rpcs3
dst=$X/config/rpcs3
if [ ! -d "$dst" ]; then
  mkdir -p "$dst"
  for item in "$src"/*; do
    n=$(basename "$item")
    case "$n" in
      dev_flash|dev_flash2|dev_flash3|dev_flash.installed-2.76|Icons|patches)
        ln -s "$item" "$dst/$n" ;;
      recordings|screenshots|savestates|captures) mkdir -p "$dst/$n" ;;
      *) cp -a "$item" "$dst/$n" ;;
    esac
  done
fi
pad="OAG Pad $lane"
mkdir -p "$dst/input_configs/global"
printf 'Player 1 Input:\n  Handler: Evdev\n  Device: %s\n' "$pad" > "$dst/input_configs/global/oag.yml"
cat > "$W/env.sh" <<ENV
W=$W
export OAG_RPCS3_DISPLAY=$disp OAG_RPCS3_GDB=127.0.0.1:$gdb OAG_RPCS3_GEOMETRY=\${OAG_RPCS3_GEOMETRY:-2000x1200x24} "OAG_RPCS3_PAD_NAME=$pad" OAG_RPCS3_SCRATCH_CONFIG=\$W/rpcs3-config.yml XDG_CACHE_HOME=\$W/xdg/cache XDG_CONFIG_HOME=\$W/xdg/config XDG_DATA_HOME=\$W/xdg/data XDG_STATE_HOME=\$W/xdg/state
ENV
[ -n "$pport" ] && echo "export OAG_PPSSPP_PORT=$pport" >> "$W/env.sh"
echo "wrote $W/env.sh; source it, then: uv run --with evdev python3 scripts/rpcs3-drive.py ..."
