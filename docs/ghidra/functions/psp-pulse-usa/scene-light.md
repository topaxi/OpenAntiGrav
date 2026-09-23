# The hull's GE light list: the circuit's authored lights, per craft

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | mesh lighting |
| **Related** | [`mesh-draw.md`](mesh-draw.md) (`Mesh_SetBatchLighting`, the `SceneLight_*` list pair), [`lighting.md` (EU)](../psp-pulse-eu/lighting.md) (the collection side), [`track.md`](../../../formats/track.md) (the spline points' light scales), [`glow-mask.md`](../../../rendering/glow-mask.md) |

Read 2026-09-23, static and live, to answer why our Assegai hull draws a dark
navy where PPSSPP draws a vivid blue. **The hull is lit by the GE's own
hardware lights, and they are the circuit's authored `AmbientLight` and
`DirectionalLight` nodes.** That is also the consumer
[`lighting.md`](../psp-pulse-eu/lighting.md) searched nine passes for and
recorded as absent.

## The draw path

An opaque hull batch (Assegai's `shipShape`, airbrakes and canopy are vertex
type `0x0121`: normals, no colour) takes `Mesh_SetBatchLighting`'s
(`0x0890d3ac`) lit branch, which is recorded into the batch set's display
list at load:

```c
SceneLight_CallLightingList(model + 0xb0);        // a CALL, replayed per frame
Gu_Material(1 | 2, mesh->0x6c | 0xff000000);
Gu_Material(4, 0);
Gu_Color(0xffffffff);                             // Gu_Material(7, white)
```

`Gu_Color` (`0x088112c8`) is `Gu_Material(DAT_08adc3ac, 7, colour)`, so it
overwrites the two material writes before it: **the material is white on
every channel**. The display list replays a `CALL` into `model+0xb0`, so what
the GE executes is whatever that list holds at draw time, not at record time.
Mesh batches with vertex colour take the other branch, and with the
`-1` ambient every batch set records, lighting is off and the GE uses the
vertex colour.

`SceneLight_Rebuild` (`0x08879b14`) rewrites `model+0xb0` from
`Vex_UpdateLightLists` (`0x08912358`) whenever the model's context
`model+0xac` is non-null. It calls `SceneLight_BuildLightingList`
(`0x0887a4f0`) with `param_2 = model+0xac` and the four bytes at
`model+0x44..+0x47` as scales. The builder emits:

- `LIGHTINGENABLE = 1`;
- the global ambient `0x5c`/`0x5d`: the sum of every `AmbientLight` in the
  context (`+0x3c` count, `+0x54` list), each channel `min(255, trunc(c *
  255))`, times the ambient scale `>> 8`. A context with no light list at all
  (`+0x6cc == 0`) gets a grey `0x30 * scale >> 8` instead;
- one GE light per `DirectionalLight` (`+0x40` count, `+0x7c` list, at most
  four): `LIGHTTYPE` `0` (directional, diffuse only), the light vector (the
  node's matrix row, negated), ambient and specular colour `0`, diffuse
  colour `min(255, trunc(c * 255)) * scale >> 8`;
- the remaining slots from the two point classes (`+0x4c`/`+0x8c`,
  `+0x50`/`+0x3ac`), sorted by strength, with their own two scales. None
  reach the player's hull on Talon's Junction.

## Read live

PPSSPP v1.20.4, `UCUS98712`, single race on Talon's Junction, Assegai,
stationary on the grid after GO. Scripts under the lane's scratch
(`lighting_probe.py`, `scenelight_probe.py`, `reread_lists.py`,
`lit_models.py`); every one is a halting breakpoint at a function entry or a
plain memory read.

- A breakpoint on `SceneLight_CallLightingList` (`0x0887a230`) during a
  restart caught 440 calls from `0x0890d430` (`Mesh_SetBatchLighting`) over
  113 distinct models, all with the same context (`0x08f29ff0`). **None fire
  per frame**: 15 s of racing hit it zero times, because the lists are
  recorded once. At record time every list was still empty (`RET`).
- Re-read in the race, 16 of those lists hold a full light list. The
  player's hull model (`craft+0x8b4` = `0x09b78670`) reads:

```text
LIGHTING=1  AMBIENT=(41,47,50)
LIGHT0=1  L=(-0.639, 0.529, -0.559)  LTYPE0=0  LAC0=0  LDC0=(249,232,153)  LSC0=0
LIGHT1=1  L=(-0.093, 0.328,  0.940)  LTYPE1=0  LAC1=0  LDC1=(78,95,117)    LSC1=0
LIGHT2=1  L=( 0.965, 0.260,  0.024)  LTYPE2=0  LAC2=0  LDC2=(47,58,71)     LSC2=0
LIGHT3=0
```

  with scales `(250, 250, 250, 250)`.
- `16_Track`'s own `track.vex` authors one `AmbientLight` `(0.1687, 0.1929,
  0.2070)` and three `DirectionalLight`s. Decoded through
  `oag_vex::lighting` (`crates/vex/examples/light_rig_probe.rs`): colours
  `x255` truncated are `(43, 49, 52)`, `(255, 238, 157)`, `(80, 98, 120)` and
  `(49, 60, 73)`, and each times `250 >> 8` is exactly the list above, to the
  unit. The three light vectors are **row 2** of each node's world matrix as
  `vex::world_transforms` returns it, to three decimals. `intensity` (`1.0` on
  all four) does not enter: the builder reads the colour's `x`, `y`, `z` and
  puts `w` in the unused alpha byte.

Confidence **90** for the list contents and their source (every value
reproduced from the disc), **85** for the white material (the three calls
are read in order and `Gu_Color`'s body is two instructions).

**The lighting equation** is the GE's, as PPSSPP implements it: per vertex,
`clamp(ambient * mat_ambient + sum(diffuse_i * mat_diffuse * max(0, N . L_i)),
0, 1)` with the normal in world space, then `MODULATE` with the texel. With a
white material that is `texel * clamp(ambient + sum(diffuse_i * max(0, N .
L_i)))`. An up-facing face on the grid gets `(0.83, 0.85, 0.74)` of its
texel.

## The scales: a per-craft copy of the track's own light level

The four scale bytes are not constants. `Craft_ApplyHullLightScale`
(`0x0883e444`), called from the per-craft update `FUN_088418e0`, copies
`craft+0xb52..+0xb55` into `+0x44..+0x47` of both hull models
(`craft+0x8b4`, and `craft+0x8b8` when present). A logged write watchpoint
on the player's `craft+0xb50` (8 bytes, 3 s) names the writers:

| PC | Function | Hits |
| --- | --- | --- |
| `0x0887c17c`/`19c`/`1bc`/`1dc` | `Spline_AccumulateLightScale` (`0x0887c11c`) | 561 each |
| `0x0887c0d8`..`0x0887c118` | `FUN_0887c088` (the first point, stored) | 187 each |
| `0x0887c90c`/`9ec`/`b08`/`b30`/`c64` | `Spline_SampleSegment` (`0x0887c7e8`) | 187 each |

`Spline_SampleSegment` evaluates the uniform cubic B-spline over four spline
points (the same `vfim 1/6` basis `oag_vex::track::basis` carries) into a
sample record, and for each of the last three points calls
`Spline_AccumulateLightScale`, which adds `(point[+0x62..+0x65] *
trunc(weight * 255)) >> 8` into the record's four bytes, wrapping as a `u8`.
If the track's version is below `0x103` the four bytes are forced to `0xff`.
So the scales are **the spline points' own bytes `+0x62..+0x65`, blended at
the craft**. `16_Track`'s grid points read `255`, and the double truncation
brings a blend of them to about `251`, against the live `250`. AI craft
inside a tunnel read `133`: 231 of `16_Track`'s 862 points author `127`.

Confidence **85** for the copy and the accumulate (bodies read, writers
named live), **75** for `Spline_SampleSegment`'s role (the basis constants
and the call order are read; the caller that selects the segment is not).

## Applied names

Mirrored in [`names.tsv`](names.tsv).

| Address | Kind | Name | Conf |
| --- | --- | --- | --- |
| `0x0887a4f0` | function | `SceneLight_BuildLightingList` | 88 |
| `0x0883e444` | function | `Craft_ApplyHullLightScale` | 85 |
| `0x0887c11c` | function | `Spline_AccumulateLightScale` | 85 |
| `0x0887c7e8` | function | `Spline_SampleSegment` | 75 |

`SceneLight_BuildLightingList` loses its `_q`: its return value is still not
read, but the list it writes is now matched to the disc field by field.

## Open

- **What the other 89 lit-branch models get.** Their lists stayed `RET`, so
  the GE runs them with lighting in whatever state the batch set left it:
  `FUN_089307b4` opens with `Gu_Disable(10)`, so unlit, white material, the
  raw texel. Which models those are (weapons, pickups, scenery) was not
  mapped.
- The point-light slots and their two scales are not reproduced; none reach
  the grid.
