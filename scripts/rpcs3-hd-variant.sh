#!/usr/bin/env bash
# One HD Time Trial pair with RPCS3 settings changed on top of the compare profile (an artifact study).
#
#   scripts/rpcs3-hd-variant.sh <lane> <circuit> <out-dir> ['Video:Key=value;Video:Key=value']
#
# The extra edits go through OAG_RPCS3_EXTRA_EDITS (rpcs3-drive.py `scratch_config`), written into the
# private config and the game's custom config, so the variant is what RPCS3 reads back in its
# `Used configuration:` dump (copied beside the frames as rpcs3-used-config.txt).
set -uo pipefail
lane=${1:?lane}; circuit=${2:?circuit}; out=${3:?out}; extra=${4:-}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
source "$W/env.sh"
export OAG_HD_CAPTURE_KIND=savestate_restore  # provenance: this script restores a state
export OAG_COMPARE_PLATFORM=hd-fury UV_CACHE_DIR=$HOME/.cache/uv UV_OFFLINE=1 OAG_RPCS3_EXTRA_EDITS="$extra"
cd "$repo"
state=$main/data/saves/hd-fury/hd-timetrial-$circuit-harimau-classic-start.SAVESTAT.zst
for attempt in 1 2 3; do
  scripts/emu-restore-state.sh "$lane" "$state" --restart > "$W/restore.log" 2>&1 && ! grep -q 'did not reload\|failed' "$W/restore.log" && break
  echo "restore attempt $attempt failed" >&2
done
mkdir -p "$out"
OAG_RPCS3_ATTACH=1 timeout 500 uv run --with evdev python3 scripts/rpcs3-hd-pair.py --out "$out" \
  --team harimau --state "$state" 2>&1 | grep -E '^(ok|MISMATCH|REFUSED|parity)|rror'
rc=${PIPESTATUS[0]}
grep -E "Write Color|Resolution Scale|Strict Rendering|Shader Precision|Renderer:|Force CPU|Texture Flush|ColorSpace|Blit|Vertex Cache|Hardware Blending" \
  "$W/xdg/cache/rpcs3/RPCS3.log" 2>/dev/null | head -30 > "$out/rpcs3-used-config.txt"
echo "$extra" > "$out/variant.txt"
exit $rc
