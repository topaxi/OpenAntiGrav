#!/usr/bin/env bash
# One HD Time Trial original frame from a COLD boot (no savestate), the capture path that keeps the circuit's glass clean:
#   scripts/rpcs3-hd-cold-pair.sh <lane> <circuit-index> <out-dir> [oag-game]
# A restored savestate paints garbage on the stand glass (docs/reverse-engineering/rpcs3-capture.md, 2026-10-21 glass study).
# The lane's env.sh should set OAG_RPCS3_GEOMETRY=2200x1300x24 so the 1920x1080 window is not clipped. Writes <out>/theirs.png
# (and ours.png when the oag-game build knows the flags). The window is resized to the profile's output by rpcs3-hd-pair.py.
set -uo pipefail
lane=${1:?lane}; circuit=${2:?circuit index}; out=${3:?out dir}; game=${4:-/home/topaxi/projects/OpenAntiGrav/target/debug/oag-game}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
source "$W/env.sh"
export OAG_HD_CAPTURE_KIND=cold_boot  # recorded in pair.json provenance; no savestate was restored
export OAG_COMPARE_PLATFORM=hd-fury UV_CACHE_DIR=$HOME/.cache/uv UV_OFFLINE=1
cd "$repo"
# The original's drawn-hull shake is random; patch it out so the still pair matches ours' `--no-hull-shake`
# (`OAG_HD_KEEP_HULL_SHAKE=1` keeps it). Applied when RPCS3 loads the game, so before the boot below.
python3 scripts/rpcs3-hd-no-hull-shake.py ${OAG_HD_KEEP_HULL_SHAKE:+--remove} || exit 3
rm -f "$W/xdg/cache/rpcs3/TTY.log"
bash scripts/emu-hd-glass-boot.sh "$lane" "$circuit" 9 1 cold-$circuit || exit 3
export OAG_RPCS3_ATTACH=1
uv run --with evdev python3 scripts/emu-run.py -- python3 -u scripts/rpcs3-drive.py --log-dir "$W/restart" restart | tail -1
# The window is placed off-centre by RPCS3 and a 1920 wide resize then runs off the right edge of the Xvfb (a clipped frame the
# trim in rpcs3-drive.py turns into 1760 px): move it to the corner and size it before the pair script looks.
DISP="127.0.0.1:$OAG_RPCS3_DISPLAY"
win=$(for w in $(DISPLAY=$DISP xdotool search --onlyvisible --name ''); do DISPLAY=$DISP xdotool getwindowgeometry --shell $w | awk -F= -v w=$w '/WIDTH/{a=$2} /HEIGHT/{b=$2} END{print a*b, w}'; done | sort -n | tail -2 | head -1 | awk '{print $2}')
DISPLAY=$DISP xdotool windowsize "$win" 1920 1080 windowmove "$win" 0 0; sleep 4
mkdir -p "$out"
extra=(); [ -n "$game" ] && extra=(--oag-game "$game")
timeout 400 uv run --with evdev python3 scripts/rpcs3-hd-pair.py --out "$out" --team harimau "${extra[@]}"
