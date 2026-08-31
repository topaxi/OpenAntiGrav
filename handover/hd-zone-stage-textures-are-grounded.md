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

- ~~Load the fifteen `zoneModeTrack{0..14}.gtf` and hold them per stage~~
  **Done, 2026-08-31** - see the section above. Fifteen decode, all distinct,
  ground-truth tested.
- **Read HD's Zone fragment program.** This is now the single gate on
  everything visible: the textures are loaded, the parameters are mapped, the
  effect has a name, and what combines them is the only unread piece. Start
  from the shader registry (`ShaderRegistry_Find`, `0x005cd728`, and
  `renderer.md`'s own reading of the program blocks) rather than from the
  parameter names - the naming has already proved misleading once, since the
  parameter the stage tint lands in is called `fogColour` and the key that
  feeds it is called `Texture Colour`.
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
