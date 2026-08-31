# HD's per-stage Zone textures are grounded now: which file feeds which sampler is read, what the shader does with them is not

2026-08-31. Split out of
[2048-and-hd-ship-an-unread-effectsettings-table.md](2048-and-hd-ship-an-unread-effectsettings-table.md)
once that thread's tint-consumer step landed and the same mechanism answered a
second question in passing. That thread stays open on its own remaining
question (HD's Zone stage trigger); this one carries the texture half, which
is now a different kind of work.

## What is established

Full evidence, addresses and reproduce commands in
[zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
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
- **`zoneTexVis` is generated, not loaded**: a 256x1, `format=8` RGB ramp
  built at runtime (`0x003d8b40`). A port has to build it; there is no disc
  file to look for. Confidence 74.
- **The `...Nearest` parameters are the same texels with patched sampler
  state**, not separate art (confidence 80 on the clone relation; the claim
  that the patched bitfields are specifically *filter* fields is confidence 45
  and deliberately unnamed).
- **Detonator aliases Zone**: `DetonatorMode{0..14}.gtf` is written into the
  same fifteen slots by a later branch of the same loader, so the parameter
  named for Zone carries Detonator's art when Detonator loaded last.

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
[effectsettings.md](../docs/formats/effectsettings.md)'s `## The EQ keys are an
audio spectrum, observed in play`.

**This raises a maintainer decision rather than a task.** A shader reading an
audio spectrum needs frequency-domain data per frame, and
[ADR-0018](../docs/architecture/adr/0018-audio-mixer-architecture.md) points
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

## Open

- What the Zone shader does with the six-plus Zone parameters. Unread. This is
  the gate on anything that *draws*, as opposed to loads.
- What writes the eight colour vec4s at `0x00c81460`/`0x00c81470`-`0x00c814d0`,
  `zoneColourTint`'s own value among them. Nothing stores there by
  displacement off the loader's base, so the writer is almost certainly
  indexed VMX (`stvx rV,rA,rB`) with no displacement to grep. **A static
  fourth sweep is not the move**; an RPCS3 write watchpoint on `0x00c81470`
  settles it directly - see
  [rpcs3-debugger.md](../docs/reverse-engineering/rpcs3-debugger.md).
- What fills the two-entry palette arrays at `0x00c81330`-`0x00c81350`
  (`zoneAnisoPalette`, `zoneAnisoPaletteOuter`, `GradientColour0..3`). Same
  limitation; confidence only 60 that they hold texture pointers at all.
- Whether `zoneMode*.gtf` being fifteen identical blanks is what shipped or an
  authoring leftover. The bytes say identical; nothing says intended.

## Next Steps

- **The visualiser glow is the largest remaining gap in what draws, and it is
  wanted by the racing circuits rather than being an arena extra.**
  `zoneTexVis` is declared in **all 16** Zone blocks of the two circuit
  materials read directly (`02_track/materials/diffuse`,
  `05_ubermall/materials/billboarddiffuse`), so the twelve circuits genuinely
  sample it and `mesh.wgsl` leaves it out. **Blocked material-side by nothing**
  - the block is on the build loop at `0x003d8b40` not reconciling: nine passes
  of 64 texels, source running `+1728` down to `+192`, destination advancing
  256 bytes a pass, against a 256-entry table. Read that arithmetic before
  generating anything, and read it with the equaliser observation in hand -
  "an RGB ramp" was the reading from before anyone knew the effect was a
  spectrum, and a per-band palette and a gradient are easy to confuse from a
  packing loop alone.
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
  [zone-shader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md).
- Generate `zoneTexVis` (256x1) rather than looking for a file. **Read the
  packing loop at `0x003d8b40` first** - and read it with the spectrum
  observation in hand, because "an RGB ramp" was the reading *before* anyone
  knew the effect was an equaliser, and a per-band palette and a gradient are
  easy to confuse from the packing alone.
- Put the audio-spectrum architecture question to the maintainer before
  building anything that consumes it - see the section above. The answer
  decides whether an FFT belongs in `oag-audio`, in `oag-render`, or in
  neither.
- Only once the shader is read: decide whether the `...Nearest` clones need a
  second sampler in the port at all, or whether one texture with two sampler
  states covers it. The spectrum reading says point sampling is load-bearing
  rather than cosmetic, so this is likely a real difference.
