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
rim     = 1 - dot(N, toEye)
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

[`ZoneGrade::follow`](../../crates/game/src/race/zone_grade.rs) derives `k`
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
  `oag_game::race::Scene::rebind_zone_art`) rather than rebuilding the whole
  drawable, gated on the same edge `sync_zone_grade`'s own log line already
  fires on - never every frame.

  **Confirmed wired, not confirmed visible.** A live log at the rebind call
  site shows the exact right pair bound at Talon's Junction zone 2's step
  (`track=zonemodetrack2.gtf track_outer=zonemodetrack1.gtf`), and
  `crates/render/tests/zone_recolour.rs`'s
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
- **The glow's sphere term is read but not drawn.** `5.0 * saturate(1 -
  0.1 * (distance - zoneColourTint.w))` now has both inputs bound, and as
  read it adds `5.0` to every fragment *inside* the sphere rather than at its
  edge - the RPCS3 frame at radius `799` shows no such flood. The reading is
  what is in doubt; it stays out until re-read.
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
