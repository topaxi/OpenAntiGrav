# HD's engine light washes the hull white on boost and blooms off it; the original's stays copper - the hull binding is the suspect, not the bloom

2026-09-20. Reported from play by the user the same evening the light landed
(`421b14c9`): "the light work looks amazing, but it exaggerates the bloom".
The user's first guess - that the bloom had been tuned against the original
before the light existed and the two now stack - is not what the code says,
and the gap has a cleaner shape than "too much". This file is the gap; the
mechanism's own history (producer, formula, capture) is on
[hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md)
and is not repeated here.

## What is established, and is not the problem

- **The bloom is measured, not tuned.** `crates/render/src/post/hd_bloom.rs`:
  every pass is the EBOOT's own fragment microcode (`scripts/ps3-microcode.py`),
  the parameters are the circuit's `.envsettings` `HDR and Bloom` values, and
  `hd_bloom.wgsl`'s `surface()` applies the original's `A8R8G8B8` clamp at the
  three points the chain samples the scene
  ([renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
  "The bloom chain, read pass by pass"). Nothing in it changed on 2026-09-20.
  **Do not tune the bloom to compensate** - it would be tuning a measured
  chain to hide an unmeasured input.
- **The light's numbers are the producer's own.** `EngineFlare_SubmitSpuLight`
  (`0x0029ff28`, confidence 85, renderer.md "The captured buffer's producer is
  found"): per craft per frame, position `flare_node.pos - flare_node.z *
  Distance` plus `U(±0.1)` jitter, `D = Radius ± 0.1`, `w = 1.0`, colour
  `(40, 10, 4)` on a Fury skin (else `(4, 10, 40)`) times `1 + 10 * boost_blend`.
  Every one of the 40 live-captured records matches, including the `440`
  magnitude at the boost snap. Two lanes found the same call site
  independently (static byte scan, live `LR = 0x2a019c`).
- **The pack/decode round trip is read and exact** (renderer.md, "The `SVC1`
  combine is read", confidence 88): the EdgeGeom packer stores
  `(rgb * 2^(7-k), k + 129) / 255`, the `SVC1` vertex program decodes to
  `rgb * 256/255`. There is no hidden `1/255` between the producer's `440`
  and the fragment program's diffuse sum - that candidate is **excluded**,
  listed here only so nobody re-chases it.
- **The per-vertex formula and the combine are read**: `max(0, 1 - |d|/D)^w
  * max(0, N.L) * colour`, summed over lights within `D`, added to
  `ambient + sun * N.L (+ lightmap)` *before* the albedo multiply (confidence
  80 / 88). `crates/render/src/mesh.wgsl::spu_light_sum` is that, with the
  `256/255`.
- **At ride height the light never reaches the track.** Measured on both
  sides (`crates/game/examples/hd_engine_light_reach_probe.rs`): all 40 of
  the original's captured positions sit 2.98-4.41 units from the nearest
  collision triangle with `D` 0.6-2.1; ours 2.85-4.53 on the same circuit
  (Talon's Junction, not Amphiseum - `hd_engine_light_which_circuit.rs`).
  Bound to the track alone the term changed zero pixels of a frame. So the
  only receiver within `D` is the craft's own hull, and **the light list is
  bound to the craft as well as the track** in `crates/game/src/race/scene/frame.rs`
  - a binding the implementing lane labelled **chosen, not measured**, on
  static evidence only: 74 ship materials compile `SVC1` twins
  (`scripts/ps3-sho.py svc-twins`), so the hull *can* take the stream.

## The observation, as a measurement

Two frames, both at the boost snap (`k = 11`, a 440-unit light a hand's
breadth from the housing), cropped around the craft's rear:

- **Original**: `data/traces/hd-spu-light-companion/race.png` (RPCS3, Talon's
  Junction, 443 km/h on a speed pad - the capture whose records include the
  `440`s). The engine housings are **copper, unsaturated**; the hull's rear
  is its livery; the bloom is the plume's and the pad's.
- **Ours**: `data/scratch/hd-engine-light/talons-t487.png` (same circuit, the
  probe's boost tick). The **whole rear of the hull washes warm-white** and
  the bloom gate picks the wash up.

Ours at rest (`after-t700.png` beside `before-t700.png`) is not the problem:
a warm tint on the housings only, which is what the reference shows. The gap
opens with `k`, which is what a `440` term added pre-albedo into a sum whose
other members are order `1` must do to any receiver within half a `D` of
it - unless the original's hull is not a receiver, or is much further from
the light than ours.

## Open

- **Does the original light the hull at all?** This is the top candidate.
  The `SVC1` selection this project has read is the *track* path:
  `LightCulling` takes the track's per-chunk bounding spheres, and
  `FUN_004074e0`/`FUN_00408fa8` - the track draw compilers - emit opcode
  `0x2d` and set the `| 0x800` variant bit for chunks the bit table marks.
  Nothing read so far shows a *ship* chunk going through that selection; the
  74 `SVC1` twins prove the hull's materials were compiled with the variant
  available, not that the runtime ever picks it. If the original's hull is
  never `SVC1`, the light is invisible at ride height by the disc's own
  numbers (it lights walls and floors within one to two units, i.e. tunnels,
  banked walls and scrapes), and the fix is `SpuLights::none()` for the craft
  in `frame.rs` - the path the implementing lane already left in place.
- **If it does, is our light closer to the housing than the original's?**
  Second candidate, geometric: the anchor `pos - z * Distance` carries the
  locator's Z axis through the craft's world matrix
  (`crates/game/src/livery/engine_light.rs`); a sign or axis slip puts the
  light inside the hull instead of behind the nozzle, and with `D` up to
  `2.1` the falloff at a housing vertex swings from near zero to near one.
  The capture gives the original's light *positions*, and the ship's own
  `ship.vex` gives the housing vertices, so the distance is measurable on
  both sides without the emulator.
- **What the speed-pad flash contributes on the original.** The reference
  frame's purple wash is a pad effect, not the engine light; a comparison
  frame away from a pad, at a Turbo-pickup boost, would separate the two.
  Not captured yet.

## Next Steps

1. **Settle the hull binding with one RPCS3 read** (the implementing lane's
   own suggestion): under `--interpreter`, break in the *ship* draw compiler
   (find it the way the track's was found - the caller that stores the
   `0x868f8229` hash's variant word for a `RigidBody` chunk) and read whether
   the `| 0x800` bit is ever set on a hull chunk during a race; or, cheaper
   and static first, `get_xrefs_to` on `SpuLight_GetVisibleSlotAddress` /
   `SpuLight_GetVisibleCount` (`0x0040d370` / `0x0040d390`) and see whether
   any caller outside the track pass hands the slot to a ship draw. Either
   answer closes the top bullet. `scripts/rpcs3-drive.py`, Xvfb `:77`, port
   `2345`, one emulator at a time.
2. If the answer is *no*: `SpuLights::none()` for the craft in `frame.rs`,
   relabel the binding **measured**, and re-take `talons-t487.png` - the wash
   and its bloom go with it. The light then shows only where the disc puts
   it (walls within `D`); say so in `docs/rendering/README.md`'s entry rather
   than leaving the rest-state housing tint that players liked.
3. If *yes*: measure our anchor-to-housing distance against the original's
   record-to-hull distance (`hd_engine_light_reach_probe.rs` already loads
   the records; add the hull's vertices from `ship.vex`), and fix the
   locator/axis/sign if they differ. Only then compare the boost frames
   again.
4. Capture a Turbo boost away from any speed pad on the original, to have a
   reference free of the pad's own flash.

What would falsify the top candidate: a hull chunk observed with the `SVC1`
bit set on the original during a race. What would falsify the second: our
light-to-housing distance matching the original's within the `±0.1` jitter.
