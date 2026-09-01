# Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`

2026-08-24, runtime-verified 2026-09-01. The law, the emitter layout, the
cross-title scan and every confidence are on
[positional-audio.md](../docs/ghidra/functions/psp-pulse-usa/positional-audio.md),
which is the durable record; this row is what is not there. **The cheapest
upgrade this project had open is done**: a live Pulse USA capture
(`scripts/psp-watch-soundemitter.py`, swapping one execution breakpoint
between `SoundEmitter_ComputeVolumeAndAngle`'s entry and return address, since
PPSSPP v1.20.4 only ever fires the most-recently-armed one) reproduced the
recovered volume/angle law exactly on **1,610/1,610 live hits**. Confidences
moved **85-88 -> 90-94** across the emitter chain; see the doc's own
[runtime-verification section](../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#runtime-verification-2026-09-01)
for the per-claim breakdown. **Two things fell out for free**: the doppler
scale `inst[0x0c]`, previously read as a field and never as a value, read
exactly `0.0005` on every hit (against the engine note only - not yet checked
against other cues); and the cone-enabled flag read `False` on all 1,610
samples, real negative evidence rather than "not looked at". **What the
capture did not reach**: the `d > radius` zero-volume gate branch itself never
fired live - the emitter's queued request stopped appearing before `d/radius`
crossed `1.0`, consistent with the existing "out of range is a latched state"
reading but not a direct read of that branch - and the doppler `1536` unit is
still unrecovered. **Three things landed as side effects in the 2026-08-24
pass and are still easy to lose**: [camera.md](../docs/ghidra/functions/psp-pulse-usa/camera.md)'s
transposition finding moved **80 -> 88** (a second subsystem assumes both
halves); [pads.md](../docs/ghidra/functions/psp-pulse-usa/pads.md)'s
`racer+0x368` = "the local player" moved **45 -> 60** on a fourth and fifth
use, still unnamed, with `oag_game::audio::sfx::Placement::CraftUnlessPlayer`
and `OPPONENT_ENGINE_SCALE` the only two lines that move if it is wrong; and
exhaust.md's "world position at `+0x50`" was **wrong** and is corrected to a
scene-node pointer. **The PS2 has no such table in either byte order**, and a
follow-up scan over 8 scales x 3 steps x 2 orders found no variant, so how
`SCES_547.48` pans is unknown - the one clearly-shaped open question here.
**Still unplaced**: the `.vex` classes `sound` `0x3e1`, `soundcone` `0x3e9` and
`speaker` `0x3cc`; the emitter cone's trigger; the doppler term's unit; ten of
fourteen emitter construction sites.

## Open

- How `SCES_547.48` (PS2) pans is unknown - no byte-order/scale/step variant matched across 8 scales x 3 steps x 2 orders
- `.vex` classes `sound`, `soundcone`, `speaker`, the emitter cone's trigger, the doppler term's unit, and ten of fourteen emitter construction sites are all still unplaced
- `racer+0x368` = "the local player" is unconfirmed on its fourth and fifth (still unnamed) use sites
- The `d > radius` zero-volume gate branch was never caught firing live, and the doppler `1536` unit is still unrecovered - both need more than a register read (see the doc's runtime-verification section for what each would take)

## Next Steps

- Catch the gate branch itself: `scripts/psp-drive.py place` a second craft's emitter straight past its radius (rather than driving there, which lets the queued request drop first) and rerun `psp-watch-soundemitter.py` - the one regime 2026-09-01's capture missed
