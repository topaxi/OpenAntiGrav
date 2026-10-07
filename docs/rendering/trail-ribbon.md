# The trail ribbon: what the assets author, per title

Asked 2026-08-23: can the exhaust trail be *derived from the shipped data*
rather than from constants, so it renders accurately on all four sources?

**Short answer: on Wipeout HD/Fury, yes and almost for free. On Pulse, no - the
data holds the ribbon's mount point and its noise texture and nothing else, and
every number that decides how it looks is a table in the executable.** Pure sits
between the two and its executable has not been read.

## Method

A sweep over every entry of each source's bulk archive, testing each blob for
`VEXX` magic and counting nodes of the `Trail` class, then decoding those nodes'
payloads. Reproduce it with a throwaway example against
`oag_assets::Archive`/`psarc::Archive`; nothing here needs a running game.

The class id is version-keyed and getting that wrong is the trap this sweep was
built to avoid. Version 6 is `vex::CLASS_TRAIL` = `0x3c8`. **Version 4 is
`0x37a`**, which is not a guess: it is the class-name run's own arithmetic,
`base + index("Trail") + gap`, with the base and gap structure that
`crates/pure/tests/class_table_ground_truth.rs` already pins at confidence 94
for eight other ids. It predicts nodes named `trail_con_left`/`trail_con_right`
in Pure's ship files, and that is exactly what is there - a prediction and an
asset check, so **confidence 92**. Version 3 has no recovered numbering at all
and 15 of Pure's files are skipped for it; that is a hole in the sweep, not a
negative result.

## What is on each disc

| Source | entries | `.vex` | files with a `Trail` node | nodes | payload sizes |
| --- | ---: | ---: | ---: | ---: | --- |
| Pulse PSP `Data.wad` | 1,142 | 340 (v4 33, v6 307) | 3 | 6 | all 64 |
| Pulse PS2 `WADS2.WAD` | 7,200 | 994 (all v6) | 10 | 20 | all 64 |
| Pure PSP `Data.wad` | 832 | 171 (v3 15, v4 156) | 27 | 53 | 52 x 64, 1 x 0 |
| HD/Fury, 7 PSARCs | 11,664 | 183 (all v6) | **0** | 0 | - |

Node names across all of them: `trail_con_left`, `trail_con_right`,
`trail_con_left_wing`, `trail_con_right_wing`, and on Pure one `motrail`.

## The payload is a mount, not a parameter block - confidence 92

**Every one of the 78 non-empty `Trail` payloads is exactly 64 bytes**, and 64
bytes is a 4x4 matrix. Decoded, each is an identity rotation with a translation
in row 3, the row-vector convention `vex::transform` documents - for example
Pulse's `trail_con_left` at `(2.6497, -0.8177, 0.5184)`.

There is no capacity, no layer count, no width, no colour, no scroll rate and no
texture reference anywhere in it. A `Trail` node says **where** a ribbon hangs
and nothing about what it looks like.

That is the whole negative result, and it is why `oag_fx::exhaust`'s
`TRAIL_*` constants cannot be replaced by a parser. On Pulse those numbers come
from `Trail_InitPreset`'s three-preset table at `&DAT_08ad226c` in `BOOT.BIN`,
with `ExhaustFlare_Init` passing preset 2 - all of it recovered and tabulated on
[exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md), "The ship's trail
parameters, from preset 2".

The one genuinely asset-side piece on Pulse is the **texture**:
`Data\Tex\engineFlare\Engine_noise.mip`, a literal string in both executables,
so an exact `wad::hash_name` hit rather than a mined candidate.

## Who authors a mount, and the divergence nobody had looked for

Named-file checks, rather than the index sweep, are what make this readable:

- **Pulse PSP**: `Assegai\shipwreck.vex` and `Triakis\shipwreck.vex`. No racing
  craft authors one - which fits the recovered runtime exactly, where a racing
  ship's ribbon is built **in code** by the `Engine Flare` constructor rather
  than bound from a node. One further file carries a pair and is not identified.
