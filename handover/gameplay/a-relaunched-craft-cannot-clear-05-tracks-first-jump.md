# A relaunched craft cannot clear 05_Track's first jump

2026-10-13, lane `jump-13-flight`. The original's reset relocation landed
(`crates/raceplay/src/breadcrumbs.rs`,
[reset-relocation.md](../../docs/ghidra/functions/psp-pulse-usa/reset-relocation.md)).
A craft that falls is put back on the oldest of its last seven grounded
positions, at `50 + 0.25 * last grounded speed`.
- On `13_Track` this matches the original: row 2949 against our 2939/2950,
  speed exact.
- It took the parked field there from 389 respawns to 2.
- At `13_Track` our crafts need no more speed than the original's: clearing at
  about 100 against 102-104, and a 98 u/s lip speed fell in the original too.

The maintainer chose to keep the port and re-baseline what it moved. The
numbers are in [ai.md](../../docs/gameplay/ai.md#after-a-reset-the-relaunch-falls-short-2026-10-13).
On `05_Track` a reset sends a craft back 50-100 samples before the first jump.
It reaches sample 150 at 107-117 u/s, where the jump needs about 120. Six rows
were raised to the measured values: `ai_clean_lap_gate` `05_Track` RAPIER, four
`ai_dekonstruct_symptoms` bounds and `ai_dekonstruct_black` RAPIER seed 2.
Lion's Den on Pulse physics falls off its start jump even at 212 u/s, so its
test now asks for one lap.

## Open

- Whether the original's relaunched craft clears `05_Track`'s first jump, and
  its lip speed. `05_Track` authors no no-crumb flag around that jump, so the
  original relaunches before it too.
- The air acceleration: ours is `-95 - 0.25 * vy` against the original's -89
  to -93 at `13_Track` (confidence 60), a 5-8 % gap near the apex. Not moved.
- Lion's Den on Pulse physics: its start jump is not cleared at any speed seen.

## Next Steps

1. Capture whether the original's relaunched craft clears `05_Track`'s first
   jump, and its lip speed: `scripts/psp-fall-drive.py` (pure-pursuit at a
   speed cap, the reset ring read every tick) with the dev-unlock byte, on
   de Konstruct Black.
2. If the original clears: a jump-physics lane, chasing the 5-8 % gravity gap
   and the acceleration after a relaunch. If it falls too, the six rows'
   re-baseline is the original's behaviour and this step closes.
3. Lion's Den on Pulse physics: why its start jump is not cleared, importer or
   physics.
