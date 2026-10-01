# Pulse's start pose: the launch boost and the grid walk are what is left

2026-10-01, `pulse-start-pose`. The weapons lane found ours 2.4 units and 1.7
degrees off the original at the same place on `16_Track`. It was not the start
pose: a force term ran on the grid in ours and not in the original. That, and the
per-slot heading, landed; the write-up and every measurement are in
[grid-state.md](../../docs/physics/grid-state.md). What is left, ranked.

## Open

- **The launch boost is not implemented, and it is the largest remaining term.**
  A craft with thrust held through the countdown gets `craft+0x294` (the
  multiplier `Ship_UpdateStartBoost` writes) at `1.4` for the frame before GO and
  `1.2` for the next 58 frames (`craft+0x2d8`/`+0x2dc` run `100` to `0` over the
  same window), then `1.0`; pressing late sees `1.0`. Ours has no multiplier, so
  from a held-thrust GO it is `3.1` units/s slower at GO + 20 and `18` units behind
  at GO + 120 (`106.1` against `124.5` on `16_Track`, `z 381.7` against `401.0` on
  `01_Track`). Every `data/traces/` launch was taken by pressing late, which is why
  the engine's launch acceleration matched them. The values live in the disc's
  `<StartBoost>` XML (`oag_tables::handling` already parses the element); which of
  `stallMul`, `normalMul` and `boostMul` the reaction time selects (`player+0x36c`,
  the "grade") is not read. A `race-start-countdown-and-launch-boost.md` row
  ("no false-start penalty exists") is the other half of the same mechanic.
- **The grid walk.** Our slots are on whole track samples (up to 1.5 units short of
  the walked distance); an exact walk was tried and made
  `metropia_reversed_is_the_originals_grid` worse (1.13 to 1.99), so it was
  reverted. The residue fits the original re-locating on its own spline after every
  step. Needs `FUN_0882663c` and the locate function read to the end, then a chain
  `p(k+1) = project(p(k) + tangent * 19.8)` tested against three circuits' eight
  slots (`orig-grid-16-single-b.json`, `orig-grid-01-single-a.json`, and
  Metropia reversed in `grid_stagger_ground_truth`).
- **Three more state-0 terms, small at rest, not ported**: roll damping `-5` not
  `-2` (`passive.rs`, outside the hover files), the `rebound` base `1.0` not the
  hull's, and the control record's airbrakes forced to full. The last only matters
  with the transition zeroing in `FUN_088486e4`; ported alone it makes the launch
  worse (75.8 against 99.8 at GO + 120 in a trial).
- **Zone** runs `Ship_HoverFourCorner`; `on_grid` is off there because its epilogue
  was not read for the guard.
- **`01_Track` slot 1's heading** is `-0.0069` in the original against `-0.0014` for
  slot 2 and `-0.0031` from our sample frame: 0.22 degrees, cause unknown.
- **The AI board moved**: `04_Track` RAPIER `CleanLap` to `Died` (one death at tick
  17714 of 18000 after four clean laps). Regenerated, not tuned; worth a look when
  the AI lane next touches that row.

## Next Steps

1. Read `Ship_UpdateStartBoost`'s grade selection (`player+0x36c`) and the
   `<StartBoost>` values off the disc; the capture recipe is
   `scripts/psp-start-pose.py --hold --full` (an afternoon if the grade is one
   comparison, a day if it is a timing window).
2. Port the multiplier into `engine::engine_force` behind the same
   `Setup::grid_frame_from_sample`-style Pulse PSP gate, and test the held-thrust
   launch against `orig-full-held2.json`'s `craft+0x294` series.
