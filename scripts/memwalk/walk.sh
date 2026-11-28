#!/usr/bin/env bash
# docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
# walk.sh pulse|hd > walk.drive; SHOTS stands for the screenshot directory.
set -euo pipefail
hold() { for _ in $(seq "$1"); do printf 'key; ticks 1000\n'; done; }
case ${1:-} in
  pulse) finish="ticks 2800; until endrace=photo within 1500
ticks 100; press cross; ticks 100; press cross; ticks 100"; end=8000; watch="press down down down" ;;
  hd) finish="ticks 2800; until endrace=other within 600
ticks 100; press cross; ticks 100"; end=7700; watch="press down down" ;;
  *) echo "usage: walk.sh pulse|hd" >&2; exit 2 ;;
esac
cat <<WALK
boot; select racebox; press cross; until page=race
set race.mode=time_trial
$(hold 250)
select launch_race; press cross; until screen=picker; press cross
until screen.id=ship-select; press cross; until stage=race within 4000
ticks 2800; until sim_tick=3000 within 600; screenshot SHOTS/a.png; until sim_tick=3600 within 700
$finish
screenshot SHOTS/b.png; until sim_tick=$end within 1500
$watch; press cross; until replay=playing within 4000
ticks 2800; until sim_tick=3000 within 600; screenshot SHOTS/c.png; until sim_tick=3600 within 700
escape; until page=replays within 400; ticks 30; press circle; until page=main within 100
ticks 60; screenshot SHOTS/d.png
$(hold 700)
quit
WALK
