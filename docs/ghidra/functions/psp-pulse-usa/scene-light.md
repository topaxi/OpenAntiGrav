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
  reach the player's hull on Talon's Junction, but they do on Tech de Ra and
  Fort Gale: see "The point and spot lights" below.

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
  `oag_vex::lighting`: colours
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

**Wired 2026-10-17.** `oag_race::Course::light_scale` carries each ring point's
blend, `Race::ship_hull_scale` reads it at the craft's `course_index`, and
`mesh.wesl` applies `(byte * scale) >> 8` per craft to the bare rig (built at
scale 256). Byte 0 scales the ambient and byte 1 the directionals - the order
is read from the builder's two scale arguments and fits the tunnel's authored
`[127, 127, 255, 255]` (the last two being the point-light pair, not
reproduced); not confirmed live. The nearest ring point stands in for the
blend at the craft's own parameter. Result: a Pulse craft darkens in a tunnel
as the original's does. Checked against HD and the others: HD's darkening is
its own sun-occlusion mechanism (`ship-sun-occlusion.md`). **Pure: ported**
(2026-10-26): its circuits author the same light nodes under its own class ids,
its builder and spline sampler are these, and its hull now takes the rig and the
scale bytes ([`psp-pure-usa/scene-light.md`](../psp-pure-usa/scene-light.md)).
An earlier "Pure authors no rig" was the reader matching Pulse's class ids.

Confidence **85** for the copy and the accumulate (bodies read, writers
named live), **75** for `Spline_SampleSegment`'s role (the basis constants
and the call order are read; the caller that selects the segment is not).

## The point and spot lights (2026-10-20, `pulse-pair-diffs`)

Found by reading the grid hull's list on Tech de Ra live (PPSSPP, the Time Trial
grid state), because our hull was brown/orange where the original's is blue-white.
That circuit authors three `DirectionalLight`s and no ambient, and the list the GE
held had a **fourth** light the three nodes do not account for:

```text
LIGHT3  LTYPE3 = 0  vector (-0.0030, 4.0426, 9.4211)  LDC3 (61, 142, 157)   scales 249
```

