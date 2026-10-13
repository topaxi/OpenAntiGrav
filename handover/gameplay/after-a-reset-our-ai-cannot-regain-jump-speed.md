# After a reset, our AI cannot get back over a jump

2026-10-13, lane `jump-13-flight`. The original's reset relocation landed
(`crates/raceplay/src/breadcrumbs.rs`,
[reset-relocation.md](../../docs/ghidra/functions/psp-pulse-usa/reset-relocation.md)).
A craft that falls is put back on the oldest of its last seven grounded
positions, a few hundred units before where it fell, at
`50 + 0.25 * last grounded speed`. On `13_Track` this matches the original
(row 2949 against our 2939/2950, speed exact) and took the parked field from
389 respawns to 2. Seven disc-backed tests went red with it, and the lead
chose to keep the port and leave them red under this thread (option C). With
the old pose all seven pass.

The old pose put a craft back at rest on the sample nearest its fall, often
past the jump, which skipped part of the lap. The new one sends it back before
the jump, and our physics and driver cannot get back over from there.

## Open

- **`05_Track` (de Konstruct Black), six tests.** The disc authors no
  no-crumb flag around its first jump, so the original also lays crumbs up to
  the lip at 162.
  - A craft reset after a fall is relaunched 50-100 samples back at 64-107 u/s.
    It reaches idx 150 at 107-117 u/s; the jump needs about 120, and a
    normal pass is 128-135. It then falls into the pit or hits the lip at
    206-209.
  - `ai_clean_lap_gate track_05_rapier`: 0 -> 16 contact ticks. The lone Ace
    overshoots off the road at idx 318 at 271 u/s, is airborne-reset to idx 89
    at 113 u/s, and scrapes on the way back.
  - `ai_dekonstruct_symptoms_ground_truth`: field dead stops VENOM 5 (bound
    1), FLASH 5 (3), PHANTOM 4 (1); lone RAPIER 1 (0).
  - `ai_dekonstruct_black_ground_truth rapier_seed_2`: 2 of 7 destroyed (bound
    1).
- **`foreign_race_ground_truth an_ai_craft_laps_lions_den_on_pulse_physics`:
  1 lap, needs 2.** On Pulse physics the craft falls off Lion's Den's start
  jump even at 212 u/s. It is relaunched at idx 1192 at about 86 u/s and falls
  again every 320 ticks. The importer authors no `flags`, which is right for
  a track with no such data.
- Whether the original clears `05_Track`'s first jump after its own reset is
  not measured. At `13_Track` the original's leader also falls at 92-102 u/s,
  so falling short is faithful there. On `05` our need for 120 u/s is this
  engine's number.

## Next Steps

1. Measure the original's field on `05_Track` with the dev-unlock byte, the
   same tools `pulse-13-capture` used (`scripts/psp-ai-track-log.py`): does a
   craft reset before the first jump fall again, and what is its speed at row
   150?
2. If the original clears where ours does not: chase the gap in
   approach speed or flight (the air acceleration here is -95 against the
   original's -89 to -93 at `13_Track`, confidence 60, `docs/gameplay/ai.md`).
   If it falls too: re-baseline the six `05_Track` rows in a commit that names
   the reset port as the cause.
3. Lion's Den: decide whether a foreign track's AI should get a lane that
   clears its start jump, or whether the test should assert the relaunch
   instead of two laps.
