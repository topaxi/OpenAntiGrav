# HD/Fury's Zone recolour, and the sphere a stage change sweeps through it

How a Zone race on Wipeout HD/Fury changes colour as its speed class rises:
what the shader computes per fragment, where every number comes from, and the
one part of a stage change that is not a colour cross-fade at all - a sphere
growing out of the player's craft that repaints the world stage by stage.

**Read on `hdfury-ps3-eu-dec.iso`** (the executable, the materials and
`/data/environments/zonemode.effectsettings`) and reproduced live on RPCS3
on 2026-09-15. The evidence pages are
[zone-shader.md](../ghidra/functions/ps3-hdfury-eu/zone-shader.md) (the
material variant's microcode),
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)
(the table, its runtime and the writers - passes twenty-four, thirty and
thirty-one are the ones this page rests on) and
[zone-speed-class-table.md](../ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md)
(what steps the stage). The file format itself is
[effectsettings.md](../formats/effectsettings.md).

## The surface, per fragment

Zone mode is not an engine program: it is a variant compiled into 1,467 of
the disc's `.rcsmaterial` files, and on every racing circuit it *replaces* a
material's shading rather than tinting it. What it computes, read out of the
fragment microcode (82) and reproduced in `crates/mesh/src/mesh.wgsl`:

```text
zoneUV  = zoneColourTint.xy * (1 - meshUV)
rim     = 1 - |dot(N, toEye)|
inside  = distance(worldPos, zoneOrigin) < zoneColourTint.w
surface = zoneTex(zoneUV).rgb * zoneEffect<I|O>.rgb
        + zoneBase<I|O>.rgb    * rim^10
        + zoneBaseAlt<I|O>.rgb * rim^5
```

`<I|O>` is the Inner copy of each parameter inside the sphere and the Outer
copy outside it. Every parameter is published twice more than that: once
beside the `zoneModeTrack<n>.gtf` texture with the `Track.*` colours, once
beside `zoneMode<n>.gtf` with the `Scene.*` colours, and which pair a chunk
reads is bit 0 of its own render-block flags in the `.rcsmodel` (85). The
`10` and `5` are inline literals in every one of the 20,084 blocks that carry
this shape; nothing per-stage drives them.
[`oag_mesh::mesh_render::Zone`](../../crates/mesh/src/mesh_render/uniforms.rs)
is the uniform, with the census, the colour-space argument and the terms
deliberately left out.

## The stage-transition wavefront

**A Zone stage change is a sphere, not a cross-fade.** `Environment_UpdateStageBlend`
(`0x003da540`) copies stage `n`'s colours into the Inner parameters and stage
`n - 1`'s into the Outer, and on the frame a stage commits it writes the
sphere's radius to `0.1` and its speed to `0.5`. Every later frame, while
`g_GamePaused` (`0x009384dd`) is clear, it runs:

```text
radius += speed        (only while radius < 20000)
speed  += 0.1
weight += 0.01         (clamped at 1.0)
```

Per call, never against a timestep. `Scene_PrepareFrame` (`0x003ad8dc`)
rewrites `zoneOrigin` from the local craft's own transform every frame, so
the sphere travels with the player. Closed form on the shipped numbers, `k`
frames after the commit: `r_k = 0.1 + 0.5k + 0.05k(k - 1)` - `207` at one
second, `4,635` at five, the cap at frame 628. The weight is the *new*
stage's share of the fog and light rig's cross-fade, `min(0.01k, 1)`; the
sphere is what the Zone material variant reads, and the two run side by side.

Where each number comes from, all of them the executable's own and none
authored on the disc:

| number | source | confidence |
| --- | --- | --- |
| `0.1`, the radius on the commit | the commit arm's own store | 85 |
| `0.5`, the start speed | `.data` default of the entry's `+0x0c`, the developer-only `Transition start speed` key's slot, unauthored | 85 |
| `0.1`, the acceleration | `.data` default of `+0x14`, `Transition acceleration`'s slot, unauthored | 85 |
| `20000`, the cap | literal at TOC slot `0x008b7c24` | 85 |
| `0.01`, the weight step | literal at `0x008b7c28` | 85 |
| the law reproduces live | RPCS3, `k = 53` reads `164.4`, `k = 252` reads `3288.7`, frame-to-frame increments exact | 94 |
| `zoneOrigin` is the local craft | pointer identity on 210 of 210 frames; moved on 207 of 207 pairs | 92 |
| inside the sphere reads Inner | two materials, opposite compiler polarity, one rule | 82 |