The two lists `World_CollectMarkerLists` fills at `+0x4c`/`+0x8c` (43 objects here) and
`+0x50`/`+0x3ac` (1) are the **`wopoint` `0x3cf`** and **`wospot` `0x3ce`** nodes, not
`PointLight` `0x132` (which has no registration site and is authored on `13_Track`
only). The object whose position (`obj+0x50`, the node's world translation) is 10.25
units from the craft - the vector's length - is a `wopoint` at
`(-58.024, -5.471, 89.360)`. Its runtime struct (`obj+0x60`) is the payload: colour
`(0.388, 0.907, 1.0, 1.0)`, inner radius `7.0`, outer radius `15.0`
(`oag_vex::world_lights`).

The vtable methods the builder calls (`obj+0x38`, then the slot), all read from the
disassembly:

| Slot | `wopoint` | `wospot` | Does |
| --- | --- | --- | --- |
| `+0x8c` | `0x0892cb54` | `0x0892d400` | copies the colour vec4 (`struct+0x00`, `+0x70` for a spot) |
| `+0x94` | `0x0892cb64` | `0x0892d410` | strength `0..1` at a position |
| `+0x9c` | `0x0892cc50` | `0x0892d640` | `position - light` (the builder negates it: light minus craft) |
| `+0xac` | `0x0892cc98` | `0x0892d688` | `distance - outer radius`, the countdown to the next re-check |

**Strength.** With `d = |craft - light|`: `1` inside the inner radius; between the radii
`t = (outer - d) / (outer - inner)`, `t^2 (3 - 2t)`; `0` beyond. A spot multiplies that
by a cone term over `cos` of the angle between its axis (the negated row 1 of its matrix,
`obj+0x60`) and the direction light-to-craft: `1` above `cos(inner/2)`, `0` below
`cos(outer/2)`, a smoothstep of `(cos - cos(outer/2)) / (cos(inner/2) - cos(outer/2))`
between (the angles are the payload's words at `+0x18`/`+0x1c`, halved by the code).

**Selection and colour.** The builder keeps the strongest `4 - directionals` candidates
(points first, then spots, in node order; a strictly stronger one replaces the first
weakest, a tie keeps the earlier). Each takes `trunc(c * strength * 255)` clamped to 255
per channel, then `(byte * scale) >> 8`, the scale being the craft's **third** spline
scale byte for a point and the **fourth** for a spot. The light is written as a GE
light of type 0 (directional) with the **unnormalised** vector `light - craft`.

**Checked three ways.** (1) Arithmetic: Tech de Ra, `d = 10.25`, `t = 0.594`, strength
`0.639`, `(0.388, 0.907, 1.0) * 163 = (63, 147, 162)`, `* 249 >> 8 = (61, 142, 157)` -
the live bytes. (2) Fort Gale's grid (the reversed layout, two directionals, scales 251),
read live: `LDC2 (5, 19, 31)` at `(-34.6, 4.97, -8.64)` and `LDC3 (245, 245, 219)` at
`(-0.147, 25.89, 9.27)` - both reproduced to the byte, vectors to 2e-3, slot order
included, by `oag_mesh::mesh_render::HullPointLights` from the disc
(`crates/render/tests/hull_lights_ground_truth.rs`). (3) Talon's Junction took none live
and none reaches the grid in ours. **Normalised or not**: the reference renderer
normalises the vector of a type-0 light. Rendered both ways against the original's
Tech de Ra hull (mean colour over three regions of the wings and fuselage): normalised
`(87,106,114) (62,74,86) (102,115,107)` against the original's `(87,105,115) (62,73,86)
(101,114,107)`; unnormalised `(100,120,134) (71,85,99) (133,149,136)`, 15-30 levels bright.
Wired: `Race::ship_hull_points` per craft per frame into the model uniform's `hull_point`,
`mesh.wesl` adds each as one more `N . L` term.

Confidence **90** for the point law (two live lists, both to the byte), **85** for the
spot's cone (one live list, which has a spot in it only if one of Fort Gale's two lights
is one - the cone's branches were not isolated), **80** for the `+0xac` countdown role
(read, not exercised: it only gates when a far light is re-tested).

Before and after, same pose, Tech de Ra hull mean colour (wing, wing, fuselage):
before `(78,76,74) (61,70,81) (91,85,75)`, original `(87,105,115) (62,73,86)
(101,114,107)`, after `(87,106,115) (62,74,87) (102,116,108)`; The Amphiseum before
`(93,102,115) (75,96,111) (87,82,82)`, original `(108,131,145) (90,123,142)
(119,133,129)`, after `(107,129,144) (87,120,140) (121,136,131)`.

Not reproduced: the countdown timers (a pure function of distance is equivalent for a light
that stays where it is, since it cannot come into range before the craft has moved the remaining
distance; a light that moves is re-tested on every call, so it is equivalent there too - see "A
light that moves" below), and non-player craft use the same list from their own position (the
original rebuilds each craft's list; the player's was the only one read live). **Pulse PS2:
checked, applies in the data, not wired** (2026-10-20, `hull-points-ps2`). The PS2 builds the same
vector and colour bytes for the same `wopoint` (read live on PCSX2), but composes its own rig around
it, and our PS2 hull has never taken any of the rig: its vertices carry a colour, so
`lit = 0` and `mesh.wesl` skips the branch. Evidence and the open items on
[`ps2-pulse-eu/hull-lights.md`](../ps2-pulse-eu/hull-lights.md). **Pure: ported** 2026-10-26: its
`wopoint` (`0x381`) and `wospot` (`0x380`) nodes reach its hull through the same slots
([`psp-pure-usa/scene-light.md`](../psp-pure-usa/scene-light.md)).

### De Konstruct, the 8-level residual (2026-10-20)

The residual on the PSP hull (ours `(102,106,89)` against the original's `(94,98,78)` on
the Time Trial grid) was read against the live lists. PPSSPP, De Konstruct White (the
reversed `05_Track`), the same Time Trial grid. **Eleven GE light lists exist in
memory** (found by scanning for the list's `0x5c`/`0x5d` ambient opening); three are
*templates*, built with the craft at the origin and the default scale `255`, and one is
the player's live hull:

```text
live hull  (scales 179, 179, 250, 250)
5c  ambient (25, 23, 16)
L0  vector (0.584, 0.557, 0.590)   LDC0 (35, 60, 85)
L1  vector (-0.501, 0.620, -0.603) LDC1 (178, 152, 83)
L2, L3 off
template   (scales 255, 255, 255, 255, model position (0, nan, 0))
5c  ambient (36, 32, 23)    LDC0 (50, 85, 121)    LDC1 (254, 217, 118)
L2  vector (-15.0967, -19.7246, -0.8759)  LDC2 (254, 246, 208)
```

(The first read of this lane, `entity + 0x8b4`, landed on a template: it is the object
with the craft pointer at `+0x10`, and its model holds no position. The earlier lane's
`+0x8b4` read on Tech de Ra hit the live one.) What the lists say:

- **The rig matches ours to the byte**, scale aside. The vectors are row 2 of the two
  `DirectionalLight`s' world matrices (a one-off read), the colours are
  `trunc(c * 255) * scale >> 8` with **no intensity** (`0.31` and `0.71` on the ambient
  and the first directional do not enter, again), and the ambient is the sum.
- **The live list takes no point light**: the craft on the grid is 75.5 units from the
  `wopoint`, outside its `50` outer radius, which is what ours finds. The template's `L2`
  is that node's **world position** to four decimals (`light - (0, 0, 0)`, strength 1 at
  24.8 units, `(1.0, 0.9719, 0.82) * 255 * 255 >> 8 = (254, 246, 208)` to the byte): the
  `wopoint` law again, on a list built before the craft was placed.
- **The scale is the cause of part of the residual.** The live scales are `179`
  (`(179, 179, 250, 250)`), the blend of a `255` control point and a `127` one at 0.59 of
  the way between them (`127 + 0.41 * 128 = 179`; the craft's projection onto the chord
  is 0.61). Ours read the nearest ring point's `187`. `Course::light_scale_at` now
  projects the craft onto the chord to the neighbouring ring point and blends the two
  (reads `(179, 179, 250, 250)` here, `hull_light_scale_ground_truth.rs`), and
  `Race::ship_hull_scale`/`ship_hull_points` use it. Region means on the new pair
  (ours before, after, original): `(123,132,111) (121,130,109) (116,125,96)`,
  `(100,94,58) (97,90,56) (95,88,55)`, `(75,68,76) (73,67,75) (65,60,69)`.
- **What is left is `(5,5,13)`, `(2,2,1)` and `(8,7,6)`**: the second region is closed
  and the first and third keep most of their gap, which was not isolated. The
  original's hull is softer and flatter (a bloom or blur the hull does not escape) and
  ours has white edge highlights; both are suspects, neither measured.
  Confidence 40 that the rest is a post effect and not the light list, which now matches.

## A light that moves: Arc Prime's floor spotlights (2026-10-26, `arcprime-spotlights`)

**What the spotlights are.** The pools of light on Arc Prime's finish stretch are four additive
floor meshes (`SL_Spotlight_ADD_GLOW`, `10_Track` nodes 195, 250, 253 and 268, the same in the
reversed layout) whose material authors a UV offset loop of 5 s, so the bright patch sweeps along
the floor. The circuit authors **no `wospot` at all**; its 43 lights are `wopoint`s, and two of
them, white `(1.0, 0.921, 0.986)` with radii 45 and 55, hang under the `Anim Transform`s
`#991` and `#993` (node numbers of the forward file), which loop in **5.0 s** too. They rest at
`(-472.6, 6.8, -232.8)` and `(-162.2, 6.8, -219.8)` and sweep `(172, 22, 70)` and `(197, 11, 57)`
units, `(-248.0, 17.9, -227.6)` being the second one at 2 s. So the spotlights are the floor
patch **and** a light that rides with it; the hull is lit by the light the way the floor is lit
by the patch. They are the only `wopoint`s of the twelve circuits (both layouts each) that sit under
an `Anim Transform`, and the PS2 disc's `10_Track` authors the same two at the same places.

The other spotlights on the circuit (a lamp, `SL_Spotlight_a`, nodes 385 and 427, throwing a
`col_spotlightalpha_GLOW_BLEND` cone, nodes 387 and 429, onto a building face lit by an additive
7-key sweep, nodes 393 and 425) light a façade about 100 units off the racing line and **author
no light node**: animation only. The cone points up and away from the line, so no craft is ever
in it; nothing to wire for them.

**What the original does with a light that moves.** The `wopoint` vtable is `0x08ad24fc`
(`WoPoint_GetStrength` is its `+0x94` slot):

- `+0x7c`, `WoPoint_Init` (`0x0892ca04`): points `obj+0x60` at the 32-byte payload, copies the
  node's world translation into `obj+0x50`, then walks the parent links (`obj+8`) for an
  `Anim Transform` (`AnimTransform_ClassTag`). Found: `obj+0x64 = 1`. Not found: `obj+0x64 = 0`.
- `+0x24`, the update slot (`Weather_Update` is the same slot of its vtable),
  `WoPoint_Update` (`0x0892cd00`): refreshes the node's world matrix when its dirty bit
  (`+0x2c & 0x1000`) is set and **copies its row 3 into `obj+0x50`, every update**.
- The list builder, for the `wopoint` list: `if (obj+0x64 == 0) { if ((countdown -= dt) <= 0)
  retest } else { retest }`. A light with the byte set is re-tested on **every call**, so the
  `+0xac` countdown (distance minus outer radius) never holds a moving light back. The builder
  reads the same `+0x50` position the update wrote.

So the original's hull light is where the sweep has put it, this frame. Before this lane ours read
every light once, at time zero, and lit the hull under the pool's **resting** place.

**Read live** (PPSSPP v1.20.4, `UCUS98712`, Time Trial on Arc Prime White, the saved grid state;
nothing in the original was changed except the craft's place for the stills). The world context
holding the lists is `0x08fc8960` (found by scanning RAM for the `+0x3c == 1`, `+0x40 == 2`,
`+0x4c == 43` shape: the hull list builder's own `a1` was another context, with no lights).
**43** `wopoint` objects, and `obj+0x64` is set on **exactly two** (`0x097cf5f0`, `0x097cf760`).
**Fourteen samples** of both lights' `obj+0x50` over 7 s of race clock (the clock at
`0x08b344b0`, 34.985 s to 42.025 s) fitted against `oag_vex::world_lights::PlacedLight::at` over
all phases in 1 ms steps: **mean error 0.03 units, worst 0.15**, at an anim clock of the race clock
plus 3.636 s (mod 5); the next-best distinct phase has a mean error of 0.34. That is the offset a
different lane read for the world meshes' texture clock on this state, 23.64 s (23.64 mod 5 =
3.64): the lights and the meshes ride one clock. **The stills of the original are qualitative
only**: the craft, placed at `(-248.0, 13.0, -227.6)`, drifted and tilted onto the wall within a few
frames (20 to 36 km/h on the HUD), so its attitude differs from ours and from frame to frame. It
reads blue-white in the frame where the light is 22 units off and dark blue in the frames where the
light is 75 or more away, the change ours shows, but the claim rests on the position fit and the
code, not on a matched frame. The hull's GE light list was not read live with a moving light in
it: the light objects and the builder's code were.

Confidence **85** that a light under an `Anim Transform` moves with it and the hull takes it where
it is (both slot bodies read, the flag and the builder's branch read, 14 live samples of the light
objects to 0.15 units; not raised to 90 for want of a live read of the hull's list), **85** for the
two names, **50** that the `wospot` twin of `WoPoint_Init` does the same (unread; no `wospot` moves
in any shipped circuit, and ours places one by the same chain).

**Wired 2026-10-26.** `oag_vex::vex::NodeChain` (lifted out of `placed_psys`, which the placed
particle effects use too) keeps a node's ancestry as a function of time;
`oag_vex::world_lights::placed_lights` returns each light with its chain when an `Anim Transform`
is above it, `PlacedLight::at(seconds)`; `HullPointLights::at(position, seconds, scales)`;
`Race::ship_hull_points(slot, seconds)` is handed `Scene`'s animation clock, the one the track's
`Anim Transform` meshes are drawn at, so `--anim-seconds` and the capture clock offset keep the
pool and its light in step. No setting: this is the original's behaviour, not an addition. The
sweep and the pool's UV loop are both 5.0 s, so they stay in phase over a race; in top-down stills
the light sits about 25 units behind the pool's bright centre at 1 s and 2 s (rough, off 960x544
frames), which is where the file puts it.

Before and after, the same craft (Assegai, the default team) and `--anim-seconds`, 480x272:
`(-248, 13, -227.6)` at 2 s, the light's own place then: hull brown before, blue-white after;
`(-162.5, 3.6, -224.7)` at 2 s, where the light rests and no longer is: blue-white before, dark
after; at 0 s there: blue-white both. `crates/render/tests/hull_lights_ground_truth.rs`
(`arc_primes_floor_light_lights_the_hull_where_the_sweep_has_put_it`) pins it: the craft under
the 2 s spot takes `(254, 233, 250)` at 2 s and none of it at 0 s, and the reverse at the
resting spot. `crates/vex/tests/world_lights_ground_truth.rs` pins the chain against
`world_transforms_at` for every light on the disc and that exactly Arc Prime's two move.

Other circuits, titles and the PS2. **Every Pulse circuit**: Arc Prime is the only one with a
moving light, in both layouts. The PSP disc's twelve circuit files (01 to 07, 09, 10, 13, 14, 16,
each in both layouts, 02 authoring no light at all) and the PS2 disc's thirty-two (all sixteen
circuits, so 08, 11, 12 and 15 as well, which the PSP sells as downloadable content) were walked;
the PS2's Arc Prime authors the same two lights at the same places. **Pulse PS2: checked, applies
in the data, not wired**: the PS2 hull takes none of the rig (see above). **Pure: ported**
(2026-10-26): Pure's hull takes its circuit's lights
([`psp-pure-usa/scene-light.md`](../psp-pure-usa/scene-light.md)); whether any Pure light sits
under an `Anim Transform` was not censused. **HD: not applicable**: its hull is lit by
its authored envsettings rig (`ship-sun-occlusion.md`) and the `wopoint` class is absorbed with no
consumer (`../ps3-hdfury-eu/vex-classes.md`). **2048 and Omega: not checked**, out of scope for a
Pulse finding.

