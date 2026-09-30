# Pulse handling after the airbrake scale fix: what is left on the reference lap

Opened 2026-09-30 by the `pulse-handling` lane. The single-seeded reference lap
(`data/traces/talons-junction-clean-lap.csv`, `--script-lead 2`) now tracks the
original to **4.6 units at tick 240** and under 5 units for the whole wall-free
window (ticks 0-255), from 43.9 before. Two fixes did it, both documented where
they belong:

- the replay harness armed no speed pads
  ([oag-trace.md](../../docs/tools/oag-trace.md), `pad_crossing_ground_truth.rs`);
- `oag_physics::airbrake::evaluate`'s forward `drag` term read `steerX` 100x
  weak ([engine.md](../../docs/ghidra/functions/psp-pulse-usa/engine.md#the-raw-steerx-is-on-the-100-scale-too)).

`lap_window_ground_truth.rs` fails if either is dropped.

## Open

- **The AI is slower to adapt than the physics, and one test is red for it.**
  The drivers fly the player's physics and were tuned against the weak term.
  `ai_clean_lap_gate`'s BASELINE was re-recorded for the change (commit
  `afaba9c7`, which names the four status changes: `05_Track` FLASH and VENOM
  and `14_Track` RAPIER `CleanLap` to `Died`, `13_Track` RAPIER losing its
  clean lap). **`difficulty_ground_truth::every_difficulty_is_quicker_than_the_one_below_it`
  fails and was deliberately not touched**: Ace's leader mean is 6683 against
  Elite's 6829 over five seeds, 2.14 % under against a 2.0 % tolerance (before
  the fix: 6728 against 6773, 0.66 % under). The pair was already inside the
  noise the test's own doc comment records; the airbrake push moved it just
  past the line. Both belong to whoever holds the AI lane.
- **Shield, VENOM, one lone Ace, `ai_clean_lap_board`** (before -> after,
  end-of-run shield; per-lap wall charge in brackets). Unchanged within noise
  on ten circuits. `05_Track` is the outlier: end 66.1 -> 86.6 (three
  respawns in the run; whether a respawn refills the pool was not checked), but wall charge 13.7 -> 96.0 and laps `1.9 1.4 2.0` ->
  `1.8 1.9 90.0` - a lap-3 wall grind. `10_Track`: end 94.8 -> 79.8, laps
  `0.0 0.0 0.0` -> `0.0 7.6 7.6` with no wall charge, so not walls. Both
  boards are in the lane's scratch directory (`board-before.txt`,
  `board-after.txt`), not tracked.
- **`05_Track` respawns went from 1 to 3** in
  `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
  (lost at ticks 186, 637, 2412; before: 407). Still a clean lap, all twelve
  circuits still clean. The craft now carries more speed through an airbraked
  corner, as the original does, so the driver that was tuned against the weak
  term may now arrive too fast. That is AI driving, not physics - the physics
  moved toward the capture.
- **A small residual without airbrakes, now the largest one left in the clean
  window.** The one-tick walk (`oag-trace run --reseed 2`, odd rows) shows the
  original decelerating 2-7 units/s^2 more than we do above about 70 units/s
  with no airbrake held, and a constant `-0.75` units/s^2 along the hull's up
  axis on every tick, speed-independent. Neither is localised yet. The up
  residual appearing even at standstill (ticks 1-3) suggests a seeding or
  gravity-frame artefact rather than a force term; check that first.
- **The crest lag is unchanged by either fix**: at `--reseed 60` ours still
  lifts off at 1606 against the original's 1595. No pad falls inside that
  window (crossings are at 161, 680, 989, 1284, 1708, 2398, 2409, 2793), so
  the missing-pad bug was never its cause. The crest thread stands as written.
- **`oag-trace drive` (and so `just scripted-sim`) still takes no pads.**
  `DriveOptions` has no track pad list; only `run` loads one.
- **The swept pad test now exists twice**: `oag_game::race::pads` and
  `oag_trace::replay::pads`. `oag-trace` cannot depend on the composition root,
  so the shared part (sweep + first containing pad) wants a home both can
  reach, `oag-vex` or `oag-race`.

## Next Steps

1. Run `oag-trace run ... --reseed 2` on the clean lap and read the odd rows'
   up-axis residual at ticks 1-5, where the craft is still; if it is `-0.75`
   there too, compare `initial_state`'s seeding of the hover (`grounded_prev`,
   the landing timers) against a two-tick run from tick 0.
2. Fit the no-airbrake forward residual against `speed^2` and `speed` over
   ticks 36-50 and 83-99 of the clean lap (airbrakes off, wall-free) to see
   whether it is the quadratic drag or the rolling resistance.
3. Give the AI lane the `05_Track` regression (respawn ticks above, the lap-3
   wall grind) and the Ace/Elite ordering; the driver should brake less or
   later into airbraked corners now that the airbrake no longer costs speed.
