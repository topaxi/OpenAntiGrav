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

- **Narrowed, not settled, 2026-09-25: a real per-ship `SVC1` gate exists
  in the ship draw path, but whether it ever actually fires on a hull is
  still unread.** `Ship_DrawModels` (`0x003ea368`, already named on
  [ship-sun-occlusion.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md))
  and its one-entity twin `0x003eb890` - the functions that actually walk
  the ship table (`PTR_DAT_008b7da0`, count `+0x933c4`, `0x1b0`-byte records)
  and issue each ship's own `Render_RunCompiledOps_q` draw - carry the same
  gate shape this page already read on the two track Zone-Stage compilers:
  `SpuLight_GetVisibleCount`, `SpuLight_AnyVisibleLightTouchesSphere`,
  `SpuLight_GetVisibleSlotAddress`, `Shader_GetVariantHash(... | 0x800)`,
  staged into the same `ctx+0x14c`/`+0x150` fields the opcode-`0x2d` handler
  already proved is read regardless of which family wrote it - gated first
  on `record+0xe4 & 0x800`, a per-ship flag word whose own writer was **not**
  found this session (a scoped brute-force instruction search did not
  converge; see renderer.md for what was and was not tried). Full read:
  [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), the
  `Ship_DrawModels` section, confidence 80 for the gate's shape, **no score**
  for whether it ever actually selects `SVC1` on a real hull. This is the
  same evidentiary position the 74 `SVC1`-twin-compiled materials were
  already in ("available, not proven picked") - one gate closer to an
  answer, not the answer. **Does not falsify the top candidate.**
  `SpuLights::none()` for the craft remains a live option; `frame.rs`'s
  binding is unchanged either way pending one of the two reads below.
- **What would close this**: (1) find `record+0xe4`'s own writer statically
  - the one function already touching a neighbouring bit
  (`Ship_AddToRenderList`, `0x003e4cc8`) does not touch `0x800`, and a wider
  search did not converge this session; or (2) a live RPCS3 read, pausing
  via the GDB stub with no breakpoints (the shape `absorb-feedback.md`'s own
  "Live on RPCS3" section already used successfully on this exact record
  table) - read the count/index at `PTR_DAT_008b7da0+0x933c4`/`+0x933c8`,
  then the u32 at the player's own record `+0xe4` mid-race, and test bit
  `0x800` directly. Either closes it at confidence ~90.
- **A crop comparison this session** (`data/scratch/hd-engine-light/crops/
  {orig,ours}-crop.png`, off `race.png` and `talons-t487.png`) shows the
  original's copper tint confined to the housing/fin geometry while this
  project's render washes flat underside panels well beyond it - but the two
  frames are not the matched team/camera/multi-frame comparison this task
  asked for (different angle, unverified team against the capture, one
  frame each), so this is a qualitative prompt only, not evidence. A
  tessellation-density mismatch near the housing is one candidate reading of
  that shape difference, but since both sides draw the same disc mesh with
  the same formula and the same light record, it would only produce a
  different result if this project's mesh is coarser there (a different LOD
  or chunk selection) than the original's - unconfirmed, and not worth
  chasing before the `0x800` bit above is settled either way.
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

1. **Settle `record+0xe4`'s `0x800` bit - this is now the single
   discriminating question, narrowed down from "does any selection
   mechanism reach a ship at all" (2026-09-25's own finding: one does, its
   shape is read, whether it ever fires is not).** Two routes, cheaper
   first:
   - Static: find the bit's own writer. `Ship_AddToRenderList`
     (`0x003e4cc8`) touches a neighbouring bit (`0x100`, a dedup guard) but
     not `0x800`; a broad `search_instructions` sweep on `stw ..., 0xe4(rN)`
     does not converge (hundreds of unrelated stack-frame-save hits across
     the whole image). Narrow it: check functions that also touch this
     record's other fields `Ship_DrawModels` itself reads alongside `+0xe4`
     (`+0xe0`, `+0xe8`, `+0xec`, `+0x100`, `+0x130`) - a material/capability
     resolve function that sets several of these at once is the likely
     shape, by analogy with the shadow/sun-occlusion bits' own writers on
     this page.
   - Live: pause via the GDB stub with no breakpoints, the shape
     `absorb-feedback.md`'s own "Live on RPCS3" section already used
     successfully on this exact record table - read the count/index at
     `PTR_DAT_008b7da0+0x933c4`/`+0x933c8`, then the u32 at the player's own
     record `+0xe4` mid-race, test bit `0x800`.

   If the bit is set (always, or whenever the ship's resolved material has
   the `SVC1` twin): the gate genuinely fires, the top candidate is
   falsified, relabel the binding measured, and go to step 2. If never set
   in play: `SpuLights::none()` for the craft in `frame.rs`, relabel the
   binding measured (the negative direction), and re-take `talons-t487.png`.
2. Only once step 1 answers *yes*: measure our anchor-to-housing distance
   against the original's record-to-hull distance
   (`hd_engine_light_reach_probe.rs` already loads the records; add the
   hull's vertices from `ship.vex`). Our anchor's world position already
   matches the original's captured records exactly (`engine_light.rs`'s own
   40/40 finding), so this is likely to come back negative, but it has not
   been measured.
3. Also only once step 1 answers *yes*: re-take a matched comparison - same
   team as the capture (verify which team `race.png` is; this session's own
   crops did not check), same circuit, player size, more than one frame,
   ideally away from a speed pad (the reference frame's white wash next to
   the ship is a pad effect, not confirmed separated from the engine light
   yet). Only then is a tessellation-density hypothesis (this session's own
   qualitative read of the crop shapes, not a measurement) worth chasing -
   both sides draw the same disc mesh with the same formula and the same
   light record, so it would only diverge if this project selects a
   coarser LOD or chunk set than the original at the same camera distance,
   which is itself unconfirmed.

What would falsify the gate candidate entirely: `record+0xe4`'s `0x800` bit
observed never set on a real ship, live or via its writer. What would
falsify the geometric-placement candidate (once reachable): our
light-to-housing distance matching the original's within the `±0.1` jitter.