- **Pulse PS2**: `Triakis\Ship.vex` **does** author a `trail_con_left`/`right`
  pair, where the PSP file for the same team does not. Nine further files carry
  pairs and are not identified. **This is a real PSP/PS2 authoring divergence in
  the ship set and it is new here** - whether the PS2 runtime binds those mounts
  or ignores them the way a code-built ribbon would is unread.
- **Pure**: **8 of 12 teams' `Ship.vex`** author a pair - `AG_Systems`,
  `Assegai`, `Auricom`, `Feisar`, `Harimau`, `Piranha`, `Qirex`, `Triakis`. So
  Pure hangs the craft's ribbon off the asset where Pulse hangs it off the
  constructor. Pure's executable has not been read, so what binds them is
  unknown, but a mount authored on eight of twelve racing craft is not
  accidental.

## The ribbon this project loaded was the disc's dead one - 2026-08-23

**`Data/RibbonEffects/enginetrail_bluered_triangle.vex` is the path the shipped
executable names**, at `0x007a2d70` in `EBOOT.elf`, in `TrailEffectManager`'s
own TOC block beside `EngineTrail/TrailEffectManager.cpp` and its
`position`/`normal`/`Uv1`/`VertexColour1` attribute names. The plain
`enginetrail_triangle` this page describes below, and this project loaded until
now, is **named nowhere in the executable**: the string sweep finds three
`enginetrail` matches and none of them is it. It sits in `DATA02` where the
`bluered` pair sits in `DATA06`, which is the Fury archive - so it is the
pre-Fury build's ribbon, left on the disc. **Confidence 92.**

The difference is not academic. The two carry different textures, and the
`bluered` pair's are **self-documenting**:

| | `enginetrail_triangle` (dead) | `enginetrail_bluered_triangle` (named) |
| --- | --- | --- |
| first texture | `hd_enginetrail.gtf` 256x256 | `hd_enginetrail_blue_alphaistrail.gtf` 256x256 |
| second | `hd_enginetrail_noise.gtf` 128x128 | `hd_enginetrail_red_alphaisnoise.gtf` 256x256 |
| samplers | `Texture1`, `Texture2` | two hashes with no preimage |
| parameters | `TrailSpeed` = 1.0, `0xe296b1ed` = 0.15 | the same two, plus `0xbb48e390` = 0.0 |

`alphaistrail` and `alphaisnoise` say outright what the two alpha channels
carry, and they cut both ways. They **confirm the rule this loader already
followed** - the second slot is the noise, in its alpha - so
`race::assets::trail_texture` needed no change beyond the model it is pointed
at. And they say the *first* slot carries **the ribbon's own shape mask**,
which this renderer's one-texture shader has no slot for. The leftover texture
is no longer "a colour map with nowhere to go"; it is the trail's silhouette,
which is a much better reason to want a two-texture ribbon.

The picture changes accordingly. Against a frame from the same autopilot run,
switching the model moves **35,738 pixels**, and what was a fat white blowout
behind the nozzle becomes a narrower trail with a visible taper and its own
colour. That is a *data* correction, not a tuning one: no `TRAIL_*` constant
moved.