## A craft no tick has located (2026-10-26, `talons-alongside`)

A posed capture (`--pose-from --ticks 0`, every compare pair) and a race's first frame run no tick, so
`standing.course_index` was `None` and `Race::ship_hull_scale` / `ship_hull_points` fell to the default
`255`: the hull drew at full light wherever it stood. On `pulse-psp-race/talons-alongside` (inside the tunnel,
spline bytes `127`) that is the whole of "ours is too bright" on the craft. The original's recorded GE list
(`psp-ge-dump.py dump`, light state read from the command words: ambient `0x59`, `LDC0..3` `0x90 + 3n`)
holds the first three directionals at `(122,114,75)`, `(38,47,57)`, `(23,28,35)`; the open road's
rig, read off another craft in the same list, is `(254,237,156)`, `(79,97,119)`, `(48,59,72)`: a scale of
about `123/255`. Ours now locates the craft where it stands (`Course::locate(position, None)`, render-side
and read-only, the standing's own fix is simulation state and stays as the tick left it) and reads the same
blend a running race reads. Hull-only boxes, ours over the original's mean luma: fin 1.92 to 1.05, right
fin 1.86 to 1.03, body 1.95 to 1.04, left fin 1.43 to 1.13, whole hull box with the plume 1.31 to 1.05;
nozzle box 1.20 to 1.05; the scenery boxes were 0.93-1.01 before and are unchanged. Test:
`hull_light_scale_ground_truth::a_craft_no_tick_has_located_is_lit_by_the_tunnel_it_stands_in` (fails
with the fallback removed). Other titles: HD's hull light is its own rig (`ship-sun-occlusion.md`) and
Pure authors no hull rig (checked, differs); the 19 HD, HD race, Omega and 2048 compare pairs re-rendered from their recorded flags are
unchanged to the printed digit, so nothing of theirs moved; Pulse PS2 takes none of this rig (not wired, see above).

## The start laser is lit by the rig too (2026-11-19, `pulse-basilico`)

Basilico's grid in PPSSPP v1.20.4 (`UCUS98712`, `data/saves/pulse-psp/time-trial-basilico-black-venom-grid.ppst`
run on to the pair's race clock 15.4487 s, then `psp-ge-dump.py dump`; two boots, one dump each).
The laser strip (`startline_laser_ADD_GLOW`, 64 x 8 `CLUT8`, 23 strip vertices, vertex type `0x121`: 8-bit UV,
8-bit normal, 16-bit position, **no vertex colour**) is drawn with `LIGHTING` on, after a lighting list is replayed
immediately before the draw (command chunk 4482, the strip at 4492; both boots):

```text
LIGHTING=1  AMBIENT=(0x90,0x7c,0x7a)  material ambient/diffuse white, alpha 0xff
LIGHT0=1  L=(-0.300, 0.550, 0.779)  LDC0=(0x32,0x2e,0x23)
LIGHT1=1  L=(-0.272, 0.337, 0.901)  LDC1=(0xe2,0xc2,0xb7)
```

That is `16_Track`'s recipe on `01_Track`: the circuit's own `AmbientLight` and two `DirectionalLight`s at scale
`255 >> 8` (`HullLights::from_track` prints ambient bytes `145,125,123` before the scale, `144,124,122` after).
The strip's normal is model +Z under a uniform-scale world matrix, so `N . L` is 0.779 and 0.901 and the sum
`clamp(ambient + sum(diffuse * N.L))` saturates to `(255, 255, 255)` in every channel: the strip's colour is its texel,
and with blend `0xa2` (`SrcAlpha` x `0xaf` / `Fix` 1) it adds 0.686 x texel. The pulse strip (`startline_laserpulse_ADD_GLOW`, chunk 4502, also `0x121`) draws under the same list.
Confidence **85** (both boots agree on every light word, the sum is computed from them; the model whose list this is was
not named in the executable, only seen replayed at the draw).

**What ours did.** The track pass bound the hull rig off (`Light::without_hull`) and drew the colourless laser vertices
through the stand-in (`0.15 + 0.75 key + 0.25 fill`, times the grey a colourless vertex carries): about 0.56 of the
texel against the original's 1.0. The core read (132, 151, 171) at x 190, rows 149 to 152 against the original's
(231..255), spread over the same four rows, which is what the 1.5 times figure and the "two sharp rows against four" in
`docs/tools/compare-page.md` ("basilico-black") were: with the core saturated after the bloom, ours' four rows are 255 and the original's
are 231, 255, 255, 246. The geometry was never the difference: ours' strip is the same 23 vertices at the same positions (bounds
`(-731.57, 2.134, 299.00)` to `(-691.53, 2.732, 299.26)`), UV `u` 49/128 to 78/128, `v` 0 to 1, and the texture's rows are the
same (rows 4 and 5 white, 3 and 6 about 0.45, 2 and 7 about 0.18, read from the dump). Why Talon's matched: its strip carries 4444
vertex colours (`ffed` to `ffa7`), which the GE reads as already lit, so no rig reaches it.

