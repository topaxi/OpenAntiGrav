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

- **The AI side is closed (2026-09-30, `ai-retune`), by something other than the
  airbrake.** The drivers had no airbrake model to update and were not arriving
  at corners faster; the three lost rows were `05_Track`'s crest lip, where a
  driver that braked at the foot of the hill arrived 3 units/s under the ~70 it
  needs. `Line::curvature` now reads only a convex pitch change as a bend
  (`docs/gameplay/ai.md`, "A valley is not a corner"). Result: `05_Track`
  VENOM and FLASH and `14_Track` RAPIER `CleanLap` again, `Died` rows 14 to 10
  of 48, `05_Track` respawns 3 to 0 in the lone-craft test,
  `difficulty_ground_truth` green at the unchanged 2.0 % (Ace 1.85 % under Elite,
  thin). End-of-run shield, VENOM lone Ace: unchanged within 3 on ten circuits;
  `09_Track` 93.0 to 89.9, `07_Track` 52.9 to 50.0, `10_Track` 79.8 to 94.4,
  `05_Track` 86.6 to 81.2. Per-lap shield on `05_Track`: `1.8 1.9 90.0` to
  `2.0 1.8 2.4`.
- **Open from that work: `07_Track` FLASH reads `Died` to `Eliminated`**, which
  is a window artefact (a second wreck at tick 17834, 166 ticks before the cut,
  respawn 167); and **a craft that clips `05_Track`'s crest is never
  rescued**: it slides back on a ~60-tick cycle for up to 5,000 ticks, never
  stopped two seconds and never far from the line, so neither rescue fires.
  That is race rules. Ace sits below Elite on four of five seeds in
  `difficulty_ground_truth`; the gap is the pace ceiling, not a driving fault.
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
3. Give a craft that clips `05_Track`'s crest lip a rescue (see the second open
   bullet). `bend_angle` flattens onto world Y; a past-vertical bank would need
   the line's own frame.