Two loose ends the sweep turned up and did not chase. `engineTrail` sits at
`0x007b3c60` in camelCase and in a different neighbourhood - plausibly a config
key. And `Data\RibbonEffects\` with bare stems (`waketrail_triangle`,
`leachbeam_triangle`) at `0x0079c1xx`-`0x0079c3xx` uses **backslashes** where
`Trail_ModelPath` uses forward slashes, so there is plausibly a second ribbon
loader that has not been found. Neither threatens the engine trail's
conclusion - no bare `enginetrail_triangle` stem exists anywhere in the
binary - but both are worth someone's next hour.

**The ribbon draws from the blue texture now, and which texture is which is
the whole of it.** The property to keep in mind: **the noise map's alpha
averages 17 of 255 and the coverage map's averages 162**, so anything that
treats the noise as the ribbon's coverage erases the trail. The one-texture
path does exactly that - correctly, for a title whose one texture *is* the
coverage - and the texture this loader hands it on HD is the noise.
`crates/game/tests/hd_engine_flare_ground_truth.rs` pins the two apart by that
same property.

HD's program is unambiguous about the roles: unit 0's alpha displaces the `u`
coordinate, and unit 1 - the blue one - supplies the colour **and** the output
alpha. `exhaust.wgsl` now does exactly that where `trail_shape` is on: the
noise's alpha offsets the coordinate, the blue texture is sampled there, and
its `rgba` replaces the sample the one-texture path would have used. The blue
alpha is a **cross-ribbon falloff** - by row, 1, 112, 166, 213, 255, 220, 172,
117 out of 255 - so the ribbon has a soft edge of its own for the first time.

`TrailSpeed` is **not** added. The material authors 1.0, and adding a whole 1.0
to a coordinate this sampler repeats is the identity, so the engine must patch
that slot per frame with something else - and what, is unread. A scroll of this
project's choosing would be the fitted coefficient this tree keeps out.

Every other title compiles the pipeline it always had - `trail_shape` is a
pipeline constant set only where the material names two textures - and Pulse
and Pure frames are byte-identical either side.

**Two things this paragraph used to flag are gone the same day the geometry
was read** (2026-08-24): HD no longer draws the PSP's three layers - it draws
its own three-fin tube (`oag_fx::exhaust::hd`), one coverage falloff, no
baked `LAYER_COLOUR`, and a `v` that runs across the fin exactly as the
program expects. See
[engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md).

**Superseded 2026-08-24: nothing about the geometry is PSP's any more.** The
per-craft state was read out of the running game - the 0x1230 block is a
54-sample ring of `{basis, position}` records, the SPU extrudes a three-fin
tube of 324 vertices per craft, and the colour, alpha, `u` and scroll laws
were all fitted exactly against the dumped buffers. The whole record, with
the tuning file that names every constant, is
[engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md);
`oag_fx::exhaust::hd` implements it, and the facing fade, depth fade,
`TrailSpeed` scroll and per-vertex alpha this page listed as "read and not
implemented" all draw now. The `0xe296b1ed` = 0.15 facing band is placed
(the shader's own clamp), and `TrailSpeed`'s live value is the flare's
wrapping phase accumulator.

## HD/Fury author the whole thing, and it already decodes

There is no `Trail` node anywhere in HD's 183 `.vex` files. It uses a different
mechanism entirely: a **`/data/ribboneffects/` family**, five ribbons, each one a
model plus a material plus textures.

| Ribbon | Files |
| --- | --- |
| engine trail | `enginetrail_triangle.{vex,rcsmodel}`, `materials/hd_enginetrail.rcsmaterial`, `textures/hd_enginetrail.gtf`, `textures/hd_enginetrail_noise.gtf` - **and the `enginetrail_bluered_triangle` pair beside it, which is the one the executable names; see the section above** |
| wake trail | `waketrail_triangle.*`, `hd_waketrail.rcsmaterial`, `hd_waketrail.gtf`, `hd_waketrail_clouds.gtf` |
| rocket trail | `rockettrail_triangle.*`, plus a separate `rockettrail_shadow_triangle.*` |
| leech beam | `leachbeam_triangle.*`, `hd_leechbeam_glow.gtf` |

**Each `*_triangle.vex` decodes to literally one triangle** through
`oag_mesh::mesh::rcs::build` - the existing PS3 path, unchanged, no new
decoder - with one texture slot filled and a **second texture loaded but not
drawn**. That is the shape of a *template*: the engine extrudes the ribbon from
the craft's position history and skins it with the material, and the second
texture is the noise map the name of the file next to it advertises. It also
explains one line of `rcs::build`'s own report that has been unexplained until
now.

So on HD the answer to "derive it from the assets" is **yes**, at confidence 90,
and the loader for it is already written. What is *not* read is the
`.rcsmaterial`'s parameters - `oag_rcs::rcsmaterial` currently decodes the
shader-variant table "and nothing else yet" - so scroll rates, widths and taper
would still have to come from somewhere.

### The per-livery reading was refuted, and its question is reopened

An earlier version of this page read the ribbon's per-craft variation off
`/data/ships/<team>/livery<N>/engine_flame1.gtf` and `engine_flame_noise.gtf`,
at a stated confidence of 75 with the gap named: nothing had established that
the *ribbon* sampled them rather than the engine **flare**.

**Reading `enginetrail_triangle.rcsmodel`'s material settled it against that
reading.** Its one material names
`data/ribboneffects/textures/hd_enginetrail.gtf` (256x256) and, as its second
texture, `data/ribboneffects/textures/hd_enginetrail_noise.gtf` (128x128). The
per-livery pair is 64x64 and is named by neither. So the engine trail's textures
are the shared `ribboneffects` ones, and the livery pair almost certainly
belongs to the flare - which is a hypothesis, not a finding: the flare's own
material has not been read.

**The flare's material has since been read, and it refutes that too** - see
"HD's flare is a model, not a sprite" below. Every `engineflare.rcsmodel` on the
disc names a `flame_01.gtf`, and no `.rcsmodel` material names either
per-livery file. The pair belongs to neither the ribbon nor the flare so far as
the model tables go.

**Which reopens the question.** There is no `fury`-named ribbon asset (all 120
`fury` paths on the EU disc are front-end - track-select emblems, `envsettings`,
campaign flyers), and the ribbon asset that does exist is shared and carries no
per-team skin. What makes one craft's trail differ from another's is therefore
**unestablished**: vertex colour supplied by the team, a second material this
sweep did not find, or nothing at all. Do not read this section as settling it.

### The blend, and three independent sources agreeing

The material's factor pair is `src 0x0302`, `dst 0x0001` -
`GL_SRC_ALPHA`/`GL_ONE`, through `oag_rcs::rcsmodel`'s already-recovered
`Factor` enum rather than a second reading of the same bytes. That is
`mesh_render::ADDITIVE_BLEND` exactly.

**So the same additive equation now has three independent sources**: Pulse's
PSP preset (`Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX 0xffffff)`), the PS2
build's GS `ALPHA_1` register write (`0x48`, decoded on
[batch-draw-state.md](../ghidra/functions/ps2-pulse-eu/batch-draw-state.md)),
and HD's own material data. Three platforms, three encodings, one equation.

The sibling ribbons discriminate, which is what says the field is really read:
the rocket trail is `0x0302`/`0x0303`, alpha-over rather than additive.


## HD's flare is a model, not a sprite - and the plume is inside it

Added 2026-08-23, and it changes two of this page's own conclusions.

### The flare

`race::load` used to ask every disc for
`Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` - a Pulse literal - and
HD carries no such entry, so every HD load report ended with *"the flare falls
back to a procedural glow"*. It was the same failure class the ribbon had, and
the answer is the same shape: **HD authors its flare as an asset**, and per
team rather than shared.

**Every one of the fourteen craft ships `Data\Ships\<Team>\engineflare.vex`
and the `.rcsmodel` beside it** - the twelve teams, plus `zone` and
`detonator`. 961 to 2,141 triangles each, built by
`oag_mesh::mesh::rcs::build` with no new decoder, exactly as the hull is. One
material each, always the shared
`data/materials/ships/engines/flame_test.rcsmaterial`, always additive
(`0x0302`/`0x0001`). Confidence 92 - a directory listing of all 11,664 archive
entries, and every pair builds.

### The plume was there all along

`livery::one`'s PS3 branch returned `boost: None` because a sweep for a
`shipboost.vex` equivalent finds nothing on the disc. That sweep was right and
its conclusion was wrong. **The plume is a subtree of the flare model**:

```text
root
  EF_Boost            <- the boost plume
    Joint_BoostLeft  -> ef_BoostLeftShape, ef_SpikesLeftShape
    Joint_BoostRight -> ef_BoostRightShape, ef_SpikesRightShape
    Joint_Diamonds   -> ef_DiamondsShape
  EF_Main             <- the always-on flame
    ef_OuterShape, ef_InnerShape, ef_Spikes1Shape .. ef_Spikes3Shape
