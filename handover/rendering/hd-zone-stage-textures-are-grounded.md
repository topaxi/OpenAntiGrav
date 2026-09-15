# HD's per-stage Zone textures are grounded now: which file feeds which sampler is read, what the shader does with them is not

2026-08-31. Split out of
[2048-and-hd-ship-an-unread-effectsettings-table.md](2048-and-hd-ship-an-unread-effectsettings-table.md)
once that thread's tint-consumer step landed and the same mechanism answered a
second question in passing. That thread stays open on its own remaining
question (HD's Zone stage trigger); this one carries the texture half, which
is now a different kind of work.

## What is established

Full evidence, addresses and reproduce commands in
[zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
twenty-second and twenty-third passes. In short:

- **The renderer binds per-stage Zone textures through named shader
  parameters**, published into the 81-entry engine parameter array at
  `*(0x008b7f04) + 0x4000`. A sampler's handle goes to entry `+0x1c` (a float
  parameter's value pointer goes to `+0x18`) - the union that made an earlier
  sweep miss every sampler in the image.
- **`zoneTexInner`/`zoneTexOuter` index `g_ZoneStageTextures`
  (`0x00c81368`)**, the fifteen handles `Environment_LoadStageTextures` fills
  from `Data/Tex/zoneMode{0..14}.gtf`; `g_ZoneStageTrackTextures`
  (`0x00c813e0`) is the same for `zoneModeTrack{0..14}.gtf`. Confidence 82.
- **The Track set is the one with the art.** This thread's parent already
  decoded all fifteen `zoneMode*.gtf` as byte-identical flat white and all
  fifteen `zoneModeTrack*.gtf` as distinct. `Scene_PrepareFrame` binds only
  the blank set; seven other publishers bind the Track set to the *same*
  parameters 60-63 in a second block. **A port that follows
  `Scene_PrepareFrame` alone will bind fifteen blank textures and see
  nothing**, which is exactly the failure that looks like a wiring bug and
  is not one.
- **`zoneTexVis` is generated, not loaded**: a 256x1, `format=8` texture,
  zero-filled at load time and **rewritten every frame** by
  `Environment_UpdateStageBlend` - corrected 2026-08-31 from an earlier
  reading that named `0x003d8b40` as its build loop; that address is a
  different, unrelated object. A port has to build it; there is no disc file
  to look for. Confidence 74 on the per-frame rewrite; see this file's own
  2026-08-31 section below for the full correction.
- **The `...Nearest` parameters are the same texels with patched sampler
  state**, not separate art (confidence 80 on the clone relation; the claim
  that the patched bitfields are specifically *filter* fields is confidence 45
  and deliberately unnamed).
- **Detonator aliases Zone**: `DetonatorMode{0..14}.gtf` is written into the
  same fifteen slots by a later branch of the same loader, so the parameter
  named for Zone carries Detonator's art when Detonator loaded last.

**2026-09-14, a rendering note for whoever picks this up**: until this
day the Zone floor drew *lavender* - not any Zone art, but
`tracktexture_with_normal`'s own `tracknormal.gtf` added as a glow by
`mesh::rcs::emissive` (the Zone tracks ship no lightmaps, so `Pick::aux`
lands on the normal map). That accumulate is refused now (`emissive.rs`,
`CIRCUIT_SURFACE_MAP_SAMPLERS`), and the floor draws its real diffuse -
`tracktexture.gtf`, a 1x1 black texel. **Black is the honest state**: the
whole Zone look is this thread's unread shader, and nothing else in the
material record paints it. Same for the pads' `weapon_pads` `_ne` map on the
four Zone tracks.

## What is *not* established, and why that bounds the work

**The Zone shader itself has never been read.** We know which texture arrives
at which named sampler; we do not know what the fragment program does with
`zoneTexInner` versus `zoneTexOuter`, how `zoneBaseInner`/`zoneBaseOuter`/
`zoneEffectInner`/`zoneEffectOuter`/`zoneOrigin`/`zoneAnisoPower` parameterise
it, or how `zoneColourTint` combines. So **the honest scope of an
implementation right now is the asset side**: decode and hold the correct
per-stage texture, exposed on the same seam the grade already uses. Inventing
a plausible-looking Zone effect from the parameter names is exactly the
stand-in `CLAUDE.md` rules out - and the parent thread already has one
recorded instance of a "legible" invention surviving review.

Also still unfound, and shared with the parent thread: **what writes
`H[e].u32@28`/`@32`, the stage index the samplers actually use**
(`H = 0x008c2cb8`, two 56-byte per-environment structs). The loader only
zeroes them. So the texture side terminates at the same unrecovered
stage-number source as the palette side does - which is fine for
implementation, because `oag_game::race::zone_grade::ZoneGrade` already
carries an **explicit** stage index and weight with no caller
(`request_stage`/`set_weight`), and the textures can ride that same seam
until HD's real trigger is recovered.

## 2026-08-31, later: the asset half is wired, and the maintainer's own play named the effect

**Wired.** `oag_title::ZoneStageTextures` carries both entry-name sets and the
stage count (`oag_hd::race::ZONE_STAGE_TEXTURES`, `None` on every other title);
`oag_game::race::load::environment::zone_stage_art` loads the **track** set
through `oag_render::mesh::ModelTexture::from_gtf` (promoted out of
`mesh::rcs::skin` so a second caller can reach it); and
`ZoneGrade::stage_art()` hands back the showing stage's texture. Checked
against `hdfury-ps3-eu-dec.iso` in
`crates/game/tests/zone_grade_ground_truth.rs`: all fifteen decode at 256x256
and **no two are the same picture**, which is the assertion that catches a
regression re-pointing the loader at the blank general set. Nothing draws them
- the load report says so in as many words.

**Also wired, on the palette side**: the three `Scene.*` keys HD's own
cross-fade actually reads were not being parsed at all. They are now
(`Scene.Texture Colour`, `Scene.Base Colour`, `Scene.Base Colour Highlight`,
plus `Scene.EQ brightness`), and `ZoneGrade::scene_tint()` assembles the exact
`float4` the original publishes as the shader parameter `fogColour`. Reported,
not drawn.

**And the effect has a name now.** From the maintainer playing HD/Fury: in a
Zone race the floor textures and billboards carry an **audio-spectrum
animation driven by the music**. That reads the whole `EQ` cluster at once -
`EQ` is an equaliser, `Scene/Track.EQ brightness` is how hard it drives (`0.0`
on `Start`, `20.0` from `Sub Venom`), `EQ colour tint`/`analogue colour tint`
are its colours, 2048's `EQ.Mid-Band Position` is a spectrum-analyser term with
no other reading, and `zoneTexVis` being a **256x1 runtime-built** texture is
exactly the shape a per-band lookup wants. The `...Nearest` clones fall out
too: a band lookup wants point sampling, which is what a nearest-filtered clone
of the same texels is for. Confidence 90 that the effect exists, 70 on the
key-by-key mapping. Full table in
[effectsettings.md](../../docs/formats/effectsettings.md)'s `## The EQ keys are an
audio spectrum, observed in play`.

**This raises a maintainer decision rather than a task.** A shader reading an
audio spectrum needs frequency-domain data per frame, and
[ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md) points
the audio path the other way - cues are a per-tick *output* of the simulation
and the mixer is hardware-free by design. Who runs the analysis, and whether it
may sit on the render side reading the mixer's output buffer, is a call for the
maintainer; a new ADR is the likely shape. **It is also a determinism
question**: anything the simulation can see must not depend on the audio
device, so the spectrum has to stay render-side or be derived from the sample
stream deterministically.

## 2026-08-31: compared against the original, and the dominant term is the one we do not draw

The maintainer supplied a real HD/Fury frame - a Zone race on Moa Therma at
the start line - and it is the first side-by-side this thread has had. Both
halves are in `data/shots/` (gitignored):
`hd_zone_moatherma_stage1_original.png` against
`hd_zone_moatherma_stage1_ours.png`.

**First result, and it corrects an assumption made earlier in this thread: the
original is not on stage `0` at a start line.** Its scene is dominated by a
strong cyan with a white blowout, and the disc names that exactly:
`1 Sub Venom.Scene.Base Colour` = `0.003922 0.847059 1.000000`, with
`Scene.Base Colour Highlight` = `0.000000 1.694118 2.000000` - over `1.0`, so
it blooms out to white. `0 Start` authors a flat `3.0 3.0 3.0` with no hue in
it whatever. So an HD Zone race opens on `Sub Venom` or later, which is
independent evidence about the unrecovered trigger: **whatever drives it does
not leave the grade where the loader put it.** The earlier note here that a
start-line comparison would be inert because `Start` authors a black
`Scene.Texture Colour` was wrong, and **wrong for a second reason found the
same day**: the port was reading the wrong half of a pair. `Scene.*` and
`Track.*` are not alternatives a consumer picks between - the texture set and
the colour group are one choice, and the set this port binds is the `Track`
one. `0 Start.Track.Texture Colour` is `9.000000 9.000000 9.000000`, the
brightest multiplier in the whole table, where its `Scene` sibling is pure
black. So the stage that reasoning called inert is the one the disc drives
hardest.

That the original is on stage 1 or later survives the correction, which is
worth stating because the correction could have overturned it: `Start` authors
black base colours in **both** groups, and the frame is strongly cyan.
`Sub Venom` authors `0.003922 0.847059 1.000000` for `Base Colour` in both
groups - the same cyan, from either half. The stage conclusion never depended
on the pairing.

**Second result, and it is the actionable one.** Ours draws the recovered
textured rule faithfully - the cyan appears, and only where the material's
albedo is black, which is what the microcode says. But the original is
*near-monochrome*: its crowds, its orange and red trackside banners, its
advertising boards and its grey concrete are all subsumed into cyan, black and
white. A term that only adds where the albedo is already black cannot do that,
so **the look is carried by something this port was not drawing**.

**What that something is was got wrong first, and the correction is the useful
part.** This entry originally said the missing piece was the untextured
`zoneBase*`/`zoneBaseAlt*` rim terms. It is not. A census over the whole disc
settled the real rule, and it is *per environment*, with two mutually
exclusive shapes:

- **All twelve racing circuits** compile `surface = zoneTex * zoneEffect +
  zoneBase * rim^10 + zoneBaseAlt * rim^5`, with **no albedo term at all**.
- **`zone_1`..`zone_4`, HD's four Zone arenas**, compile the black-mask shape
  (`albedo + zoneCol * (1 - blackMask)`) - 130 blocks of 20,214.

The three materials the shader page was written from included two arena ones,
which is how the rare shape got published as the rule.

And the rim terms are **not** what carries it either: `rim` is near zero
head-on, so they are silhouette-only. **What carries the look is the albedo
being absent from the equation** on every racing circuit - which is exactly
why the original's crowds, banners and advertising vanish while ours kept
showing them. Confidence 86 on the split, **82** on the missing albedo:
there is no dataflow proof that the ~8% of blocks which do fetch albedo RGB
keep it out of the surface.

So the gap was not an unfed input, not the visualiser, and not a missing
material family - it was one term too many in the surface equation.

**Tooling this produced**: `--zone-stage N` (`oag_game::race::Options::zone_stage`).
HD rests wherever its loader left the grade, so before this there was no way to
put an HD Zone race on a named rung at all, and no way to compare a frame
against an original that is visibly on one.

## 2026-08-31: the Zone sky is answered - it is a **file swap**, and the gradient is a second, unread mechanism

Full evidence in
[zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md). The Next
Steps item below is done; what it asked for - read it, do not implement from
the key names - is what happened, and the answer was neither of the two options
that item framed.

- **A Zone race loads `Data/Tex/ZoneSky.gtf` instead of the circuit's own
  `sky.gtf`.** `Environment_LoadRaceScene` (`0x003f3fb0`) branches on one byte
  and calls the *same* loader with the *same* `"skycube"` tag into the *same*
  handle slot under the *same* sampler-state patch, so the picture is the only
  thing that changes. It is 64x64 where `01_vineta_k`'s is 2048x2048 - 1,024
  times fewer texels a face - which is the measured form of "solid colours or
  gradients". Confidence 84.
- **The gate is `g_ZoneEffectsActive` (`0x00d45f84`), and its writer is found**:
  `(1 << mode) & 0x00206040`, i.e. mode ids **6, 13, 14 and 21**.
  `mode-manager.md` had guessed 6, 13 and 14 were Zone, Zone Battle and
  Detonator and marked it *not established*; this mask is independent evidence
  for it, and 14 is already pinned as Detonator. Which of 6 and 13 is which is
  still unknown. Id 21 does not fit and is recorded, not explained.
- **The horizon/zenith gradient exists too, and its consumer is located** - the
  four vec4s are read at `0x003adf58` behind the same gate, scaled, lerped
  inner-to-outer on the CPU, packed into two RGBA words and passed to
  `FUN_005ebd58`, which has exactly one caller in the image. **What it draws is
  unread, so this port draws no gradient and says so.** That is the honest
  result, not a gap in the work.

**Two of this thread's own premises were wrong and are corrected there**:
`zone_3/sky.gtf` is 262,784 B, not the 12,583,040 the other three are, so "all
four arenas share one cubemap" does not generalise - and more usefully, the
branch being either/or makes all four arenas' `sky.gtf` **dead art in Zone
mode**. They are never what a Zone race shows.

**Wired**: `oag_title::RaceDefaults::zone_sky`, `oag_hd::race::ZONE_SKY`, and
`hd_sky_model` taking it when `Mode::Zone`. Checked against
`hdfury-ps3-eu-dec.iso` in `crates/game/tests/zone_sky_ground_truth.rs`: a Zone
race and a Single Race on the **same named circuit** resolve different cubemaps,
the Zone one 64x64 against `01_vineta_k`'s 2048x2048, and five of its six faces
carry real art rather than a flat fill. The size is the load-bearing assertion:
no `sky.gtf` on the disc is 64x64.

**Deferred, with the reason nailed down**: the sky yaw sign. The maintainer was
asked for a reference frame and could not capture one now, so **every in-race
frame already in `data/reference/hd-capture/` was looked at instead** - all
seven. None can settle it: `anulpha/00..02` are inside an enclosed tube with no
sky, and the four Talons Junction frames show its sky only as a blown-out white
haze behind structure, the same failure Vineta K's produced. So this is not a
matter of choosing a better frame from what is on disk; it needs a new capture,
on a **racing circuit**, with open sky and one measurable feature. A Zone frame
can never serve now that the Zone sky is measured at 64x64. Written up in
`crates/render/src/mesh/sky_cube.rs` so it is not re-attempted.

## 2026-08-31, later: the HUD names the same fifteen rungs, and two of them are pinned to zone numbers

Split off into
[hds-zone-ladder-draws-and-the-zone-to.md](../frontend/hds-zone-ladder-draws-and-the-zone-to.md),
which is about the widget; what belongs here is the evidence it produced about
**this** thread's open question, the stage index nothing writes.

- **The fifteen `zonemode.effectsettings` rungs are speed classes with names in
  the language plugin**, one for one: `MSC_SVENOM` = `SUB-VENOM` against
  `1 Sub Venom`, through `IG_HUD_MACH1` = `MACH 1` against `13 Mach 1`, to
  `IG_HUD_SUPSON` = `SUPERSONIC` against `14 Supersonic`. So the palette ladder
  and the HUD's speed-class ladder are the same fifteen rungs, which is what
  makes the HUD a second window onto this index.
- **Two zone-to-rung anchors, from the maintainer.** A Zone frame of the running
  original reads `SUB-VENOM` at zone 1, and their play names zone 2 as
  `Venom` - explicitly as *the exception* to "not every zone is a class bump".
  So the ladder steps at zone 2 and does not step at every zone after it.
- **That is the shape 2048's recovered table has** (`0`-`1`, then `2`-`8`, then
  `9`-`16`), which strengthens the case that HD's unfound writer walks a
  threshold table of the same kind rather than assigning `stage = zone`.

**This does not close the question and was not used to.** `ZoneGrade` still
rests where HD's loader leaves it, and the HUD's class name is read off
`ZoneGrade`'s own stage rather than off the zone counter, so nothing here has
been wired on a guess. What it does is give the watchpoint hunt two frames whose
answers are known in advance.

## 2026-08-31, later still: **the writer of `+0x640` is found**, and this thread's central question is closed

`Hud_UpdateZoneSpeedClass` (`0x00049718`) ends with `stw r3, 0x640(r29)` where
`r3` is `14 - i` off a fourteen-record threshold table at `0x00860d44`. Full
evidence in
[zone-speed-class-table.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md);
wired as `oag_hd::race::ZONE_STAGES`, so `oag_title::RaceDefaults::zone_stages`
is no longer `None` on this title and **an HD Zone race escalates its colour
grade**.

**It was found from the HUD, not from the field.** Three passes here searched for
the writer by offset and by dataflow and came back empty - the folded-index-bias
trap this thread already records. What worked was going in through the *string
ids* the Zone HUD displays, which the language plugin already named: one `grep`
of the ELF for `MSC_SVENOM` landed in a contiguous fourteen-string blob, and the
only references to it were the table. **On this binary a known string is a better
handle than a known field**, and that is the transferable part.

`14 - i` lands on the fifteen `.effectSettings` rungs one for one, so the
palette ladder this thread reads and the speed-class ladder the HUD shows are the
same index - which is also why the HUD's class name and the circuit's grade can
no longer disagree.

## 2026-09-01: the visualiser's reach is measured, and the billboard question closes on geometry rather than on a second shader

The wiring itself was already complete and is confirmed end to end this pass:
`oag_audio::spectrum::Analyzer::process` (render-ahead thread) ->
`Output::spectrum()` -> `race::scene::frame::Scene::render`'s `zone_spectrum`
-> `Drawable::write_zone_vis` -> `oag_render::mesh_render::zone::write_vis` ->
bind group 2 bindings 4/5 -> `mesh.wgsl`'s `zone_glow`, which applies the
recovered term on **both** shading paths and carries the `(1 - windowDepth)`
factor. Nothing in the chain was missing. What was missing was knowing **how
far the term reaches**, and that is now measured rather than assumed.

**The gate is universal.** All 18,050 fragment blocks on the disc that fetch
`zoneTexVis` carry exactly one negative saturating-`ADD` literal: `-0.5` in
16,834 and `-1` in 1,216, and nothing else. That includes every
`*billboard*`, `*crowd*`, `*banner*`, `*screen*` and `*scanline*` material.
So the "second fragment block with a different gate" fork of the old Open
item is refuted by count, not by argument. `-1` reads as the same gate
authored off (`saturate(N.y - 1)` is zero for a unit normal) at confidence
70, held apart from the count's own 88 because it leans on an unnamed
fragment opcode - see
[zone-shader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md)'s new
section for the reasoning and its limit.

**The normal-space fork is refuted too.** `billboarddiffuse`'s vertex program
writes `MOV o[TC1].xyz, v[1].xyzx` - the authored vertex normal, untouched -
and the fragment program normalises that same register and dots it against
the directional light's direction. `N.y` is world up.

**And the premise the old item rested on is wrong.** "A vertical billboard's
normal has `N.y ~ 0`" was never measured. Measured, the billboard family is
*mixed*: `wes_billboardholographicscanlines` 32.7%, `billboarddiffuse` 27.2%
and `cf_billboard1` 20.9% of vertices above the gate, against 0.0% for
`nr_crowd_bustle`, `cf_cheap_crowd` and `ns_adbanner`, and 80%+ for the track
surfaces. **So this port already draws part of the billboard half** - the
up-facing panels, canopies and angled screens - and what it cannot draw is
the crowds and the flat advertising faces. Both censuses are assertions, not
reports: `zone_shader_census_ground_truth.rs`'s
`the_visualiser_glow_is_gated_to_up_facing_surfaces_everywhere` and
`no_billboard_or_crowd_material_compiles_an_ungated_visualiser`, and
`rcsmodel_vertex_ground_truth.rs`'s
`billboard_geometry_is_mixed_where_crowd_and_banner_geometry_is_not`.

**Nothing was changed in the renderer, deliberately.** The obvious "fix" -
widening the gate to `abs(N.y)` or a smoothstep so billboards light up - is
exactly the plausible-looking stand-in `CLAUDE.md` forbids, and the disc says
in 18,050 places that the original does not do it.

**One quantitative consequence of the unrecovered band mapping, now concrete
rather than theoretical.** `write_vis` spreads 32 bands linearly across all
256 texels, and the band index is the stage texture's own alpha.
`zonemodetrack9` and `10` occupy alphas `{31..40}`, which land at positions
`3.77` to `4.86` of 31 - **bands 3 to 5, about 140-165 Hz**. On those two
stages this build's floor therefore shows three adjacent low-mid bins moving
together, not a spectrum, where `zonemodetrack6`/`7`'s `{1..162}` spans bands
0-20 and does read as one. That is arithmetic off the measured alpha
histogram rather than a rendering observation, and it is a property of the
*invented* uniform spread rather than of anything recovered - see the last
Open item below.

## 2026-09-04: the sky gradient's draw is read, and it is a double cone, not a dome

Picked at random by `/oag-handover`. Full evidence in
[zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md)'s new
section, itself corrected mid-pass the same day - the first read called the
sweep 18 latitude bands, and the actual TOC constants (read with
`scripts/ps3-toc.py resolve`, not inferred) say otherwise. `FUN_005ebd58`,
the sole consumer of the horizon/zenith colour pair, is renamed
`Sky_DrawGradientDome` (confidence 82): it builds **one** 18-segment ring
swept through a full 360° (`18 × 20°` steps, both baked constants) and fans
it to two fixed apexes at `±0.75×radius` - a double cone on one equator ring,
not a multi-band hemisphere. The two packed colours are per-vertex, not
per-pixel - the cone toward one apex blends from one colour to the other as
it approaches its point, the cone toward the other apex holds the first
colour flat throughout. **This closes the open question the previous pass
deliberately declined to guess at**: it is geometry with a real gradient, not
a screen-space quad and not a fog term - so a two-colour gradient really is
what a port should draw here. **What is not closed**: which apex is
zenith-ward in world space (confidence ~65, structural inference not a
measurement) and, more concretely, what blend/depth state and draw order this
call uses against the `ZoneSky.gtf` cubemap the same function already draws -
neither is set anywhere near the call site, so it was set further upstream
than this pass read. That is the real blocker on drawing it, not the shape.

Not done in this pass, and worth flagging for whoever picks this up next: no
Rust code was written. This was scoped as the RE half only, per this skill's
own guidance to keep a pure-RE step separate from the implementation it
unblocks - see Next Steps below for what implementing it needs.

## Open

- ~~What the Zone shader does with the six-plus Zone parameters.~~ **Read**,
  see [zone-shader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md)
  and the visualiser glow section above.
- ~~**Whether `Environment_UpdateStageBlend`'s per-frame `zoneTexVis` write is
  itself audio-reactive**, or a plain stage-progress fraction.~~ **Answered
  2026-09-01, statically, with no watchpoint needed** - it is audio. `f1` is
  the return of a sound-system getter called once per band; see
  [zone-visualiser.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md)
  and the section above. What is still unfound is one step further back: what
  *fills* the sixteen band floats at `g_sound_system + 0x24`.
- ~~What writes the eight colour vec4s at `0x00c81460`/`0x00c81470`-`0x00c814d0`,
  `zoneColourTint`'s own value among them.~~ **Found statically on
  2026-08-31** (`Environment_UpdateStageBlend`, indexed `stvx` exactly as this
  bullet predicted - twenty-fourth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)),
  **and the one lane that stayed open, `zoneColourTint.w`'s sphere radius,
  closed on 2026-09-15**: the same function advances `H[e].+0x08` by a
  per-call speed that itself accelerates (`+0x10 += +0x14`), reset to `0.1f`
  at every stage commit and capped at `20000`; `Transition start speed` and
  `Transition acceleration` are those two fields' schema keys, unauthored on
  disc, so the `.data` defaults `0.5f`/`0.1f` shipped. Thirtieth pass, 85.
- What fills the two-entry palette arrays at `0x00c81330`-`0x00c81350`
  (`zoneAnisoPalette`, `zoneAnisoPaletteOuter`, `GradientColour0..3`). Same
  limitation; confidence only 60 that they hold texture pointers at all.
- Whether `zoneMode*.gtf` being fifteen identical blanks is what shipped or an
  authoring leftover. The bytes say identical; nothing says intended. **What
  is measured as of 2026-09-15 is that the blanks are load-bearing**: the
  Scene/Track bit is authored per chunk in the `.rcsmodel` (the chunk header's
  `+0x08` record, halfword `+0x06`, bit 0 set = Track) and is set on only the
  track-surface chunks - 124 of Talon's Junction's 983 - so the other 859
  sample the blank Scene set under the stage's flat `Scene.*` colours while
  only the road gets the patterned `zoneModeTrack*` art. Whether that was the
  design or a leftover that happened to look right is still not something the
  disc can say. Thirtieth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
- ~~**The recovered glow gate produces the floor half of the maintainer's own
  observation and cannot produce the billboard half.**~~ **Measured
  2026-09-01, and both forks this item named are refuted - see the section
  below.** The gate is universal and the answer is geometry: the recovered
  rule lights the up-facing fifth to third of a billboard mesh and cannot
  light the crowds or the flat banners. What remains open is narrower and is
  restated as its own item below.
- **What lights HD's crowds and flat banners in a Zone race is still
  unexplained.** They measure 0.0% above the `N.y > 0.5` gate, so the
  visualiser glow reaches none of them, and on a racing circuit the surface
  equation drops the albedo entirely - which is why the original's crowds
  and advertising vanish into cyan rather than staying coloured. Whether
  they carry any *animation* at all, as opposed to a flat recoloured
  surface, is not established either way: the maintainer's report names "the
  billboards" without separating the up-facing panels (which this port now
  draws) from the vertical ones. **The next move is a question to the
  maintainer, not a sweep** - do the vertical advertising faces pulse with
  the music, or only the angled screens and the floor? A sweep cannot answer
  it; the microcode has already been counted exhaustively.
- ~~**The index-to-frequency correspondence inside `zoneTexVis` is
  unrecovered, so this build's 256-texel strip is a uniform spread of 32
  bands.**~~ **Recovered and replaced 2026-09-01.** The layout is sixteen
  ten-segment bar meters; the uniform spread was the invention that made the
  effect invisible. The reading in this item - "only ever touches texels 1-10
  and 161" - was the twenty-sixth pass seeing **one band's** pointers before
  the `0x28` advance, and it was the clue: those are band 0's ten segments
  and the smooth block's base.
- ~~**What fills `g_sound_system + 0x24`..`+0x60`, the sixteen band floats.**~~
  **Found and ported 2026-09-01**: `SoundSystem_UpdateBandLevels`
  (`0x00307e78`), an auto-ranging normaliser - each band against its own
  decaying peak and rising floor, six constants read off the TOC. It replaced
  this project's invented `-40 dB` log curve and the analyser-side fade that
  went with it. `get_field_access_context` on the array's own address is what
  found it after a `stfsx` sweep of the module came back empty; record that
  technique, it is the one that works on this binary.
- **The band centre frequencies are not in the executable, and that is now
  the answer rather than a gap.** Four sweeps put the magnitudes in filter
  records no PPU code writes, on an engine (SCREAM/MultiStream, named by its
  own error strings) whose analysis is not in the image. So
  `oag_audio::spectrum::band_frequencies` stays this project's own
  permanently. What *would* still move it: finding where `+0x614`/`+0x618`
  are allocated and what they are handed to - see
  [zone-visualiser.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md).
  The init chain is now mapped three levels deep (`SoundSystem_Init` ->
  cellAudio port setup -> MultiStream bus setup) and the allocation is in
  none of them, which narrows where to look rather than lifting the negative.
  **[stale 2026-09-15: computed before the lvlx reimport; re-run per toolchain.md#ps3]**
- **Whether the sixteen are frequency bands or per-channel levels.**
  Confidence 75 on frequency, from three indirect corroborations - two
  unrelated consumers draw them as ten-segment bars, there is a 32-entry
  sibling array, and the shipped art tags sixteen regions. Against it: the
  source indexes *two blocks of eight*, and eight is this title's 7.1 channel
  count, with the two block pointers sitting immediately before the speaker
  direction table. Nothing this project draws changes either way; the reading
  in the docs and in `oag_audio::spectrum` says 75 rather than pretending.
  A watchpoint during a race with one speaker driven settles it in one read.
- **A second visualiser exists and is unported.** `FUN_003ce2c0` calls the
  same band getter with the same ten-segment bar loop, instruction for
  instruction. Which screen it serves is unread; the front end's own
  `"Music Pulse Base"`/`"Music Pulse factor"`/`waveTexture` strings are the
  obvious candidate, and if it is the menu background then this project's
  `zone::write_vis` is already most of the port.
- **What `lfs f0,0x4(r4)` is** - the fixed per-frame gain the original
  applies to every band before the hold. This port applies none.
- **Unverified lead: the band index may be read through a mip-blurred
  sample.** `mesh.wgsl`'s `zone_glow` fetches the band from
  `textureSample(zone_tex, zone_nearest_sampler, ...)` - implicit LOD, on a
  mip-mapped stage texture - so a grazing floor view can select a lower mip
  whose alpha is an *average* of neighbouring texels, i.e. a band id that
  belongs to neither. The original's `...Nearest` clone exists precisely to
  stop interpolation across band boundaries
  ([zone-shader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md)),
  which is an argument that the mip chain wants excluding too. Nobody has
  confirmed the sampler state the original patches into the clone, and
  nothing was changed on a suspicion; `textureSampleLevel(..., 0.0)` is the
  experiment if a floor ever reads as the wrong band at distance.

## Next Steps

- ~~**Bind per chunk, not per scene** (renderer lane; read on 2026-09-15, not
  implemented).~~ **Done, 2026-09-15**, later the same day:
  `oag_rcs::rcsmodel::Mesh::is_track` exposes the halfword's bit 0,
  `mesh::slots::ZONE_TRACK` carries it per chunk, and `mesh.wgsl` binds the
  Track pair where it is set and the Scene pair (the flat-white `zoneMode*`
  set beside the `Scene.*` colours) elsewhere. On `Sub Venom` (the only
  stage a headless capture reaches - the ladder re-shows it every frame) the
  road and the track-side walls kept the art and the scenery went flat, and
  flat means **white**: `Scene.Texture Colour` `0.72 0.91 0.96` on a white
  texture, multiplied by the stage's `Constant Ambient Colour` `1.5`, clamps
  at the target. Authored numbers through the read equation, with no
  tonemap; the check against a Zone frame of the original is still owed and
  is what would say whether that multiply belongs on the Zone surface.
  Evidence and the disc-wide value survey: thirtieth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
- ~~The Zone sky is its own thing, and this port draws the wrong one~~
  **Done, 2026-08-31, and the answer was neither option the item framed** - see
  the section above and
  [zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md). It is a
  *file swap*: `Data/Tex/ZoneSky.gtf`, 64x64, replacing the circuit's own
  `sky.gtf` on the same gate byte, through the same loader call. The
  gradient is a **second** mechanism on the same gate whose draw is unread, so
  nothing draws it. The item's own instinct - do not implement from the key
  names - was right, and the microcode route it proposed was the wrong one:
  there is no sky *material* on this disc, `"skycube"` is a resource tag, and
  the answer was in the loader's control flow.

- ~~What `FUN_005ebd58` draws, which is the only thing between here and the
  gradient.~~ **Done, 2026-09-04, and refined the same day.** Renamed
  `Sky_DrawGradientDome` (confidence 82) - not a multi-band dome, a **double
  cone**: one 18-segment ring swept a full 360° (both counts read straight off
  the TOC, not inferred), fanned to two fixed apexes at `±0.75×radius`. See
  [zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md)'s
  gradient-dome section, including its own same-day correction of the first
  pass's "18 latitude bands" misreading. **It is geometry with a real
  two-colour vertex gradient, not a screen-space quad or a fog term** - the
  question this item posed is answered, not guessed at. **Still open, and now
  the actual next step - a blocker, not a TODO**: no blend mode or depth state
  is set anywhere near the call site, so how this cone composites with the
  `ZoneSky.gtf` cubemap the same function draws (draw order, blend mode) is
  unread, and which apex is zenith-ward in world space is a structural
  inference (~65) rather than a measurement. Drawing this without answering
  either is exactly the guess this thread has repeatedly declined to make -
  the asset side (`ZoneGrade::scene_tint()`) already has the two colours;
  nothing consumes them as geometry yet, and nothing should until the
  compositing is read (more static work, tracing state upstream of
  `0x003ae1bc`) or captured live (the patched RPCS3 watchpoint build,
  `just build-rpcs3-watchpoints`, on a Zone race).

- **The visualiser glow is the largest remaining gap in what draws, and it is
  wanted by the racing circuits rather than being an arena extra.**
  `zoneTexVis` is declared in **all 16** Zone blocks of the two circuit
  materials read directly (`02_track/materials/diffuse`,
  `05_ubermall/materials/billboarddiffuse`), so the twelve circuits genuinely
  sample it and `mesh.wgsl` leaves it out. **Corrected 2026-08-31: `0x003d8b40`
  was never `zoneTexVis`'s build loop.** It reads ten passes, not nine, and
  builds a different, 640-texel object 172 bytes away in the same struct - a
  sparse three-colour glyph, not a ramp. `zoneTexVis` itself is zero-filled at
  load and **is** rewritten every frame, by `Environment_UpdateStageBlend` -
  see [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
  twenty-sixth pass. Whether the value it writes is itself audio-reactive is
  still open; the "not reconciling" arithmetic this bullet used to point at
  was never going to reconcile, because it was the wrong function.
- **A third shape exists inside the circuit bucket**, and it is where the
  `zoneColourTint.w` radius correlation was measured:
  `01_vineta_k/materials/cf_constantcolourglow` is a *circuit* material whose
  twelve Zone blocks declare `zoneBase*` and sample **nothing at all**. So
  "rim-only" is not the arena shape - the arena shape is the black-mask one -
  and a census binning on "has `zoneBase*`" swallows this sub-population into
  the circuit bucket correctly but invisibly. Worth knowing before anyone
  reasons from the two-shape split as though it were exhaustive.
- ~~Load the fifteen `zoneModeTrack{0..14}.gtf` and hold them per stage~~
  **Done, 2026-08-31** - see the section above. Fifteen decode, all distinct,
  ground-truth tested.
- ~~Draw the untextured `zoneBase*` family~~ **Done, and the premise was
  wrong** - there is no untextured family, and the rim terms are silhouette-only.
  What landed instead is the racing-circuit surface equation with the albedo
  dropped, gated per environment off the disc's own census, with craft excluded
  because `data/materials/ships/*` and `data/weapons/*` carry no Zone blocks at
  all. Measured before and after on the Moa Therma frame: saturated pixels
  outside the cyan band went **50.4% -> 0.7%**, against the original's **3.4%**.
- ~~Read HD's Zone fragment program.~~ **Done** - it is not an engine program
  at all but a variant compiled into 1,467 of the disc's 1,590 `.rcsmaterial`
  files, which is why every search of the executable came back empty. Full
  rule in
  [zone-shader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md).
- ~~Generate `zoneTexVis` (256x1) rather than looking for a file.~~ **Done,
  2026-08-31.** The `0x003d8b40` loop this bullet pointed at turned out to be
  a different object entirely (corrected the same day, see this file's own
  2026-08-31 section and
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
  twenty-sixth pass) - `zoneTexVis` itself is confirmed rewritten every frame,
  which settled the architecture question below in favour of a genuine
  render-side spectrum: `oag_audio::spectrum::Analyzer` runs a 32-band
  Goertzel transform on the render-ahead thread, `Output::spectrum()` hands
  the levels to `oag-game` once a frame, and
  `oag_render::mesh_render::zone::write_vis` builds the 256-wide lookup from
  them, tinted by the showing stage's own `EQ colour tint`. `mesh.wgsl`'s
  `zone_glow` draws the term end to end - pixel-tested in
  `crates/render/tests/zone_recolour.rs`, and checked against real HD disc
  data with `--zone-spectrum-test` (`--screenshot`'s own audio device is
  null, so this is the only way a capture shows the glow at all): 476,000 of
  1,175,040 pixels move by more than a nudge against the same frame without
  it. Wired into `ZoneGrade::zone_uniform` (`effect.w` now carries
  `Track.EQ brightness` instead of a placeholder zero) and
  `race::Scene::render`. `write_vis` interpolates its input across the whole
  256-wide strip rather than block-repeating it - a shipped stage texture
  samples only a narrow, arbitrary window of indices (`zone-shader.md`'s own
  histogram, and the twenty-sixth pass's texels `1`-`10`/`161`), and a block
  mapping would put that whole window inside one repeated value. Missing an
  authored `EQ colour tint` blanks the lookup rather than substituting white.
  See the `## Open` bullet below on what this gate still cannot produce.
- ~~Put the audio-spectrum architecture question to the maintainer before
  building anything that consumes it~~ **Answered by building it**, per the
  above: render-side, in `oag-audio`'s own `spectrum` module, never reaching
  `oag-gameplay`.
- The `...Nearest` clones needed a second sampler, not a shared one - `mesh.wgsl`
  reads `zone_tex` through both `zone_sampler` (linear, the surface term) and
  `zone_nearest_sampler` (nearest, the glow's own band index), the same
  texture bound twice under different sampler state in bind group 2.
