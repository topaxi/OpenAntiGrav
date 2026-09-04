# Shadows: a ladder of four techniques, and only one of them is the original's

> **Design, not status.** Nothing in this page is built. It exists so the
> setting's shape is agreed before an enum and a menu row make it expensive to
> change, the way [`motion-blur.md`](motion-blur.md) did for its own row.

Three things settled this page's shape, and all three were measured against the
discs rather than assumed:

1. **Pulse's shadows are authored geometry, and the payload's layout is now
   decoded** - see [What the Pulse disc actually authors](#what-the-pulse-disc-actually-authors)
   below. It is *not* a blob sprite and it is *not* a `Mesh` payload.
2. **`shadow` `0x3cb`, `blob` `0x3e0` and `textureBlob` `0x3df` are authored
   zero times** across all 415 `.vex` files on the Pulse PSP disc, so three of
   the four classes the roadmap lists under shadow are inert - the same kind of
   converged negative `PointLight` already is.
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
[`Reconstruction`](../../crates/game/src/display/reconstruction.rs) - variants
named for the technique - and not
[`MotionBlur`](../../crates/game/src/display/motion_blur.rs), whose "strength,
not technique" framing works only because every one of its tiers is one shutter
fraction of one tick.

| Setting | What casts | What the shape comes from | What receives |
| --- | --- | --- | --- |
| `off` | nothing | - | - |
| `blob` | each craft | HD: the disc's own `ambient_shadow.gtf`. Elsewhere: a generated radial falloff, **as a substitute for a missing asset only** | the track ribbon under the craft |
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
[`shadow_occluder_ground_truth.rs`](../../crates/formats/tests/shadow_occluder_ground_truth.rs).

| Class | Count | Note |
| --- | --- | --- |
| `Dynamic Shadow Occluder` `0x3c3` | **129**, in 83 files | the real mechanism |
| `shadow` `0x3cb` | **0** | inert |
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
| `+0x50 + 32n` | `m` records of 16 bytes; not unit, not points inside the box | open |

### Both record arrays are decoded: `n` planes and `m` vertices

- A **32-byte face record** opens with a unit plane normal (3 x `f32`, on
  129/129), carries a `u32` at `+0x0c` giving that face's vertex count - 3 or
  more, never more than the hull owns - and then indices.
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

**The box is authored rather than derived**, so the disc-wide assertion is
containment and not equality. The counter-example is instructive:
`BEData.wad#20` is a *flat* hull with all eight vertices at `y = -0.06195458`
that declares its `y` maximum as the denormal `0x00800000` - a value no
extent computation would produce.

Confidence: **layout 88** (exact closure, 129/129, twelve independent
`(n, m)` pairs). **Interpretation now 85** - the plane/vertex split is no
longer a reading: the vertex array reproduces an independently stated bounding
box on the hulls checked by hand, the `w` column is `1.0` on every record on two
discs, and the face records' vertex counts are all in range. What remains
unfound is the runtime reader, so a Ghidra name still waits on
[the rubric](../reverse-engineering/confidence-rubric.md)'s evidence rule
rather than on the layout.

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
  composite - *onto the track*. The receiver is the road, not the scene. They
  sit at positions 4, 5, 9 and 10 of the thirteen-job frame order
  ([`rcsmaterial.md`](../formats/rcsmaterial.md), "The frame's pass order").
- Material flags `ShadowToAlpha`, `ShadowMap`, `Spot0`..`Spot3`, with
  `ShadowToAlpha` bound to the shadow compositing pass at `0x405d48`.
- Sampler names `shadowMapTex` `0x730df9ee` and
  `directionalLight0ShadowTex` `0x9becc725`; `shadowMatrix` and
  `paraboloidReflectionTex` are named in the engine's parameter table
  ([`rpcs3-capture.md`](../reverse-engineering/rpcs3-capture.md)).
- The lightmap's **alpha is a baked shadow mask**, and the renderer already
  reads it ([ADR-0026](../architecture/adr/0026-hd-authored-lighting-is-linear.md)).
- Nine `ambient_shadow.gtf`, 128x64, one per team, **fully decoded and drawn by
  nobody** ([`gtf.md`](../formats/gtf.md#the-9-b8-files-are-ship-shadows-and-their-own-remap-broadcasts-them)).

Those nine are the reason the `blob` tier is not an invention on HD: the disc
ships the craft's soft silhouette as a texture, so `blob` **plays the disc's
own image** and falls back to a generated circle only for a title that has no
such asset. That is the narrow case the never-invent rule allows - a substitute
for the missing asset alone, never an override of data we do have.

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
  is found too**: a two-sided depth-fail stencil shadow volume, colour-mask
  bracketed, its RSX register identities (stencil test/func/op, two-sided
  stencil, cull-face and colour-mask toggling) cross-checked against a local
  `rpcs3`'s own `gcm_enums.h` rather than assumed. The actual PSARC entry is
  still unfound.

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
   [`shadow_occluder_ground_truth.rs`](../../crates/formats/tests/shadow_occluder_ground_truth.rs),
   four `#[ignore]`d tests on the model of
   [`skycube_ground_truth.rs`](../../crates/formats/tests/skycube_ground_truth.rs),
   pinning the counts above, the `0x50 + 32n + 16m` closure on 129/129, the two
   constants, the unit-vector record heads, the 97/129 padded box, the 119/10
   split and Pure's zero. Every number on this page is asserted there, so the
   design below rests on something `just test-data` can re-check rather than on
   a survey nobody can reproduce.
2. **The setting, with only `off` and `blob` live.**
   `crates/game/src/display/shadows.rs` - its own file from the start, since
   `display.rs` is already why `reconstruction.rs` and `motion_blur.rs` were
   split out - tests in `display/tests.rs`, a `graphics.shadows` key, and a
   menu row on the GRAPHICS page. `original` and `mapped` are **not** offered
   until they exist. One day.
3. **`blob`.** A quad under each craft, sampling HD's own `ambient_shadow.gtf`
   where the title has one and a generated falloff where it does not, ray-cast
   down onto the track ribbon for its height and orientation. This is the tier
   that gets the pass plumbed into
   [`draw-order.md`](draw-order.md)'s queue, so it costs more than it looks.
   Two days.
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
5. **`original`, per title.** Pulse: draw the 119 local-space hulls - the
   geometry and the projection are both now decoded, so this is a rendering
   task, not a reverse-engineering one. HD: the shadow-map path its four jobs
   and material flags describe. 2048: the `track_proximity_shadow` pair plus
   the precomputed environment shadows, with the shadow direction read
   straight out of `.EnvSettings`. Pure: absence, reported.
6. ~~**Measure 2048**~~ - **done 2026-09-02**, and it moved the design: 2048 has
   a shadow runtime of its own, a shadow light of its own, and six occluders
   that re-proved the payload closure on a second platform.
7. **`mapped`.** A cascaded shadow map, opt-in, this project's own - and on HD
   it ships *alongside* the authored path rather than above it, with `original`
   still the default there.

Step 1 has landed. Steps 2 and 3 are unblocked and are the useful thing to
start; step 4 is the one with an unknown in it, and steps 2-3 do not wait on
it.

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
