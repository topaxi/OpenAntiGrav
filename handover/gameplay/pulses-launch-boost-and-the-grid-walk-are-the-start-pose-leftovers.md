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
  [launch-boost.md](../../docs/physics/launch-boost.md). Its own leftovers: ~~the perfect-start
  effect trigger `FUN_08904fd4`~~ **wired 2026-10-04** (the pad's flare and `TURBO`,
  [perfect-start.md](../../docs/ghidra/functions/psp-pulse-usa/perfect-start.md)); the AI's grade 3,
  **confirmed from both ends 2026-10-04 (85) and not ported** - the in-rule version (the field
  timing its first thrust into the perfect window) was tried and reverted because it put eight
  `ai_dekonstruct_black` seeds over their destroyed-craft bounds (launch-boost.md, Open); and the
  PS2/other-PSP discs applied by extension.
- ~~**The grid walk.**~~ **Landed 2026-10-02 (`pulse-grid-walk`)**: the walk is a projection onto
  the lifted B-spline, stepped `19.8` along the located record's own tangent, with the record
  scaled `0.999756` by a half-float immediate in the original's evaluator; the heading is the
  edge chords'. `01_Track` 1.73 to 0.001, `16_Track` 0.62 to 0.043, Metropia 1.13 to 0.070; see
  [grid-state.md](../../docs/physics/grid-state.md) and
  [grid.md](../../docs/ghidra/functions/psp-pulse-usa/grid.md#the-grid-walk-read-to-the-end-2026-10-02).
  Its own leftovers: the junction hop (the walk clamps at a path's end; `03_Track`'s front slot is
  one control point from it, under 0.1 unit), `25_Track` reversed (its node is 25.9 units under
  the nearest sample, so the node-anchored grid stays there, what the original does was not
  captured), the PS2/Pure/HD layouts (unread, unported), and the EU PSP build (not compared).
- ~~**Three more state-0 terms**~~ **Tried on top of the boost, none pays, none ported**
  (2026-10-01): forced airbrakes with the brake held at zero leave the pose at GO and
  every launch speed unchanged to three decimals; roll damping `-5` and the `rebound`
  base `1.0` move the pose by 0.002 to 0.004 units either way. The earlier 75.8 against
  99.8 loss was the brake ramping on the grid and not being zeroed at release. See
  [launch-boost.md](../../docs/physics/launch-boost.md#the-three-other-state-0-terms-tried-and-left-out).
- ~~**Zone**~~ **Landed 2026-10-02 (`pulse-zone-start`)**: the engine's Zone branch is gated on flags bit 1, the
  grid state, so a Zone craft stands through the countdown; `on_grid` covers Zone and the auto-speed carries the launch
  multiplier (1.159 coasting over held at 28 frames, as read live); see
  [zone-start.md](../../docs/ghidra/functions/psp-pulse-usa/zone-start.md). Its own leftovers: the four-corner
  epilogue's `50.0` bank coupling (`0x0884b76c`) **landed 2026-10-02** (`pulse-zone-rest`, zone-rest.md), and the capture patched `g_game_mode` on a Single Race
  grid, so a native Zone race was not watched (Zone is greyed on a fresh profile).
- ~~**`01_Track` slot 1's heading**~~ is the edge chord, resolved with the grid walk above.
- **The AI board moved**: `04_Track` RAPIER `CleanLap` to `Died` (one death at tick
  17714 of 18000 after four clean laps). Regenerated, not tuned; worth a look when
  the AI lane next touches that row.

## Next Steps

1. ~~The grid walk~~ landed.
2. ~~Zone~~ landed.
3. The junction hop in `oag_gameplay::grid_walk::locate`, only if a circuit-direction's
   grid ever comes within a control point of a path end again (today only `03_Track`, by under
   0.1 unit).