```

Five shapes each, on all fourteen craft; 350-961 triangles for `EF_Main` and
616-1,492 for `EF_Boost` across the eight teams that reach a grid. Every node
transform in the file is the identity, so the model is authored about its own
origin and the craft's `Engine Flare` locator is what places it. Confidence 90
on the split - the group names say what they are and nothing else in the file
distinguishes the ten shapes.

**`EF_Main` is confirmed on screen and `EF_Boost` is not.** Five HD frames
(ticks 120, 160, 240, 300, 400 of the same autopilot run) were captured either
side of a stashed build and diffed: each changes 297-916 pixels, all of them in
a 60x150 box at the nozzle, which is the always-on flame appearing and nothing
else. **No sampled tick shows a plume-sized change**, so either the run never
tripped the boost gate or the plume drew where the ribbon had already saturated
to white. The draw path is the one that already draws Pulse's plume, and the
model is asserted non-empty per craft by
`crates/game/tests/hd_engine_flare_ground_truth.rs` - but *the `EF_Boost` draw
itself is unconfirmed in a capture* and should be recorded that way.

**The trigger's mechanism is read and its source is not.** `EF_Boost` is drawn
on this engine's own boost gate - `Exhaust::plume_visible`, the condition
recovered for *Pulse's* plume. Reading HD's `EngineFlare_Update` on 2026-08-23
showed its gate is the same three parts: a countdown timer, a hard snap to 1.0
while it is above a threshold, an exponential decay after. So this is a matched
mechanism rather than an inference off a node name - **confidence 88**, up from
80. What *writes* that timer is outside the flare's own vtable and is still
unread, so how long HD's boost lasts remains this project's number, and the
loader says so on every race. See
[engine-flare.md](../ghidra/functions/ps3-hdfury-eu/engine-flare.md).

### What the flame's own shader does

`flame_test.rcsmaterial`'s fragment microcode is read instruction by
instruction on
[engine-flare.md](../ghidra/functions/ps3-hdfury-eu/engine-flare.md). Three
things follow for the picture:

- **The colour set's fourth byte is the flame's opacity ramp** - the program's
  last instruction multiplies `VertexColour1.w` into the output alpha, and the
  attribute is named by hash preimage rather than inferred. `mesh/rcs.rs` files
  that byte as the sun-occlusion mask, which is right for the circuit materials
  it was measured on and wrong here. `livery::flare::alpha_ramp` moves it into
  the vertex alpha for this one model and leaves the mask unset. Confidence 92.
- **The flame is unlit, and its numbers are on the disc.** Its colour is the
  sampled `rgb` times a scalar, with no `N.L`, no ambient and no lightmap - and
  the **sampled alpha is never read**, the fourth component being the vertex
  ramp from the moment the program puts it there. The five constants the
  program patches are *not* in the shader file: they are in each craft's own
  `.rcsmodel` material record, in a per-instance parameter table that was the
  unread tail of that record until now. `power1` = 10, `scale1` = 0.3,
  `min1` = 0.45, an alpha scale of 2.0 and a colour scale of 1.0, identical on
  all fourteen craft and read per craft anyway. `oag_mesh::mesh::Flame`
  carries them to `mesh.wgsl` as pipeline constants, and the whole path is off
  for a material that does not declare the full set.
- **`Speed` = 2.0 is authored and `time` is not**, which is what says the
  latter is an engine clock. The surface scrolls and this renderer does not
  animate it - read, not implemented, and said in the load report. `EF_Main`
  also carries one `.vex` attribute, `AnimEnd = 1.0`, which nothing reads.

### The per-livery textures, refuted again and with a stated limit

The section above left `engine_flame1.gtf`/`engine_flame_noise.gtf` as a
"hypothesis, confidence 75" that they were the *flare's*. **They are not.**
Every `engineflare.rcsmodel` on the disc names `flame_01.gtf` - under
`data/ships/flame_test/`, `data/weapons/textures/`, or the craft's own
`textures_dlc/livery1/` on a Fury ship - and no material names either
per-livery file.

**The limit on that claim**: the sweep read the material records embedded in
`.rcsmodel` files, not standalone `.rcsmaterial` ones, which
`oag_rcs::rcsmaterial` decodes only in part. So what is earned is *no
`.rcsmodel` material on the disc names them*, which is enough to keep them
unwired and not enough to say nothing does.

One incidental oddity worth recording: `egx_c1/engineflare.rcsmodel` names
`ships/auricom_c1/textures_dlc/livery1/flame_01.gtf` where every other Fury
ship names its own directory. That reads as a shipped data bug rather than as
anything to reproduce.

### The two engine effects that stay unwired

`/data/psys/wo_engine_flare.pob` (3 emitters: `WO_ENGINE_FLARE`, `rings`,
`core`) and `/data/psys/wo_engine_jetflare.pob` (1 emitter) both **parse
cleanly** and neither is in `RACE_EFFECTS`. So their absence is a missing
*trigger*, not a missing decoder - and per `CLAUDE.md` an effect with no
recovered trigger stays unwired rather than fired on a guess. Wiring either
needs HD's executable read; the third of the family,
`wo_ship_engineflare.pob`, is already wired and rides the nozzle.

## The engine trail is one of four ribbons, and `WakeTrail` is a second manager

Read 2026-08-24, prompted by a player's report of an effect under the craft
that no page here had a name for. The executable's own string run puts the
engine trail in a **family**:

```text
0079c1d0  'rockettrail_triangle'
0079c1e8  'waketrail_triangle'
0079c200  'leachbeam_triangle'
0079c218  'RocketTrail_Shadow_triangle'
0079c1c0  '%s%s.rcsmodel'          composed against 'Data\RibbonEffects\'
```

Four `*_triangle` ribbon models beside
`enginetrail_bluered_triangle`, all composed through one `%s%s.rcsmodel`
format against `Data\RibbonEffects\`. Only the engine trail has been read
([engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)).

### `WakeTrail` is structurally the engine trail's twin

Its TOC block is the same block, field for field, as `TrailEffectManager`'s:

| | `Trails` (engine trail) | `WakeTrail` |
| --- | --- | --- |
| job name | `'Trails'` `0x008b444c` | `'WakeTrail'` `0x008b3184` |
| embedded SPU job | `0x00811680`, `0x4d40` bytes | `0x00816400`, **`0x1240` bytes** |
| manager global | `0x00aed460` | `0x00ad81f0` |
| technique | `StaticUncompressed` | `StaticUncompressed` |
| attributes | `position` `normal` `Uv1` `VertexColour1` | the same four |
| that `0.8` | `0x008b4450` | `0x008b31c8` |

The job registration reads the name at `0x002a8678` and the draw state is
built by `0x002a8f58` - which fills the same engine shader-parameter table
`Trail_BuildDrawState` does
([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md)). Its block also
carries a constant run the engine trail's does not: `0.1`, `-1.0`, `1.0`,
`0.5`, `0.125`, `10.0`, `50.0`. Unread.

**Its SPU job is a fifth the size of the engine trail's** (`0x1240` against
`0x4d40`), which fits a simpler ribbon - no three-fin extrusion, no per-craft
intersection test.

### What its material does, read

`/data/ribboneffects/materials/hd_waketrail.rcsmaterial`, 896 bytes, one
fragment block of 18 instructions. Two samplers and **one parameter**:

| Hash | What | Where |
| --- | --- | --- |
| `0x3bdc0403` | `hd_waketrail.gtf` | unit 0 |
| `0xd5d2652f` | `hd_waketrail_clouds.gtf` | unit 1 |
| `0x906b67ba` | **`time`** | patch fslot `0x34` |

```text
@0x01  MUL R0.zw, uv.xy, {1, 0.2}       <- unit 1 sampled at v x 0.2
@0x03  TEX R1.x, R0.zwzz unit1          <- the cloud texture
@0x09  MAD R2.y, {time}, {0.1}, cloud*0.5  <- v scrolls with the engine clock
@0x0d  TEX H0.w, f[TC3] unit0           <- coverage at the authored uv
@0x0f  TEX H1.xyz, R2 unit0             <- colour at the displaced one
@0x0b  MUL_SAT R1.w, f[POS].z, {0.75}   <- the same depth fade the trail has
@0x11  MUL H0.w, H0, R1 END
```

So it is the engine trail's arrangement in miniature: a second texture
displaces the first, the output fades by window depth with the **identical**
`0.75`, and the scroll rides **engine shader parameter slot 0** - the same
global seconds clock the flame surface uses, `time * 0.1` here against the
flame's `time * 2.0`.

### `MagstripWake` is a separate thing, or the same thing named twice

`'MagstripWake'` (`0x00782f90`) and `'MagstripWake.cpp'` (`0x00782fa0`) sit in
a different string run entirely, next to `'Transparency'` and
`'arc_anchor_point'`. `ps3-toc.py attrib` puts two functions behind that
`.cpp`: **`0x00109e40`** and **`0x0010a0c0`**. Neither has been read, and
nothing yet connects them to the `WakeTrail` manager above.

**A player reports an effect below the craft on the magstrip**, which is what
`MagstripWake` reads as by name and is consistent with a ribbon laid on the
track surface rather than trailing in the air. That is a sighting, recorded as
one: it is not evidence about which of the two names draws it, and this page
poses that as the question rather than answering it.

### Not implemented, and what it would take

Nothing here is drawn. The three steps, in order: read `0x00109e40` /
`0x0010a0c0` and `0x002a8f58` to learn what the ribbon is anchored to and how
many samples it keeps; dump the `WakeTrail` manager's own block live the way
`scripts/rpcs3-trail-dump.py` does the engine trail's, since the SPU job's
output buffer is readable at its RSX address; and only then extrude. The
material and both textures are already decodable today.

## What follows for the code

`oag_fx::exhaust` holds Pulse-PSP's preset as `TRAIL_*` constants and every
title races on them, which is exactly why "accurately across the games" is
structurally impossible today. The shape the evidence argues for:

1. **Per-title trail parameters in the title packages**, the way
   `oag_pulse::race` already carries what Pulse ships - not a parser, because
   there is nothing to parse on Pulse. `oag-render` reading a title package is
   already how the plume and the presentation tables work and is allowed by the
   dependency rules ([ADR-0022](../architecture/adr/0022-title-packages.md)).
2. **Mount points from the asset where one exists**, since a `Trail` node's
   64 bytes *are* usable and are the one thing the data does supply. That
   immediately covers Pure's eight ships and Pulse's wrecks.
3. **HD through `ribboneffects`**, not through the same type - its ribbon is a
   textured template with a material, not a preset, and forcing it into a
   parameter struct built for Pulse would be the fourth column that does not
   fit.

**Point 3 is implemented as of 2026-08-23** and points 1 and 2 are not.
`oag_title::exhaust::Exhaust` is the axis - an enum, because a title either
names a texture or authors a template and no source does both - and HD now
loads its ribbon's noise texture and its blend off the disc through
`race::assets::trail_texture` instead of falling back to a procedural glow.

**What is asset-derived on HD now**: the noise texture and the blend equation,
both named by the material.

**What is still PSP's on HD, and is a documented divergence rather than a
finding**: every geometry number - `TRAIL_SAMPLES`, the taper, the three layers,
their half-widths, their scroll rates - and the three baked layer colours. HD's
own ribbon code is unread, so fitting those to a picture would be an invention
where leaving them is at least a labelled borrowing.

**What is located and deliberately unwired**: `hd_enginetrail.gtf`, the 256x256
colour map. This renderer's ribbon shader samples one texture and the one it
samples is a noise map, so the colour map has no correct slot to go in;
putting it in the noise slot would be a plausible-looking substitution of the
kind `CLAUDE.md` names. The load report says so on every HD race.

## Open

- Pure's executable is unread, so what binds its eight authored mounts, and what
  parameters it uses, is unknown. It is an unencrypted ELF like Pulse's.
- Whether the PS2 runtime binds `Triakis\Ship.vex`'s authored mounts, or builds
  its ribbon in code as the PSP does and ignores them.
- The 15 version-3 `.vex` files on Pure's disc are excluded from the sweep, the
  version-3 class numbering being unrecovered.
- 1 of Pulse PSP's 3 hits and 9 of the PS2's 10 are unidentified by name.
- The single 0-byte `Trail` payload on Pure - every other one of the 78 is 64.
- `hd_enginetrail.rcsmaterial`'s own contents, which would supply HD's scroll
  and width parameters. (The per-livery-texture question it was also meant to
  settle is answered above, from the flare's side.)
- The flame's **scroll**: `Speed` and `time` are read, the instructions are
  read, and nothing animates.
- Four parameter hashes in `flame_test.rcsmaterial` with no preimage yet, one
  of them the unit-0 sampler; two of the four carry values this renderer uses
  by the code slot they patch.
- **`SpecScale` is authored per material instance** (235 on a hull's paint, 500
  on its glass), which is what would retire `renderer.md`'s "32 is a stand-in"
  note. Nothing reads it. Same for `Bloom` = 1.0 on `emissive_bloom`.
- What *writes* the flare's boost timer at `+0x12c` - the gate's own mechanism
  is read, its source is not - and whether `EF_Main` is ever modulated
  (`AnimEnd`, `Speed` and `time` are all declared and all unread).
- **A second, sprite-shaped flare path**: `Engine_Flare_Rich.gtf` and the
  registry names `engineflare_vp`/`engineflare_fp` exist in the executable, and
  nothing establishes which of the two a race draws, or whether it draws both.
  **Since 2026-09-15 it does**: `EngineFlare_RenderTick` builds the sprite's
  own four-vertex quad (a 4:1 streak, alpha from a `powf(view_dot, 32)`
  highlight) and skips it for the craft the current view belongs to -
  measured live the same day at 92, the player's craft turned away on 30
  of 30 frames and all seven AI crafts passed - see
  [engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md),
  "Ninth session" and "Tenth session". Both paths draw; the sprite is for
  the other crafts, and this renderer draws it that way:
  `oag_fx::exhaust::hd::Sprite` carries the traced size and fade law
  and `race::effects::hd_sprite_quad` never builds one for slot 0. Two
  values in that code are chosen rather than read - the per-view distance
  scale (1.0) and the sign of the view dot - and are labelled so.
- A matched-pose HD reference frame from RPCS3. None has ever been captured, so
  every HD picture in this tree is judged against the loader report rather than
  against the original.

## 2026-10-07 (`hd-rocket`): the Rocket's smoke trail is this family's `rockettrail_triangle`, and it is not drawn

Filmed on RPCS3 (held state 0, Triangle, `scripts/rpcs3-hd-weapon.py`, Talon's
Junction grid, two boots, both fired on video frame 173 and read the same):
every rocket leaves a **thick white smoke ribbon** that streams ahead of the
craft for about 0.8 s of video time, and the fire frame lights the whole
scene yellow-white for about two frames. The body itself is not resolvable at
the chase camera's distance. The burst on the wall is a white flash, then pale
sparks and grey-brown smoke for roughly a second.

This build, same circuit, `--give rocket`, fire on `square`, far camera: three
small dark rocket bodies and then a white flash and grey puffs at the wall,
**no ribbon and no launch light**. `WO_ROCKET_FLARE` plays (it is wired) and
shows nothing at that range, so the smoke is not the flare's.
Pair, original top, ours bottom, 0.1/0.3/0.6/0.9/1.2/1.8 s after fire:
`data/reference/hd-capture/rocket-ref/pair_rocket.png` (gitignored, not in the tree).

The disc authors the ribbon: `data/ribboneffects/rockettrail_triangle.{vex,rcsmodel}`,
`materials/hd_rockettrail.rcsmaterial`, `rockettrail_shadow_triangle.*` and
`data/tex/rocket_smoketrail.gtf`. The string table's pointer for
`rockettrail_triangle` sits at `0x00920fc0`, referenced from the `WakeTrail`
block at `0x008b3140`, so the ribbon is built by the `WakeTrail`-style manager
(`0x00ad81f0`), not the engine-trail one. **Not read:** that job's width,
taper, lifetime and per-tick sampling, and what spawns it per rocket. Without
them any ribbon drawn would be a guess, so nothing is drawn (confidence that
the trail exists and is this asset: 80; its law: unread).
