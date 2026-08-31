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

- Load the fifteen `zoneModeTrack{0..14}.gtf` through `oag_formats::gtf` and
  hold them per stage beside `ZoneGrade`, selected by the same explicit stage
  index the grade already uses. Bounded, testable against
  `hdfury-ps3-eu-dec.iso` the way
  `crates/game/tests/zone_grade_ground_truth.rs` already tests the palette
  half, and it draws nothing it cannot justify. **Bind the Track set, not the
  general set** - the general set is fifteen copies of a flat white texture.
- Generate `zoneTexVis` (256x1, RGB ramp) rather than looking for a file.
  Read the packing loop at `0x003d8b40` first; do not guess the ramp.
- Read HD's Zone fragment program. That is the step that turns loaded
  textures into a drawn effect, and until it lands, anything drawn is
  invention. Start from the shader registry (`ShaderRegistry_Find`,
  `0x005cd728`, and `renderer.md`'s own reading of the program blocks) rather
  than from the parameter names.
- Only once the shader is read: decide whether the `...Nearest` clones need a
  second sampler in the port at all, or whether one texture with two sampler
  states covers it.