They ship as
[`oag_hd::race::ZONE_TRANSITION`](../../crates/hd/src/race.rs), an
[`oag_title::ZoneTransition`](../../crates/title/src/race/transition.rs) -
`Some` on HD/Fury and `None` on 2048, whose own rate (`DAT_816c6bc8`) is
unread and is left an absence rather than borrowing HD's numbers.

### How this port runs it

[`ZoneGrade::follow`](../../crates/raceplay/src/zone_grade.rs) derives `k`
from the race's own zone counter and zone clock - the zone the showing stage
began at on the ladder, times the 600 ticks a zone lasts, plus the ticks into
the current zone - rather than counting frames. Three reasons, each of which
a per-frame counter would get wrong: a `--screenshot` capture builds its
scene *after* the whole tick loop and syncs the grade once; the windowed loop
may tick twice or not at all between two renders; and a paused race must
freeze the sphere, which a clock that stops does for free. Nothing enters
`World`. The uniform then carries both stage pairs, the craft's position as
`origin` and the radius, and `mesh.wgsl` selects per fragment.

**The one substitution made**: the original advances per PS3 frame on a
variable timestep; this port's frame is its fixed 60 Hz tick
([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)). Stated,
not measured.

**Chosen, not measured: the opening stage is shown whole.** A race opens on
the ladder's zone-0 stage (`Sub Venom`), and this port settles it with no
sphere in flight. The live trace suggests the original does run a commit at
race start - its settled `burst-0` reads a speed counter of `95.9` some
sixteen seconds in, which is `k = 954` from a commit at about the race's
first frame - so a `Start`-to-`Sub Venom` sweep during the countdown is the
likelier original behaviour, and whether the player ever sees `Start`'s
black outside it before the sphere clears the visible track is unmeasured.
The maintainer's own observation is `Sub Venom`'s cyan already on the start
line, which is what settling gives.

### Open

- **The radius's unit.** The shader compares it to a world-space distance in
  the original's own units; whether this renderer's `.rcsmodel` world is in
  the same units is unverified, and the value is passed through unscaled.
  One cheap measurement bearing on it: the port's craft moves 1.6 to 2.3
  units a tick at zones 0 to 2 on Talon's Junction (from `--log-every`
  telemetry), against the PS3's measured 1.1 to 1.4 units a frame at zone 2
  on Vineta K - the same order, on different circuits at different speeds.
- ~~**The stage texture does not follow the sphere.**~~ **Fixed 2026-09-15.**
  `StageArt` now carries all four textures - `track`/`scene` (Inner) and
  `track_outer`/`scene_outer` (Outer) - bound at `mesh.wgsl`'s bindings
  1/10 and the new 11/12, and `zone_sample`/`zone_glow` select per fragment
  on the same `zone_inside` test the colours use. `ZoneGrade::stage_art_pair`
  fills the Outer pair from the wavefront's previous stage, falling back to
  the Inner texture per slot when there is nothing to sweep out. The
  stage-change edge rebuilds bind group 2's four texture views alone
  (`oag_mesh::mesh_render::zone::rebind`, called from
  `oag_raceplay::Scene::rebind_zone_art`) rather than rebuilding the whole
  drawable, gated on the same edge `sync_zone_grade`'s own log line already
  fires on - never every frame.

  **Confirmed wired, not confirmed visible.** A live log at the rebind call
  site shows the exact right pair bound at Talon's Junction zone 2's step
  (`track=zonemodetrack2.gtf track_outer=zonemodetrack1.gtf`), and
  `crates/mesh/tests/zone_recolour.rs`'s
  `the_transition_sphere_samples_the_inner_texture_inside_and_the_outer_outside`
  proves the shader selection on a real adapter with two solid-colour
  textures. But `hd_zone_wavefront_zone2_k150.png` and `..._k300.png`,
  reproduced against this fix, are **byte-identical** (ImageMagick
  `compare -metric AE`, 0) to the pre-fix captures - `zonemodetrack1.gtf` and
  `zonemodetrack2.gtf` do differ on disc (distinct SHA-256, first differing
  byte 41,089), so the texture change is real but not visible in either
  frame. The likely reason is the surface equation's own additive Track
  multiplier: `Start` alone is known to author `Track.Texture Colour` at
  `9.0` (`docs/formats/effectsettings.md`), and any texel that large
  saturates the linear-light sum before the sphere's own colour step (which
  *is* visible in both frames) ever reaches the output - so a texture swap
  under a saturating multiplier can be real and still draw the same pixel.
  Unmeasured for zones 1-2's own multipliers; worth a frame at a stage pair
  with a smaller one, or a `zonemodetrack*` pair viewed head-on rather than
  at the grazing angle a chase camera puts a road surface at, where the
  `rim^5`/`rim^10` colour summands already dominate over the texture term.