**Fix.** `mesh_render::mark_rig_vertices` marks the colourless vertices of the two textures `lit` 2.0
(`HullLights::TRACK_LIT`); the track pass binds the rig with `w` 2.0 (`Light::for_track`), which `mesh.wesl` reads as "only
a vertex marked 2.0 takes it"; the craft's scene keeps `w` 1.0 and every lit vertex. Loader line `start laser: N colourless
vertices lit by the circuit's rig` (56 on the PSP Basilico, 0 on PS2 and Pure). Test:
`crates/raceplay/tests/start_laser_rig_ground_truth.rs`. Pair `pulse-psp/basilico-black` 5.08 to 4.38 and `basilico-white`
4.30 to 3.50; the laser band (rows 140 to 160) mean absolute error 18.6 to 9.0; no other of the 29 Pulse PSP race pairs
moves by more than 0.01 (`docs/tools/compare-page.md`).

PS2: **checked, differs.** The PS2 disc's laser vertices carry colours, so nothing is marked and `pulse-ps2/basilico-black` is
byte-identical (14.76). Pure: marks nothing (its texture names are not these); not measured.

## Which models take the rig

Read 2026-11-20 (`pulse-lit-scenery`); the static half in Ghidra, the live half in PPSSPP v1.20.4 (`UCUS98712`).

