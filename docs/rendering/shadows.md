# Shadows: a ladder of four techniques, and only one of them is the original's

> **Design, and now mostly status.** Steps 1-6 of [the plan](#the-plan-in-order)
> have landed: the census, the setting (`graphics.shadows`), the `blob` tier,
> the occluder's record arrays and its runtime reader, the 2048 measurement,
> and - on **Pulse only** - `original`. **`mapped` (step 7) is still design
> **All four tiers are built.** `original` draws on Pulse and on HD/Fury by
> each title's own mechanism - authored occluder hulls there, a coverage map
> here - and on 2048 or Pure it draws nothing and the load report says so.
> `mapped` is this project's own and works on every title, with one named gap
> in step 7 below.
> The page was written before any of it, so the setting's shape was agreed
> before an enum and a menu row made it expensive to change, the way
> [`motion-blur.md`](motion-blur.md) did for its own row.

Three things settled this page's shape, and all three were measured against the
discs rather than assumed:

1. **Pulse's shadows are authored geometry, and the payload's layout is now
   decoded** - see [What the Pulse disc actually authors](#what-the-pulse-disc-actually-authors)
   below. It is *not* a blob sprite and it is *not* a `Mesh` payload.
2. **`shadow` `0x3cb`, `blob` `0x3e0` and `textureBlob` `0x3df` are authored
   zero times** across all 415 `.vex` files on the Pulse PSP disc, so three of
   the four classes the roadmap lists under shadow are inert - the same kind of
   converged negative `PointLight` already is. **"Inert" is about the data, not
   the code, and for `shadow` `0x3cb` the distinction turned out to matter**:
   its registration is what installs the direction every shadow projects along,
   and a live instance would override that direction - see
   [the direction](#where-the-direction-comes-from-and-it-is-authored).
3. **Only Wipeout HD has a usable light rig.** That single fact is what stops
   the ladder being one quality dial, and it is the answer to "we'd need proper
   light sources": yes, and today exactly one title has one.

## Why this is not a Low/Medium/High dial

[ADR-0013](../architecture/adr/0013-anti-aliasing-architecture.md) opens with
"**Not one dial**" because MSAA, a spatial post pass and a temporal one act at
three different points in the frame. Shadows are the same case and worse: a
blob quad, a projected occluder hull and a shadow map act at different points,
compose differently, and **the ladder is not monotonic in quality**. A flat
authored hull projected onto the road can read *worse* than a soft circle on a
track whose floor curves under the craft.

So this follows
[`Reconstruction`](../../crates/display/src/display/reconstruction.rs) - variants
named for the technique - and not
[`MotionBlur`](../../crates/display/src/display/motion_blur.rs), whose "strength,
not technique" framing works only because every one of its tiers is one shutter
fraction of one tick.

| Setting | What casts | What the shape comes from | What receives |
| --- | --- | --- | --- |
| `off` | nothing | - | - |
| `blob` | each craft | HD: the disc's own `ambient_shadow.gtf`; 2048 and Omega: theirs for the Zone craft. Elsewhere: a generated radial falloff, **as a substitute for a missing asset only** | the track ribbon under the craft |
| `original` | what the title itself authors | Pulse: `Dynamic Shadow Occluder` `0x3c3` hulls. HD: the disc's four shadow jobs and its `ShadowToAlpha`/`ShadowMap` material flags. Pure: **nothing, and it says so** | per title |
| `mapped` | every mesh in the scene | a depth map rendered from the light | every mesh in the scene |

**`mapped` is a change of *scope*, not a promotion over `original`.** On a title
whose own mechanism is already a shadow map - which is HD, and possibly 2048 -
it does not render shadows "better"; it renders *more of them*, dynamically,
and in doing so **discards art the disc authored**. See
[What `mapped` actually changes, per title](#what-mapped-actually-changes-per-title).

`off` is the default until `original` exists, and then `original` becomes the
default for the titles that have one. It stays the comparison setting either
way - the same role `MotionBlur::Off` plays.

**There is no `raytraced` variant**, and adding one now would break ADR-0013's
own rule that "a row for infrastructure that is not there is worse than no
row." See [Pushing it further](#pushing-it-further-and-what-it-is-actually-blocked-on).

## What the Pulse disc actually authors

Measured across all 382 version-6 `.vex` files in every `.wad` on
`pulse-psp-usa.chd`, and asserted by
[`shadow_occluder_ground_truth.rs`](../../crates/vex/tests/shadow_occluder_ground_truth.rs).

| Class | Count | Note |
| --- | --- | --- |
| `Dynamic Shadow Occluder` `0x3c3` | **129**, in 83 files | the real mechanism |
| `shadow` `0x3cb` | **0** | none authored - but its *registration* sets the shadow direction, below |
| `blob` `0x3e0` | **0** | inert |
| `textureBlob` `0x3df` | **0** | inert |
| `Dynamic Point Light` `0x3c2` | **0** | inert |
| `lensflare` `0x3de` | **0** | inert |
| `DirectionalLight` `0x131` | 142 | already a converged negative; see the roadmap |

Wipeout Pure authors **none of the seven**, across all 222 of its `.vex` files
on `pure-psp-eu.chd`. Its shadow tier is honest absence, stated in the loader
report and not substituted for.

**That zero took a version filter to earn, and the first attempt at it was a
false negative.** Pulse is version 6 throughout; Pure is version 4 with 15
version-3 files ([`pure-status.md`](../formats/pure-status.md)), so a sweep
hard-coded to version 6 walks **zero** Pure files and reports zero occluders -
which reads exactly like the finding. Searching Pure for *Pulse's* class ids is
sound because the exporter's 22 class names are one contiguous run in `BOOT.BIN`
that is identical string for string and index for index on both titles, with
ids running consecutively along it. The test pins the file count per disc for
this reason: a walk that covers less now fails on the count rather than
returning a plausible zero.

### The occluder payload closes at `0x50 + 32n + 16m`

`n` is a `u16` at `+0x00`, `m` a `u16` at `+0x02`, and that expression equals
the payload length on **129 of 129** nodes, across at least twelve distinct
`(n, m)` pairs and payload lengths from 272 to 4432 bytes. A wrong stride does
not fit twelve independent pairs, which is the same closure argument that
settled `WO Track`, `section` and the skycube.

The `0x50` header, as far as it is read:

| Offset | Content | Evidence |
| --- | --- | --- |
| `+0x00` | `u16` n | closure |
| `+0x02` | `u16` m | closure |
| `+0x04`, `+0x08` | stale PSP main-RAM pointers, some zeroed | values like `0x09fca65c` sit in PSP RAM; the skycube payload has the same kind of baked pointer |
| `+0x0c`..`+0x24` | bounding box, min then max, packed 3+3 floats | a real box on 129/129 |
| `+0x24` | `u32` `5` | **129/129** |
| `+0x28` | `f32` `1.0` | **129/129** |
| `+0x30`..`+0x50` | leans shadow-relevant: `min.y` is floored toward the ground plane on most (not all) nodes above it, `max.y` patched with the true value on some (not most) nodes where the packed box is invalid | 97/129 match within `1e-6` tolerance, which hides some hard-zero substitutions as "matches" against the `~1e-38` denormal sentinel; of the rest, 12/14 positive-`min.y` nodes are floored and 16/54 denormal-`max.y` nodes are patched - neither exceptionless nor explained - [`shadow-occluder.md`](../ghidra/functions/psp-pulse-usa/shadow-occluder.md) |
| `+0x50` | `n` records of 32 bytes, **each beginning with a unit vector** | 129/129 |
| `+0x50 + 32n` | `m` records of 16 bytes: `(w, x, y, z)` with `w` first, `w == 1.0` on every record on both discs | 129/129 |

### Both record arrays are decoded: `n` planes and `m` vertices

- A **32-byte face record** opens with a unit plane normal (3 x `f32`, on
  129/129), carries a `u32` at `+0x0c` giving that face's vertex count - 3 or
  4, never anything else - and then the two `u16[4]` arrays below.
- A **16-byte vertex record** is `(w, x, y, z)` with **`w` first**, and
  `w == 1.0` on every record on both discs. A homogeneous point.
- **`m` counts slots, not vertices**: 150 slots across the Pulse disc sit at
  the origin, unused.

`pulse_mine` decodes to a **tetrahedron** - four unit normals, one straight
down and three up-and-outward at 120 degrees, over four vertices whose
component-wise extent is *exactly* the declared bounding box. `pulse_bomb` is a
**pentagonal frustum**: five vertices at `y = 2.6341` over five at
`y = 2.1059`, plus two spare slots. Both are convex hulls of a weapon pickup,
which is what the class name promised.

### The 16 bytes at `+0x10` are the edge graph, and it closes on itself

The last unread field, read 2026-09-04, and the one drawing an occluder was
waiting on. A face record's tail is two four-slot `u16` arrays:

| Offset | Content |
| --- | --- |
| `+0x10` | `u16[4]`, the face across each edge - `0xffff` where there is none |
| `+0x18` | `u16[4]`, vertex indices, the fourth repeating the first on a triangle |

Edge `s` runs from vertex slot `s` to slot `s + 1`, wrapping, so the loop is
closed whatever the count and the runtime can walk four edges unconditionally.
Measured across all 129 nodes and 4,381 faces on `pulse-psp-usa.chd`, and
asserted by
[`shadow_occluder_ground_truth.rs`](../../crates/vex/tests/shadow_occluder_ground_truth.rs)'s
`the_face_records_index_the_vertex_array_and_each_other`:

- **Adjacency is reciprocal on 14,328 of 14,328 edges.** The face named across
  an edge owns that same edge. This is the closure argument: a wrong stride or
  a wrong offset does not produce a consistent edge graph on fourteen thousand
  edges.
- **Every vertex index is in range**, 4,381 of 4,381 faces; every count is 3 or
  4 (3,184 triangles, 1,197 quads) and nothing else.
- **Every `0xffff` edge is owned by no other face**, 4,381 of 4,381. On 4,377 of
  them the only such edge is a triangle's degenerate fourth (`v0` to `v0`); the
  other four faces are two flat two-face hulls (`Data.wad#242`, `#244`) with
  twelve genuine boundary edges between them. One rule, exercised twice.
- **Every triangle is coplanar with its own declared plane**, 3,184 of 3,184,
  and its declared normal agrees with the geometry of the three vertices it
  indexes to within **0.028 degrees** - two quantities stored in different parts
  of the record, agreeing.

**Quads are not planar, and that is authored rather than a decode error.** 300
of the 1,197 spread further than `1e-4` of their hull's own scale from their
declared plane, the worst at `5.3e-2`: bilinear quads out of an exporter. A
sliver quad's first three vertices can even describe a normal 180 degrees from
the declared one, which is exactly why the normal check above is stated over
triangles - three points always describe a plane, four authored ones need not.
The runtime tests against the record's own declared normal, so none of this
costs it anything.

**Two faces of 4,381 are odd and are carried rather than rejected**:
`Data.wad#840` `shadowShape` face 13 of 110 winds against its own normal, and
`Data.wad#744` `shadowShape` face 9 of 18 has no area. Refusing them would
refuse two whole hulls over two faces, and only a caller building a volume can
decide what to do with a reversed face - `oag_vex::shadow_occluder`'s
`Face::winding` is how it asks.

The parser is [`oag_vex::shadow_occluder`](../../crates/vex/src/shadow_occluder.rs),
whose `Occluder::silhouette` is the edge walk this layout exists for: an edge is
on the silhouette when exactly one of the two faces meeting there faces the
projection direction, which the `+0x10` array answers in one lookup instead of a
search.

**The box is authored rather than derived**, so the disc-wide assertion is
containment and not equality. The counter-example is instructive:
`BEData.wad#20` is a *flat* hull with all eight vertices at `y = -0.06195458`
that declares its `y` maximum as the denormal `0x00800000` - a value no
extent computation would produce.

Confidence: **layout 88** (exact closure, 129/129, twelve independent
`(n, m)` pairs). **Interpretation now 92**, up from 85 on 2026-09-04 when the
index arrays were read: the reciprocal edge graph closes over 14,328 edges and
a triangle's two independently stored descriptions of its own plane agree to
0.028 degrees. Two things keep it out of the 95-100 band - no runtime trace of
`Shadow_RenderOccluderVolume` exists, and the second title's six hulls re-prove
the *payload* closure rather than this indexing.

### Where the direction comes from, and it is authored

**Read 2026-09-04**, and it is the last thing `original` on Pulse was blocked
on: a shadow cannot be drawn without a direction, and picking one would have
been an invention.

`Shadow_RenderOccluderVolume` projects along `(1, -10, 2)` normalized -
`(+0.09758954, -0.97589540, +0.19517908)`, 12.60 degrees off straight down.
The ratios are what settle it: `z / x` is *bit-exactly* `2.0` and `y / x` is
`-10.0000003`. The vector is `4.8e-6` short of unit, uniformly, which is one
normalize done with a fast reciprocal square root rather than an exact one -
so `oag_pulse::shadow::AUTHORED_AXIS` carries the shipped bits rather than a
recomputed exact vector. It is a **local** axis: the node's own world matrix
carries it into world space, so a craft's shadow direction tilts with the
craft.

The constant lives in `.bss` at `0x08b62540` (`g_shadow_direction`) and is
written by `Shadow_RegisterClass` (`0x08923518`) from four immediates, in the
same function that registers class **`0x3cb`** - `shadow`. A second address,
`g_shadow_direction_override`, is selected instead while any `shadow` node is
alive, and since the disc authors none, the constant is what every shipped
scene uses.

**Why it read as unfindable before**: both addresses are reached through PRX
relocations with `addr_base = 1`, so the loader adds segment 1's base and the
obvious reading of the `lui`/`addiu` pair lands inside `.text` instead - the
same trap `shield-pickup.md` records. Full instruction-level evidence,
including the relocation entries, is on
[`shadow-occluder.md`](../ghidra/functions/psp-pulse-usa/shadow-occluder.md#the-projection-direction-is-normalize05--5-1).

Confidence 88 on the value, 82 on the override.

### Two populations, and they are not the same feature

| | Count | Box centre | Node names |
| --- | --- | --- | --- |
| local-space | **119** | at the origin | `shadowShape`, `shadow_lodShape`, `shadow_agsShape`, `shadow_mineShape`, `shadow1Shape`, `shadowlodShape` |
| world-space | **10** | hundreds of units out | unnamed |

The named 119 are per-object and carry a LOD variant, and `shadow_agsShape`
names a team (AG Systems) while `shadow_mineShape` names a weapon - so these are
**a craft's and a pickup's own shadow hull**. The unnamed 10 sit at track
coordinates with extents up to 882 units: **track-side occluders**, a different
job. A tier that draws one is not drawing the other, and the plan below builds
the 119 first because that is the shadow a player looks at.

## What Wipeout HD authors, which is a real shadow-map pipeline

HD is the best-evidenced title and the only one with a light to cast from. All
of this is already read and recorded:

- A sun direction, a sun colour and an ambient constant in the `.envsettings`
  beside `track.vex`, **already decoded and already drawn through**
  ([`envsettings.md`](../formats/envsettings.md)).
- Four distinct render jobs, and **their names bound the feature**:
  `RenderModelShadowMaps` and `RenderSpotShadowMaps` generate, then
  `RenderModelShadowsOnTrack` and `RenderModelAmbientShadowsOnTrack`
  composite - *onto the track*. They sit at positions 4, 5, 9 and 10 of the
  thirteen-job frame order ([`rcsmaterial.md`](../formats/rcsmaterial.md),
  "The frame's pass order"). **This page used to say "the receiver is the
  road, not the scene" here, and that was wrong by half - read 2026-09-21**:
  `Job RenderShips` itself opens with a second per-ship map, the track
  within ten units of the craft drawn from the sun through its own
  `SunOcclusionLightmap`/`SunOcclusionVertex` technique, and the hull's
  `ShadowMap` variant gates its sun by that map times its own depth map. The
  craft is a receiver of the road's baked shadow - see
  [`ship-sun-occlusion.md`](../ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md)
  and [the hull section below](#hds-hull-is-a-receiver-too-of-the-roads-own-mask).
- Material flags `ShadowToAlpha`, `ShadowMap`, `Spot0`..`Spot3`, with
  `ShadowToAlpha` bound to the shadow compositing pass at `0x405d48`.
- Sampler names `shadowMapTex` `0x730df9ee` and
  `directionalLight0ShadowTex` `0x9becc725`; `shadowMatrix` and
  `paraboloidReflectionTex` are named in the engine's parameter table
  ([`rpcs3-capture.md`](../reverse-engineering/rpcs3-capture.md)).
- The lightmap's **alpha is a baked shadow mask**, and the renderer already
  reads it ([ADR-0026](../architecture/adr/0026-hd-authored-lighting-is-linear.md)).
- Nine `ambient_shadow.gtf`, 128x64, one per team, **fully decoded, and now
  drawn by the `blob` tier below**
  ([`gtf.md`](../formats/gtf.md#the-9-b8-files-are-ship-shadows-and-their-own-remap-broadcasts-them)).

Those nine are the reason the `blob` tier is not an invention on HD: the disc
ships the craft's soft silhouette as a texture, so `blob` **plays the disc's
own image** and falls back to a generated circle only for a title that has no
such asset. **With one qualifier, read 2026-09-11**: HD/Fury itself never
draws them - `Job RenderModelAmbientShadowsOnTrack`'s run function is empty
and the compiler that would bind `ambientShadowTex` is unreferenced dead
code ([`shadow-model-maps.md`](../ghidra/functions/ps3-hdfury-eu/shadow-model-maps.md)).
The asset is the disc's; the decision to draw it is this project's. That is the narrow case the never-invent rule allows - a substitute
for the missing asset alone, never an override of data we do have.

### Which craft have a disc silhouette, per title (census, 2026-10-06)

Searched every archive, patch and DLC pack each title holds, by name, for
`ambient_shadow` and `shadow` in a texture directory, and the executables for a
path that names one. A silhouette is found **beside the race's hull model**
(`shadow::silhouette_entry`), so it follows whatever hull a mode flies.

| Title | Silhouettes on the disc | Which craft | Wired |
| --- | --- | --- | --- |
| Pulse (PSP, PS2) | none: `blob` `0x3e0` and `textureBlob` `0x3df` authored 0 times in 415 `.vex` | none | labelled generated falloff, a stand-in |
| HD / Fury | nine `/data/ships/<team>/textures/ambient_shadow.gtf`, 128x64 `B8` | ag_systems, assegai, egx, feisar, goteki, piranha, qirex, triakis, zone | drawn; **Icaras and Auricom ship none** |
| 2048 | one: `hdships/Zone/Textures/Ambient_Shadow.gxt`, 128x64 | the Zone hull only | **drawn in Zone races** (every slot flies that hull) |
| Omega | one: `hdships/zone/Textures/Ambient_Shadow.gnf`, 128x64 | the Zone hull only | **drawn in Zone races**; `Zone_VR/Textures/Ambient_Shadow.gnf` (patch `data08`, 128x64) is VR's hull, VR is not planned |

- **2048 and Omega's native craft ship none** (2048's twenty `Ships\<team>2048\<n>`
  and twelve HD-derived teams, Omega's roster), in any archive, patch (v1.04,
  1.07) or DLC pack, under any `shadow`/`blob` name. Only environments
  (`ds_stone_shadows`), crowds, particles (`dark_blob`) and a rocket-trail
  material match, none of them a craft's. A non-Zone race on either title keeps
  the generated falloff for every slot, and `Scene::shadow_geometry`'s WARN
  fires, truthfully, when the player chooses the `blob` tier.
- **HD's Icaras and Auricom** ship no `ambient_shadow` in `DATA02`/`DATA03` or any
  other archive; the same WARN counts them.
- **No executable names the file.** HD, 2048 v1.04 and Omega (base and patch)
  carry no `ambient_shadow` path literal, and the models (`ship.vex`, 2048's
  `ship.rcsmodel`) do not name it either; HD's engine has the strings
  `AmbientShadow`, `ambientShadowTex`, `ambientShadowMatrix` and
  `ambientShadowBlendFactor`, 2048's `AmbientShadow`, Omega's none. So the
  path is the directory convention the nine HD files prove, not a recovered
  name; whether 2048 and Omega's own code ever binds theirs is **unread**.
  The disc's image is played regardless, as on HD.
- **Same image three times.** The decoded Zone silhouette matches HD's `zone`
  one in size, in the red channel `shadow.wgsl` reads, and in row order
  (`crates/game/tests/shadow_ground_truth.rs`,
  `zone_craft_blob_shadow_is_the_discs_own_on_2048_and_omega`): the row-sum
  profile is within a tenth of HD's straight and ten times that reversed, so
  Omega's `.gnf` needs **no** row flip, unlike its eight `bottom_up_gnf`
  front-end images. A mode-aware path also means an HD Zone race now takes
  `zone`'s silhouette rather than the player's team's, matching its hull.
- **Cross-check 2048 vs Omega: ported.** One reader (`assets::decode_texture`)
  and one lookup serve both; each title has its own disc-backed assertion in
  the test above. What differs is only the container (`.gxt` Vita swizzle,
  `.gnf` PS4) and the directory case (`Zone` against `zone`).

## What `mapped` actually changes, per title

The ladder is not monotonic, and this is where that bites hardest. Asked
whether `mapped` improves on HD, the honest answer is *it depends what you are
measuring*, and on one axis it is a regression.

| | Pulse | HD / Fury | 2048 |
| --- | --- | --- | --- |
| what `original` is | 129 authored occluders, mechanism unread | a real shadow-map pipeline, **models onto the track**, plus a stencil-volume path | a track-proximity projection plus precomputed per-circuit ship environment shadows |
| the light it casts from | **none** - no usable rig, see below | the sun in `.envsettings` | **its own `ShadowLight`**, stated apart from the sun |
| `mapped` adds | dynamic shadows where the disc has one hand-placed hull per object | ship-on-ship, ship-on-scenery, scenery-on-scenery; cascades; a resolution the PS3 could not afford | the same, plus shadows that are not tied to track proximity |
| `mapped` **costs** | little; there is no authored lighting to lose | **the baked shadows in the lightmap's alpha**, which are art, not an approximation of a real-time result | the precomputed environment shadows, which are likewise authored |

That middle-column cost is the reason `original` stays the default on HD even
once `mapped` exists. HD's static shadows are painted into the lightmap alpha
and the renderer already reads them
([ADR-0026](../architecture/adr/0026-hd-authored-lighting-is-linear.md)); a
cascade that replaces them replaces an artist's decision with a shadow-map
projection, and this project's whole premise is that the disc's answer wins
where the disc has one. `mapped` is therefore offered as *more shadows*, not as
*better shadows*, and whether it looks better on HD is a screenshot comparison
nobody has run.

### 2048 measured, 2026-09-02: its own runtime, and it is not HD's

**Measured, not inherited.** The string tables of all three executables were
compared directly - `data/extracted/vita/PCSF00007/patch-v104/eboot.elf`,
`data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf` and Pulse's
`BOOT.BIN` - and 2048's `.vex` corpus was swept the same way Pulse's was.

**2048 does not have HD's shadow-map pipeline.** Every one of HD's four job
names is absent, and so is every sampler it binds:

| Only HD | Only 2048 | Both |
| --- | --- | --- |
| `Job RenderModelShadowMaps`, `Job RenderSpotShadowMaps`, `Job RenderModelShadowsOnTrack`, `Job RenderModelAmbientShadowsOnTrack` | `track_proximity_shadow_vp` / `_fp` | `Shadow_Importer.cpp` / `.h` |
| `shadowMapTex`, `shadowMapTexSize`, `directionalLight0ShadowTex`, `textureSpot0..2ShadowTex`, `ambientShadowTex`, `ambientShadowMatrix`, `ambientShadowBlendFactor` | `shadowLightDirection`, `shadowLightSpecularDiffuse`, `shadowFactors`, `ShadowColour`, `shadowMap` | `shadowMatrix`, `ShadowSelect`, `AmbientShadow` |
| `ShadowToAlpha`, `ShadowMap`, `LiveStencilShadow_vp` / `_fp`, `/shadow.stencilvolume` | `Lighting.ShadowLight direction`, `Lighting.ShadowLight specular colour`, `Lighting.ShadowLight diffuse nova scale`, `Lighting.Ship shadow direction` | `Dynamic Shadow Occluder`, `Lighting.Debug.Draw Ship Shadows`, `RocketTrail_Shadow_triangle` |

What 2048 does instead, on the evidence of its own names:

- **A track-proximity projection.** `track_proximity_shadow_vp`/`_fp` is a
  shader pair, and the name says the shadow is placed by *proximity to the
  track* rather than by sampling a depth map. The **container** is now read -
  [`docs/formats/gxp.md`](../formats/gxp.md), 97,899 of 97,899 GXP programs
  decoding across the executable and all three packages - and it says two
  things this page could not say before. `track_proximity_shadow_fp` is one of
  `eboot.elf`'s 67 programs and **binds no parameter whose name contains
  "shadow"**, so whatever it draws it does not sample a named shadow texture.
  And the *track's* shadow term is a **single-channel sampler called
  `shadowMap` at texture unit 3**, bound on 1,799 fragment programs
  immediately beside `lightmap` (4,467, unit 1, four channels) and
  `occlusionMap` (1,895, unit 2) - so it is a one-channel texture sampled per
  pixel with the lightmap, not a depth map the renderer projects and compares.
  That is a mechanism, and it is inconsistent with HD's shadow-map pipeline in
  the same direction the string comparison above already pointed. **What the
  shader itself computes is still unread**: the USSE instruction stream is
  located exactly and not decoded, which is where the PS3 pair
  ([`scripts/ps3-microcode.py`](../../scripts/ps3-microcode.py), 37,461 of
  37,461 HD blocks) still goes further than the Vita one.
- **A second, separate kind**, precomputed per circuit: the memory budget line
  `**** PRECOMPUTED TRACK (SHIP ENV SHADOWS) (main uncache=%.2fkb, vram=%.2fkb) ****`
  reports its own cost, and `Lighting.Debug.Draw Ship Env Shadows` toggles it
  apart from `Lighting.Debug.Draw Ship Shadows`.
- **A light of its own for them.** `Lighting.ShadowLight direction`,
  `... specular colour`, `... diffuse nova scale` and `Lighting.Ship shadow
  direction` are a shadow light stated separately from the sun. **This is the
  one title that answers the light-source question outright** - it authors a
  direction specifically for ship shadows.
- Six `DirectionalNovaTextureNoShadow*` permutation names appear 507 times
  each, so its shader set is permuted on shadow presence.

### The occluder class survives the whole lineage, and 2048 proved the closure

The `.vex` sweep expected zero and **found six**, which is the more useful
answer. All six are weapons, shipped twice over
(`data/Weapons/` and `data/Weapons2048/`):

| File | Node | n | m | bytes |
| --- | --- | ---: | ---: | ---: |
| `pulse_bomb.vex` | `shadowShape` | 11 | 12 | 624 |
| `pulse_mine.vex` | `shadow_mineShape` | 4 | 4 | 272 |
| `pulse_shuriken.vex` | `shadowShape` | 8 | 8 | 464 |

**Every one closes at `0x50 + 32n + 16m`, carries the same `+0x24 == 5` and
`+0x28 == 1.0`, and opens every 32-byte record on a unit vector** - and those
three `(n, m, bytes)` triples are the same ones Pulse's `BEData.wad` carries
for the same three weapons. So the payload layout is a **two-title,
two-platform, two-endianness** result rather than a Pulse quirk, which lifts
the layout's confidence and is why the plan's step 4 is worth doing at all.

Two things follow for the setting:

- **2048's craft author no occluder.** Only weapons do. Its ship shadows come
  from the proximity/env system above, so `original` on 2048 is that system,
  not this class.
- **HD carries a stencil-volume path** - `LiveStencilShadow_vp`/`_fp` and
  `/shadow.stencilvolume` - beside its shadow maps. A stencil shadow volume is
  extruded from an occluder's silhouette **planes and edges**, which is exactly
  the shape `0x3c3`'s two record arrays have. **Confirmed on Pulse's own
  binary, 2026-09-03**: `Shadow_RenderOccluderVolume`
  (`0x089038c8`, [`shadow-occluder.md`](../ghidra/functions/psp-pulse-usa/shadow-occluder.md))
  reads exactly this payload and extrudes a silhouette-edge stencil volume
  from it - not a lineage argument any more, a runtime reader, at confidence
  84 (Probable: `DynamicShadowOccluder_RegisterClass` statically links class
  `0x3c3` to this function's own method table, byte for byte, but it is one
  binary's own internal consistency rather than cross-file or runtime
  corroboration).
- **Now traced on HD's own binary too, 2026-09-04**:
  [`shadow-stencilvolume.md`](../ghidra/functions/ps3-hdfury-eu/shadow-stencilvolume.md).
  The `LiveStencilShadow` technique is a real registered shader (three named
  constants: `worldViewProj`, `lightDirection`, `extrusionDistance` -
  `lightDirection` is new evidence that HD's version, unlike Pulse's, may take
  a light as input), and a per-model flag bit (`self+0xe4`, bit `0x2`) builds a
  path ending in the literal leaf `shadow.stencilvolume` (confidence 65 that
  the directory it joins into is fixed rather than per-model - see that
  page) and loads it through a hashed resource cache into an explicit
  vertex-buffer/index-buffer pair plus a computed bounding box - the same
  coarse shape as Pulse's `.vex` payload (leading counts, a geometry array, a
  bbox) but a genuinely different topology encoding (confidence 82 on the
  vertex/index split, closed by an exact arithmetic invariant between two
  independently-read allocator functions - see that page). **The draw call
  is found too**: a two-sided depth-fail stencil test over the extruded box
  volume (see below), colour-mask
  bracketed, its RSX register identities (stencil test/func/op, two-sided
  stencil, cull-face and colour-mask toggling) cross-checked against a local
  `rpcs3`'s own `gcm_enums.h` rather than assumed. **The record format is now
  byte-verified, 2026-09-04**: all 39 real `shadow.stencilvolume` files on
  the disc (one per `data/ships/<name>/`) decode with an identical header
  and reveal a fixed, unwelded six-face box (24 vertices, 108 indices - 12
  real face-quad triangles plus 24 degenerate, zero-area ones, checked
  across all 39 files), with only vertex positions varying per ship - the
  topology itself is byte-identical across every file (confidence 82 -> 92).
  This retracts an
  earlier reading of the min/max fields as a "bounding box": the offsets
  they actually reduce over are the vertex *normal*, not the position, per
  this closed layout. **The vertex shader is disassembled too, same day**:
  `LiveStencilShadow_vp` extrudes **only the vertices whose normal faces the
  light**: `DP3C` writes `dot(normal, lightDirection)` to the condition
  register and the `lightDirection * extrusionDistance` add is predicated
  `(GT.xxxx)` on it - a per-vertex silhouette extrusion of the unwelded box,
  the same construction as Pulse's runtime reader (confidence 84, disassembly
  with hash-confirmed parameter names). A first reading on 2026-09-04, made
  before `scripts/ps3-microcode.py` decoded vertex-program predication,
  called this a rigid shift of the whole box; retracted 2026-09-14 on
  `shadow-stencilvolume.md`. Checked directly against both draw functions:
  neither re-uploads a different `extrusionDistance` between the two stencil
  passes - one extruded volume serves both.
  See [`shadow-stencilvolume.md`](../ghidra/functions/ps3-hdfury-eu/shadow-stencilvolume.md)
  for the full numbers.

**What these string comparisons do and do not prove.** A name in a binary is
strong evidence the code path exists and near-conclusive that an absent one
does not, given all four of HD's job names and all seven of its shadow samplers
are missing together rather than singly. It is *not* evidence about what runs
in a race - dead code keeps its strings. Confidence: **80** that 2048 does not
use HD's shadow-map pipeline, **70** that it draws a track-proximity ship
shadow, **75** that it authors a dedicated shadow-light direction.

## Why 2048 reads better than HD on weaker hardware

Raised from play rather than from the disc, and worth answering here because it
decides whether `mapped` is a promotion or a downgrade on either title. The
short answer: **it is not that 2048 bakes more, and it is not the shadows
alone. 2048 bakes *uniformly and directionally*, and spends nothing on a
runtime light rig.**

**The obvious explanation is wrong.** Measured across every archive on both
titles:

| | lightmap entries | total |
| --- | ---: | ---: |
| Wipeout HD / Fury | 793 | **925.2 MiB** |
| Wipeout 2048 | 941 | **867.1 MiB** |

*(Counted on filenames ending `-lmap.gtf`/`-lmap.gxt`. A substring match on
`lmap` is wrong and inflates both sides - `NormalMap.gxt` contains it.)*

HD bakes *more* bytes than 2048 does. So "2048 prebakes and HD does not" is not
the difference - both bake heavily, and HD slightly harder.

**What differs is coverage and kind.**

- **2048's baked lighting is one system with a name, and it ships a full
  technique ladder.** Recovered verbatim from the executable's technique table:

  ```text
  Ambient  ShadowColour  DepthOnly  DirectionalNoGamma
  Directional                     {,VL1,VL2,VL4,VL6}
  DirectionalIBL                  {,VL1,VL2,VL4,VL6}
  DirectionalIBLOcclusion         {,VL1,VL2,VL4,VL6}
  DirectionalNovaVertex           {,VL1,VL2,VL4,VL6}
  DirectionalNovaTexture          {,VL1,VL2,VL4,VL6}
  DirectionalNovaTextureNoShadow  {,VL1,VL2,VL4,VL6}
  NovaShipOcclusion
  ```

  Two things fall straight out of the ladder's *shape*. **Shadowing is an axis
  of the lightmapped path only** - `DirectionalNovaTextureNoShadow` exists and
  there is no `DirectionalNovaVertexNoShadow`, so the shadow term rides with
  the lightmap, not with vertex colours. And **`ShadowColour` is a technique in
  its own right**, which is what a shadow-compositing pass looks like.
  2048 also carries image-based lighting (`DirectionalIBL`) and an occlusion
  variant of it, and four vertex-blend techniques (`wo.VBE.LocalTransform`,
  `StaticTransform`, `StaticQuake`, `Skinning`).
- **HD has no Nova at all** - zero occurrences in its executable - and its own
  coverage is uneven where 2048's is not. This project already measured HD's
  attribute census: of 983 chunks, **327 declare `lightmapUV` and no colour
  set, 351 the reverse, 0 both, and 305 neither**. Roughly a third of HD's
  geometry carries no baked lighting of any kind and falls back to flat
  ambient.
- **The bake is not richer per texel, and "Directional" does not mean a
  directional basis.** A Nova lightmap and an Ile lightmap for the same circuit
  are the *same texture*: 1024x1024 BC3 with a full mip chain, and the byte
  counts close exactly - HD's `ile_mesh_combine-lmap.gtf` declares **1,398,128**
  bytes against a computed 11-level chain of 1,398,128, and 2048's
  `nova_mesh_combine_track_1-lmap.gxt` declares **1,398,096** against a 9-level
  chain (stopping at 4x4) of 1,398,096. Same format, same resolution, same
  channel count. So `Directional` in `DirectionalNovaTexture` names the *light
  type* in the technique, not a radiosity-normal basis - an earlier reading of
  this page said otherwise and was wrong.
- **HD spends the difference at runtime, and 2048 does not have that spend at
  all.** `Lighting.Enable dynamic lights`, `Lighting.Enable spu vertex lights`,
  `Lighting.Debug.Draw light volumes`, `Lighting.Debug.Draw spu light volumes`,
  `Lighting.Debug.Stall for spu lights`, `Lighting.Spotlight colour scale` are
  all HD-only. 2048 has no dynamic light rig in its vocabulary whatsoever.

**And on the shared circuits 2048 bakes substantially *less*.** The thirteen
ported HD/Fury circuits make this a controlled comparison - same geometry, two
pipelines - and 2048 ships **0.65x** HD's lightmap bytes across the twelve that
pair up cleanly: 556 textures and 569.8 MiB against 759 and 880.5 MiB. Per
circuit it ranges from 1.30x (`Vineta_K`) down to **0.19x** (`tech_de_ra`) and
0.35x (`talons_junction`).

So the trade is the one a weaker machine should make, and it is sharper than
"bake more": 2048 gets its look from **fewer lightmap bytes over more uniform
coverage**, with a technique ladder that lights every surface class, and keeps
only a track-proximity ship shadow and a precomputed environment-shadow set at
runtime. HD throws more lightmap data at the problem, leaves roughly a third of
its geometry outside the bake entirely, and makes the difference up with a
real-time rig a PS3 has to afford every frame.

**Two caveats, because neither is measured here.** *Per pixel, 2048's hardware
is not obviously weaker*: it targets 30 Hz at 960x544 against HD's 60 Hz at
1280x720, which is roughly three and a half times the GPU time per pixel -
platform knowledge, not a finding from these discs. And art direction is not
controlled for at all. This section explains a mechanism the binaries do show;
it does not prove the mechanism is what a player notices.

**What this means for the setting.** `mapped` on 2048 would replace a uniform
directional bake plus authored shadows with a real-time cascade - almost
certainly a downgrade, and more clearly so than on HD. `original` stays the
default on both, and `mapped` is offered as a scope change on both, not a
quality tier. It also makes 2048, not HD, the title whose *lighting* is worth
copying if the enhanced-rendering flag ever backports one title's look onto
another.

## Ile and Nova: the DLC answers the migration question outright

**2048's DLC1 and DLC2 carry thirteen ported HD/Fury circuits, and every one of
them was re-baked.** That makes the ported content a controlled experiment -
same circuit, same geometry, two lighting systems - and the file names settle it
without a single byte being decoded.

| | lightmap file naming | material source flags |
| --- | --- | --- |
| HD / Fury | `ile_mesh_combine_*-lmap.gtf` | `IleLightmap`, `IleVertex` |
| 2048 | `nova_mesh_combine_*-lmap.gxt` | `DirectionalNovaTexture`, `DirectionalNovaVertex` |

**`Ile` and `Nova` are two generations of the same offline pipeline.** The
convention is otherwise identical - `<system>_mesh_combine_<part>-lmap.<ext>` -
and the two material flags map one to one: HD's `IleLightmap`/`IleVertex` is
2048's `DirectionalNovaTexture`/`DirectionalNovaVertex`, the same "baked into a
texture or baked into vertex colours" split this project already documented on
[`rcsmaterial.md`](../formats/rcsmaterial.md) without knowing what `Ile` named.

The migration was a replacement, not a port:

- **941 of 941** of 2048's lightmaps are `nova_`-named, the ported HD circuits
  included. **Not one HD lightmap survived** into 2048.
- **HD's executable contains no `nova` string; 2048's contains no `Ile`
  string.** The two systems do not coexist anywhere.
- Every ported circuit was baked **twice**, once per direction - `lmaps/` and
  `lmaps_reversed/` both present on all thirteen (for example `Sol_2` 16 and
  16, `Ubermall` 15 and 15, `modesto_heights` 35 and 35).
- What 2048 *did* keep from HD is asset-level, not lighting: `hdships/` liveries
  and normal maps, and the three weapon occluder hulls above, which are Pulse's
  rather than HD's.

**So the DLC did not inherit HD's look; it was relit.** That is the strongest
form of the previous section's argument, because the geometry is held constant:
whatever a player sees when a Wipeout HD circuit looks different in 2048 is the
lighting system, not the art.

Two things this does **not** establish. It does not say what Nova's basis
actually is - directional bakes range from a single dominant direction to a
full radiosity-normal basis, and only the `.gxt` payload plus the shader would
settle which. And it does not say the re-bake was an *improvement*: re-baking
was forced regardless, since the Vita cannot run HD's SPU light rig and the
texture format changed from `.gtf` to `.gxt` anyway. Confidence that Nova
replaced Ile wholesale: **90**. Confidence about what Nova computes: not
assessed, because nothing here read it.

## 2048 authors its shadow light per circuit, in plain text

`.EnvSettings` is a `"key"=value` text file, so this needs no decoding at all.
From `data/art/published/environments/altima/track.EnvSettings`:

```text
"Lighting.Sun direction"=-0.163048 0.557124 0.814265
"Lighting.Physical Sun direction"=-0.163048 0.300000 0.814265
"Lighting.ShadowLight direction"=-0.310000 0.450000 -0.850116
"Lighting.ShadowLight specular colour"=0.980000 0.910000 0.780000
"Lighting.ShadowLight diffuse nova scale"=0.300000
"Lighting.Ship shadow direction"=0.450000 0.600000 1.200000
"Lighting.Nova prelit scale bias power"=1.000000 0.000000 1.000000 0.000000
```

**Four independent direction vectors**, and on a native circuit all four
differ: the sun, a "physical" sun, a shadow light, and a ship-shadow direction.
The shadow direction is a deliberate artistic choice, not derived from the sun.
`Lighting.Ship shadow direction` is not unit length (1.415 here), so it carries
a magnitude as well as a direction.

**On a ported HD circuit it is a default.** `DLC1/environments/Anulpha_Pass`
sets `Ship shadow direction`, `Sun direction` and `Physical Sun direction` to
the *same* vector, `0.126447 0.982212 0.138820`. The ports got the key filled
in; the native circuits got it art-directed.

**The same circuit in HD has no shadow direction at all.** HD's
`/data/environments/15_anulpha_pass/track.envsettings` is 36 keys against
2048's 66 for the same circuit, and its `Lighting.*` block is about a runtime
rig - `Enable dynamic lights`, `Spotlight colour scale`, `Ambient false
direction`, `Prelit ambient colour scale`/`power`. There is no shadow key in
it, because HD's shadows come out of the shadow-map jobs rather than off an
authored vector. 2048 adds, among the extra thirty: a shadow light, twelve
numbered fog regions against HD's single global fog, and an exposure triple
(`ExposureScale`/`Max`/`Speed`).

**So the Pulse problem does not exist on 2048.** Its `original` tier has an
authored light direction per circuit, sitting in a text file this project can
already read - the one title where the shadow's direction requires neither an
invention nor a decode.

## Pushing it further, and what it is actually blocked on

The `mapped` tier - one cascaded shadow map, everything casts, everything
receives - is this project's own enhancement and not a recovery, so it belongs
behind the same opt-in that any backported effect does, and its doc must say so
the way `BoostFovKick` and `MotionBlur` already say it of themselves.

**Ray tracing is a documented future tier with no enum variant**, blocked on
three things in this order:

1. **A light to trace against.** Pulse has none: `DirectionalLight` is a
   converged negative with no reader and no `GU_LIGHT0..3` slot ever enabled,
   `AmbientLight`'s colour never reaches the render path, `PointLight` is never
   registered, and `Dynamic Point Light` is authored zero times. **HD and 2048
   both have one** - HD a sun in `.envsettings`, 2048 a `ShadowLight` with its
   own direction authored specifically for ship shadows - so those two are
   where a traced shadow has a defensible direction rather than an invented
   one, and Pulse is where it would have to be invented.
2. **Acceleration structures in wgpu.** Ray queries are experimental and
   effectively Vulkan-only, so a variant would be absent on most adapters -
   exactly the failure `Msaa` documents for a `2x` level, where a pipeline
   built at an unsupported sample count fails device validation rather than
   degrading.
3. **A reason.** The cheaper win on the same infrastructure is ray-queried
   *contact* shadows layered on top of `mapped`, not a full traced pass.

Until all three move, this stays a paragraph and not a row.

## The plan, in order

Each step is a landing that can be reviewed on its own.

1. **Make the census durable - done, 2026-09-02.**
   [`shadow_occluder_ground_truth.rs`](../../crates/vex/tests/shadow_occluder_ground_truth.rs),
   four `#[ignore]`d tests on the model of
   [`skycube_ground_truth.rs`](../../crates/vex/tests/skycube_ground_truth.rs),
   pinning the counts above, the `0x50 + 32n + 16m` closure on 129/129, the two
   constants, the unit-vector record heads, the 97/129 padded box, the 119/10
   split and Pure's zero. Every number on this page is asserted there, so the
   design below rests on something `just test-data` can re-check rather than on
   a survey nobody can reproduce.
2. **The setting, with only `off` and `blob` live - done, 2026-09-04.**
   [`crates/display/src/display/shadows.rs`](../../crates/display/src/display/shadows.rs),
   its own file from the start since `display.rs` is already why
   `reconstruction.rs` and `motion_blur.rs` were split out; tests in
   `display/tests.rs` naming `original` and `mapped` as values that must
   *not* parse, so whoever lands one deletes their line beside a new variant.
   The key lives in `[render_profiles.<title> (<platform>)]` rather than flat
   in `[graphics]`, for both of that table's reasons: a quad per craft on a
   grid of eight is render cost, and what a value *means* is per (title,
   platform). The
   GRAPHICS row landed with step 3 rather than with this step - a row for
   infrastructure that is not there is worse than no row (ADR-0013) - and
   `--shadows` was the override in between.
3. **`blob` - done, 2026-09-04.** [`oag_render::shadow`](../../crates/render/src/shadow.rs)
   draws a ground-aligned quad per craft;
   [`oag_raceplay::shadow`](../../crates/raceplay/src/shadow.rs) places it,
   casting along the craft's **own** down axis (not world gravity, so a
   magstrip and an inverted section work) against the circuit's own collision
   geometry, filtered to `Floor`/`MagFloor` so a craft beside a barrier does
   not get its shadow up the wall. Blended `SrcAlpha`/`OneMinusSrcAlpha` over
   black, which is `dst * (1 - a)`; depth-tested and not depth-writing; drawn
   after the track and the pads and before the hulls.

   **The silhouette is the disc's where the disc has one.** All nine of HD's
   `ambient_shadow.gtf` load and draw, and the polarity is *measured* rather
   than assumed - decoded, the corner texel is 0 and the craft's own outline
   runs to 212 of 255, so the stored byte is coverage and the shader reads the
   red channel (the descriptor's `remap` broadcasts it there and forces alpha
   opaque, so reading `.a` would draw a full-strength rectangle). Every other
   title gets `Silhouette::falloff`, which is ours, reported per slot in the
   load report, and is why `off` stays the default.

   **What is ours besides the falloff, each said where it is written**: the
   height fade and its `FADE_REACH`, the `LIFT` off the surface, and where in
   the frame the quad is drawn - the original's own draw-order key has no
   layer for a blob, because no title in the lineage draws one.

   **The fade's first cut was wrong in a way only a capture showed.** Faded
   linearly from the ground, a craft resting at its own ride height came out
   at strength `0.030`: uploaded, drawn, and invisible - 5,262 pixels changed
   by a maximum of 2. The fade is full strength up to the craft's own hover
   target and only falls off above it.

   **Diffed `off` against `blob` on four titles**, at a matched camera pose
   each, so the tier is measured drawing rather than assumed to: Pulse PSP
   8,595 pixels changed at a worst delta of 43, Pulse PS2 4,637 at 75, Pure
   16,924 at 166, HD/Fury 25,022 at 77. 2048 does not race yet. **Every
   judgement about how it *looks* is from a headless capture** - the fade's
   shape, the falloff's darkness and `LIFT`'s size are all ours, and none has
   been seen in a window.
4. ~~**Decode the occluder's two record arrays**~~ - **done 2026-09-02**: they
   are `n` planes and `m` vertices, above. ~~What is left of this step is the
   **runtime reader in Ghidra**~~ - **done 2026-09-03**:
   `Shadow_RenderOccluderVolume` (`0x089038c8`, confidence 84,
   [`shadow-occluder.md`](../ghidra/functions/psp-pulse-usa/shadow-occluder.md))
   reads the same `n`/`m`/bbox fields this payload decode pinned, derives its
   projection direction from the occluder's **own local axis**, transformed
   by its own world matrix - not from any light - and extrudes a stencil
   shadow volume. It is also the craft's own drop-shadow renderer
   (`exhaust.md`'s `g_craft_scale` finding was the same function from a
   different angle). **The static link from vex class `0x3c3` to this
   function's method table is also traced now**:
   `DynamicShadowOccluder_RegisterClass` (`0x0890446c`, confidence 84) passes
   `0x3c3` to `Vex_RegisterClass` and installs the method table whose byte
   offset `+0x44` holds `Shadow_RenderOccluderVolume`'s address, exactly -
   not a fixed-stride "slot", since two other classes' own tables checked
   the same way sit at deltas that aren't multiples of any common stride.
   Both cap at confidence 84 (Probable): the chain is unambiguous but is one
   binary's own internal consistency, not corroboration across files or a
   runtime trace. Step 4 is fully closed either way; only a runtime trace
   would move it higher, and it is optional polish rather than a blocker on
   step 5.
5. **`original`, per title. Pulse landed 2026-09-04; the rest have not.**
   A craft's own `Dynamic Shadow Occluder` hull, its silhouette taken against
   `oag_pulse::shadow::AUTHORED_AXIS` in the hull's own space, projected along
   that axis through the craft's world matrix onto the surface the craft's
   downward cast found, and fanned into triangles - `oag_render::shadow`'s
   `Cast` and `hull_triangles`.

   **What this shares with the original and what it does not.**
   `Shadow_RenderOccluderVolume` extrudes a stencil volume from the same
   silhouette; this rasterizes that volume's *ground cap* directly. For an
   attached occluder - which every craft hull is - the original's own far cap
   is a ground plane too: it reads a height off the parent and divides by the
   direction's vertical component rather than casting a ray, so the polygon is
   the same one. **What it loses is shadowing on anything that is not that
   plane**: a craft passing under a bridge does not darken the bridge, and one
   craft does not shadow another.

   **Two bugs on the way to it, both of which still drew a plausible dark
   shape** - which is why the check that landed with them is a *shape*
   comparison and not a "something was drawn" one:

   - The fill was fanned from the ring's centroid. A craft's silhouette is not
     convex (53 of the disc's 129 hulls are concave, the worst by 1.6 times its
     own scale), so the fan produced overlapping and inverted triangles: a
     22-edge ring enclosing 27 square units drew as a crumpled star. Ear
     clipping fills it once, and the filled area now equals the ring's exactly
     (26.96 against 26.96 on Assegai's).
   - The hull was projected in the payload's own space. A `.vex` node's
     vertices are in the space of whatever `Transform` nodes enclose it, and
     `vex::world_transforms` composes that chain - `Occluder::placed` bakes it.
     On Pulse's craft that chain happens to be the identity, so this one cost
     nothing visible; it would have cost everything on a model where it is not.

   The assertion that now guards both: the shadow hull's plan-view footprint
   against the craft mesh's own, rasterized into one grid -
   **covers 93 % of the ship and spills 4 % beyond it**
   (`the_shadow_hull_is_the_shape_of_the_ship_it_belongs_to`). The hull *is* a
   simplified ship: same wingspan, same taper, same forked tail.

   **One number in it is ours and has no evidence behind it**:
   `HULL_DARKNESS`, how dark the polygon is drawn. What a stencil volume is
   *darkened by* is decided by the pass that fills it, and that pass is unread.
   At full alpha the first capture of this tier was a black hole in the road,
   and `0.35` is what it draws at now. **That number is not derived from
   anything** - an earlier note here read as though it came from HD's own
   `ambient_shadow.gtf` peak of `212/255`, which it does not; that asset is a
   different title's, a different mechanism's, and half again as dark.

   **HD landed 2026-09-04 too, and its mechanism is a different one.** 2048:
   the `track_proximity_shadow` pair plus the precomputed environment shadows,
   with the shadow direction read straight out of `.EnvSettings`. Pure:
   absence, reported - and now with evidence on the *direction* too, since the
   three immediates that make Pulse's axis appear in neither Pure build.

### HD's `original` is a coverage map, and the microcode is what says so

The obvious reading of "shadow map" is a depth buffer compared against an
interpolated reference. **HD does not do that**, and the receiving side settles
it rather than either job's name. `talons_junction/track_surface.rcsmaterial`
block #9, disassembled with
[`scripts/ps3-microcode.py`](../../scripts/ps3-microcode.py):

```text
@0x18  TXP R1.x, f[TC0] unit2            <- shadowMapTex (0x730df9ee), projected
@0x19  ADD H0.w, -R1.xxxx, {0, 0, 0, 1}  <- H0.w = 1 - shadow
@0x1b  MAD H0.xyz, R0.xxxx, R2, {fog}    END
```

One channel, sampled projectively, used directly. **Nothing in the block
compares anything against anything**, which this page took to mean the texture
holds shadow *coverage* and not depth - and that is why `oag_render::shadow::map`
renders casters as flat coverage into an `R8Unorm` target with no depth
attachment, no bias and no comparison sampler.

**Corrected 2026-09-21, and the renderer's choice survives it.** The caster
job binds the map as its *depth* target with the colour mask fully off
(`Rsx_SetRenderTargets` with the record in the depth slot, then
`Rsx_SetColorMask(0,0,0,0)` / `Rsx_SetDepthMask(1)`, front faces culled) -
[`ship-sun-occlusion.md`](../ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md).
So the map is depth, and the absent compare instruction is the RSX doing the
comparison inside the depth-texture sample. For a map that only ever holds
one ship, sampled by a road that is always below it, "compared depth" and
"coverage" give the same answer at every texel, which is why the coverage
target stays: it is the cheaper equivalent, not a different picture. Where the
two would differ - a hull sampling its *own* map for self-shadowing - this
project does not draw, and says so below.

| | The original's | This project's |
| --- | --- | --- |
| what the map holds | depth, rendered colour-masked-off and compared by the sampler ([`ship-sun-occlusion.md`](../ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md)) | coverage, which is equivalent for a road under a craft |
| who casts | models (`Job RenderModelShadowMaps`) | the craft |
| who receives | the track surface, whose material declares `shadowMapTex` | the same, as a per-model pipeline constant |
| the projection | `shadowMatrix`: **one orthographic map per ship**, looking from 70 units up `Lighting.Sun direction` at the ship, fitted to the ship's own bbox, near 1 / far 140 - read on 2026-09-11, [`shadow-model-maps.md`](../ghidra/functions/ps3-hdfury-eu/shadow-model-maps.md) | the same sun; one map fitted to the whole grid, which is ours |
| the map's size | `shadowMapTexSize`, **value unread** | 1024, ours |
| where `1 - shadow` lands | the fragment's **alpha**, which `ShadowToAlpha` names, consumed by `RenderModelShadowsOnTrack`'s accumulate half: the track chunks within 50 units of the ship redrawn inside the stencil volume under `SRC_ALPHA / ONE_MINUS_SRC_ALPHA` - **found 2026-09-11**, the colour that variant computes still unread | multiplied into colour, because this renderer's alpha is the bloom's glow mask |
| how dark | whatever that unread pass does | `MAP_STRENGTH`, `0.5`, ours |

**Proved inert where it should be**: every pipeline binds the map and samples
it, so a Pulse capture is the guard - `original` on Pulse before and after this
landed is byte-identical, 0 pixels differing at threshold 0. On HD the same
comparison against `off` changes 1,289 pixels at a worst delta of 58.

The caster pass has one observable and it is not the frame: a shadow map is
consumed by a sampler inside another shader, so
[`shadow_map_coverage.rs`](../../crates/render/tests/shadow_map_coverage.rs)
renders one caster, reads the texels back, and asserts coverage at the centre,
clear texels at the border, the fitted edges within two texels, and that a
second pass with no casters clears it.

### HD's hull is a receiver too, of the road's own mask

Read 2026-09-21 -
[`ship-sun-occlusion.md`](../ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md).
Each ship record carries **two** maps in one sun-view box, and the second is
the one that answers "why does a craft driving into a painted shadow stay
lit": before `Job RenderShips` draws a hull it renders the track chunks
within 10 units of the sun's line through that craft, from the sun, through
the chunks' own `SunOcclusionLightmap` / `SunOcclusionVertex` technique - a
fragment program that writes nothing but `lightmap.a` (or the colour set's
fourth byte) - into a per-ship 256-texel-or-smaller map cleared to black. The hull's
`ShadowMap` variant then computes

```text
sun_gate = compare(own depth map) * occlusion(that map, blue channel)
colour   = albedo * vcol * (ambient + sun * N.L * sun_gate)
```

so a craft over a road the artists painted dark goes dark with the road, at
the same texel boundary, because the *same mask* is being sampled; and a
craft over nothing reads the clear value, which is black on most circuits
(white in Zone and on six named tracks).

**This project draws both halves.** The self-shadow half is
`oag_render::shadow::self_shadow`: a 512-texel `Depth32Float` layer per
craft - slot 0's depth map is 512 against its occlusion map's 256 in
`g_ShipMapSizes`, a ratio that is read even though the slot assignment is
not - holding the craft's own opaque ranges drawn front-face-culled through
the **same matrix** as its occlusion layer (owned by `occlusion::Maps` so the
two cannot drift), cleared to far, compared in `mesh.wgsl` by four
half-texel taps against `ndc.z - 0.001` - the bias and the four taps are
this project's, the RSX's one hardware-compared `TXP` being unread past the
instruction. Front-face culling is the original's `SET_CULL_FACE 0x404` and
here it is the right trade, unlike the `mapped` tier's: the caster is the
receiver, so every sunlit face compares against the hull's far side and a
wing's underside is still nearer the sun than the fuselage under it.
Measured on Talon's Junction at tick 4800, sun `[-0.78, 0.58, -0.24]`:
2,019 pixels darken, every one of them on the Feisar's own hull, by a ratio
from 0.16 to 0.999 (median 0.87) - the cockpit recess, the inner faces of
the engine pods, the underside by the nozzle - which is the across-the-hull
variation the occlusion tap alone could not give. At 256 texels it was
1,401 pixels, so the map's density is a third of that count and the sun's
own geometry is the rest: a flat hull under a 35-degree sun shadows little
of itself, and the gate's ceiling is item (e)'s 19 % either way.
`OAG_DUMP_SELF_SHADOW=<png>` writes the player's depth layer. One
deviation: a craft whose occlusion layer drew nothing names no layer and so
gets neither tap, where the original's hull always compares. Pulse PSP EU at
tick 2100 is byte-identical before and after.

The occlusion half is
`oag_render::shadow::occlusion`, which renders the same map - the track's draw calls
(opaque, cutout and transparent alike, as the original keys on lighting
family and not on blend) whose bounds come within 10 units of the sun line
through each craft, through a sun-view [`Fit`](../../crates/render/src/shadow/map.rs)
centred on that craft, 12 units wide (the original's fallback bbox cube
plus the coverage map's margin) and 70 deep either side (the original's
near 1 / far 140), writing `lightmap.a * sun_mask` (the two carriers
`mesh.wgsl` already multiplies as the road's own gate) over black into one
layer of an 8-layer `R8Unorm` array - and `mesh.wgsl` gates the hull's sun
term, diffuse and specular, by a projective sample of its layer.
`OAG_DUMP_SUN_OCCLUSION=<png>` beside `--screenshot` writes the player's
layer out, which is how this was checked: Talon's Junction reads 12/255
under the craft in the grid-start tunnel (tick 2100) and 245/255 on the
glass floor (tick 4800). **How dark the hull then goes is bounded by how
much of its light the sun is on this side**: with the gate forced to zero
the Feisar hull at tick 4800 loses only 19 % of its brightness, so a craft
in a tunnel reads darker but not dark - that is the scene-calibration
question `HANDOVER.md`'s "frame too bright" thread owns, not this
mechanism's. Ambient is untouched, as the microcode
has it. Nothing here has a strength knob: how dark the hull goes is the two
maps' answer, not a constant of this project's.

Tier-wise it belongs to `original`, because it is HD's own mechanism and
consumes HD's own data; `blob` and `off` leave the hull as before, and
`mapped` does not use it either - that tier's depth map already shadows the
hull geometrically, and adding the baked mask on top would double-count the
bridge.
6. ~~**Measure 2048**~~ - **done 2026-09-02**, and it moved the design: 2048 has
   a shadow runtime of its own, a shadow light of its own, and six occluders
   that re-proved the payload closure on a second platform.
7. **`mapped` - landed 2026-09-04, one cascade, and with a named gap.** A
   depth map every surface casts into and every lit surface reads, opt-in, this
   project's own: `shadow::map::Map::render_depth` fills it and `mesh.wgsl`
   compares against it. Offered on every title, because unlike `original` it
   asks the disc for nothing but geometry - and on HD it ships *alongside* the
   authored path rather than above it, with `original` still the one to prefer
   there.

   **Nothing in it is recovered.** The depth format, the 2048 resolution, the
   single cascade and its fit, the four-tap comparison, the slope-scaled bias,
   the darkness - all this project's. The one thing it takes from a title is
   the *direction*: the circuit's own sun where there is one, and the direction
   that title's own shadows are cast along where there is not
   (`oag_pulse::shadow::AUTHORED_AXIS`), so switching tiers changes what is
   shadowed rather than where the light is. That fallback is not cosmetic:
   `Light::stand_in`'s direction is straight *up*, and a vertical light puts
   every craft's shadow exactly beneath it where the craft hides it.

   **Two bugs found and fixed on the way, both of which shadowed *something* and
   so looked like they worked:**

   - ~~**The map lookup was mirrored vertically.**~~ **Retracted 2026-09-11 -
     this "fix" was the mirror.** The claim was that glam's `rh::proj::directx`
     projections output Y-down NDC; they are **Y-up** with a `0..1` depth,
     which glam's own `camera/rh/proj.rs` states, and a wgpu render target
     rasterises `ndc.y = +1` into texel row 0. So `uv.y = 0.5 - ndc.y * 0.5`
     was right and the `ndc.y * 0.5 + 0.5` that replaced it read every map
     mirrored about its horizontal axis. A caster near the middle of its own
     map still lands near the middle under a mirror, which is exactly why
     both tiers appeared to work with it: on HD the coverage tier put each
     craft's shadow on the wrong side of the craft with its silhouette
     turning the wrong way as the craft turned relative to the sun, and in
     the `mapped` tier - whose box is centred `MAPPED_AHEAD` in front of the
     player - the mirror threw the player's own shadow about twice that far
     away, which is the "gap" recorded below. Pinned by an off-centre caster in
     `shadow_map_coverage.rs`, read back through the receiver's own formula.
   - **The shadow term reached one fragment entry point of five.** `mesh.wgsl`
     has `fs_main`, `fs_main_blend`, `fs_main_alpha_test` and the two
     `_velocity` twins, and a **race draws through the `_velocity` pair** - so
     the first cut shadowed the scenery that a blended material happened to
     draw and left the entire road untouched. Four debugging passes were spent
     on the projection maths before the entry points were counted.

   **The gap, stated rather than tuned away: a craft's own shadow does not
   appear on the road.** *(2026-09-11: the mirrored lookup above is the
   likely cause and is fixed; whether this closes the gap has not been
   re-measured with a diff, so the paragraph stands until it is.)*
   Scenery-on-scenery and the rest of the frame do
   shadow. What has been ruled out, each by a separate run: the depth pass
   writes depth (`shadow_map_coverage.rs` reads the texels back), the map has
   content at the road's own texels, the road computes a `uv` inside the map,
   the road and the craft land 13 texels apart at 2048 as the light's tilt
   predicts, and the craft's caster carries 13 ranges and 4,335 indices. What
   is *not* ruled out, and is the first thing to check: the caster shader
   applies `mvp * position` where `mesh.wgsl`'s vertex stage applies
   `model * (node_anims.transform[xform] * position)` - so any geometry baked
   in an anim node's space is cast from the wrong place, and the caster pass
   binds no node matrices at all.

Steps 1-6 have landed, step 5 on Pulse only. **What is next is a title, not a
step**: `original` on HD is its shadow-map pipeline and on 2048 its
proximity pair, and both are their own piece of work. Step 7, `mapped`, is
untouched and stays that way until someone wants it.

**Verified on HD against `off`**, one camera pose, 2026-09-04: 1,289 pixels
change at a worst delta of 58, and the difference carries the road's own
hexagon pattern - which is what a multiply into the surface colour looks like
and a flat decal does not.

**Verified on Pulse against both other tiers**, at one camera pose, 2026-09-04:
`off` against `original` changes 4,625 pixels at a worst delta of 134, `off`
against `blob` 5,259 at 70, and - the check that says the two tiers draw
*different things* rather than the same thing twice - `blob` against
`original` 5,720 at 126. The projected hull's own placement is asserted rather
than eyeballed: every vertex lands exactly one `LIFT` above the contact plane,
and the polygon's centre sits `0.836` units from the contact point under a
craft whose hull is `10.5` long
(`crates/game/tests/shadow_ground_truth.rs`).

## The shadow blinked, 2026-10-05: a flat polygon on a road that is not flat

The report from play: shadows "flickering a bit, especially pronounced on Pulse
Zone races". The maintainer's Pulse (PSP) profile runs `original`, so the
shadow in question is the projected occluder hull
([`hull_triangles`](../../crates/render/src/shadow.rs)).

**What the cause was.** `hull_triangles` lays the whole hull on one plane, the
plane of the one collision triangle the craft's downward ray hit, a unit above
nothing but that plane (`LIFT`, 0.05). A hull is five units across and a road
curves; away from the hit triangle the plane stands under the road, and *which*
triangle is hit changes tick to tick as the craft crosses seams (the contact
normal steps by 0.02-0.04 per component when it does). So a part of the polygon
is buried under the road on one tick and clear of it on the next: a shadow
whose edge blinks. Faster craft cross more seams per second, which is why a
Zone race, where the speed climbs, shows it more.

**Measured, in this order** (all on the default Pulse circuit, `--autopilot`,
the maintainer's profile of 4x MSAA and high motion blur):

1. *Not the render settings.* The on-minus-off shadow mask swings 7-8 % tick to
   tick in all four of {motion blur off, high} x {MSAA off, 4x}, identically
   (`mean|d|` 1081, 1073, 1072, 1081 thousand). Upscaler and TAA were off in
   this profile (`reconstruction = "off"`), so they are not in the path.
2. *Not a placement jump.* Per tick the hull's area, centroid, strength and
   height are smooth (area 27.3-27.7 over 40 ticks; the centroid moves 0.03
   units at a seam).
3. *The polygon is buried.* `Race::floor_above` casts from each polygon point
   along the normal into the circuit's own floor. In Zone, late in a lap
   (ticks 6000-6060), an unconformed polygon sat up to **0.70 units** under the
   road at `LIFT` 0.05, on **22 of 60 ticks**; at 3000 ticks 0.28 on 9 of 60;
   at 900 ticks 0.12 on 2 of 60. The same circuit as an ordinary race, at
   `LIFT` 0.15: 0.10 on 1 of 60 at 3000 ticks and nothing at 6000.
   [`shadow_stability_ground_truth.rs`](../../crates/game/tests/shadow_stability_ground_truth.rs)
   prints all of it.
4. *The picture agrees, and says more.* Changed pixels, shadow on minus off,
   per tick over 20 ticks (Zone, tick 6000-6020): before, 3102 at worst against
   a steady 10-11 thousand, with a tick-to-tick alternation of 13.8 %. The
   collision floor is not what the player sees, though: conforming the polygon
   to the collision floor alone still dipped to 5026 (8.8 %), because the
   visible road stands a little proud of it. Raising `LIFT` is what closed
   that: 0.15 gives a worst tick of 10610 and 3.7 % (the road's own texture
   sliding under a shadow accounts for that residual); 0.3 and 0.6 look the
   same. Conforming matters too: `LIFT` 0.15 *without* it still dipped to 9398
   (7.2 %).

**The fix, two parts, both in the render side** (no simulation change):

- `oag_render::shadow::conform_to_floor` splits each hull triangle once at its
  edge midpoints and moves every vertex to `LIFT` above the circuit's floor
  under it (`Race::floor_above`: a unit above, two below, nearest hoverable
  hit). A flat triangle over a concave road is buried between its own vertices
  however well they are placed - fitting vertices alone left 0.12 at a
  triangle's centre, one split brought it to 0.056.
- `LIFT` from 0.05 to 0.15. **Chosen, not measured**: the smallest of the
  values tried (0.05, 0.15, 0.3, 0.6) that removed the dips.

The test is disc-backed (`the_shadow_polygon_is_not_buried_under_the_road`): the
conformed polygon stays within 0.1 of the floor over three 60-tick Zone
windows, and the unconformed one must still exceed 0.3, so the probe cannot
lose its sensitivity unnoticed. The blob tier shares `LIFT`.

**What was not done.** The original was not captured: nothing here says
whether PPSSPP's software renderer shows the same blink, and Pulse's stencil
volume is depth-tested against the visible road, so it plausibly cannot bury
at all. The plume covers most of the shadow at the default chase camera, so
the blink reads as a flicker at the craft's tail rather than a shape; the
frames are in the lane's scratch report. `Race::floor_above` runs about 300
casts per craft per frame at `original`; not profiled.

## Constraints this touches

- **The occluder parser lands in `oag-formats`, which
  [`just check-determinism`](../architecture/determinism.md) scans.** `f32`
  only, no `mul_add`, no reassociation. The drawing side in `oag-render` is
  exempt and may use anything.
- **No gameplay crate learns about shadows.** A shadow is drawn from the state
  snapshot the simulation already emits; nothing in `oag-gameplay` gains a
  field for it.
- **`shadows.rs` is a new file under the 1,000-line rule** and its
  `#[cfg(test)]` block belongs in `display/tests.rs`, since `TEST_BASELINE` is
  empty and rule 2 has no exemptions.
