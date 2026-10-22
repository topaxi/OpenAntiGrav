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

**Ported 2026-10-22** (`oag_raceplay`'s `hull_shake`, law in `oag_title::hull_shake::HullShake::HD`, fade input taken as
`1.5 * speed` per `engine-trail.md`'s direct read). **Off for still pairs on both sides:** ours `--no-hull-shake`
(`compare_env.ours_flags`), and the original patched by `scripts/rpcs3-hd-no-hull-shake.py`, which RPCS3 applies on both
decoders. All three HD capture scripts run the patch first. See `hull-shake.md`, "Ported".

## Open

- **Speed or thrust?** The fade input is taken as speed (`engine-trail.md`, eight craft, four sessions), but `physics.md`
  calls the same field thrust. Both say the shake is gone at racing pace. They disagree on a craft coasting fast after a
  throttle release.
- **Frame time.** The 0.05-0.09 offsets were read under the interpreter, whose frame time varied, so they are not a 60 Hz
  size. Nothing has been calibrated against them.
- **Lane masks.** The masks at `0x00c47730` are BSS, so the x/y/z order of the three angle writes is assumed.
- **Not read:** the constructor's seeding of `P0..P3`, and whether opponents run the block (ours shakes every craft).
- **Talon's (`hd/talons`) is recaptured patched:** craft offset `(0, -9)` -> `(0, 0)` px, craft difference
  32.95 -> 10.84.
- **The other 12 HD originals:** still captured with the shake on. Their pairs stay comparable to what they were (ours
  draws none), but the craft offset on them includes the original's shake until they are recaptured.
- **Cross-title:** Omega and 2048 inherit HD's law unread, and a constant search found neither. Pulse and Pure are
  not read.

## Next Steps

1. **Recapture the other 12 HD originals cold with the patch**, about 5 minutes each:
   `scripts/rpcs3-hd-cold-pair.sh <lane> <circuit-index> <out> ""` per circuit (indices are in
   `compare-rerender-all.py`'s `CIRCUITS`), or `just compare-rerender-all --platform rpcs3`. Then run
   `just compare-rerender-ours --only compare-page/hd` and measure each pair's craft offset with a craft mask.
2. **Sample live,** about 30 minutes. Restore a Time Trial state with
   `scripts/emu-restore-state.sh <lane> <state> --interpreter` and stop at `0x000ebc74` with `r3 = ship+0x6b00`.
   - Read `f1`, `f2` and `P0..P3` (`+0x40..+0x70`), then `+0x80`, `+0x10`, `+0x00` and `+0x20`.
   - Take one read just after a throttle release at speed: thrust drops at once, speed does not.
   - Stop again at `0x000ebc88` and check that the node equals `HullShake_Apply` of those values.
   - Restore fresh for each attach. Build the probe from `scripts/rpcs3-hd-hull-light.py`'s helpers.
   - Compare amplitude and rate per unit of `f1` against ours (`hull_shake`'s tests).
