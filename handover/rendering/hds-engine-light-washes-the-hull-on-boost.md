# HD's engine light washes the hull white on boost and blooms off it; the original's stays copper - the binding is measured correct now, the wash's shape is the open suspect

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
- **The hull binding is measured, not chosen, as of 2026-09-25.**
  `Ship_DrawModels` (`0x003ea368`) and its one-entity twin (`0x003eb890`)
  perform the identical per-object `Enable_spu_vertex_light` gate this
  project already read on the track's Zone-Stage compilers -
  `SpuLight_AnyVisibleLightTouchesSphere` against the ship's own bounding
  sphere, `Shader_GetVariantHash(... | 0x800)` on a hit - so the original
  does draw the hull with the `SVC1` twin, conditionally, during a race.
  Confidence 80, renderer.md "The hull binding is settled". **Do not remove
  the binding or gate it off** - that would recreate a state the original
  never produces. See "Open" below for what is still unmeasured (the wash's
  own shape, not whether it happens at all).
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

- **Settled 2026-09-25: yes, the original does light the hull, conditionally.**
  `Ship_DrawModels` (`0x003ea368`, already named on
  [ship-sun-occlusion.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md))
  and its one-entity twin `0x003eb890` - the functions that actually walk
  the ship table (`PTR_DAT_008b7da0`, count `+0x933c4`, `0x1b0`-byte records)
  and issue each ship's own `Render_RunCompiledOps_q` draw - perform the
  identical per-object `Enable_spu_vertex_light` gate this page already read
  on the two track Zone-Stage compilers: `SpuLight_GetVisibleCount`,
  `SpuLight_AnyVisibleLightTouchesSphere`, `SpuLight_GetVisibleSlotAddress`,
  `Shader_GetVariantHash(... | 0x800)`, staged into the same `ctx+0x14c`/
  `+0x150` fields the opcode-`0x2d` handler already proved is read
  regardless of which family wrote it. Full read:
  [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
  "The hull binding is settled", confidence 80. **This falsifies the top
  candidate** - the hull is a real, conditionally-gated `SVC1` receiver in
  the original, not a structurally-dead one, and `SpuLights::none()` for the
  craft would be wrong, not merely untested. `frame.rs`'s binding is
  therefore correct as implemented; it is relabelled below rather than
  reverted.
- **What is still open: the wash's own shape.** A crop comparison this
  session (`data/scratch/hd-engine-light/crops/{orig,ours}-crop.png`, off
  `race.png` and `talons-t487.png` - camera angles differ too much for a
  pixel measurement) shows the original's copper tint confined to the
  housing/fin geometry while this project's render washes flat underside
  panels well beyond it. Candidate, **not measured**: sparser tessellation
  near the housing on this project's `ship.vex` reading would let one or two
  nearby vertices' ~440-magnitude term interpolate across a much larger
  screen area than the same falloff produces on a more subdivided original
  mesh - the same light, the same formula, a coarser receiver. Untested:
  comparing vertex density at the housing between this project's mesh and
  the original's own geometry; no ground-truth vertex count from the PS3
  side exists yet to compare against.
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

1. **Done, 2026-09-25, statically**: the hull binding is settled - see
   "Open" above and renderer.md's "The hull binding is settled". No
   `SpuLights::none()` change; the binding stays as implemented.
2. **Measure vertex density at the housing**, `ship.vex` against whatever
   ground truth for the original's own geometry can be found (a decoded
   `.rcsmodel` chunk count near the `Engine Flare` locator, or a live
   RPCS3 vertex-buffer read) - the live candidate for the wash's different
   *shape*, per "Open" above. If this project's housing mesh is coarser
   (fewer, larger triangles near the nozzle) than the original's, that is
   the fix: subdivide or re-check the LOD/chunk selection feeding the
   housing at player-camera distance, not the light or the binding.
3. Measure our anchor-to-housing distance against the original's
   record-to-hull distance anyway (`hd_engine_light_reach_probe.rs` already
   loads the records; add the hull's vertices from `ship.vex`) - a cheap
   independent check that rules geometry *placement* in or out before
   chasing tessellation. Our anchor's world position already matches the
   original's captured records exactly (engine_light.rs's own 40/40 finding),
   so this is likely to come back negative, but it has not been measured.
4. Capture a Turbo boost away from any speed pad on the original, to have a
   reference free of the pad's own flash - useful once a same-angle
   comparison is possible, not blocking the tessellation check above.

What would falsify the tessellation candidate: this project's housing mesh
carrying the same or higher vertex density near the `Engine Flare` locator
as the original's. What would falsify the geometric-placement candidate:
our light-to-housing distance matching the original's within the `±0.1`
jitter.