**The per-model rule (confidence 85).** A model has a light list at `model + 0xb0` that `Vex_UpdateLightLists` (`0x08912358`)
rebuilds, and the list is only ever rebuilt if two things hold:

1. `model + 0xac` is non-null. `Vex_LoadModel` (`0x08912b80`) sets it to the nearest ancestor on the parent chain
   (`model + 8`, then each node's `+ 8`) whose class token at `node + 4` is the `FUN_08a6bd6c` token, the scene-light context
   (`0x08913048`-`0x08912ff4`; 0 when no ancestor has it).
2. The model's node is visited. `Vex_UpdateLightLists` is slot `+0x34` of the model method table `0x08ad17a4` (its one byte
   reference is `0x08ad17d8`), and the only dispatcher is `SceneNode_SubmitTree` (`0x08944160`, `jalr` at `0x089441b0`):
   a node whose flag word `+0x2c` has bit `0x8` set is skipped, along with its children.

A model that fails either keeps `RET` as its list, which is the "97 of 113" above: hidden or pooled models (weapon and pickup
bodies not in play) and models parented under no context. `SceneLight_Rebuild` (`0x08879b14`) writes the list once, then
again on alternate frames while the context has lights (`DAT_08ac2280 & 1` against the list's own parity byte).
Live: a breakpoint on `0x08912358` in Basilico's grid state fires for exactly **8 models** per frame, in the same order each
frame, all from `ra = 0x089441b0`: `01_Track\start_grid.vex`, `01_Track\track.vex`, two `Assegai\ship.vex` and four
`Pulse_Adverts` billboards; the track and ship models share context `0x08f29ff0`'s heap address and each billboard has
its own.

**Which batches replay the list** is `Mesh_SetBatchLighting`'s branch above: normals present and no vertex colour. So the
selector is per model and the use is per batch, never per texture: every colourless batch with normals in `track.vex` and
`start_grid.vex` draws under the circuit's rig.

**Census (confidence 80).** One GE dump per circuit's Time Trial grid state (`psp-ge-dump.py dump` then `census`, the
`lighting` and `light0..3` columns), vertex type with normals and no colour (`vt & 0x60`, no `vt & 0x1c`). Basilico,
Amphiseum (black), De Konstruct, Outpost 7 (black and white) and Talon's Junction, one boot each, and Amphiseum, De Konstruct
and Outpost 7 (white) a second time with identical (vertex type, vertex count, lighting, light-enable) tuples:

