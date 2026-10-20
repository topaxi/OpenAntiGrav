#!/usr/bin/env bash
# Every kept HD Time Trial state, one after the other: restore it into a fresh `serve` (the profile's video
# settings written into the game's custom config), `Restart Race`, then `rpcs3-hd-pair.py`.
#
#   scripts/emu-env.sh <lane> <display> <gdb-port>      # once
#   scripts/compare-hd-batch.sh <lane> <out-dir> [only-substring]
#
# A pair is refused (exit 3) when its environment differs from the profile; the batch carries on and reports
# the failures at the end. Needs `OAG_COMPARE_PLATFORM` (set here), a short XDG_CACHE_HOME is NOT used: the
# pad socket path comes from env.sh.
set -uo pipefail
lane=${1:?lane}; out=${2:?out dir}; only=${3:-}
main=/home/topaxi/projects/OpenAntiGrav
repo=$(cd "$(dirname "$0")/.." && pwd)
W=$main/data/scratch/$lane
# shellcheck disable=SC1091
source "$W/env.sh"
export OAG_COMPARE_PLATFORM=hd-fury UV_CACHE_DIR=$HOME/.cache/uv UV_OFFLINE=1
cd "$repo"
declare -A TITLE=(
  [vineta-k]="Vineta K" [anulpha-pass]="Anulpha Pass" [moa-therma]="Moa Therma" [chenghou-project]="Chenghou Project"
  [metropia]="Metropia" [sebenco-climb]="Sebenco Climb" [ubermall]="Ubermall" [sol-2]="Sol 2" [talons]="Talon's Junction"
  [the-amphiseum]="The Amphiseum" [modesto-heights]="Modesto Heights" [tech-de-ra]="Tech de Ra")
failed=0
for state in "$main"/data/saves/hd-fury/hd-timetrial-*-harimau-classic-start.SAVESTAT.zst \
             "$main"/data/saves/hd-fury/hd-timetrial-talons-assegai-start.SAVESTAT.zst; do
  name=$(basename "$state" .SAVESTAT.zst)
  [ -n "$only" ] && [[ "$name" != *"$only"* ]] && continue
  stem=${name#hd-timetrial-}
  if [[ "$stem" == talons-assegai-start ]]; then
    circuit=talons; team=Assegai_c1; craft="Assegai Fury Concept (Assegai_c1)"; id=hd-talons-assegai-c1
  else
    circuit=${stem%-harimau-classic-start}; team=harimau; craft="Harimau classic"; id=hd-$circuit
  fi
  echo "== $name"
  ok=0
  for attempt in 1 2; do
    if scripts/emu-restore-state.sh "$lane" "$state" --restart 2>&1 | tee "$W/restore.log" | tail -2 \
        && ! grep -q 'did not reload\|failed' "$W/restore.log"; then ok=1; break; fi
    echo "restore attempt $attempt failed"
  done
  [ $ok = 1 ] || { echo "-> FAILED restore"; failed=$((failed+1)); continue; }
  meta=$(printf '{"id":"%s","title":"%s, %s, Time Trial start","track":"%s, Time Trial","craft":"%s","state":"Time Trial, a few seconds after GO of a fresh Restart Race; lap clock matched to the original (read from its RAM); ship and camera pose read from the original"}' \
    "$id" "${TITLE[$circuit]}" "$craft" "${TITLE[$circuit]}" "$craft")
  OAG_RPCS3_ATTACH=1 timeout 500 uv run --with evdev python3 scripts/rpcs3-hd-pair.py --out "$out/${id#hd-}" \
    --team "$team" --state "$state" --meta "$meta" 2>&1 | grep -E '^(ok|MISMATCH|REFUSED|parity)|rror'
  [ "${PIPESTATUS[0]}" = 0 ] || { echo "-> FAILED rc=${PIPESTATUS[0]}"; failed=$((failed+1)); }
done
python3 scripts/rpcs3-drive.py stop >/dev/null 2>&1 || true
echo "failed pairs: $failed"
exit $failed