- ~~**The glow's sphere term is read but not drawn.**~~ **Drawn 2026-11-15.**
  The term is `5.0 * saturate(1 - 0.1 * |distance - zoneColourTint.w|)`: the
  first reading lost the source `abs`, which made it a fill of the sphere's
  interior. With it the term is a white band ten units either side of the
  wavefront, the edge that sweeps out with the new stage's colours.
  Evidence: [zone-shader.md](../ghidra/functions/ps3-hdfury-eu/zone-shader.md#the-wavefront-ring).
  Headless on Talon's Junction (`--ticks 1500` to `1640`) the band is first a
  circle on the road round the craft, a radius of a few tens of units, and
  then moves out past the near track with the lime following it. That puts
  the zone 2 step just before `--ticks 1500`, not at 1200 as the table below
  has it; why the two disagree is not measured. Drawn only while the radius
  is positive, so a title with no transition law (radius `0`) draws no ring
  round the craft.
- **Which live frame confirms the polarity.** `burst-2.png` at radius
  `3288` does: the near road green and the near walls lime (`Venom`'s
  `Track.Texture Colour` and `Scene.Texture Colour`), the far scenery still
  `Sub Venom`'s cyan. `burst-1.png` at radius `799` is *not* discriminating
  and should not be read as a counter-example: the boundary sits past most
  of the visible road, and the near track's teal there is `Venom`'s own
  `Track.Base Colour` (`0.0039 0.8118 0.7255`) through the `rim^5` summand a
  road shows at a glancing angle - the thirty-first pass's "the new stage's
  teal" is the same reading. Comparing a road chunk against a wall chunk
  says nothing about the sphere either way, since the two read different
  colour groups of the same stage; only same-class near against far does.

## Seen, headless

`just play hd --race --mode zone --track /data/environments/talons_junction/track.vex
--autopilot --no-audio --ticks N --screenshot ...`, with zone 2 (`Sub Venom`
to `Venom`) stepping at tick 1200. Frames under `data/shots/` (gitignored, as
every capture this tree cites), `hd_zone_wavefront_zone2_k<N>.png`:

| tick | `k` | radius | what the frame shows |
| --- | --- | --- | --- |
| 1210 | 10 | 9.6 | all `Sub Venom` cyan; the sphere is under the craft and hidden by it |
| 1260 | 60 | 207 | the road and the near tunnel walls in `Venom`'s lime and green, the far tunnel still cyan |
| 1350 | 150 | 1,193 | a curved boundary across the road ahead, lime near it and cyan beyond; the building on the right lime, the far tunnel cyan |
| 1500 | 300 | 4,635 | the near circuit entirely `Venom`, the distant skyline still `Sub Venom` cyan with the boundary across the buildings |

That is a boundary sweeping outward from the craft with the new colours
inside it, at the radii the law predicts - in this renderer's units, which is
the open question above.

**Reproduced 2026-09-15 against the stage-texture fix**, the `k150` and
`k300` rows only (the pixels a chase-camera road shows this circuit's own
Track multiplier through, per the struck Open item above): both frames came
back byte-identical to the ones this table already names, which is the
finding, not a broken repro - see that item for the saturation reading and
what would tell the two apart.

## Chunks a Zone race does not recolour (2026-11-16, `zone-pad-re`)

**Not every chunk takes the Zone surface.** Every draw consumer of the
executable drops the `ZoneMode` bit of the material permutation word for a
chunk whose render-block flags have any of bits 9, 10 or 11
(`Mesh::render_flags & 0x0e00`), so that chunk runs its material's plain lit
program with its own albedo (confidence 90: the instruction at `0x00401760`
and its six siblings, and three RSX frames of a Zone race on Moa Therma in
which the speed pads and the bit-11 signs are the only plain draws). The bits
are the speed pads (418 chunks), the weapon pads (208) and 315 authored
signs, billboards and screens.

What such a chunk shows is its own picture under the Zone stage's rig, with no
lightmap term (a Zone frame switches the lightmap and baked-vertex sources off
for every draw):

```text
light  = stage Constant Ambient Colour + saturate(N.L) * stage Sun colour
colour = albedo * light + the material's own glow and specular
fog    = the stage's Track fog on a track-flagged chunk, else its Fog (or Alt Fog on bit 5)
out    = 0.5 * fogged colour
```

At Sub Venom (`Constant Ambient Colour` 1.5, `Sun colour` 0, `Track Fog
density` 0) a speed pad is `0.75 * albedo + 0.5 * ne.a * Colour`: the
light-grey plate with a dark outline and cyan tiles the original shows.
Evidence, the per-material census and the 2048/Omega check:
[zone-plain-chunks.md](../ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md).

**Wired** (`slots::ZONE_PLAIN`): HD's flagged chunks off the render flags, and
the pads of 2048's container (Omega's and 2048's HD-lineage circuits,
inherited from HD) off the pad's material or shape name; see
[zone-plain-chunks.md](../ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md),
"2048 and Omega". The stand-in for the start gantry's card
(`oag_mesh::mesh::ZONE_EXEMPT_RATE`, chosen) is probably the bit-11 flag,
unchecked.

