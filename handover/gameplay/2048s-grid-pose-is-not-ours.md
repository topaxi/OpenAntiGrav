# 2048's grid pose is not ours' placement: three circuits, three different terms

2026-10-26, lane `2048-queens-offset`. The 2048 compare pairs sit 2 to 8 px off in the **world** and 0 to 2 px in the hull. The hull
shake (HD's, patched out in the original's memory) is not the cause; the craft's resting pose is. Read live out of Vita3K's guest
memory with `vita-2048-pair.py --read-pose`, original minus ours, identical on every cold boot (three of Queens Mall):

| Pair | Position (units) | Heading | Pitch at rest | Ours put on the original's pose |
| --- | --- | --- | --- | --- |
| Queens Mall | `(0.000, +0.047, -0.017)` | `-0.35` degrees | **`+1.15`** (original `+0.29`, ours `-0.86`) | floor and scenery within 2 px |
| Empire Climb | `(+0.465, +0.026, +1.453)` | **`-2.05`** degrees | `-0.13` | scenery 0, floor 1 px |
| Vineta K (DLC1) | `(+0.532, +1.846, -0.002)` (the `y` is the HD hull's node, see below) | `-0.62` | `0.00` | within 1 px with ours' height |

Evidence and method: [compare-page.md](../../docs/tools/compare-page.md), "The hull shake and the grid pose"; the hull shake itself
on 2048 and Omega: [vita-2048-eu-v104/hull-shake.md](../../docs/ghidra/functions/vita-2048-eu-v104/hull-shake.md),
[ps4-omega-eu/hull-shake.md](../../docs/ghidra/functions/ps4-omega-eu/hull-shake.md); grid state: [grid-state.md](../../docs/physics/grid-state.md).

## Open

- **What gives the original's pose is not recovered.** Ours places the 8th slot at the authored `Start Position` (up forced to `+y`, forward
  projected flat, so yaw only) lifted to hover height, or turned to the spline's tangent where the node points the wrong way
  (`oag_raceplay::spawn::face_the_way_the_track_runs`), and the hover settle gives pitch. The original's rows are tilted (Mall `+0.29`
  degrees, Empire `-1.06`), so it keeps more than a yaw. Candidates, none tested: the authored node unmodified (Mall's `Start Position`
  says `(-0.0061, 0, -1)` after the fix-up and the original says `(0, +0.005, -1)`: yaw differs, so not as is), the spline sample's own frame
  (Mall's first sample has `fwd = (0, +0.0147, -0.9999)`, `x = 0` like the original; Empire's nearest sample read, 5.9 units from the
  slot, is `-6.8` degrees against the original's `-2.49`), or the grid's own `start position N` nodes: the 2048 binary builds
  `"start position %d"` (`StartPosition_Importer.cpp`, `0x814d8954`) and ours reads one.
- **Why ours rests `-0.86` degrees nose-down on both 2048-hull circuits** (`-0.015`, Mall and Empire) and level on the HD-derived hull: a hover-rig
  trim, or the ground under the craft, not read. The original's Mall pitch `+0.29` is the track's `+0.84` climb minus about `0.55`.
- **The DLC pair's `+1.846` in `y`**: the original's drawn node of the HD-derived Feisar sits that far above ours' craft position while the floor
  agrees to 1 px at ours' height, so it is the hull's pivot, not the placement. Not read where it comes from.
- **The camera's own shake stays on** in every original capture (the third `HullShake_Apply` call, `0x811be926`, is the chase camera's node with `invert = 1`
  and cannot be overwritten). It moves the hull by +-4 px between boots. Ours draws no camera shake on the grid.
- **Omega**: same law at `0x012f7200` in the base executable; its pair has no pose channel and the patch v1.07 executable is not in Ghidra.

## Next Steps

1. Read 2048's grid placement, about 2 hours: from `"start position %d"` (`0x814d8954`) and `StartSequenceCamera.cpp`'s `"start position 1"`
   (`0x814a8b60`) to the code that puts the eight craft down; the answer is a pose per slot. Anchor: the live poses above are the test.
2. Wire the rule in `oag_gameplay::spawn`/`oag_raceplay::spawn` for 2048 only, behind `Title` data (no `title.name` branch), then re-render the three pairs;
   `--read-pose` is the check, the ground truth is `pose_delta` going to zero.
3. Seed ours' hull-shake spline with the constructor's `rand() % 500` steps (`hull-shake.md` on 2048), a view-side change.