| Circuit | Lit-capable draws | `lighting` 1, the circuit's lights | `lighting` 0 |
| --- | --- | --- | --- |
| Basilico | 20 | 14 (12 `0x121`, 2 `0x123`), 2 lights | 6: the hull's extra pass (one 39-vertex and five draws of 313 vertices) |
| De Konstruct | 25 | 19, 2 or 3 lights | the same 6 |
| Outpost 7 black | 17 | 11, 2 lights | the same 6 |
| Outpost 7 white | 23 | 14, 2 lights | the same 6 and 3 more: a 317-vertex model drawn in model space (box about 6 x 2 x 14, the world matrix near identity), three textures, no light enabled |
| Amphiseum | 20 | 14, 4 lights | the same 6 |
| Talon's Junction | 17 | 11, 3 lights | the same 6 |

So in every dumped frame the lit-capable draws not under the rig are the craft's extra pass, which is drawn unlit by
design ([mesh-draw.md](mesh-draw.md), "The extra pass's colour is the light the hull stands in"). Basilico's second lit
strip (`vt 0x123`, 255 vertices, chunks 2730 and 2778) is in the 14. The 317-vertex model on Outpost 7 white is the one
draw of a lit-capable batch with no light enabled that is not the extra pass; what it is (a craft drawn in another pass) is
not identified, and it is one boot. Raw dumps and the lists stay under `data/`, not committed.