## The Zone surface's light, and the frame's post chain (2026-11-19, `zone-grade-sky`)

**A chunk on its Zone program is lit by the stage alone, and the frame is
resolved at `0.6` of its light.** `Scene_PrepareFrame`'s Zone arm switches
every draw's lightmap and baked-vertex sources off, writes the `HDR and Bloom`
block from a table of its own (adaption boost `5.0`, blur sizes `1.0`,
whatever the circuit authors) and sets `Bloom feedback` to `-0.4`, which the
resolve turns into `1.4 * scene - 0.8 * quarter-res scene`:

```text
light   = stage Constant Ambient Colour + saturate(N.L) * stage Sun colour
colour  = light * surface + glow          ; surface as in the first section
out     = 0.5 * fogged colour
final   = saturate(lerp(scene, 2 * feedbackBuffer, -0.4) * exposure + bloom)
```

At Sub Venom (`Constant Ambient Colour` 1.5) a scenery chunk is `0.75 *
zoneTex * Scene.Texture Colour`: Vineta K's buildings read near white, as the
original draws them, where this port lit them by the circuit's lightmap and
showed them grey-brown. Addresses, the live read (RPCS3, the settings block
and the scene target of one paused frame) and the confidence per law:
[zone-frame.md](../ghidra/functions/ps3-hdfury-eu/zone-frame.md). Wired as
`oag_title::ZoneFrame` (`oag_hd::race::ZONE_FRAME`), HD/Fury alone; the pair
`hd/zone-vineta-k-z5` moves 28.77 -> 7.42.

## The light-bar term in a Zone race (2026-11-19, `zone-pair-score`)

