# HD's drawn hull shakes off its body every frame; ours draws the body, so every HD pair's craft sits 3-13 px off

2026-10-22 (`hd-hull-bay`). The largest craft difference left on the 13 HD compare pairs is placement,
not material: our craft sits 3 to 13 px from the original's (Talon's Junction `(0, -9)`, Sol 2
`(+13, -6)`, Modesto Heights `(-12, +1)`), while the floor beside it aligns within 1 px. Ours draws
exactly the RAM body; the original does not. Evidence and numbers:
[hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "The drawn hull is not the body";
the code site: [ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md), "Not the last step".

What is measured (RPCS3, interpreter):

- The hull's `EdgeGeom` matrix (`0x005d8488`, `r11`, scale 0.75) equals `ship+0x40` (copy at `+0x80`)
  and is 1.1-1.8 degrees and up to 0.09 units off the body rows at `body+0x1d0`, about an axis that
  differs per read. Lag and the camera are ruled out (the original's craft is still between frames
  0.12 s apart; the near floor aligns).
- In `Ship_UpdateEntity` (`0x000eadb8`) the node matrix equals the body at `0x000ebc78` and carries the
  offset at `0x000ebc88`, different on consecutive frames. Between them:
  `FUN_000d0c70(ship+0x6b00, f2 = *(r9+0x4c4))` and `FUN_000d39d0(ship+0x6b00, node, 0)`.
- Posing ours with one read halves Talon's craft difference (32.37 -> 16.04 at zero shift). That was a
  temporary override to prove the cause, not a fix; do not land a per-pair pose as accuracy.

**Read from the disassembly 2026-10-22** ([hull-shake.md](../../docs/ghidra/functions/ps3-hdfury-eu/hull-shake.md),
`HullShake_Update` 75, `HullShake_Apply` 72). It is a uniform cubic B-spline over random points at `obj+0x40..+0x70`, with
the parameter at `obj+0x80` advanced by `dt`, so one new point a second. The points are x `4 * (r - 0.5)`, y and z
`1.8 * (r - 0.5)`, with `r = rand() * 2^-30` in `[0, 1)`. `rand` (`0x0043c280`) returns 30 bits, so the points are centred. From the previous frame's value `q` the hull is turned by roll
`0.025 * q.x`, yaw `-0.01 * q.x` and pitch `0.025 * (q.y - q.z)`, composed `Ry * Rz * Rx * node` in the hull's own frame.
It is moved by `4 * (p - q)` along the node's rows. Both terms are scaled by `clamp((80 - thrust_s) * 0.02, 0, 1)`, where
`thrust_s` is the body's `+0x4c4` (engine thrust) smoothed 0.99 per frame. The shake is full at rest or coasting and
nearly gone under full throttle.

## Open

- What body `+0x4c4` is. `physics.md` measured it as thrust; `xfade.md` scales it by 3.6 like a speed. That decides
  whether a craft coasting fast shakes, so it blocks the port.
- The frame time. The 0.05-0.09 offsets were read under the interpreter, whose frame time varied (two consecutive
  offsets differ by 0.61 in every component), so they are not a 60 Hz size. Do not calibrate the port against them.
- The `vsel` lane masks at `0x00c47730` are BSS, so the x/y/z order of the three angle writes is assumed.
- How the constructor seeds `P0..P3` and `obj+0x80`. Whether opponents run the block: it is inside the per-ship update,
  but no player test above `0x000ebb28` was read.
- What our sim carries for HD's body `+0x4c4` thrust (the fade input), at our fixed 60 Hz. HD's `0.99` filter and its
  one-frame difference are per frame, so 60 Hz is their own rate.
- Cross-title: Omega and 2048 not found by a constant search (needs their ship update's call site). Pulse and Pure
  not read.

## Next Steps

1. Sample live, about 30 minutes. Restore a Time Trial state with `scripts/emu-restore-state.sh <lane> <state> --interpreter`
   and stop at `0x000ebc74` with `r3 = ship+0x6b00`. Read `f1` (the frame time) and `f2` (body `+0x4c4`), and also
   `P0..P3` (`+0x40..+0x70`), `+0x80`, `+0x10`, `+0x00` and `+0x20`. Read once while coasting just after a throttle
   release at speed: thrust drops at once, speed does not. Judge rate and offset per unit of `f1`. Stop again at `0x000ebc88` and check that the node
   equals `HullShake_Apply` of those values. Restore fresh for each attach: the GDB stub wedges on a second attach.
   Build the probe from `scripts/rpcs3-hd-hull-light.py`'s helpers (`stop_at`, `place.find_player`, `place.read_pose`)
   and land it under `scripts/`.
2. Port it to the drawn model matrix in `oag_raceplay`: view only, unhashed, its own seeded `Rng`, stepped on the fixed
   60 Hz tick (not per rendered frame), fed by whatever step 1 shows `+0x4c4` to be. Then run `just compare-rerender-ours --only compare-page/hd`. The pairs cannot match a random shake frame
   for frame, so judge the port by amplitude and rate against step 1's samples, and use the craft-mask offset only as
   a sanity check.
