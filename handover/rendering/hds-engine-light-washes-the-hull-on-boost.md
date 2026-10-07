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

- **The bloom is measured, not tuned.** `crates/post/src/hd_bloom.rs`:
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
  80 / 88). `crates/mesh/src/mesh.wgsl::spu_light_sum` is that, with the
  `256/255`.
- **At ride height the light never reaches the track.** Measured on both
  sides (`crates/game/examples/hd_engine_light_reach_probe.rs`): all 40 of
  the original's captured positions sit 2.98-4.41 units from the nearest
  collision triangle with `D` 0.6-2.1; ours 2.85-4.53 on the same circuit
  (Talon's Junction, not Amphiseum - `hd_engine_light_which_circuit.rs`).
  Bound to the track alone the term changed zero pixels of a frame. So the
  only receiver within `D` is the craft's own hull, and **the light list is
  bound to the craft as well as the track** in `crates/raceplay/src/scene/frame.rs`
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

- **Candidate 1 (boost blend at the compared frames) is closed - it does
  not explain the gap.** `EngineFlare_Update`'s snap-and-decay law is
  measured identical on both sides already (direct decompile: `*(this+0x144)
  = 1.0f` under the boost-timer gate, `*= 0.8` per substep otherwise -
  `engine-flare.md`, "The boost gate is a timer, a snap and an exponential
  decay"), and `Flame::advance` is the same two branches. There is no
  ramp-vs-snap bug to find.
- **The `x11`-vs-`x6` comparison this thread had been running compared two
  different ships, not two blend states of one ship.** `bp-081.png`'s
  player is confirmed Piranha (renderer.md's own "Live, RPCS3" entry); the
  wash this thread chased (`rear-ab.png`, `talons-t487.png`) is
  `Assegai_n1`. A same-command A/B this session across three ships
  (`data/scratch/hd-light-boost/{piranha,assegai_n1,harimau}-snap.png`,
  same circuit, same tick, same boost cadence) found Piranha does not wash
  and both Assegai_n1 and Harimau do - **and `EngineLightData.xml` gives a
  disc-verified reason**: Piranha's `Distance` is `0.4` (anchor pulled back
  off the hull), Assegai's and Harimau's are `0.0` (anchor exactly at the
  flare locator). Full survey and the reasoning: renderer.md, "Candidate 1
  ... is closed ... candidate 2 ... gets a disc-verified, per-ship
  mechanism" (2026-09-25, later still).
- **Answered 2026-10-02: the original's `Distance = 0.0` hulls do not wash.**
  Live on RPCS3, the player's own light read boosted (`(36, 90, 360)`) and the
  hull clean, on `Assegai` and `Harimau` (team read from memory, not the
  screen); falsifier met, so "our wash is correct per ship" is **false** and
  the `Distance` correlation is not the original's behaviour. Method, the
  trap of picking the player's light out of the visible list, and the frames:
  renderer.md, "The original does not wash a `Distance = 0.0` hull at the
  boost snap" (confidence 75, one verified boot per ship). The paired frame of
  ours is **not** at the same stretch of track (sunlit, a pale spine already at
  rest), so the evidence of our wash is a same-place on/off A/B of the craft's light term, now on the classic `assegai` (tick 295: spine and wing go flat white, tick 220 mild) as well as `assegai_n1`.
- **Candidate 3's clamp reading, not measured.** The `SVC1` vertex program
  decodes into `o[TC0]`, a texcoord output, not `o[COL0]`/`o[COL1]` - RSX
  colour-output clamping would not apply there, consistent with
  `mesh.wgsl`'s own unclamped `spu_light` varying, but this is a read of
  which register the microcode targets, not a tested hardware behaviour.
- **The white wing rims on the `_n1` skins** are not the SPU term. Whether
  the original shows them is unread.

## Next Steps

1. Read the per-vertex weight in `EdgeGeom` (`0x007f6d80`,
   `data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`) for what it does with
   a hull chunk: the space `|d|` is taken in against `D`, where the normal for
   `N.L` comes from, and which lights the hull's stream is built from. Each is
   a candidate for why the original's panels beside the nozzle get weight zero
   at a `360` light; none is measured.
2. Identify the record every `0x003eb368` stop carried (`0xca6e90` on one boot)
   as the player's hull or not: the model-pointer read in the drive script's
   `snapshot` (`model + 0x204c`) failed on this binary, so it was not read.
3. A same-place original/ours pair: drive our craft onto the Talon's Junction
   pad (the original's player light at `(-237, -74, 55)` on the pad; our trace
   frame did not line up with the light list's, so the pad location in our
   coordinates is still to find) so the paired frame stops being confounded by
   lighting. The on/off A/B on our side exists; this is the other half.
4. Second verified boots for `Assegai` and `Harimau` (the extra attempts missed
   the pad; the player's boost depends on the line taken).

What would falsify "the original's hull has weight zero here": a frame of the
original, player's own light at `360`, with the rear panels flat white. What
would name the law: the `EdgeGeom` read in step 1 reproducing the clean hull
from the captured light record and the hull's own vertices.

## 2026-10-07 hd-hull-wash (partial)

Second boot of the original on Assegai at the pad, player light `(36,90,360)`, hull clean (renderer.md, "The hull reaches `EdgeGeom` as a node-local chunk"). Every hull chunk is flag bit 1 (kind 2, node-local), so `EdgeGeom` takes the `0x3030` branch; whether its rows are a world matrix or the decode quads is unresolved. Next: identify the DMA piece at `$18`, then inject into the list the job really reads. Steps 1 and 3-4 above are partly done; Harimau and Piranha not re-booted.