**A chunk drawn through the Zone surface adds no `_ne.a * colour` light bar.**
The lit `_ne` programs (the pads, and `diffuse_normal_specular_emmissive`'s
rails, start line and start bays) add `_ne.a * colour` after the light, the
colour a material parameter ([pads.md](pads.md)). Across all four PS3
archives, 77 materials have a lit variant that adds it, and **none** of their
Zone variants (those declaring `zoneColourTint`) gates that same parameter on
`_ne.a`: confidence 85 (a census of the disc's own fragment programs,
`crates/mesh/tests/hd_zone_ne_census_ground_truth.rs`, plus one live frame).
On Vineta K's `diffuse_normal_specular_emmissive` (lit colour parameter
`0x7611a2d8`) eighteen of the twenty Zone variants gate nothing on `_ne.a` and
two gate `zoneBaseInner` (`0x1cc42e87`, renderer.md's hash table) instead.

The live frame: the adopted pair `hd/zone-vineta-k-z5` shows the start bays
under the craft as flat Zone teal with dark seams, where this port drew the
bays' blue `LIFT` light bars (`ds_pit_cs`, `_ne` `ds_pit_ne.gtf`). Isolated by
`OAG_ONLY_MATERIAL=ds_pit` and by zeroing each post-light additive in turn:
only the `_ne` term carried the blue.

`shade.wesl` multiplies the term by `1 - zone_program`, the mode-12 rule's
shape. The speed and weapon pads keep their bars, because their chunks draw
the plain program in a Zone race (the section above). The pair's mean grey
moves 37.40 -> 36.83; the start-bay crop matches by eye.

**Open**: the two Zone variants that add `_ne.a * zoneBaseInner` are not
drawn. Which variant a chunk runs depends on the permutation word (lighting,
shadow), unread for these chunks; the frame shows no rim-coloured bar on the
bays.

**2048 and Omega: not checked here.** The census read HD's four PS3
archives only. Whether 2048's PSP2 programs or Omega's Zone variants add a
light-bar term is open; this port's `_ne` path (`mesh::rcs::pad_ne`) binds on
HD materials alone, so neither title's picture moved.

## On 2048's container (2026-11-19, `zone-road-2048-omega`)

An HD circuit that 2048 (DLC1's `Vineta_K`) or Omega (`01_vineta_k`) ships in its own container draws
HD's recolour: DLC1's fragment programs declare HD's Zone family (`zoneEffectInner`, `zoneTexInner`,
`zoneOrigin`, `zoneColourTint`, `zoneAnisoLookupTex`) and none of 2048's `zoneGrowing*`
([2048-material-params.md](../formats/2048-material-params.md), "The Zone surface"). What the
container lacked was HD's per-chunk Scene/Track choice (the render-block bit above): without it every
chunk took the `Scene` group, so the road drew `Scene.Texture Colour` flat, a pale sheet
(`2048-vita/zone-dlc-vineta-k-z5` 86.04, the road boxes' mean grey 161 and 135 against the original's
93 and 58).

The mesh object's `+0x0a` flags are `0x0101` or `0x0201` on the road and on everything else alike
(DLC1 Vineta K, 1,822 mesh objects: no material correlates), so they are not the bit. **The shape name
stands in: a mesh object whose Maya shape name contains `track` (`wohdtrackShape`, `TRACK_0Shape`,
`TRACK_Shape`) takes the Track group** (`oag_mesh::mesh::rcs::psp2::is_track_shape`). **Chosen, not
measured**: no confidence score. The result is the original's picture in structure: dark road, the
stage's yellow line, the lime wall (Scene group) beside it. The hash join to HD's own
chunks does not exist (0 of 1,822 DLC1 mesh hashes meet HD's 1,811 chunk hashes), so the rule could not be
checked against HD's `is_track` chunk by chunk.

| pair | before | after | road box left / right, mean grey: original, before, after |
| --- | ---: | ---: | --- |
| `2048-vita/zone-dlc-vineta-k-z5` | 86.04 | 62.92 | 93 / 58, 162 / 135, 38 / 25 |
| `omega/zone-vineta-k-z5` | 74.25 | 72.92 | 39 / 53, 118 / 94, 30 / 47 |

**Omega's colour is still wrong, and the stage is not the cause.** The structure matches (dark road,
lime lines) but the hue is HD's Flash purple where the original is olive green (frame mean
`87 103 39`). Rendered at each of stages 1 to 12, none matches; the closest is stage 9 (`71 63 25`,
mean absolute difference 52.5 against 71.4 at stage 4). **Omega's own ladder was then read** (lane
`omega-zone-ladder`, [zone-speed-class-ladder.md](../ghidra/functions/ps4-omega-eu/zone-speed-class-ladder.md)):
its Zone HUD walker carries HD's fourteen thresholds and stores `14 - i`, so zone 5 is stage 4 on Omega
too. The gap is the surface, not the ladder: with the shape-name Track rule off (every chunk on the
Scene group) the walls turn olive (mean RGB `114 139 39` against the original's `38 58 6`, hue right,
three times too bright) and the road pale blue-white against the original's dark olive, so neither
group's HD law reproduces Omega's road. Omega's GCN surface programs are the unread part. rmse
0.340 -> 0.372 on that pair is this hue gap.

