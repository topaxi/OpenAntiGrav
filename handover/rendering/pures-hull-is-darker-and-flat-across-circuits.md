# Pure's hull is 1.8x to 2.8x darker than the original's, and ours does not move with the circuit

Measured 2026-10-22 (`pure-fidelity`, item 3, measure only; `crates/mesh` and `crates/render` untouched).
Pairs: Time Trial, Vector, Feisar, posed from RAM, same pose (hull offset 0 to 1 px).

## Open

Mean RGB over the teal body (a 10x14 px patch at the hull's centre, one patch on every pair), original
then ours, and the ratio ours/original on green:

| circuit | original | ours | ratio | whole-frame mean grey (original) |
| --- | --- | --- | ---: | ---: |
| Vineta K | 159 176 155 | 56 68 53 | 0.39 | 143.6 |
| Chenghou Project | 156 168 138 | 61 74 58 | 0.44 | 97.4 |
| Blue Ridge | 132 149 124 | 57 69 55 | 0.46 | 102.9 |
| Modesto Heights | 110 94 74 | 61 73 58 | 0.78 | 72.3 |

The yellow wing panels agree (ratio 0.45 to 0.6, Modesto about 1). Read:

- **Ours is flat**: 56 to 61 red on all four circuits, from a fixed stand-in rig. **The original's moves with
  the circuit**, in the order of the scene's own brightness (Vineta brightest, Modesto darkest). A texture or
  a constant tone stage would give the same ratio on all four; it does not, so the difference is a
  per-circuit lighting input that ours lacks, not the texture and not the tone curve.
- Pure's `track.vex` authors **no** `AmbientLight`/`DirectionalLight` node (`hull lights: the circuit
  authors none; hulls keep the stand-in rig` in the loader report on Chenghou), so Pulse's rig reader
  (`mesh_render::HullLights::from_track`, switched on by `Looks::hull_lights`) finds nothing. Turning the
  rule on for Pure's PSP changed no pixel on any of the four pairs (tried and reverted).

## Next Steps

1. In `/pure/BOOT-psp-pure-usa.BIN` find where the hull's lighting is set per craft per frame (Pulse's
   `Craft_ApplyHullLightScale` analogue; start from the craft draw that `FUN_0892195c`'s neighbours call) and what
   it reads: a per-circuit colour in the `.vex` (fog, sky, a light node under another name) or the sprite
   lighting list.
2. Read the same value for the four circuits, check it orders as the four ratios above, then feed it to the
   hull's light the way Pulse's `HullLights` is fed (a `Looks` rule with `Origin::Measured`).
3. Re-render the four pairs (`scripts/compare-rerender-ours.py --no-page <copy>`) and compare the teal and
   yellow patches against the table.
