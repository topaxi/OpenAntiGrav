# HD's hull washes white on boost and blooms off it; the original's does not - the hull binding is measured correct, so the gap is in the light term's inputs

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
- **The SPU term is what flattens our inner rear panels to white.** The A/B
  was on Talon's Junction at player size, with a fired Turbo (it arms the
  same `exhaust.boost` the flame's blend reads). On `Assegai_n1` at tick
  295, the panels either side of the nozzle go flat white with the craft
  binding and keep their structure without it:
  `data/scratch/hd-svc1-bit/rear-ab.png`. Two same-command runs differ by
  about 2 % of pixels at that tick from motion blur, so read the frames, not
  a pixel count. The white wing rims and the pink on the `_n1` wings stay
  with the binding removed. They are a separate term.

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

- **Why the same term washes our panels and not the original's.** The
  original's boosted frame is `data/scratch/hd-svc1-bit/bp-081.png`: RPCS3,
  a speed-pad boost at 449 km/h, the Piranha planform in a dark,
  yellow-striped livery, classic blue light at about `x6`. Its hull glows at
  the nozzle and housings and keeps its livery elsewhere. Candidates, none
  measured:
  1. The space the `EdgeGeom` job evaluates `|d|/D` in. If opcode `0x30`
     (`RenderOps_BuildEdgeGeomJob`) hands the SPU object-space light
     positions and the hull's world matrix is scaled, the reach of `D`
     differs from ours by that scale.
  2. The anchor-to-panel distance on both sides. The anchor matches the
     original's records; the panels' distance from it is unmeasured.
  3. The boost blend at the frames compared: ours near the snap (`x11`),
     the original's visible boosted light about `x6`.
- **The livery in the original's frames is not identified.** The planform
  is Piranha, but `Piranha`, `Piranha_c1` and `Piranha_n1` do not match the
  dark, yellow-striped paint in `bp-081.png` or `race.png`. The Fury flag
  also differed between the two boots: `race.png`'s light is `(40, 10, 4)`
  and this boot's is `(4, 10, 40)`.
- **The white wing rims on the `_n1` skins** are not the SPU term. Whether
  the original shows them is unread.

## Next Steps

1. Read how opcode `0x30` builds the `EdgeGeom` light array: world or
   object space, and whether `D` is scaled. Then read the hull's world
   matrix scale. This is the cheapest of the three and could explain the
   whole gap.
2. Measure our anchor-to-panel distance for the hull in `rear-ab.png`
   against `D`, with `hd_engine_light_reach_probe.rs` plus the hull's
   `ship.vex` vertices.
3. Take a matched pair: same livery, Talon's Junction, player size,
   several boosted frames at a known blend. `data/scratch/hd-svc1-bit/drive.py`
   is a working private RPCS3 driver (it expects to sit at
   `<checkout>/data/svc1/`). It walks into a race, reads the craft table,
   and arms `Z0`s with stop-reply-safe stepping. `craft+0x6ae0 -> +0x204c`
   gives the hull record index. The team field is not read yet.

What would falsify candidate 1: the job gets world-space lights, or the
hull matrix is unscaled. What would falsify candidate 2: our panels sit
further than `D` from the anchor, so the wash would need another cause.
