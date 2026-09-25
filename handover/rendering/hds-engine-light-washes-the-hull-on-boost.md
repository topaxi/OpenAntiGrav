# HD's hull washes white on boost and blooms off it; the original's does not - the hull binding is measured correct, so the wash is some other term

2026-09-20. Reported from play by the user the same evening the light landed
(`421b14c9`): "the light work looks amazing, but it exaggerates the bloom".
The user's first guess - that the bloom had been tuned against the original
before the light existed and the two now stack - is not what the code says,
and the gap has a cleaner shape than "too much". This file is the gap; the
mechanism's own history (producer, formula, capture) is on
[hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md)
and is not repeated here.

## What is established, and is not the problem

- **The hull binding is the original's (2026-09-25, confidence 90).**
  Settled both statically and live. The hull `ship.vex` is registered with
  flag word `0x1ccb` (`Ship_ReloadModelForSkin`, `li r5,0x1ccb` at
  `0x000dc044`). `ModelRecord_Create` (`0x003f0348`) stores it at
  `record+0xe4`, and nothing else in the renderer writes that word. On
  RPCS3 in a Talon's Junction race, the player's hull record `0x00cb1ab0`
  read `0x1ccb`. A `Z0` on the gate's taken branch (`0x003eb368`) stopped
  10 times with that record in `r19`, at cruise and with a boosted light in
  the visible list. Full read: renderer.md, "The hull's `record+0xe4 &
  0x800` bit is set at load". **Do not remove the craft binding.**
- **The SPU term is not what washes our hull.** Talon's Junction, player
  size, a fired Turbo, ticks 250/290/305, on `Piranha` and `Piranha_n1`. An
  A/B that took the craft's `SpuLights` out moved 0.07-0.12 % of the
  frame's pixels, all at the central nozzle. The white wing rims and the
  pink wash on `Piranha_n1` are still there with the binding removed, at
  rest as well as boosted.

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

- **What does wash the hull, if not the SPU term?** Nothing below is
  measured yet. The original's boosted frame on RPCS3 is
  `data/scratch/hd-svc1-bit/bp-081.png`. It shows a speed-pad boost at 449 km/h, the Piranha
  planform with a dark, yellow-striped livery and a classic blue light. Its
  hull keeps its livery, and only the nozzle glows. Candidates:
  1. The flare and plume quads `EngineFlare_PlaceShapes` scales by
     `EF_Main`/`EF_Boost` at boost. They are additive, and they sit over
     the rear of the hull.
  2. The Fury skins' own hull material. `Piranha_n1`'s white rims show at
     rest with the SPU term off.
  3. Bloom picking up either of the two above. The chain is measured, so
     this is an input question, not a tuning one.
- **The livery in the original's frame is not identified.** The planform
  is Piranha, but neither `Piranha`, `Piranha_c1` nor `Piranha_n1` matches
  the dark, yellow-striped paint in `bp-081.png` or `race.png`. The Fury
  flag also differed between the two boots: `race.png`'s light is
  `(40, 10, 4)`, this one's is `(4, 10, 40)`. So a same-team comparison
  still needs the livery named first.

## Next Steps

1. Name the original's livery, then take a matched frame on both sides:
   same livery, Talon's Junction, player size, several boosted frames.
   `data/scratch/hd-svc1-bit/drive.py` is a working private RPCS3 driver
   (it expects to sit at `<checkout>/data/svc1/`): it walks into the race, reads the craft table, and arms
   `Z0`s with stop-reply-safe stepping. Reading `craft+0x6ae0 -> +0x204c`
   gives the hull record index. The team field is not read yet.
2. On our side, A/B the flare/plume quads the same way the SPU term was
   A/B'd, at a boosted tick with `--press square` (fires the held Turbo).
   Measure the pixel share each term moves before touching anything.
3. Compare `Piranha_n1`'s rim against the original's `_n1` at rest before
   treating it as part of this thread; it may be the skin's own paint.

What would falsify candidate 1: removing the flare/plume from the hull
region leaves the wash. What would falsify candidate 2: the original's
`_n1` hull shows the same rims at rest.