**HD is unchanged**: `hd/zone-vineta-k-z5` re-rendered with this change is pixel-identical to main's
(0 of 2,073,600). The rule lives in the psp2 loader, which HD never takes.

## What a Zone race costs (2026-11-26, `zone-cost`)

**The question.** On a Steam Deck, HD's Zone race on Chenghou Project holds about 75 % under
dynamic resolution where a time trial on the same circuit holds 100 %. The Deck was unreachable
for this work, so everything below is the desk machine: an RX 7800 XT and the same CPU's Raphael
iGPU (2 CU, RDNA2, the closer relative of the Deck's APU), RADV, Mesa 26.3.1, shared with other
lanes (load 6 to 45). Autopilot, seed 1, `--no-audio`, default settings.

**Only the race pass grows.** Per-pass GPU time, `--frame-log` in a window under a headless
weston, means over two runs of 44 s each. Read this table for which passes move, not for absolute
cost: `frame_limit = "off"` is not a valid value, so both devices ran under the default 240 fps
limiter, and the 7800 XT, idle for most of each frame, reads well above the offscreen figures
below (likely down-clocked; clocks not recorded). The rows listed add up to 1.0 ms more in Zone on
the iGPU: the race pass's 1.5 less what the other passes save.

| ms | 4K, 7800 XT: time trial / Zone | 1280x800, iGPU: time trial / Zone |
| --- | --- | --- |
| race pass | 1.16 / 1.27 | 5.09 / 6.62 |
| HD bloom (with the Zone feedback resolve) | 0.30 / 0.28 | 0.58 / 0.63 |
| advert cards | 0.05 / 0.00 | 0.48 / 0.05 |
| glass | 0.03 / 0.03 | 0.70 / 0.60 |
| HUD and UI | 0.08 / 0.04 | 0.24 / 0.14 |
| shadow map, self shadow | 0.02, 0.01 / the same | 0.07, 0.06 / 0.10, 0.09 |

The offscreen bench (`OAG_RENDER_BENCH_RACE`, 5,400 ticks, two runs within 3 %, 4K) agrees:
race pass 877 us in Zone, 675 us in the time trial, bloom 200 and 193. Omega's Zone on the same
circuit reads 730 us (its tone map is not in that bench). At 1920x1080: 365, 335 and 320 us.
The bloom timer closes on `hd encode`, so the Zone feedback blend and the Zone bloom block are
inside the flat bloom figure.

**Speed is not it.** The race pass is flat from zone 1 to zone 9 (every 600-tick bucket), no
stage change shows as a spike, and with `--motion-blur high` the blur costs the same in both
modes (1,017 and 1,020 us at 4K).

**Inside the race pass it is the Zone surface itself.** Throwaway shader switches at 4K: Zone
889 us; without the visualiser glow 848; without the stage sample, rim terms and glow 739; with
the whole recolour compiled out 716, against the time trial's 703. So about 170 us of the
extra is the work of the first section above, per pixel.

**The original pays more per pixel in Zone too.** On Chenghou's lightmapped surfaces, comparing
each material's lit race program with its `ZoneMode` one (30 pairs): no Zone program samples the
lightmap, none fetches fewer textures, and all but `mageffect_modded` are longer
(`lambert_simple` 27 instructions and 2 fetches against 39 and 3, `diffuse_specular` 43 and 2
against 50 and 4, `diffuse_normal_specular_emmissive` 50 and 3 against 59 and 5). The flat look
is the stage texture replacing the albedo, not a cheaper program. This is a count of the
programs, not a timing of the PS3. Held by
`crates/mesh/tests/hd_zone_ne_census_ground_truth.rs`.

**What was ours: the light chain.** The circuit's pipelines still computed the lightmap, prelit
and vertex light chain and then overwrote the result with the stage's ambient and sun, a branch
on a uniform the driver cannot drop. Where the grade is stage-lit and enabled on every stage it
can show (`ZoneGrade::lights_every_stage`: HD and Fury), the circuit is now built with
`zone_stage_light`, which states the light outright. An emissive plain chunk (a pad, a sign)
takes neither Zone branch, so it is named as the `1.0` and no sun its program leaves, and a model
where an earlier branch relights such a vertex is not specialised
(`specialise::zone_light_exact`). All 16 HD circuits and arenas tried, and a Detonator race,
specialise.

Race pass before and after, Zone, interleaved runs (median of per-frame times):

| Device, size | before | after | pairs where after is faster |
| --- | ---: | ---: | --- |
| 7800 XT, 3840x2160 | 870 us | 819 us | 3 of 3 |
| 7800 XT, 1920x1080 | 350 us | 343 us | 6 of 6 (one a tie) |
| iGPU, 1280x800 | 6,485 us | 6,168 us | 7 of 8 |

One run is `OAG_RENDER_BENCH_RACE=<stem> oag-game <hd iso> --race --mode zone --track
/data/environments/04_chenghou_project/track.vex --seed 1 --autopilot --no-audio --size <size>
--ticks <n> --screenshot out.png` (`--mode time_trial` for the reference, `MESA_VK_DEVICE_SELECT`
to pick the device), reading `gpu_scene_us` from `<stem>.csv` after the first 60 frames. The two
builds alternate, under the gate lock: 3,600 ticks at 4K (load 12 to 18), 1,800 at 1920x1080
(load 7) and on the iGPU (load 13).

**Picture: unchanged.** 32 frames, before against after, 0 differing pixels: Zone on Chenghou
(ticks 150 to 7,300), Talon's Junction, Vineta K, Anulpha Pass,
`zone_1` and `zone_4`, a Detonator race, a two-player split, `--circuit-lighting simplified`, an
HD time trial and single race, and, on titles that never specialise, a Zone race and a time trial
on Omega and a Zone race on Pulse.

**Does the difference explain the Deck?** Partly. With dynamic resolution on (iGPU, 1920x1080,
`target_fps` 60, 70 s runs, load 8 to 37; mean linear scale from the frame log's `render=`):

| Build | time trial | Zone, two runs | Zone seconds below full |
| --- | ---: | ---: | ---: |
| an earlier session, before | 0.998, 0.995 | 0.967, 0.959 | 30 and 34 of 66 |
| before | 1.000 | 0.959, 0.957 | 29 and 32 of 64 |
| after | 1.000 | 0.962, 0.977 | 33 of 61, 21 of 64 |

Zone is the arm that leaves full size, the same direction as the report, and the fix does not
move it by more than the runs differ from each other. A scalable cost 25 to 30 % higher puts a
controller at about 0.88 to 0.9 of the scale when the time trial sits exactly at its budget, not at
0.75. On the Deck itself it settles at about 0.80 at 3840x2160; the extra depth is the GPU
clock falling under the controller (with the clock pinned it settles at 0.96)
([dynamic-resolution.md](dynamic-resolution.md#why-the-4k-settle-is-so-deep-the-gpu-clock-falls-with-the-scale)).

**Not measured, and open:** what a separate Zone program for the circuit would save. The Deck and
sound are measured
([dynamic-resolution.md](dynamic-resolution.md#on-the-steam-deck-hd-zone-against-a-time-trial)):
the race pass grows by 1.60 ms there at 3840x2160, and a Zone race without sound settles at the
same scale. Dropping the lightmap fetch and the albedo mix as well
measured about 45 us more at 4K, but skipping the fetch moves pixels: our Zone surface still gates
its specular by the lightmap's alpha, where the original's Zone programs sample no lightmap.
