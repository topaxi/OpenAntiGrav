# Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`

2026-08-24. The law, the emitter layout, the cross-title scan and every confidence are on [positional-audio.md](../docs/ghidra/functions/psp-pulse-usa/positional-audio.md), which is the durable record; this row is what is not there. **Nothing is runtime-verified** - one PPSSPP breakpoint on `SoundEmitter_ComputeVolumeAndAngle` with two craft at a known separation takes 82-88 to 90 and is the cheapest upgrade this project has open. **Three things landed as side effects and are easy to lose**: [camera.md](../docs/ghidra/functions/psp-pulse-usa/camera.md)'s transposition finding moved **80 -> 88** (a second subsystem assumes both halves); [pads.md](../docs/ghidra/functions/psp-pulse-usa/pads.md)'s `racer+0x368` = "the local player" moved **45 -> 60** on a fourth and fifth use, still unnamed, with `oag_game::audio::sfx::Placement::CraftUnlessPlayer` and `OPPONENT_ENGINE_SCALE` the only two lines that move if it is wrong; and exhaust.md's "world position at `+0x50`" was **wrong** and is corrected to a scene-node pointer. **The PS2 has no such table in either byte order**, and a follow-up scan over 8 scales x 3 steps x 2 orders found no variant, so how `SCES_547.48` pans is unknown - the one clearly-shaped open question here. **Still unplaced**: the `.vex` classes `sound` `0x3e1`, `soundcone` `0x3e9` and `speaker` `0x3cc`; the emitter cone (recovered, no trigger found); the doppler term (read, unrecovered unit); ten of fourteen emitter construction sites.

## Open

- Nothing about positional audio is runtime-verified yet
- How `SCES_547.48` (PS2) pans is unknown - no byte-order/scale/step variant matched across 8 scales x 3 steps x 2 orders
- `.vex` classes `sound`, `soundcone`, `speaker`, the emitter cone's trigger, the doppler term's unit, and ten of fourteen emitter construction sites are all still unplaced
- `racer+0x368` = "the local player" is unconfirmed on its fourth and fifth (still unnamed) use sites

## Next Steps

- Set a PPSSPP breakpoint on `SoundEmitter_ComputeVolumeAndAngle` with two craft at a known separation (confidence 82-88 -> 90) - the cheapest upgrade currently open
