# Pulse's start pose: the grid walk is what is left

2026-10-01, `pulse-start-pose`. The weapons lane found ours 2.4 units and 1.7
degrees off the original at the same place on `16_Track`. It was not the start
pose: a force term ran on the grid in ours and not in the original. That, and the
per-slot heading, landed; the write-up and every measurement are in
[grid-state.md](../../docs/physics/grid-state.md). What is left, ranked.

## Open

- ~~**The launch boost is not implemented.**~~ **Landed 2026-10-01 (`pulse-launch-boost`)**:
  the grade is selected by when the thrust first lands, the values are the disc's
  `<StartBoost>`, held-through is within 0.4 units of the original at 120 frames; see
  [launch-boost.md](../../docs/physics/launch-boost.md). Its own leftovers: the AI's grade 3
  (read, not watched, not ported), the perfect-start effect trigger `FUN_08904fd4`, and the
  PS2/other-PSP discs applied by extension.
- **The grid walk.** Our slots are on whole track samples (up to 1.5 units short of
  the walked distance); an exact walk was tried and made
  `metropia_reversed_is_the_originals_grid` worse (1.13 to 1.99), so it was
  reverted. The residue fits the original re-locating on its own spline after every
  step. Needs `FUN_0882663c` and the locate function read to the end, then a chain
  `p(k+1) = project(p(k) + tangent * 19.8)` tested against three circuits' eight
  slots (`orig-grid-16-single-b.json`, `orig-grid-01-single-a.json`, and
  Metropia reversed in `grid_stagger_ground_truth`).
- ~~**Three more state-0 terms**~~ **Tried on top of the boost, none pays, none ported**
  (2026-10-01): forced airbrakes with the brake held at zero leave the pose at GO and
  every launch speed unchanged to three decimals; roll damping `-5` and the `rebound`
  base `1.0` move the pose by 0.002 to 0.004 units either way. The earlier 75.8 against
  99.8 loss was the brake ramping on the grid and not being zeroed at release. See
  [launch-boost.md](../../docs/physics/launch-boost.md#the-three-other-state-0-terms-tried-and-left-out).
- **Zone** runs `Ship_HoverFourCorner`; `on_grid` is off there because its epilogue
  was not read for the guard.
- **`01_Track` slot 1's heading** is `-0.0069` in the original against `-0.0014` for
  slot 2 and `-0.0031` from our sample frame: 0.22 degrees, cause unknown.
- **The AI board moved**: `04_Track` RAPIER `CleanLap` to `Died` (one death at tick
  17714 of 18000 after four clean laps). Regenerated, not tuned; worth a look when
  the AI lane next touches that row.

## Next Steps

1. The grid walk: read `FUN_0882663c` and the locate function to the end, then test the
   chain `p(k+1) = project(p(k) + tangent * 19.8)` against the three circuits' eight slots
   (an afternoon if the locate function is short, a day if it is the VFPU one).
2. Zone's four-corner hover epilogue guard, so `on_grid` applies there (and the launch
   window runs from the right frame).