**Wired.** The no-normals rule (unlit, white material) is the one [`psp-pure-usa/hull-shine.md`](../psp-pure-usa/hull-shine.md), "The sky", measured on Pure's sky cube in a recorded GE list; `oag_mesh::mesh::build` now applies it to every batch, so Pure's sky builds unlit at the source (`unlit_sky` still converts a sky vertex that has normals and no colour). `mesh_render::mark_rig_vertices` marks every vertex with normals and no colour (`GpuVertex::lit` 1.0) as `lit`
2.0, the rig; `oag_mesh` builds a vertex of a batch with no normals as `lit` 0.0 and white, since the GE draws it unlit
(raw texel). The start-laser texture list is gone. Test:
`crates/raceplay/tests/start_laser_rig_ground_truth.rs` (no `lit` 1.0 left on Basilico, the laser strip marked, coloured
batches untouched, the flash sprites held back on `05_Track`).

**Held back, chosen not measured: the animated flash sprites** (`cameraflash_glow_add`, `07_camflash_Glow_Add`; colourless,
with normals, under `Anim Transform` nodes, 6 to 9 vertices each). Marked with the rest they moved `de-konstruct-black`
3.39 to 3.46 and `outpost-7-black` 3.30 to 3.34 and nothing else; bisecting by texture names these two alone (a single
texture's marking reproduces the full +0.065 and +0.037, and every other lit texture is below 0.0005). No dump has a flash
in frame (they blink), so the state the original draws them in is unread; they keep the stand-in.

**Pairs (`pulse-psp` and `pulse-psp-race`, the 27 race pairs, mean grey against the original; control re-render of
`de-konstruct-black` with the unchanged tree reproduced the stored 3.395 exactly):**

| Pair | Before | After |
| --- | --- | --- |
| `amphiseum-black` | 3.925 | 3.359 |
| `outpost-7-white` | 3.792 | 3.621 |
| `talons-boost` | 9.126 | 9.000 |
| `basilico-black` | 4.380 | 4.344 |
| `de-konstruct-black` | 3.395 | 3.395 |
| `outpost-7-black` | 3.297 | 3.299 |
| `talons-white` | 3.946 | 3.957 |
| `fort-gale-black` / `fort-gale-white` | 19.641 / 14.408 | 19.648 / 14.411 |
| `talons-black`, `talons-alongside` | 3.217, 2.354 | 3.218, 2.356 |
| the other 16 | | unchanged to 0.001 |

Amphiseum's side image (rail, stands and the start gantry) matches the original's lit scenery closely; the loss on
`talons-white` (0.011) and the two Fort Gale pairs (0.007, 0.003) is under the noise a one-tick difference makes and is not
explained.

**Not done: the raw texel for the 97 `RET` models.** Those models are not in the circuit's own `.vex`; no pair moment has one
in view, so their raw-texel draw stays unmeasured. **PS2: checked, differs** (every colourless-with-normals vertex is
a PSP GE case; the PS2 pairs re-rendered byte-for-byte, mean grey unchanged on all 20). **Pure: checked, not wired.**
`mark_rig_vertices` is called only by the Pulse PSP loader; Pure's `Vex_UpdateLightLists` twin was not located, so its track
keeps the stand-in until its own dump shows its colourless batches lit.

## Applied names

Mirrored in [`names.tsv`](names.tsv).

| Address | Kind | Name | Conf |
| --- | --- | --- | --- |
| `0x0887a4f0` | function | `SceneLight_BuildLightingList` | 88 |
| `0x0883e444` | function | `Craft_ApplyHullLightScale` | 85 |
| `0x0887c11c` | function | `Spline_AccumulateLightScale` | 85 |
| `0x0887c7e8` | function | `Spline_SampleSegment` | 75 |
| `0x0892cb64` | function | `WoPoint_GetStrength` | 90 |
| `0x0892cc50` | function | `WoPoint_GetVectorFromLight` | 90 |
| `0x0892cc98` | function | `WoPoint_GetDistanceBeyondRadius` | 80 |
| `0x0892d410` | function | `WoSpot_GetStrength` | 85 |
| `0x0892d640` | function | `WoSpot_GetVectorFromLight` | 90 |
| `0x0892d688` | function | `WoSpot_GetDistanceBeyondRadius` | 80 |
| `0x0892ca04` | function | `WoPoint_Init` | 85 |
| `0x0892cd00` | function | `WoPoint_Update` | 85 |

`SceneLight_BuildLightingList` loses its `_q`: the list it writes is matched
to the disc field by field. Its return value is read too (2026-11-10): the
float sum of every light's colour times its scale byte, truncated and clamped,
which `SceneLight_Rebuild` makes the material colour of the hull's extra pass.
Law, evidence and the Metropia numbers: [mesh-draw.md](mesh-draw.md), "The
extra pass's colour is the light the hull stands in".

## Open

- **What the other 97 lit-branch models get.** Their lists stayed `RET`, so
  the GE runs them with lighting in whatever state the batch set left it:
  `FUN_089307b4` opens with `Gu_Disable(10)`, so unlit, white material, the
  raw texel. The selector is now read ("Which models take the rig"): not submitted,
  or no context ancestor. Which hidden models they are was not listed.
- The point and spot slots are reproduced for a craft's own position (see above);
  the AI craft's lists and the PS2 executable's own were not read.
- `WoSpot_Init` (the `wospot` twin of `WoPoint_Init`) was not read; no shipped circuit
  authors a `wospot` under an `Anim Transform`, so nothing depends on it.
