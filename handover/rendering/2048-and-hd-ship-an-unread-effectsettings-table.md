---
categories: [rendering, tooling]
---

# Both titles' Zone grade escalates and draws now; two narrow render questions remain

2026-08-28. Surfaced while chasing the 2048 Zone bugs this session also fixed
(`oag_title::ZoneCraft::PlayerShip`, `oag_title::ZoneCircuit::SameCircuit` -
2048 has no dedicated Zone ship or Zone environment; Zone runs on whatever the
player already picked). That raised the obvious question: if there is no
dedicated environment, what makes a Zone race on Altima *look* like Zone? The
answer is a title-wide colour-grade table, present on both HD-lineage
titles, that this engine has never opened.

**The files, read directly out of each PSARC with `scripts/psarc.py cat`:**

| Title | Path | Size | Scope |
| --- | --- | --- | --- |
| 2048 | `Data\art\published\environments\<circuit>\ZoneMode2048.effectSettings` | 43,812 B | Ships in all 10 base circuits, but **byte-identical** - `md5sum` matches and `diff -q` is empty between `altima`'s and `cathedral`'s copies. One shared table, duplicated per directory rather than looked up once. |
| HD/Fury | `/data/environments/zonemode.effectsettings` | 21,160 B | **One file, title-wide** - not per-circuit at all. |
| HD/Fury | `/data/environments/zonemodedlc3.effectsettings` | 43,417 B | A second, larger revision, close to 2048's size. |
| HD/Fury | `/data/environments/detonatormode.effectsettings` | 43,240 B | Same mechanism, Detonator mode. |
| HD/Fury | `/data/environments/detonatormodedlc3.effectsettings` | 43,230 B | Detonator's DLC3 revision - not listed in this thread's first pass, found while writing the ground truth for the parser below. |

**2026-08-28, later the same day: a parser landed.** `oag_tables::effectsettings`
([`effectsettings.rs`](../../crates/tables/src/effectsettings.rs),
[docs/formats/effectsettings.md](../../docs/formats/effectsettings.md)) reads all
five files above plus 2048's ten identical copies, validated against the real
disc/PSARC in `effectsettings_ground_truth.rs`. It reuses `EnvSettings::parse`
for the tokeniser and adds a stage index/name read off each key's own prefix.
**2026-08-30: the render-application half landed** - a Zone race now grades
its circuit off this table, driven by an explicit stage index and weight.
**Later the same day the trigger landed too, on 2048**: its seventeen-record
zone-number ladder is read out of the executable and a 2048 Zone race now
escalates on its own. **2026-08-31: HD/Fury's own trigger landed too**
(`Hud_UpdateZoneSpeedClass` writes the per-craft stage field the blend reads;
see the section below) **and its recolour is confirmed drawing**
(`Environment_UpdateStageBlend` writes all seven Zone vec4s, `mesh.wgsl`
draws the subset - twenty-fifth pass, `## Open` below). **2026-09-01: the
visualiser (`zoneTexVis`) closed too and is already implemented** -
`oag_audio::spectrum` + `mesh.wgsl`. **2026-09-03: the Scene/Track selection
mechanism and `zoneOrigin`'s writer are both found too** (see the two newest
sections below). **2026-09-15: the sphere radius's writer and the
Scene/Track bit's author are both read** - the radius advances inside
`Environment_UpdateStageBlend` itself on a quadratic the two dev-only schema
keys parameterise, and the per-chunk bit is a **file field** of the
`.rcsmodel` (the chunk header's `+0x08` points at a 0x40-byte per-chunk
record whose `+0x06` halfword is authored on disc). What remains is
`zoneAnisoPalette` and the renderer wiring; see Open and Next Steps.

Plain text, same `"Key.Subkey"=float [float...]` shape `oag_tables::envsettings`
already parses for `.envsettings` - just never pointed at this extension.
`hd-status.md` already listed `.effectsettings` among HD's "genuinely new,
unread" formats; this session is the first time anything read the bytes.

**Both are organised into named, numbered stages**, keyed by zone number:

- 2048 (`ZoneMode2048.effectSettings`), 13 stages, `0`-`12`: `Start, Sub
  Flash, Flash, Sub Rapier, Rapier, Sub Phantom, Phantom, Super Phantom, Zen,
  Super Zen, Subsonic, Mach 1, Supersonic`.
- HD (`zonemode.effectsettings`), 15 stages, `0`-`14`: the same list with
  `Sub Venom, Venom` inserted right after `Start` - 2048's numbering is
  shifted down by exactly 2 to compensate, so the *set* is identical past
  `Start`, only the count differs.
- `detonatormode.effectsettings` carries the **exact same 15-stage name
  list** as `zonemode.effectsettings` - Detonator reuses Zone's ladder
  wholesale.

Each stage carries a full palette. 2048's key groups: `Window Colour
{1,2,3}.{Gradient 1-3, Emissive}`, `Colour {1-8}.{Colour, Emissive}`, `Cube
Animation Colour 1`, `EQ.{Colour A/B/C, Mid-Band Position, BG Colour A/B/C,
BG Mid-Band Position}`, `Track Paint.{Primary,Secondary} Colour`, `Growing
Texture.{Colour, Scale Bias, Factors}`, `Background.Diffuse Colour`,
`Sky.{Horizon,Zenith} Colour`, and **two independent fog blocks** -
`Fog.Environment Fog Colour` and `Fog.Track Fog Colour`, each its own RGB
plus its own density. HD's vocabulary differs (`Scene.{Base Colour, Base
Colour Highlight, Texture Colour, EQ brightness}`, `Track.{the same four}`,
`Lighting.{Constant Ambient Colour, Sun colour, Sky reflection colour,
Fog/Alt Fog/Track Fog colour+density, Prelit Colour Power/Scale}`, `Sky
{horizon,zenith} colour`, `EQ {colour,analogue colour} tint`) but the shape -
per-stage scene/track/fog/sky/EQ blocks - is the same design, just a later
authoring pass.

**The texture side confirms 2048's copy is HD's, ported rather than
reauthored - and corrected 2026-08-28: HD ships the same doubled set 2048
does, not a single one.** The first pass here found `zonemode0..14.gtf` and
called 2048's `zoneModeTrack*.gxt` a 2048-only doubling; `zonemodetrack0..14.gtf`
exists on HD too (also `DATA02.PSARC`, also 87,552 B each), just not in the
first `psarc.py list` sweep that built the file table above.

| Title | "General" set | "Track" set | Size each |
| --- | --- | --- | --- |
| HD | `/data/tex/zonemode{0..14}.gtf` | `/data/tex/zonemodetrack{0..14}.gtf` | 87,552 B |
| 2048 | `data/Tex/zoneMode{0..14}.gxt` | `data/Tex/zoneModeTrack{0..14}.gxt` | 349,568 B |

15 files in each set on both titles, matching HD's 15-stage count exactly
(2048's own table only names 13 stages - see Open). **The two sets are not
equivalent, decoded from HD's own art**: all fifteen `zonemode*.gtf` are
byte-identical to each other (`md5sum` confirms it) and decode
(`oag_texture::gtf`) to a flat, uniform white texture with nothing in it -
every one of 65,536 texels on every one of the fifteen files. All fifteen
`zonemodetrack*.gtf` are **distinct** from each other, both by `md5sum` and
by decoded content: a greyscale image with a varying alpha channel whose
pattern visibly changes across the sampled stages (0, 1, 7, 14) - blocky
interlocking shapes at `Start`, fine vertical stripes partway through the
ladder, a dense small grid at `Supersonic`. So the "Track" set is where
`Growing Texture`'s real art lives; the "general" set is a placeholder. 2048's
own byte pattern matches exactly - `zoneMode*.gxt` all identical,
`zoneModeTrack*.gxt` all distinct - checked without decoding either, since
both use an undecoded format (`0x0c`, `U8U8U8U8` - corrected 2026-08-28 from
an earlier `U4U4U4U4` mislabel, see Open). Whether the
`Track Paint` key block pairs with the "Track" texture set by more than name
is still a guess.

**The table now parses, and nothing still reads it at runtime.** Neither Zone
loader (Pulse's `ZoneCraft::ModelsInTeam` path, HD's `ZoneCircuit::Separate`,
or 2048's new `ZoneCircuit::SameCircuit`) wires a stage lookup in yet. A Zone
race in this engine still looks identical to an ordinary race on every title
except for the HUD and the ship-model swap Pulse alone does.

**2026-08-28, later still: with a Ghidra project open on `vita-2048-eu-v104`,
2048's own loader confirms the mechanism - and reveals the four ported HD
`zone_N` circuits get no palette at all.** Named `Environment_Load`
(`0x8102f6d0`, confidence 80,
[zone-environment-fallback.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md)),
called once per race from the already-identified track loader
(`zone-audio.md`'s `FUN_8121bce2`). Two findings:

1. Even on the ordinary path, it builds a **second, previously unknown
   title-wide file**, `ZoneMode2048default.effectSettings`, alongside the
   per-circuit `ZoneMode2048.effectSettings` this thread already found.
2. When a circuit has no native `art\published\environments\<circuit>\`
   tree - true of the four `zone_N` circuits HD's own DLC ports into this
   title - the loader reaches directly for HD's own title-wide files,
   `data/ZoneEnvironmentHDFury/{ZoneMode,DetonatorMode}HDFury[DLC3].effectSettings`,
   picked by mode and DLC3-vs-base.

**Verified reachable, not dead code**: DLC2 ships those four circuits under
`data/art/published/DLC1/environments/zone_N/` - the extra `DLC1` segment is
exactly what makes the primary path probe fail, which is exactly what this
fallback needs to fire. **Verified absent**: none of the five files this
section names exist anywhere on disc - checked directly against all three
packages (base, DLC1, DLC2). **So racing Zone or Detonator on any of the
four ported circuits loads no palette at all** - the original game's own
data falling short of its own code, not a gap in this project's reading.

**2026-08-28, a second Ghidra pass the same day: `Zone_UpdateStage`
(`0x81044cfc`) is the real per-stage selection this thread has been asking
about.** Traced the three functions `Environment_Load` calls after building
its paths - full evidence on the same
[zone-environment-fallback.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md).
Three findings:

1. `Environment_LoadEffectSettingsFiles` (`0x8104619e`, confidence 78) loads
   **both** built paths independently into a shared cache - not
   primary-then-fallback. A missing file is a silent no-op for that one
   insert, confirmed since `ZoneMode2048default.effectSettings` doesn't
   exist and every base-package Zone race still runs.
2. `Zone_InitStageState` (`0x81044202`, confidence 75) is a plain lazy-init
   of the stage-blend state block.
3. `Zone_UpdateStage` (`0x81044cfc`, confidence 80) reads a per-craft field
   at `+0x634`, **clamps it to `0xc` (12)** - an exact match against 2048's
   own thirteen-stage table - and cross-fades the current/next stage's row
   of a 356-byte-stride table (`DAT_816c4890`). Called once at load and
   **every frame** from the main render-update loop, both times with
   `param_1 = 0` (the local player's own craft). Fires
   `"ZONE_Wave"`/`"ZONE_Pulse"` cues on a stage change.

**Settles that the four unshipped-palette circuits don't crash**: the
per-frame call is gated on whether *any* environment path was probed
successfully, not on whether the effectSettings file loaded, so the
blend math keeps running against whatever is already resident at
`DAT_816c4890` - plausibly zero-initialised `.bss` on a cold boot, not
verified. **Does not settle**: what struct `+0x634` belongs to, what writes
it, or whether `DAT_816c4890` is even the parsed effectSettings data rather
than something built from it - `Environment_LoadEffectSettingsFiles` inserts
into a *different* cache, and the link between the two was not traced.

**2026-08-28, a parallel pass, on the user's own lead: does 2048's DLC code
carry an HD-equivalent effect of its own?** Decompiled the DLC fallback's
own `FUN_8102493c` - the function `Environment_Load` calls directly with the
raw HDFury effectSettings path, never opened before now - and named it
`Environment_LoadHDFuryContent` (`0x8102493c`, confidence 75, full evidence
on the same
[zone-environment-fallback.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md)).
Yes and no:

- **Yes**: the four ported circuits get their own load-time subsystem. It
  loads the effectSettings resource into its own cache, preloads all
  fifteen files of whichever "Track" texture set the active mode names -
  **confirming `zonemodetrack{0..14}.gtf`/`DetonatorModeTrack{0..14}.gtf`'s
  identification from the executable's own control flow**, not just the
  byte-pattern inference above - and builds what reads as procedural
  geometry, plausibly the `Growing Texture` mesh itself.
- **No**: it is parallel to, not feeding, `Zone_UpdateStage`'s mechanism.
  Checked against its full 1,664-line decompile: it never touches `+0x634`,
  `Zone_UpdateStage`, or `DAT_816c4890`. It also independently corroborates
  the "reversed circuit" flag hypothesis from the section above, on
  different data, with the same reversal shape.

**And re-confirmed against `Environment_Load`'s own full decompile, not a
summary this time**: the DLC branch that calls this function returns
without ever reaching the label that calls `Zone_UpdateStage` at load time.
So on the four ported circuits, `Zone_UpdateStage` runs *only* from its
per-frame caller - turning the earlier "plausibly zero-initialised `.bss`"
hedge into a confirmed mechanism (nothing in either branch of
`Environment_Load` ever populates `DAT_816c4890` for these circuits; only
*what* it holds instead remains unverified).

**2026-08-28, later still: the live Ghidra project had silently reverted -
fixed, and the fix uncovered that HD's executable really does parse this
file's text.** Picking this thread back up, the live `ps3-hdfury-eu` program
had lost both the TOC fix and every rename `zone-effectsettings-loader.md`
documents (plain `PowerPC:BE:64:A2ALT-32addr`, `.opd.FUN_003d6dc8` instead of
`Environment_LoadStageTextures`) - whether from a Ghidra restart reopening an
older save or a `save_program` that never landed is untracked, not worth
chasing once the fix needed redoing anyway. Rebuilt/reinstalled
Ps3GhidraScripts, reimported under `--ps3-cspec`, reapplied all 118 rows of
`names.tsv` clean, `just check-names` green project-wide. **Recorded as a
trap for next time**: a live Ghidra project's applied state is not durable
between sessions - verify a known rename before trusting a decompile.

With the state trustworthy again, `Environment_LoadStageTextures`'s full
decompile (finally readable end to end) settles the question this thread's
own "Does not settle" bullet raised: HD's executable opens
`ZoneMode.effectSettings` (`FwFile_OpenByPath`), reads it whole
(`FwFile_ReadChunked`), and tokenises the text with a generic,
effectSettings-agnostic keyed-config reader (`FwKeyedText_ParseBuffer`/
`FwKeyedText_ParseEntry`, confirmed shared - six call sites elsewhere in the
binary, one of them a function-pointer table slot) into a real per-stage
struct table: 15 stages, `0x250` bytes each, the same base address the
already-found "reversed circuit" `memcpy` block operates on - stage count,
stride and total size all check out together for the first time this thread.
**Still not found: who reads that table.** The per-stage *texture* loading
this thread already traced (`Resource_GetOrCreateByName` against a hardcoded
filename table) is a separate subsystem in the same function, feeding
different memory, not fed by this parse.

Checked directly against the real disc while at it: HD's own stage 0 carries
`Lighting.*` keys spelled differently from `.envsettings`' constants
(`"Sun colour"` vs `SUN_COLOUR`'s `"Sun color"`, no `Sun direction` key at
all) - close enough to *look* like the same field this project's
`envsettings_light`/`envsettings_fog` already read and draw, not close
enough to reuse, and one field short of a full light rig. **So this pass
does not wire anything into the renderer**: the file's content is now
confirmed parsed by the game, but its consumer inside the executable is
still unfound, and overriding render state off a name resemblance alone on
an unfound-consumer file is exactly the guess `CLAUDE.md` rules out. Full
evidence, including the byte-level trace of every function in this chain:
[zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-28-a-fifth-pass-the-live-project-had-reverted-and-once-restored-the-files-own-content-turns-out-to-be-parsed-after-all).

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

## Open

- **2026-08-31, the Zone shader's own inputs are traced and the recolour
  draws.** `Environment_UpdateStageBlend` (`0x003da540`) turns out to write
  all seven Zone vec4s itself - the twenty-third pass had concluded that only
  an RPCS3 write watchpoint could settle it. Four prior sweeps missed it
  because they keyed on `displacement(r14)` and the stores are `stvx rV, r30,
  rB`, indexed with no displacement and no literal to grep. `zoneEffectInner`
  / `Outer` are `Scene.Texture Colour` of stage `n` / `n - 1`, `zoneBase*` is
  `Scene.Base Colour Highlight`, `zoneBaseAlt*` is `Scene.Base Colour`, and
  `zoneColourTint.xy` is the file's own title-wide `Texture U scale` /
  `Texture V scale`. `oag_render`'s `mesh.wgsl` draws that subset.
  Confidence 84-86, full evidence in the twenty-fourth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).

  **What that leaves open, all of it narrow:**

  1. ~~Which of the two parallel feeds a material sees~~ **Answered,
     2026-09-03 - and the framing was wrong.** The pairing (twenty-fifth
     pass, confidence 86: `zoneMode*`/Scene and `zoneModeTrack*`/Track are
     a single choice, not two independent feeds) still stands. What doesn't
     is "which prologue a draw enters through" - `FUN_003ff860` has no
     second entry point at all. It's a **loop** over a per-context table of
     draw entries, and each entry's own flag bit picks Scene or Track for
     that entry, cached per call so the underlying parameter block is
     written at most once per group. Confidence 80 on the mechanism
     (control-flow-reconstructed decompile, offsets cross-checked against
     the twenty-second pass's own parameter formula); confidence 55 on what
     the flag bit means about the entry (plausibly per-material, not traced
     to the `.rcsmaterial` compiler to confirm). Full trace: twenty-eighth
     pass of
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
     `oag_render` binds the track set unconditionally, which matters: `Start`
     authors `Scene` black and `Track` at `9.0`, and this pass doesn't
     change which one a port should bind - it explains the mechanism the
     disc itself uses to choose, for whoever wires per-material selection
     later.

     **2026-09-05, the bit is sourced, and it is not per material.** The
     record is built in one expression in `Scene_SubmitVisibleChunks`:
     `rec[2] = (*(short *)(*(int *)(chunk + 8) + 6) << 1) | (chunk[7] == 2)`.
     So the Scene/Track bit is bit 0 of `(*(chunk+8))->0x06`, read *outside*
     the per-surface loop - every surface of a chunk gets the same value.
     **Per chunk, not per material**; the confidence-55 guess is superseded,
     at 85. What is per material is which *publisher* runs at all. See the
     Next Steps entry and the twenty-ninth pass.

     **2026-09-15, the bit is authored on disc.** The runtime chunk is the
     file's own chunk header and its `+0x08` word is a file offset the RCS
     loader relocates through the file's own relocation table (the header
     word at `+0x04` names it; 41,861 of 41,861 chunks on the disc have
     their `+0x08` in it). It points at a 0x40-byte per-chunk record whose
     `+0x06` halfword is non-zero on 5,948 chunks as shipped; bit 0 - the
     Scene/Track selector, **set = Track** - is set on 4,365 chunks in
     exactly the 37 track-shaped models, and the same bit is what
     `Scene_BuildStaticChunkMask` feeds the shadowed-track redraw passes. No
     runtime store to the halfword exists (program-wide `sth ,0x6(` sweep,
     102 sites, all accounted for). Confidence 85 on "authored", 80 on
     "means track surface". Thirtieth pass of
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
     **This changes what a port should bind**: `oag_render` binding the
     Track set for every chunk is right for 124 of Talon's Junction's 983
     chunks and wrong for the other 859, which the original draws with the
     Scene set. Not implemented this pass (RE lane); see Next Steps.
  2. **`zoneOrigin` (`0x00c81550`) - writer found, 2026-09-03; the radius
     writer found 2026-09-15.** `Scene_PrepareFrame` writes it every frame,
     one call frame above `Environment_UpdateStageBlend` (which is why the
     twenty-fourth pass found no writer *inside* the blend and correctly
     didn't claim more than that), from a resolved entity's own **`+0xb0`**
     float4 (the twenty-seventh pass wrote `+0x80`; it skipped a `li
     r0,0x30` in the listing - corrected in the thirtieth pass, which also
     re-confirmed the site on the complete post-`lvlx` image), gated on a
     viewport/id lookup rather than on Zone mode. Confidence 78 - the
     chain's two ends (the object-array base, `zoneOrigin` itself) are
     TOC-verified twice over; whose `+0xb0` (a 4x4 at `+0x80` with the
     position in its last row is the reading offered at 60) and which
     viewport `id` selects is not chased. Full trace: twenty-seventh and
     thirtieth passes of
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
     **The radius** (`H[e].f32@0x08`, copied per frame under a lane-3-only
     `vsel`) is advanced by `Environment_UpdateStageBlend` itself on the
     "already applied" arm: `radius += speed` while under `20000`, `speed +=
     accel`, per call; the commit resets radius to `0.1f` and speed to
     `+0x0c`. `+0x0c` and `+0x14` are exactly where
     `Environment_RegisterStageSchema` registers `Transition start speed`
     and `Transition acceleration`, unauthored on disc, so the `.data`
     defaults `0.5f`/`0.1f` are what shipped. The colour weight `+0x18`
     advances `0.01f` per call to a clamp at `1.0`. Confidence 85. Inner/outer
     is a **stage-transition wavefront** - a sphere expanding out of
     `zoneOrigin` that repaints the world with the new stage - and every
     number it needs is now the disc's own.
  3. ~~`zoneTexVis`~~ **Closed and implemented, 2026-09-01 - the `0x003d8b40`
     reading above was wrong.** That address builds an unrelated struct field
     172 bytes away; `zoneTexVis` itself is a load-time zero-fill, rewritten
     every frame by `Environment_UpdateStageBlend` as sixteen ten-segment bar
     meters fed by `SoundSystem_GetBandLevel` - it is the original's
     **audio-spectrum visualiser**, not a static ramp, confirmed three ways
     against the shipped `zoneModeTrack*.gtf` art. `oag_audio::spectrum` now
     supplies the same sixteen bands (the disc's own count, not chosen) and
     `mesh.wgsl` draws them. What stays this project's own, permanently: the
     band centre frequencies and the magnitude curve - the original's per-band
     source is middleware (Sony SCREAM/MultiStream), not PPU code in this
     executable. Full trace:
     [zone-visualiser.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md),
     twenty-sixth pass of
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  4. **`zoneAnisoPalette` / `zoneAnisoPaletteOuter`.** Unchanged: no filling
     write located, and `zonemode.effectsettings` authors no
     `Scene.Aniso Power` on any of its fifteen stages either, so the rim term
     has neither a palette nor an exponent from the disc.

- **2026-08-31, later: the tint consumer is `fogColour`, and the two questions
  that replace it are both narrow.** See the struck-through first entry under
  Next Steps for the finding itself; the entries below it are kept as the
  record of how the search ran, not as live questions. What is genuinely still
  open from that line of work:

  1. ~~Who consumes `0x00c49120` and `0x00c49130`.~~ **Answered the same day,
     and the negative published against them was wrong.** Both are bound to
     **`fogColour` itself** - `0x00c49120` from `FUN_003ff860` at `0x00400658`,
     `0x00c49130` from `FUN_00400a00` at `0x00401800`, both writing the same
     table offset `0xf8` that `Scene_PrepareFrame` writes with `0x00c49110`. So
     the engine keeps **three fog-colour buffers and one parameter** and picks
     between them at runtime, behind a bit test on a halfword flag, defaulting
     to `0x00c49120`. `0x00c49100` is parameter 4, `eyePositionWorldSpace`.
     Confidence 80, every store hand-read. Which of the three a given draw
     wants, and whether they line up with the `Fog`/`Alt Fog`/`Track Fog`
     triple the file authors, is **not** established - that is the question
     that replaces this one.

     **Answered 2026-09-05, and no draw picks between them.** Counting the
     non-stack stores to the parameter table's `+0xf8` across all eight
     publishers gives exactly three in the whole image, **one each in three
     different publishers** - `Scene_PrepareFrame` (`0x003ab430`),
     `FUN_003ff860` (`0x00400658`) and `FUN_00400a00` (`0x00401800`) - with
     the other five publishers holding none. So it is **one fog buffer per
     publisher**, and the two that differ are the blended pass and the
     unblended one. Confidence 80. They do *not* line up with the file's
     `Fog`/`Alt Fog`/`Track Fog` triple, which the twenty-second pass already
     placed at `+0x1a0`/`+0x1d0`, uncross-faded. Twenty-ninth pass of
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  2. ~~Which schema key parses to `iVar8 + 0x3aac`.~~ **Answered the same
     day** - it is `Scene.Texture Colour`'s own fourth lane, i.e.
     `Scene.EQ brightness`, per the measured schema table. The seventeenth
     pass's attribution stands; the field is a lone scalar because the blend
     stores a 16-byte key's rgb and its fourth lane to two different places.

- **2026-08-31, the object is found and the consumer search is now bounded.**
  The stalled `0x90(r1)` step is resolved: the slot is filled through a one-line
  setter (`FUN_005d4958`, `{ *r3 = r4; }`), which is why no `stw`/`std` to it
  exists. The object is a **stack struct at `sp+0x460`** in `Scene_PrepareFrame`,
  initialised by `FUN_005d4868` - which sets `obj->0x4 = 0xf0`, the very word
  that later gets `|= 0x000c0000`.

  **Two corrections**: `obj+0xd8` is not an inherent sub-object, it is assigned
  at `0x003ab2a4` from `block[0x7c60]` (a runtime pointer stored inside the
  scene block), so the pointer table is `*(block + 0x7c60)`, one indirection
  further than recorded. And `obj` is **stack-allocated**, dying when
  `Scene_PrepareFrame` returns.

  **That second fact is what the search needed.** A dirty flag on a stack object
  cannot be honoured after the function returns, so **the consumer is inside
  `Scene_PrepareFrame`'s own call graph**, not somewhere in a 26,000-function
  image. The unbounded search that defeated the `0x198`, dirty-bit and
  `.cpp`-filename keys is now bounded by construction - that is the single
  biggest change to this thread's tractability.

  **2026-08-31, ruled out: `FUN_005d6e78` is ruled out, and a second writer of
  the same tint is found instead.** Checked directly rather than trusted: its
  disassembly is six `lvx`/`stvx` pairs and a `blr`, no `bl` anywhere, and it
  never reads its first argument (`obj`) at all - it only writes `obj+0x50..
  0xb0` from an unrelated source range. The callee list this candidate was
  carried forward with (`0x005bd0d8` etc.) does not match what is live in the
  project now, the same "state not durable between sessions" trap this thread
  already named once for a rename, this time for a call graph. Ruling it out
  is now confidence 92, not a guess.

  **In its place**: widening the search to a literal-address scan for
  `0x00c49110` (the technique that found `0x00c81a5c`'s own reader) finds a
  genuine second, TOC-verified writer - `FUN_003ea368`/`FUN_003eb890`, reached
  through a completely different call path than `Scene_PrepareFrame`, using
  the identical `+0xd8`/`+0xf8` dirty-flagged protocol, gated on Detonator
  mode (`g_GameState+0xe0 == 0xe`) and a per-entity flag. Real corroboration
  that the protocol is a general renderer mechanism, not private to the scene
  blend - but this is a writer, not the consumer search itself resolving.
  `FUN_005d4a08`, called right after, is a tag-dispatch interpreter over the
  entity's own compiled list and is the most concrete remaining lead, but its
  dispatch table (`PTR_PTR_008bf21c`) mixes real pointers with inline float
  constants and was not decoded this pass. Full detail, including why the
  entity type itself is deliberately left unnamed (confidence <50), in
  `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
  twentieth pass.

  **2026-08-31, twenty-first pass: the table read above was a misread - fixed,
  and it explains `FUN_005d6e78` completely.** `PTR_PTR_008bf21c` is a TOC
  slot, not the table; the real 16-entry table lives at the address that slot
  holds (`0x00927518`), and every entry is a genuine pointer to a TOC-verified
  OPD descriptor (no float constants mixed in after all - that reading missed
  one level of indirection). Decoding it: `FUN_005d6e78` (now renamed
  `Render_SetClipPlanes`, 65) turns out to be a generic "install six vec4s
  into this render context's `+0x50` slot" primitive, called both by
  `Scene_PrepareFrame` (with the scene block's planes) and by this dispatcher's
  own tag 12 (with six vec4s read straight out of the compiled bytecode) -
  fully explaining why it never touches `+0xd8`/`+0xf8`. `FUN_005bd0d8` (now
  `Render_ClassifyAgainstPlanes`, 65) - the address the original brief tied to
  "RSX command emitters" - is actually a six-plane point classifier
  (inside/outside/intersecting), not an RSX emitter at all. `FUN_005d4a08`
  (now `Render_RunCompiledOps`, 58) is a generic per-object setup interpreter:
  control flow, sub-list calls, and culling/plane state - none of its eleven
  read opcodes touch the `+0xd8`/`+0xf8` tint slot. **The tint consumer is
  still open**, narrowed to: one of three unread opcodes (tags 7/13/14), or -
  more likely given how generic everything else in this table is - something
  entirely outside this dispatcher, at whatever step actually flushes the
  lazily-filled parameter table. Full detail in the doc's twenty-first pass;
  three renames applied and saved live, `names.tsv` updated.


- **2026-08-31, where the static route stands on this thread, and what is left.**
  The chain is read end to end as data: `zonemode.effectsettings` ->
  `g_effect_settings_stages` -> `Environment_UpdateStageBlend` blends
  `Scene.Texture Colour` into `0x00c81a5c` -> `Scene_PrepareFrame` reads it at
  `0x003aaf8c`, splats and merges it -> `0x00c49110`/`0x00c49120`/`0x00c49130`
  -> `&block[0x110]` published into `*(obj+0xd8) + 0xf8` with `*(outer+0x4) |=
  0x000c0000`. Everything to that point is confidence 82-88.

  **Three static keys have now been tried and each is exhausted**, so a fourth
  attempt should not repeat them: the `0x198` field offset (189 image-wide hits,
  will not converge); the `0x000c0000` dirty bit (212 setters, and **zero**
  immediate tests of any kind, so the flag is consumed via a register- or
  table-held mask); and the `.cpp`-filename route (neither `FUN_006ce6e0` nor
  `FUN_00107200` references a single string).

  **The one static route not yet tried** is the allocator tag: identify the
  object behind `*(obj+0xd8)`, find where it is allocated, and read the
  `__FILE__` pointer `FwMemAllocator_Allocate` is passed - the technique
  `docs/ghidra/functions/ps3-hdfury-eu/memory.md` used to name the memory layer.
  A first attempt stalled at the first step: in `Scene_PrepareFrame` the object
  is the stack local `0x90(r1)`, which is **read** at `0x003ab198` and later but
  has no `stw`/`std` writing it anywhere in the function, so it arrives by some
  form this pass did not identify (a frame larger than it looks - the prologue
  builds `0x680` via `li r0,0x680`, not a plain `stdu` - is the likeliest
  explanation, making `0x90(r1)` an *incoming* slot rather than a local). That
  is where the next static session should start, and it is a fresh dig rather
  than a finish.

  **But the read watchpoint is the better instrument and is now the bottleneck**,
  blocked on vector load/store coverage in the patched RPCS3 - see
  `docs/reverse-engineering/rpcs3-debugger.md`, "Z2 misses vector stores". Once
  that lands: write watchpoints on `0x00c49110`/`0x00c49120`/`0x00c49130` to
  confirm the block runs and the gate byte `0x00d45f84` is open in a Zone race,
  then a **read** watchpoint on `0x00c49110`, which names the consumer and
  settles the buffer's identity outright.

  **Superseded in part, 2026-08-31**: the **read** watchpoint is no longer
  needed - the consumer is `fogColour` and the buffer is the scene render
  block, initialised by `Scene_InitRenderBlock`, so neither question is worth
  a build cycle now. The **write** watchpoints are still the way to confirm
  the gate byte `0x00d45f84` is open in a real Zone race, and a write
  watchpoint on `0x00c81470` is the way to settle who writes `zoneColourTint`
  (see [hd-zone-stage-textures-are-grounded.md](hd-zone-stage-textures-are-grounded.md)).


- **2026-08-30, THE READER OF `0x00c81a5c` IS LOCATED.** Nineteen passes in,
  the "no reader located" row is answered: `Scene_PrepareFrame` reads it
  itself, at `0x003aaf8c`. Full evidence in
  `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`
  ("a nineteenth pass").

  **Why eighteen passes missed it, and this is the transferable lesson.**
  Every earlier sweep looked for the field the way the *writer* addresses it -
  `0x3aac(rX)` off the stage base `iVar8 = 0x00c7dfb0`. The reader does not
  address it that way: each of the three floats has **its own dedicated TOC
  pointer slot** (`0x008b71cc`/`0x008b71d4`/`0x008b71dc`), so the load is
  `lfs f0, 0x0(r9)` with displacement zero. A displacement sweep, a branch
  scan and an OPD-reference scan are all structurally blind to that. **When a
  static search for a consumer of a fixed address comes back empty in this
  binary, search for the address as a TOC-slot literal before concluding
  anything** - that one check would have found this on pass two.

  What the read does: splats each scalar to a `float4`, `vsel`-merges it into a
  neighbouring vector, and stores into a large scene block at the **fixed**
  address `0x00c49000`, offsets `+0x110`/`+0x120`/`+0x130` - i.e. the fixed
  addresses **`0x00c49110`, `0x00c49120`, `0x00c49130`**. Confidence 88 on the
  read.

  **Second correction, 2026-08-31**: the first write-up named `+0x7c00`/`+0x7c10`
  as two of the three destinations. Wrong - those stores carry `v10`/`v11`
  *after* they have been reloaded from unrelated addresses; the merged tint is
  in `v13`/`v0`/`v1`, going to `+0x130`/`+0x110`/`+0x120`. Mis-attributing the
  nearest `li` value to the wrong store in an interleaved run is the same class
  of error as the sign trap. It cost a real test: a watchpoint run armed
  `0x00c50c00`/`0x00c50c10`, two addresses that were never destinations. The
  published pointer is likewise `table+0xf8 = &block[0x110]` (at `0x003ab430`),
  not `table+0x1b8`.

  **Watchpoint caveat**: all three destinations are written by `stvx`, a vector
  store. A write-watchpoint hook that covers only `stw`/`stfs` will report
  nothing here however correct the address, so a zero-hit result proves nothing
  about `Scene_PrepareFrame` until the hook is known to cover `stvx`.

  **Correction, same day:** the first write-up said the destination was a
  *runtime* buffer `*(0x008c713c)`. That was a sign error - the raw
  displacement field `0x9d78` is **signed**, so it is `-0x6288` and the slot is
  `0x008b713c` (holding `0x00c49000`), not `0x008c713c` (which holds 0 and
  reads convincingly as an uninitialised pointer). Fixed in the doc. The
  correction *helps*: the destinations are static, so a watchpoint on them
  needs no runtime address mapping. **General trap for this binary: a raw 16-bit
  displacement above `0x7fff` is negative, and mis-signing one yields a slot
  that exists, reads cleanly, and means nothing.**

  **Settled 2026-08-31 - `0x00c49000` is the scene render block**, initialised
  by `Scene_InitRenderBlock` (`0x003aa618`), whose own body maps a good part of
  its layout (identity matrices at `+0x00`-`+0xf0`, the `float4` run at
  `+0x7ba0`-`+0x7c30`, the parameter-array pointer at `+0x7c60`). The paragraph
  below is the record of the attempt that did not close it, kept for the
  technique rather than the question.

  **The buffer's identity is still open at 55, and the static attempt to close
  it came back ambiguous rather than empty.** `0x00c49000` is touched from
  exactly three places (`Scene_PrepareFrame`, `FUN_006ce6e0`, `FUN_00107200`),
  is a structured block (small fields at `+0xc0`-`+0x200`, a dense `float4` run
  at `+0x7ba0`-`+0x7d90`, and nine sub-blocks at stride `0xbe0` from `+0xf80`),
  and - importantly - **neither toucher calls an RSX constant uploader or a
  parameter binder**, so it is not handed straight to the shader-constant path.
  `FUN_006ce6e0` installs `base+0x7c00` as a pointer into `*(obj+0xd8)+0x198`;
  neither it nor `FUN_00107200` references any string, so the `.cpp`-filename
  route is unavailable for either.

  **That indirection is now read, and it explains the missing upload call.**
  `Scene_PrepareFrame` does not upload the block - it **registers** it, writing
  pointers into a table hanging off `*(obj+0xd8)` field by field and setting
  `*(outer+0x4) |= 0x000c0000` after each. At `0x003ab38c` it installs
  `&block[0x7c10]` - one of the three `Scene.Texture Colour` destinations - into
  `table+0x1b8`. So the data path is complete: stage colour -> `0x00c81a5c` ->
  `0x00c50c10` -> published to a consumer through a dirty-flagged pointer table.
  A table of "where this value lives" pointers, filled per frame and flagged
  dirty, is the shape of a lazily-uploaded parameter table, and the same concept
  `Shader_InitParamEntry` implements on the shader side. **Buffer identity 55 ->
  65, still unnamed**: what consumer honours `0x000c0000`, and whether this is
  *the* shader parameter table, are unread.

  **The dirty-bit key was then run, and it fails - the static route is
  exhausted.** Code-only scan (below `0x750000`, so `.rodata` in the text
  segment is not mis-decoded): 212 `oris rA,rS,0xc` **setters** across the
  render layer (36 in the three block-touching functions), and **zero**
  `andis.`/`andi.` immediate tests of those bits anywhere; the 76 `rlwinm`
  bit-12/13 extractions are all outside the three functions and none follows a
  `lwz rX,0x4(rY)`. So `|= 0x000c0000` is a **generic** dirty convention, not a
  signature of this block, and the flag is consumed through a register- or
  table-held mask - meaning **no immediate-keyed scan can find the consumer**.
  That is a boundary, not a shortfall: the remaining instrument is a watchpoint.
  Best target is a **read** watchpoint on `0x00c50c10` (the address that gets
  published), which names the consumer outright.

  Gated on a byte at `0x00d45f84`, which is **not** established as Zone-specific
  (seven TOC slots, ~40 users across the scene/render/front-end code). Do not
  write "Zone enables it" anywhere on the strength of this.

  **Superseded, 2026-08-31: it is Zone-specific, named `g_ZoneEffectsActive`,
  and its own writer is found.** [zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md)
  traces the byte to `Environment_LoadRaceScene`
  (`0x003f3fb0`), which sets it to `(1 << mode) & 0x00206040 != 0` - exactly
  mode ids **6, 13, 14 and 21**, three of which `mode-manager.md` had already
  flagged as the Zone/Zone Battle/Detonator candidates with no `ModeManager`
  of their own, and one of which (`14`) `zone-effectsettings-loader.md`'s
  tenth pass independently pinned as Detonator. Confidence 82. The same page
  also answers the maintainer's sky-gradient play report: the gate switches
  `Data/Tex/ZoneSky.gtf` in for the circuit's own `sky.gtf` wholesale (a file
  swap, not a computed gradient - confidence 84), and locates but does not
  yet identify the consumer of a *second*, genuinely computed horizon/zenith
  gradient gated on the same byte (confidence 80 on the read, 55 on what it
  draws - below this project's naming threshold).

- **2026-08-30, `FunkLayerColour2d_fp` is a clean negative.** The one
  post-chain program the input enumeration did not cover: its fragment program
  is `MOV H0, f[TC0]` with zero parameters and zero samplers, its vertex
  program declares only `colour`, and its single draw wrapper
  (`FunkLayer_DrawColourQuad`, `0x003cbc58`) has eleven call sites that every
  one of them feeds a **constant** - `Scene_PrepareFrame` passes a hard-coded
  opaque red `(1,0,0,1)`, the other ten are the `FunkLayerCorruption` glitch
  overlay. Note the argument is made on the *call sites*, not on the shader:
  a flat quad under a multiply blend would be a legitimate full-screen tint, so
  the shader's shape alone would not have settled it. Details in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.


- **2026-08-30, the post-process route is closed - HD has no live whole-frame
  grade.** The `fullscreenTintColour` lead in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` was chased to the end, and
  the useful result is the *enumeration* rather than the one parameter: the
  resolve pass's five full-screen colour inputs are now all accounted for and
  none is fed from `g_effect_settings_stages`. `fullscreenTintColour` is bound
  every frame with `(0,0,0,0)` (its global is cleared per frame and written
  non-zero by nothing in the image); `saturation`/`finalScale`/`finalBias` are
  photo mode's exposure controls and both float4s are built **grey**, so they
  cannot express a hue; `colourScale` is bound every frame with `(1,1,1,1)`,
  its three setters having no caller anywhere. **[2026-09-15: re-run against the `lvlx`-aware PS3 language, negative holds - see renderer.md's dated addendum to this section.]** Full evidence, with the
  whole-image scans that make it an enumeration rather than a failed search, in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` ("The resolve's full-screen
  colour inputs, enumerated") and cross-referenced from
  `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.

  **What this means for this thread.** The eighteenth pass proved by play that
  HD's Zone mode really does recolour scene and track, and listed three
  plausible mechanisms: a post-process tint, a fog-driven material parameter, or
  an indirect-call reader of `0x00c81a5c`. The first is now gone. It also means
  **2048's composite-grade finding must not be carried across as a template for
  HD** - the two engines differ here, and the recolour has to be happening
  before the resolve. `FunkLayerColour2d_fp` (the post chain's flat-colour quad)
  is the one program that could still paint a full screen by a route the
  enumeration does not cover, and is unexamined.


- **2026-08-28, a play-based lead pointing straight at this thread.** Fixing
  today's `ZoneCircuit::Separate` regression (HD's menu asking
  `catalogue::tracks_of_kind(xml, "Zone")` for zone circuits when HD's own
  four are `type="Race"` + `zone="true"`, not `type="Zone"` - see
  `zone-flies-its-own-environment-now-the-menu.md`) surfaced a user
  recollection from playing the original HD/Fury and Pulse: ordinary
  circuits like Anulpha Pass (HD) and Moa Therma (Pulse) carry **a "zone
  pendant"** in the original menu, and their own words on the mechanism -
  "maybe the zone-annotated tracks are zone only, while the others just use
  an effect over the normal tracks" - describes almost exactly what this
  thread already found and left unwired: `zonemode.effectsettings` /
  `zonemodedlc3.effectsettings` are **title-wide**, not per-circuit, and
  2048's `SameCircuit` shape already proves the "any circuit + a colour-grade
  effect" mechanism is real on this engine lineage, not a guess. What that
  implies for HD specifically, unverified at the time this bullet was first
  written: HD's Zone mode may not be `ZoneCircuit::Separate`-only (four
  dedicated `Zone_N` environments and nothing else) - it may *also* let a
  player pick any ordinary circuit and race it Zone-mode with
  `zonemode.effectsettings` laid over it, the pendant marking which ones
  qualify.

  **2026-08-28, later the same day: implemented, on the user's explicit
  instruction to widen it now rather than wait for the executable.** Two
  corrections and a correction-to-the-correction first: "Moa Therma isn't on
  HD" (this bullet's own first draft) was **wrong** - checking a directory
  name for the string "Moa Therma" missed it, because HD stores that circuit
  under the non-descriptive id `03_Track`; reading `03_Track` up through
  `DATA06`'s `entries.xml` (the copy that names all 28 ids) returns `MOA
  THERMA` directly. And the four `zone="true"` circuits are not generic
  placeholders either: the same lookup on `25_Track`..`28_Track` returns
  **Pro Tozo, Mallavol, Corridon 12, Syncopia** - four real, distinct track
  names, each with no `reversed="true"` sibling, unlike all twelve ordinary
  circuits. Both findings corroborate the widening rather than contradicting
  it. `oag_title::ZoneCircuit::Separate` now carries `also_race_circuits:
  bool` (`true` on HD, `false` on Pure, which stays exactly its own four -
  no play-based lead pointed at Pure needing the same treatment); HD's
  `menu_tracks` now offers all 28 `PI_Track` entries in Zone mode (the four
  zone-exclusive ones first) and `variant_of` stops substituting entirely on
  this title, so the load path actually races the picked circuit rather than
  silently rewriting it underneath a menu that now offers it. Ten
  `zone_ground_truth` tests and the full `just` gate pass, including a new
  HD-specific load test proving an ordinary circuit's geometry survives Zone
  mode unsubstituted. **What this is and is not**: a play-based recollection
  plus the two disc-level corroborations above, not a decompiled selector -
  confidence stays where a hypothesis sits, not where the rest of this file's
  measured findings do. `zonemode.effectsettings` itself is still not wired
  into rendering, so an ordinary circuit raced in Zone mode on this engine
  currently looks exactly like a normal race on it - what would close that is
  unchanged from the paragraph above: either HD's own Zone-mode circuit-select
  screen decompiled (the loader trail is already found and stuck on a TOC
  defect two items below), or further play-based confirmation of the
  mechanism's edges (does every ordinary circuit qualify, or only some; does
  the pendant itself ever appear in this engine's own front end once drawn).
- The stage-index <-> zone-number mapping is inferred from the names alone
  (`"Zone N <Name>"` / `"N <Name>"`), not checked against `Zone_Update`'s own
  10-second-per-step timer (`docs/gameplay/race-modes.md#zone`,
  `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`). Nothing confirms stage
  `N` is shown while the zone counter reads `N`, versus some other indexing
  or interpolation between stages. **Sharpened this session, not settled**:
  extracted HD's `speech_zone.bnk` and read its cue table with `oag-wad
  sounds` directly (rather than trusting `psp-audio.md`'s prose count) -
  cues 26-39 are fourteen **consecutive** `MR_*` names, `MR_SVE`/`MR_VEN`/
  `MR_SFL`/`MR_FLA`/`MR_SRA`/`MR_RAP`/`MR_SPH`/`MR_PHA`/`MR_SUP`/`MR_ZEN`/
  `MR_SUZ`/`MR_Z_SUB`/`MR_Z_M1`/`MR_Z_SUP`, matching this table's own 14
  non-`Start` stage names in the same order, one for one, with no gap - so
  the `MR_Z_*` prefix on the last three is cosmetic, not a second block.
  That is strong evidence this **is** the `SpeedClass`/`NextSpeedClass`
  ladder HD's own HUD and announcer already use - see
  `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`.
  **Sharpened again the same day, on 2048's own executable**: `Zone_UpdateStage`
  reads a per-craft field, clamps it to `12`, an exact match against 2048's
  own thirteen-stage table - real selection logic, found and named (see the
  new section above). **Still open**: what struct that field belongs to,
  what writes it, and whether HD's own selection works the same way - no
  write site was found and this is 2048's executable, not HD's, so per
  `CLAUDE.md` it stays unwired until the write side closes the loop.

  **2026-08-30: both write sites found, independently, same day.**
  2048's own `+0x634`/`+0x638` writer is `Hud_UpdateZoneSpeedClassWidget`
  (`0x81197d6c`, confidence 82) - the Zone-mode HUD's Speed Class
  number/lights widget, identified via its named init sibling's scene-node
  lookups (`"ZoneNumber"`/`"ZoneLight%d"`/`"ZoneSpeedLogo"`). **Not a
  race-progress or timer system** - it computes the class index every frame
  from a percentage-shaped value against a 17-entry Mach-number threshold
  table (`MX_CLASS`, `M1_9_CLASS`, ... `D_CLASS`, corroborated independently
  by `Speed_Class_A_Plus_0`-style strings and `speedclass/*.tga` paths
  elsewhere in the binary) and happens to write into the same per-craft
  field the colour-grade blend reads. Confirms HD's own
  `speech_zone.bnk`/`effectSettings` stage-name evidence from the
  executable side, not just audio/file content. Full trace on
  [zone-environment-fallback.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md#2026-08-30-a-fourth-pass-0x6340x638s-writer-is-found---its-the-zone-mode-huds-own-speed-class-widget-not-racephysics-logic).
  **HD's own write site was found the same day, independently and first**
  (see two sections above) - a different function, `Environment_UpdateStageBlend`,
  a self-contained request/commit pair rather than a HUD widget - so 2048's
  answer ended up corroborating the "Speed Class, not a bespoke timer"
  thesis rather than being needed to find HD's own writer. Both titles'
  write sites are now closed; what remains open on both is the *consumer*
  of the resulting blend (render wiring), unchanged from before.
- 2048's table names 13 stages but the title ships 15 files in each texture
  set - still unexplained, though sharpened by the texture-content finding
  below: it is a mismatch in the "Track" set that carries the real art, not
  spare copies of the blank placeholder. Either two stages are
  unnamed/implicit, the extra two are unused leftovers from the HD port, or
  the correspondence between stage and texture index is not 1:1 to begin
  with.
- Why `detonatormode.effectsettings` carries Zone's own 15-stage speed-class
  ladder is unread. Detonator has no "zone number" that ramps the same way
  Zone does (or does it? - unconfirmed), so either the table is reused
  wholesale regardless of relevance, or Detonator has an escalation mechanic
  of its own that has not been looked for. **One data point, not a
  resolution**: `Environment_Load` treats Zone and Detonator as genuinely
  separate tables even in its own fallback path (`ZoneModeHDFury` vs
  `DetonatorModeHDFury`, picked by a mode flag) - the engine's own code does
  not assume they are the same table, whatever the shipped files' shared
  stage names might suggest.
- **Checked, not proven absent.** `scripts/mine-names.py`'s own candidate
  generation (executable strings, template combinations, the plugin
  definition's real track locations) was matched against each disc's
  `Data.wad` - 1,409 candidates for Pulse, 1,509 for Pure. Every `zone`-named
  entry either resolves is geometry, the shared hull, audio, or HUD/UI -
  Pulse's `screen_zone.xml` was read directly and is a front-end track-select
  carousel layout, its only colour a `0xCF000000` UI drop-shadow constant,
  not a palette; Pure's four `Data\Zone\0N_Zone\TrackStartup.xml` are 162-163
  B, too small for fifteen palettes. A further 322 hand-guessed candidates
  (stage-name fragments against both titles' own path conventions) added
  nothing. Their Zone circuit is plain `Skycube`/`fogCube` geometry
  (`docs/formats/skycube.md`, `docs/formats/vex.md`). **A hash-named WAD
  cannot be proven empty**, only searched - but two independently generated
  candidate sets agreeing on nothing is real evidence, not just "not
  checked."
- **Which texture is `Growing Texture` is settled for HD; what the
  parameters do to it is not.** The "general" `zonemode{0..14}.gtf` set is a
  red herring - all fifteen byte-identical, all decoding
  (`oag_texture::gtf`) to a flat white texture with nothing in it. **The
  "Track" `zonemodetrack{0..14}.gtf` set is where the real art is**: fifteen
  distinct files (by `md5sum` and by decoded content), a greyscale image
  with a varying alpha channel that visibly changes shape across the sampled
  stages (0, 1, 7, 14) - blocky interlocking shapes at `Start`, fine
  vertical stripes partway through the ladder, a dense small grid at
  `Supersonic`. Consistent with the name - an escalating visual keyed to
  speed class - but **what the shader does with `Growing
  Texture.Colour`/`.Scale Bias`/`.Factors` against this art is unread**; no
  HD executable has been read for its Zone shader. **2048's own copy
  couldn't be decoded, but its byte pattern already corroborates the same
  split**: `zoneMode*.gxt` all identical, `zoneModeTrack*.gxt` all distinct
  - checked from raw bytes alone. Both sets carry format byte `0x0c` -
  **`SceGxmTextureBaseFormat U8U8U8U8`, corrected 2026-08-28 from an earlier
  `U4U4U4U4` mislabel** in `docs/formats/gxt.md`'s own table, found while
  chasing this exact decode: the raw `format`, `0x0c001000`, bitwise-matches
  `U8U8U8U8 | SceGxmTextureSwizzle4Mode::ARGB` against the public vitasdk
  headers, `0x0c000000 | 0x00001000`, corroborated two ways (the enum's own
  ordering and the exact bit match) - one of the six format codes
  `docs/formats/gxt.md` already catalogues (99 of 9,910 files) but leaves
  undecoded. **Not just a channel-order question**: `U8U8U8U8` needs no
  block math, but whether its four bytes a texel read raster or the same
  Morton/twiddle order `.gxt`'s own `UBC2` reader needs is unset by anything
  in the file (`type` at descriptor `+0x10` is `0` on these textures, the
  same value that told the `UBC2` reticle nothing and had to be settled by
  decoding both ways and looking at the picture).

  **2026-08-28, later the same day: decoded.** `oag_texture::gxt::Format::Argb8888`,
  confidence 80. Twiddle order settled the same way, at texel rather than
  4x4-block granularity - raster order on `zoneModeTrack{0,7,14}.gxt` decodes
  to the same horizontal-banded noise `UBC2`'s wrong-order control gives;
  twiddled decodes cleanly, every texel's `A`/`R`/`G` bytes agreeing exactly
  (0 mismatches across 65,536 texels checked) as a binary stencil mask, `B`
  alone carrying a continuous gradient. **What 2048's own "Track" art
  depicts, now answered**: not the same picture HD's independently-decoded
  `.gtf` copy showed (that was a different container/codec read long before
  this decoder existed) - 2048's version is a thin horizontal mask band whose
  *shape* escalates across the stage ladder (solid at stage 0, increasingly
  dashed by 7 and 14) while its *area* holds fixed at exactly 2,048 of 65,536
  texels on every sampled stage. See `docs/formats/gxt.md`'s new `U8U8U8U8`
  section and `crates/texture/tests/gxt_ground_truth.rs`'s
  `the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages`, whose
  renders (stages 0, 7 and 14, not kept in the repository) this rests on.
  **Still open**: what the shader does with `Growing Texture.Colour`/`.Scale
  Bias`/`.Factors` against this art - no HD executable has been read for its
  Zone shader, so wiring this into a render is unstarted.

## Next Steps

- ~~Read the remaining five `Track.*` rows of the cross-fader.~~ **Done
  2026-08-31**; the whole block is the table in the twenty-fourth pass.
- ~~Read which of the eight parameter publishers a material's draw goes
  through, the same question that decides `zoneMode*` versus
  `zoneModeTrack*`.~~ **Answered 2026-09-05, and the "same question" half of
  that sentence was wrong** - the two are orthogonal axes. A draw goes
  through the publisher that owns the draw **bucket** it was routed into, and
  the two Zone-relevant buckets are split on `material+0x10` bit 0, the
  transparency mode `rcsmodel.md` already measured from the file side (85).
  Scene versus Track is a *separate* bit, and it is **per chunk**: record bit
  1 is bit 0 of `(*(chunk+8))->0x06`, loaded outside the surface loop in
  `Scene_SubmitVisibleChunks`, which retires the twenty-eighth pass's
  "plausibly per material" reading rather than confirming it (85). Full
  trace, including the eight publishers listed by address and the five-way
  material class that picks the input list: twenty-ninth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  ~~**Nothing in it changes what a port binds** - `oag_render` keeps binding
  the Track set, because the per-chunk bit still has no located writer.~~
  **It does change it - see the next item.**
- ~~**Find the writer of `block->0x06`**~~ **Found, 2026-09-15, and it is not
  an instruction: the halfword is authored in the `.rcsmodel`.** The chunk
  header's `+0x08` is a relocated file offset to a 0x40-byte per-chunk record
  (the "render block"); `+0x06` of that record carries the bits as shipped,
  bit 0 set = Track. Surveyed over all 643 models: 15 distinct values, and the
  sorter's routing bits 2-4 are authored on only three circuits (Vineta K,
  `03_track`, Ubermall). Full table in the thirtieth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  One correction on the way: `+0x74` is not on this record but on the object
  its `+0x00` points at.
- ~~**Port it** (renderer lane, not done here): have `oag_rcs::rcsmodel` expose
  the record's `+0x06` per chunk (`Mesh`-level; the header's `+0x08` word,
  resolved as a file offset) and bind `zoneModeTrack*` when bit 0 is set and
  `zoneMode*` otherwise.~~ **Ported, 2026-09-15.**
  `oag_rcs::rcsmodel::Mesh::render_flags`/`is_track` read the record;
  `mesh::slots::ZONE_TRACK` carries the bit per chunk; `mesh_render::Zone`
  holds both colour groups and bind group 2 both stage textures; `mesh.wgsl`
  selects per fragment. The disc counts reproduce from the reader (41,861
  relocated, 4,365 in 37 files, 124 of 983 on Talon's Junction -
  `crates/rcs/tests/rcsmodel_ground_truth.rs`). What a headless capture of
  Talon's Junction shows, **on `Sub Venom`** - at the time this paragraph was
  written every headless capture landed there regardless of `--zone-stage`,
  because `Scene::sync_zone_grade` re-shows the recovered ladder's rung every
  frame and zone 0 is stage 1, overwriting the override from tick 1.
  **Fixed the same day, later**: `ZoneGrade::pin_stage`
  (`fix(zone): --zone-stage pins the colour grade instead of losing to the
  ladder`, 2026-09-15, covered by `zone_grade/tests.rs`) makes the override
  stick, so a `--zone-stage 0` capture is no longer overwritten from tick 1
  - `Start`'s dark-environment frame has not actually been captured this
  way, only the mechanism that would reach it is fixed. This paragraph's
  own capture predates the fix. The near
  track-side walls and gantries (track-flagged chunks) are dark teal with
  cyan rim edges and the road is lit cyan; the elevated scenery and
  everything in the distance (scene chunks) is **blown-out white**. That
  white is the data through the read equation, not a defect of the split:
  `zonemode1.gtf` is flat white, `Scene.Texture Colour` is `0.72 0.91 0.96`,
  and `mesh.wgsl` then multiplies the surface by the stage's own rig
  (`lit_linear = surface_linear * authored`, with `Constant Ambient Colour`
  `1.5`), so every channel lands above `1.0` and the target clamps it - the
  missing tonemap stage `renderer.md` already names. Zeroing the rim term
  or clamping `rim` to one changes none of it (both tried). Before the split
  every surface had the road's patterned cyan. On `Flash` (1,800 ticks) the
  scenery is flat lavender and the road orange. **Not checked against a
  frame of the original**, which an RPCS3 capture on `Sub Venom` would
  settle; if the original's scenery is not white there, the light multiply
  on the Zone surface is the term to re-read, not the chunk split. Still in
  reach from the same field: the five-bucket routing (record bits 3-5),
  which decides which fog buffer and publisher a chunk goes through.
- **Wire the wavefront** (renderer lane): origin = the entity's `+0xb0`
  (re-centred every frame), radius per frame after a commit `r_k = 0.1 + 0.5k
  + 0.05k(k-1)` capped at `20000`, colour weight `min(0.01k, 1)`. All disc
  numbers; nothing chosen. What is still unmeasured is whether the radius's
  world units match `oag_render`'s.
- Two smaller ones left by the same pass: which draw list `FUN_004053e0`
  walks and who consumes bucket B1 (`g+0x311e8`), and what `material+0x10`
  bit 7 means - it is tested before the transparency mode and short-circuits
  it, and nothing on either page names it.
- ~~Settle whether the two Zone publications are sequential or
  alternative~~ **Answered, 2026-09-03, and it was neither.** Both readings
  assumed `FUN_003ff860` is entered once per draw and chooses a path. It
  isn't - it loops over a per-context draw-entry table, and each entry's
  own flag bit independently picks Scene or Track, cached so the
  underlying parameter block writes at most once per group per call. See
  the corresponding item in `## Open` above.
- ~~Find `zoneOrigin`'s writer, or establish it has none~~ **Found,
  2026-09-03** - one call frame above `Environment_UpdateStageBlend`, in
  `Scene_PrepareFrame`. See the corresponding item in `## Open` above.
  **Re-confirmed on the complete image 2026-09-15, with one correction**: the
  source is the object's `+0xb0` float4, not `+0x80` (`li r0,0x30` before the
  `lvx v0,r3,r0`). What is left is narrower still: whether `+0x80` is a 4x4
  whose last row that is (offered at 60 - two other callers of the same
  accessor split that row into x/y/z scalars) and which entity
  `session[id]->+0x6adc` selects.
- ~~Close `zoneTexVis`'s build loop~~ **Done, 2026-09-01** - see the
  corresponding item in `## Open` above. The address named here
  (`0x003d8b40`) turned out to build an unrelated field; `zoneTexVis` itself
  is the audio-spectrum visualiser, and `oag_audio::spectrum` +
  `mesh.wgsl` already implement it.

- ~~Find the tint consumer for `0x00c49110`/`0x00c49120`/`0x00c49130`~~
  **Done, 2026-08-31 - it is the shader parameter `fogColour`.** Both halves
  of that step ran. The three unread opcodes (which were tags **11, 13 and
  14**, not 7/13/14 - the previous pass's tag numbering was off by a slot in
  the 7-11 band) are a bounding-volume trio, so the dispatcher is now fully
  mapped and its exclusion is exhaustive rather than probable. The consumer
  was then found by the route the thread had listed as untried: `*(obj+0xd8)`
  is a **link-time constant**, `0x00d42220`, installed by
  `Scene_InitRenderBlock` (`0x003aa618`, newly named) - so there was no
  allocation to read a `__FILE__` tag off, which is why that route kept
  stalling. `0x00d42220` is `*(0x008b7f04) + 0x4000`, the 81-entry engine
  shader parameter array [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)
  already recovered, so `+0xf8` is parameter **7**'s value pointer:
  `fogColour`. Confidence 90, corroborated by eleven other parameters in the
  same publication run mapping onto sensible scene-block sources. Full
  evidence in
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
  twenty-second pass.

  **What it says the tint actually is.** The merge mask is
  `[0,0,0,0xffffffff]`, so each blended scalar becomes a colour's **`.w`**,
  not one of its channels; and `Environment_UpdateStageBlend` writes the three
  vec4 bases too (`+0x3630`/`+0x3640`/`+0x3650` off the stage base). Lined up
  against the fourteenth pass's measured key-to-offset table (51 keys,
  confidence 88), that makes each `(rgb, scalar)` pair the two halves of one
  authored 16-byte key, stored apart and reassembled per frame - and the table
  names the three cross-faded fields as `Scene.Texture Colour`,
  `Scene.Base Colour Highlight` and `Scene.Base Colour`, with `Scene.EQ
  brightness` occupying `Scene.Texture Colour`'s own fourth lane. **So
  `fogColour` = `Scene.Texture Colour.rgb` + `Scene.EQ brightness` in `.w`**,
  and this *confirms* the seventeenth pass's `Scene.Texture Colour`
  attribution rather than unsettling it. An intermediate reading of these
  three pairs as the three fog blocks (`Fog`/`Alt Fog`/`Track Fog`) was
  withdrawn on that evidence - `Lighting.Fog *` sits at `+0x1a0`/`+0x1d0` and
  is not cross-faded at all.

  What is left from this step is one narrower question, listed below rather
  than here: who consumes `+0x120`/`+0x130`.
- Find the write site for `Zone_UpdateStage`'s `+0x634` field - the field the
  clamp-to-12 match confirms drives stage selection, but nothing this pass
  traced writes it. **Searched a third time, 2026-08-28, and still not
  found**: `Zone_UpdateStage`'s own two callers, `Zone_InitStageState`, the
  craft state machine and an EMP-bar HUD function were all fully decompiled
  and ruled out (see
  `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`) -
  the base pointer has dozens of direct readers across the executable, so
  this next attempt should either identify the containing struct's type
  first (narrowing which of those readers are plausible owners) or use a
  runtime watchpoint if one becomes available, rather than repeat the same
  blind decompile sweep. **A fourth candidate, checked and ruled out the
  same day**: `Environment_LoadHDFuryContent` (the DLC-specific loader) - it
  is a genuinely separate subsystem with no connection to `+0x634` at all,
  so it does not narrow the search either. Only once this is read does
  wiring a stage into a race stop being a guess.
- Trace how the raw bytes `Environment_LoadEffectSettingsFiles` inserts into
  `DAT_816c7148` end up in `DAT_816c4890`'s own table - **sharpened, not
  settled, third pass**: `DAT_816c4890` is confirmed built by
  `Environment_LoadEffectSettingsFiles` itself, inside a block that reverses
  a table in place (gated on a flag plausibly meaning "reversed circuit",
  unconfirmed), not a passive alias of the raw-file cache as it looked from
  the outside last pass. What is still unread is the actual parse/copy step
  that gets bytes from `DAT_816c7148` into that region at all - no call into
  a parser was found in the traced block.
- ~~Wire the current stage's fog/sky/ambient into the renderer for one title
  as a proof of concept~~ **Done, 2026-08-30, on HD/Fury and for Zone
  specifically.** `oag_title::ZonePalette` names where each title keeps its
  table (`TitleWide` on HD, `BesideCircuit` on 2048, `None` on Pulse and
  Pure, which were searched rather than assumed);
  `oag_tables::effectsettings::StagePalette`/`blended_palette` read one
  stage and cross-fade it against the stage before it; and
  `oag_raceplay::zone_grade::ZoneGrade` holds the recovered struct's own
  three fields (`+0x00` current, `+0x04` requested, `+0x18` weight) with a
  `commit` that reproduces `Environment_UpdateStageBlend`'s gate, applying
  the result to the same `mesh_render::Fog`/`Light` path `.envsettings`
  already drives. Checked end to end against `hdfury-ps3-eu-dec.iso` in
  `crates/game/tests/zone_grade_ground_truth.rs`: a Zone race load carries
  the fifteen-stage table, and `Sub Venom` fogs `[0, 1.305882, 1.8]` at
  density `0.0021` where `Start` authors none. **The trigger is still open**
  and is now the *only* thing between this and a race that escalates on its
  own - `request_stage`/`set_weight` are a seam with no caller (a seam in this project's own code, not an executable negative; the HD ladder driving it was recovered 2026-08-31). Two smaller
  questions the wiring surfaced are in
  [effectsettings.md](../../docs/formats/effectsettings.md)'s `## Open`: which
  way `+0x18` runs (the commit zeroes it, `cross_fade_rgba8`'s own doc says
  a fresh stage starts at `1.0` - both cannot be right), and which schema
  keys the three blended runtime fields actually are, which is also what
  decides whether the byte-domain blend should apply to keys this build fades
  in floats.
- ~~Add `SceGxmTextureBaseFormat` `U8U8U8U8` to `oag_texture::gxt`~~ **Done,
  2026-08-28** - see the Open section above and `docs/formats/gxt.md`'s
  `U8U8U8U8` section. What is left from this step is the shader side: what
  `Growing Texture.Colour`/`.Scale Bias`/`.Factors` do with this art, which
  needs an HD Zone-shader read this pass did not attempt.
- **The database-defect blocker is cleared, 2026-08-28, and the trail past it
  is named.** The maintainer re-ran `scripts/import-ps3-eboot.sh --ps3-cspec`;
  verified directly (language now `PowerPC:BE:64:A2ALT-32addr-PS3`, 26,100
  functions, real imports) and this binary's `names.tsv` re-applied 114/114
  clean. `FUN_003d6dc8` - where the trail from `ZoneMode.effectSettings`'s
  load site stopped - redecompiled to something structurally different from
  the old (TOC-defective) "SpeedBar/HUD widget" reading. The first read of
  the new decompile guessed a missing second parameter; that was wrong -
  `analyze_dataflow` traced the mystery value straight back to this
  function's own entry `r2`, i.e. its own already-established real TOC
  (`0x008bd3c4`), lost to Ghidra's constant folding only because several
  indirect (vtable-style) calls sit between entry and first use. Resolving
  the TOC-relative reads by hand found a table of per-stage texture
  filenames - one resolved directly to the string `Data/Tex/DetonatorMode0.gtf`
  - handed to a get-or-create-by-name resource cache. **Renamed, in
  `names.tsv`**: `Environment_LoadStageTextures` (72) and
  `Resource_GetOrCreateByName_q` (65). Full evidence, including the ABI
  mechanics of the TOC-loss and the resolved memory addresses, on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  **Also surfaced, not confirmed**: a table-reversal block just before the
  texture loop, gated by an unidentified flag, has the same shape as 2048's
  own `Environment_LoadEffectSettingsFiles` reversing a fifteen-stage table
  under its own "plausibly reversed circuit" flag - independent corroboration
  that *something* gets reversed on both titles, not of what the flag means
  on either one.

  **2026-08-28, later the same day: confirmed it's a texture, not a generic
  resource.** `_opd_FUN_005de2d0`, the cache-miss path `Resource_GetOrCreateByName`
  calls, carries two unambiguous strings read directly from memory:
  `"ERROR: Can't load placeholder texture %s"` and `"WARNING: Can't load %s,
  using default texture"`, plus a fallback-to-placeholder recursion and a
  GCM-texture-flag-shaped resource construction on the found path. **Renamed**:
  `Texture_LoadWithFallback` (80); `Resource_GetOrCreateByName`'s own
  confidence moved 65 -> 72, crossing the `_q` threshold now that its one
  exercised call site is confirmed rather than plausible. The found/not-found
  probe both paths key off, `_opd_FUN_005d9610`, is read too: a generic
  filename-in-search-path resolver, not Zone-specific - **renamed**
  `FwFile_ResolveInSearchPaths` (72).

  **The reversal flag is narrowed, not closed.** `iVar12` (the reversal
  condition's own base pointer) is `g_GameState`, confirmed two ways: the TOC
  math resolves it to that exact address, and the very next condition checks
  `+0xe0` - `g_GameState`'s already-documented mode field
  ([mode-manager.md](../../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md))
  - against `14`, one of the ids that page already bucketed as "no
  `ModeManager`, Zone/Zone Battle/Detonator the obvious candidates". Two
  independent functions now key real behaviour on id `14` specifically,
  strengthening that bucket without saying which of the three it is. **Not
  chased further**: `get_xrefs_to` on `g_GameState` returns 100+ hits, the
  same "dozens of direct readers" shape 2048's own `+0x634` search hit three
  times - the next attempt needs `g_GameState + 0x1bc`'s pointee struct typed
  first, or a runtime watchpoint, not a fourth blind sweep.
- Worth checking against the user's own play on one of the four ported
  `zone_N` circuits in Zone or Detonator mode: a visible symptom (no fog, no
  palette shift, a black/flat scene) would corroborate "the blend math runs
  against zero-initialised data" more cheaply than a runtime trace would.
- ~~Read the schema table `FwKeyedText_ParseEntry` looks entries up
  against~~ **Done, 2026-08-29**: the vocabulary is read, not the 36-byte
  runtime record table itself (still in unreadable `.bss`) but a 73-entry
  compile-time array of key-name strings (`g_EffectSettingsSchemaKeyNames`,
  `0x008b79cc`), whose boundaries are measured (a float ends the block
  before it; `iVar8 + 0x1000`, the already-identified per-stage destination
  base, sits immediately after it ends) rather than reconciled after a
  guess. Full list, boundary evidence, and per-file verification against
  all four shipped `.effectSettings` files on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-the-schema-table-is-read-and-it-is-the-full-recognised-vocabulary-not-a-guess).
  This settles the vocabulary/spelling question `docs/formats/effectsettings.md`
  used to answer with a guess (73 recognised keys; only 8 never appear in
  any shipped file), and surfaces key groups (`Scene`/`Track` gradients,
  sky/aurora/airbrake, Detonator-only mine/bomb colours) no file-side read
  had found. **What it does not settle**: the actual write site of the
  runtime `.bss` schema pointers (`*(param_1+4)`/`*(param_1+8)`) was not
  traced to this array - the connection is structural (measured boundaries,
  verified content, a sibling array in the same binary using the identical
  shape), not a traced data flow, since `.bss` cannot be read without a
  live process. **The next concrete step is unchanged in shape, sharper in
  target**: find who reads `iVar8 + 0x1000` (the per-stage struct table
  this schema's destination pointers write into) - that is the actual
  stage-selection trigger `CLAUDE.md` requires before any of this reaches
  the renderer. **Not a new lead, but worth citing precisely**: the
  schema's `%s` fill-in values (entries 7-21) being the same 15
  speed-class names `effectsettings.md`'s own file-side reading and HD's
  `speech_zone.bnk` `MR_*` cues already agreed on is a third, independent
  source for the same ladder - it does not by itself establish *how* the
  original selects a stage, and does not license assuming HD reuses
  Pulse's own already-implemented elapsed-time zone counter
  (`crates/race/src/zone.rs`) without checking HD's executable for the
  same mechanism first.

  **2026-08-29, later the same day: a strong candidate for the reader,
  not confirmed - and both its source pointers are now identified.** A
  byte-pattern search for the same TOC-relative load
  `Environment_LoadStageTextures` uses for `iVar8`, OPD-verified per
  `memory.md`'s method rather than trusted from Ghidra's own xrefs, found
  two real hits and ruled one out: `FUN_003d0b98` walks all 15 stage bases
  but only to `memset(stageBase, 0, 0x250)` them (`li r4,0x0`/`li
  r5,0x250` confirmed at every one of the 15 calls) - a clear/reset, not
  a reader, and a genuine negative result: don't re-check it. `FUN_003da540`
  is the live one: it reads a per-entity field, uses it and that value
  minus one to index *two* adjacent stages by the table's own `0x250`
  stride, and copies three RGBA-shaped fields from each into a small
  blended-output area - the same cross-fade shape as 2048's own
  `Zone_UpdateStage`, the first time that shape has shown up anywhere in
  HD's executable rather than only 2048's. **Both source pointers
  resolved by hand, not assumed**: the saved-TOC arithmetic this page's
  own TOC-defect section already established identifies one as `iVar8`
  itself (confirmed - the identical `-0x5a84` displacement
  `Environment_LoadStageTextures` uses) and the other as `*(int*)0x008b7944`,
  the very global `Environment_LoadStageTextures` fills with its own
  default float block - so the per-entity array `uVar55` (the stage
  value) is read from is a **known global**, not an unidentified one.
  **Not renamed**: which entity the per-entity index (`param_2`, before
  its own `*0x38`) selects (craft, camera, something else - traced one
  level up into ambiguous camera/viewport-selection-shaped code, not
  settled) and what consumes the blended output are both still open; a
  name would assert more than is established. Full trace, including the
  exact decompiled slice and the four-function-deep call chain into it
  (whose own last hop, `FUN_0067f078`, turned out to have the *same*
  OPD-self-reference artifact this pass also found and corrected in an
  earlier claim about `FwKeyedText_ParseBuffer` - see the correction on
  the doc page), on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-later-the-same-day-who-reads-ivar8--0x1000---two-candidates-found-one-ruled-out-one-strong-and-unconfirmed).
  **Next concrete step, sharpened now that the source array is a known
  global rather than an unknown one**: find who **writes** the stage
  value at `*(int*)0x008b7944 + n*0x38` - a single specific global and
  field pattern, and a far more tractable target than tracing
  `FUN_003d9970` (~3 KB, what `FUN_003da540` hands its read fields to) or
  resolving `FUN_003aa888`'s camera/viewport code cold.

  **2026-08-29, a fourth pass: the write to offset `+0` (the stage index
  itself) is still not found, but the global is confirmed flat (not a
  pointer), a write with no located caller was found and correctly *not*
  called a vtable method the second time round, and a real write to a
  neighbouring field (`+4`) turned up in a part of `FUN_003da540` neither
  pass had actually read.** `FUN_003da540` (~15.5 KB) and `FUN_003d0b98`
  (~24 KB) are both far bigger than either pass's own excerpt suggested -
  measured with `get_function_by_address` this time. `FUN_003d0b98`
  re-checked with two nets scoped to itself (`stwx`, the `*0x38` stride
  idiom): still nothing past the already-confirmed memsets, so "ruled
  out" stands, on a function most of which is still unsurveyed.
  `FUN_003cdc90` writes offset `+0` for entries 0 and 1 to zero (a real
  write to the exact pattern) but its only `get_xrefs_to` hit is its own
  OPD range, and a second-order check (`get_xrefs_to` on *that* address)
  returns nothing - so it has no located caller **[2026-09-15: re-run against the `lvlx`-aware PS3 language, negative holds - see zone-effectsettings-loader.md's thirty-third pass]**, corrected from a first
  read that called it a vtable "Reset" method before checking that far.
  New, found by disassembling rather than trusting the prior slice:
  `FUN_003da540` itself writes entry `n`'s offset `+4` (gated by a flag
  byte at `+0xc`) from a small object-table lookup - offset `+4` is the
  same field `FUN_003ce2c0` (a second ~10.2 KB function, found here,
  that independently re-derives the identical cross-fade blend
  `FUN_003da540` does, corroborating the mechanism) switches over into a
  fixed `+0x74` field that then picks one of nine draw/audio branches.
  `+0x74` now has **two disagreeing producers** - this switch, and
  `FUN_003cddc0`'s random-non-repeat write from the previous pass - not
  reconciled. Full trace on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead).
  **Nothing wired into Rust from this pass**: the stage-selection write
  itself is still unfound and the `+4`/`+0x74` mechanism has two
  unreconciled producers, both short of `CLAUDE.md`'s bar for firing an
  effect on a recovered trigger. **Next step**: trace who sets the `+0xc`
  flag byte that gates `FUN_003da540`'s own `+4` write - a single byte at
  a known offset, more tractable than a further blind sweep for the
  `+0` write.

  **2026-08-29, a fifth pass: the `+0xc` answer was already in this
  thread's own decompile, plus three more of the fourteen per-entry
  fields placed by offset.** A blind `stb` sweep across two more giant
  functions (`FUN_003d0b98`, `FUN_0009bb30`) hit the exact same
  "drowns you" trap the fourth pass's advisor call had already flagged
  for `stw` - both results are generic small-struct-clear noise, not
  evidence about this field, and cost real budget for nothing. The real
  answer: `FUN_003cdc90`'s already-quoted decompile sets dword `[3]`/
  `[0x11]` (both entries' own `+0xc`) to `0x3f000000` - big-endian, so
  the top byte read at `+0xc` is `0x3f`, non-zero, which is exactly the
  state that skips `FUN_003da540`'s own gated write. Whether that is an
  intentional flag packed into a float's top byte or a coincidental
  alias is not settled either way - PowerPC decompile has no bitfield
  syntax to distinguish them - and `FUN_003cdc90` still has no located
  caller (previous pass), so even a confirmed write does not establish
  when it runs. Side benefit: dword `6` (`+0x18`) is the exact offset
  `FUN_003ce2c0` reads as blend weight, seeded from a real neighbouring
  default (`0x008b7944 + 4`) rather than a literal zero; dword `13`
  (`+0x34`, each entry's last field) is seeded `1.0f`. Layout now known
  for five of fourteen dwords per entry: `+0x00` stage, `+0x04` event
  code, `+0x0c` the aliased flag, `+0x18` weight, `+0x34` a `1.0f`
  field. Full write-up on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fifth-pass-the-0xc-flag---answered-from-data-already-in-hand-and-a-course-correction-on-search-scope).
  **Still nothing wired into Rust**: layout knowledge, not a trigger -
  `FUN_003cdc90`'s missing caller is the harder blocker than the flag's
  own meaning. **Next step, unchanged in kind**: the stage-index write
  (`+0x00`) itself remains the actual target; a caller for
  `FUN_003cdc90` (an indirect/computed call Ghidra's static xrefs won't
  show, so this likely needs a runtime watchpoint rather than another
  static sweep) would settle both when reset happens and, by extension,
  when advancing past it must happen too.

  **2026-08-29, a sixth pass: the reason five passes missed the write -
  `0x008b7944` is a pointer slot, and the "closed" candidate list was one
  fifth of the real one.** `read_memory` at `0x008b7944` returns
  `0x008c2cb8`; the fourth pass's "flat data, not a pointer" check
  (`inspect_memory_content` at `+0` and `+4` showing "the same run
  shifted") is true of any address and proves nothing. The array is at
  `0x008c2cb8` in **initialised** `.data`, so its compile-time contents
  are readable: exactly **two** entries of `0x38`, holding precisely the
  values `FUN_003cdc90` writes - so that function is a
  reset-to-defaults, and the start state (`+0x00 == 0`, both entries) is
  known without ever finding its caller. `search_byte_patterns` for
  `008c2cb8` finds **five** TOC slots holding it; every prior pass
  searched one (`-0x5a80(r2)`). The other four open up eleven functions,
  seven of them entirely unread - each one's TOC read from its own OPD
  entry, not inferred from a verified neighbour's address. Exhaustiveness checked properly this
  time: no slot points mid-array (`008c2cf0`: no matches) and no
  absolute materialisation exists (operand search `0x2cb8`: thirteen
  hits, none this address). Two clean negatives: `FUN_0009bb30` - the
  brief's leading candidate - and `FUN_0007b388` both have TOC
  `0x008ad4d8` (OPDs read directly), so their `-0x5a80`/`-0x61a4` hits
  resolve to unrelated globals; they never touch this array, and the
  fifth pass's 117-hit `stb` sweep over `FUN_0009bb30` was spent on the
  byte-pattern trap. Two corrections to earlier offset claims, both from
  the same cause - **this compiler folds a constant bias into the index
  register before adding the base**, so a bare `d(rX)` displacement is
  not the struct offset: the gate `FUN_003da540` tests is `+0x2c`, not
  `+0x0c` (bias `0x20` at `0x003da5f8`), it reads `0` in `.data` so the
  gate **passes** rather than skips, and the fifth pass's "flag aliased
  into a `0.5f` top byte" story is void; and `+0x74` is not a field of
  this struct at all (past the two-entry array's `0x70` end), so the
  fourth pass's "two disagreeing producers for `+0x74`" dissolves rather
  than needing reconciliation. The `+0x04` write at `0x003da674` does
  stand - re-verified with the bias checked. Also: the four
  `void { return; }` stubs are all real vector getters (a fourth
  independent confirmation that `+0x00` is the stage index), and they
  name a new global `0x008c1430` holding the current entity index - and
  with two entries, that index reads as viewport/player, not stage.
  All fourteen per-entry dwords are now placed by value and eight by
  role. Full write-up on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-sixth-pass-the-array-is-a-pointer-away-the-candidate-list-was-one-fifth-of-the-real-one-and-two-offset-readings-were-off-by-a-folded-bias).
  **Still nothing wired into Rust, nothing renamed**: the `+0x00` write
  remains unfound, and the array's owning subsystem is still inferred
  from its consumers rather than established. **Next step, now bounded
  rather than open-ended**: sweep the eleven newly-reachable functions
  (`FUN_003e29e0`, `FUN_003ff860`, `FUN_00400a00`, `FUN_00403a30`,
  `FUN_004053e0`, `FUN_004074e0`, `FUN_00408fa8` unread; `FUN_003df360`,
  `FUN_003fc140`, `FUN_003aa888`, `FUN_002b61c8` sampled) with the
  per-function `rlwinm ..., 0x6, 0x0, 0x19` enumeration plus a +-0x30
  disassembly window at each hit - a handful of sites per function, never
  a bare-mnemonic sweep. **Better still, and what five static passes now
  argue for**: `0x008c2cb8 + 0x00` is a fixed `.data` address, so one
  RPCS3 run of a Zone race with a write watchpoint on it settles in a
  single shot both whether the field is ever non-zero and who writes it -
  including through the indirect calls that have kept `FUN_003cdc90`'s
  caller hidden for three passes.

  **2026-08-30, a seventh pass: the eleven functions are checked, and
  none of them writes `+0x00`.** All seven of the sixth pass's remaining
  unread/sampled functions were traced the way that pass specified - per
  function, from each `n*0x38` computation's own `rlwinm`/`rlwinm`/`subf`
  triple, bias checked, not a bare-mnemonic sweep. Five of the
  `-0x50e4(r2)` family (`FUN_00403a30`, `FUN_004053e0`, `FUN_004074e0`,
  `FUN_00408fa8`, `FUN_003fc140`) are the same read-only shape already
  confirmed for that family's other three members: a `+0x18` blend-weight
  read feeding a distance compare, then two `+0x1c`/`+0x20` reads feeding
  an unrelated telemetry call. `FUN_003df360` reads `+0x30` once, straight
  to an output pointer. `FUN_002b61c8` (second module, TOC verified via
  its own OPD) writes `+0x30`/`+0x34` only - the same two fields
  `FUN_003aa888` already established, nothing at `+0x00`. **All five
  TOC-verified paths to the array are now fully read, not sampled, and
  the static search space for this exact address is closed** - short of
  an indirect/computed call no static tool here can enumerate. Full
  trace on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-seventh-pass-the-last-two-candidate-families-are-exhausted-and-the-0x00-write-is-not-in-any-of-them).

  **Also checked the same day, since it changes what the remaining
  avenue costs**: the RPCS3 build on this host (`rpcs3-bin` AUR package
  `0.0.42.19777-1`) is confirmed the exact build `rpcs3-debugger.md`
  measured `Z2` (write watchpoint) against - same host, not a stale
  note - and it genuinely does not implement `Z2` (empty reply, not
  `OK`). **Checked upstream too, not left as a maybe**: fetched
  `RPCS3/rpcs3`'s own `master` `GDB.cpp` directly - only `Z0` is
  handled, `Z1`-`Z4` all fall through to the same empty reply, and this
  has been true since GDB support shipped in 2017 (commit history since
  is refactors only). GitHub's issue/PR search for "watchpoint" returns
  zero results either way - nobody has filed for this. `rpcs3-git`
  tracks the same `master`, so switching to it gains nothing. See
  [rpcs3-debugger.md](../../docs/reverse-engineering/rpcs3-debugger.md#the-gdb-stub-is-real-and-needs-no-special-build)
  for the full evidence.

  **2026-08-30, an independent verification pass, same day: the seven
  verdicts survive, two of their evidence sets did not, and "closed"
  turned out to be one specific gap short of true.** Re-running the
  enumeration from scratch (not reusing the sweep's own site list) found
  `FUN_00403a30` and `FUN_003fc140` each had three-to-four base-load
  sites the sweep missed entirely (nine real sites against six and five
  listed); all of them traced out as loads too, so no verdict changes.
  More importantly: **every pass so far, this one included, has only
  asked "which functions reference a TOC slot holding this address" -
  and a writer that receives the array pointer as a call *argument*
  rather than loading it from a TOC slot would answer to neither
  question.** `FUN_003df360` already demonstrates the shape in this
  thread's own evidence (it reads the array via its own TOC slot, then
  forwards the result through a caller-supplied output pointer) - a
  writer built the same way round is invisible to every search run so
  far. Full corrected counts and the new gap on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-an-eighth-pass-independent-verification-two-undercounted-function-evidence-sets-corrected-and-the-real-gap-in-closed).
  Also reconfirmed on this binary, not just the PSP side:
  `search_instructions`'s `mnemonic` filter is exact-match despite its
  own docs, so a prefix sweep like `mnemonic="st"` for "any store"
  silently returns zero rather than matching `stw`/`std`/`stfs` -
  [toolchain.md](../../docs/reverse-engineering/toolchain.md#search_instructionss-mnemonic-filter-is-exact-match-not-substring).

  **So there is one more concrete, bounded static step before the
  RPCS3-patch decision needs an answer**: a caller sweep of the thirteen
  now-fully-read accessor functions, checking whether any call site
  passes an array-derived pointer into a further function - not a new
  address search, a call-graph one.

  **2026-08-30, later the same day: found, by the caller sweep above -
  just not by its own hypothesis.** The `+0x00` write is real and inside
  one of the fifteen already-known direct-toucher functions itself, not
  in an argument-forwarded callee (that hypothesis came back negative
  across all fourteen others, checked in full). **Renamed
  `Environment_UpdateStageBlend`** (`0x003da540`, confidence 85,
  `names.tsv` updated, independently byte-verified in-session before the
  rename). It computes `n*0x38` once at entry and reuses it six times via
  a plain `subf` - every prior pass's `rlwinm`-keyed search found only
  the first reuse or two, never the fourth, which is where the write is.
  The mechanism is a self-contained request/commit pair: `entry[n].+0x04`
  (already documented, written elsewhere in the same function from
  `g_GameState.mode`-dependent sources) holds a *requested* stage: every
  call checks whether `+0x04 == +0x00` (already applied) and, if not,
  draws a transition effect and commits `+0x00 = +0x04`. No external
  caller ever needed to pass a pointer in, which is exactly why the
  forwarding hypothesis came back empty even though the write is real. A
  second, unrelated `+0x00` write turned up as a side effect -
  `Environment_LoadStageTextures` unconditionally zero-resets the whole
  array on every effectSettings load - but that is a reset to the
  constant `0`, not a stage-selection write. Full trace, the byte-level
  re-verification, and a live tooling trap (the shared Ghidra bridge's
  "current program" is not stable across concurrent sessions - always
  pass the full `program` path once more than one program can be open)
  on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-ninth-pass-the-0x00-write-is-found---environment_updatestageblend-at-0x003da540-confidence-85).

  **This closes the RPCS3-patch decision point above - a runtime watch
  was never needed to find this write, so there is nothing to greenlight
  or decline here anymore.** What it does not close: the render-side
  wiring this whole page has deferred since its first pass.
  `Environment_UpdateStageBlend` establishes *when* and *to what* the
  stage index changes (a request/commit pair driven by
  `g_GameState.mode`), not what the renderer does with the resulting
  index once it has it - that is `docs/formats/effectsettings.md`'s own
  open question and the next concrete step for this thread, separate
  from the RE work this section closes out.

  **2026-08-30, a tenth pass: 2048's trigger is recovered and wired; HD's
  Zone trigger is not, and mode `0xe` turned out to be Detonator.** Three
  results, one of them a correction to two doc pages.

  **2048, closed.** The 17-entry threshold table
  (`0x8151faf8`, now `g_zone_speed_class_thresholds`) was read with
  `read_memory` rather than decompiled around, and the "inconsistency" that
  stopped the fourth pass is a **sentinel**: seventeen records against
  sixteen names in the blob at `0x8148a4a0` (now
  `g_zone_speed_class_names`), record `0` carrying the unreachable threshold
  `9999` and sharing record `1`'s pointer, every record below it strictly one
  name apiece - checked against the blob's own 12/8-byte string layout, not
  assumed from order. Thresholds: `9999, 90, 85, 80, 75, 70, 65, 60, 55, 50,
  45, 40, 33, 17, 9, 2, 0`; class written is `0x11 - i`, clamped to `0xc` by
  `Zone_UpdateStage`. **The units are zone numbers, confidence 78**: the
  walked value is `sprintf`'d as the first `%d` of `"%d/%d"` into the
  authored scene node `ZoneNumber`, the second `%d` is `FUN_812c4e0c`'s
  per-event target, and the same block stores the matched record's threshold
  and the one above it into the race state as a progress bar between two
  class boundaries. What is *not* there is a traced increment of the field
  itself - the widget's `+0xc` was never resolved to its owning object.
  **The play-based lead this thread's Open section carried is corroborated on
  its first half and not its second**: the bands (`0`-`1`, `2`-`8`, `9`-`16`,
  `17`-`32`, `33`-`39`, then every five to `90`) really are "every few
  zones", but nothing singles out the *named* classes - every boundary is one
  class step and the `Sub`/named names simply alternate. Recorded split
  rather than reshaped. **Also stated rather than glossed**: `0x11 - i`
  yields `1`-`17` against a thirteen-stage table, so classes `13`-`17`
  squash onto stage `12`. That is a real lossy squash at the top, not a clean
  fit.

  **HD, not closed, but its four source branches are all read now.**
  `Environment_UpdateStageBlend`'s `+0x04` sources: mode `0xe` ->
  `RaceManager->+0x2e10`; modes `0xd`/`0x15` -> per-viewport entries of the
  same object; **everything else, Zone included, -> `craftArray[n]->+0x640`**,
  no writer found. **[2026-09-15: re-run against the `lvlx`-aware PS3 language, now covering all five store mnemonics plus the `li`/`addi`-immediate indexed-store shapes - still negative, still not exhaustive over every indexed-store shape, see zone-effectsettings-loader.md's thirty-third pass.]** The `0x640(` operand sweep returns nothing on this array,
  which proves nothing given the folded-index-bias trap the sixth pass
  recorded; `FUN_003d0b98` (the ~24 KB neighbour carrying the
  `SpeedClass`/`NextSpeedClass`/`SpeedClassParent` node names, and the
  thematic next place to look) was sampled at those sites and is building
  node-name buffers there. So HD carries `zone_stages: None` and 2048's
  thirteen-stage numbers are **not** transplanted onto its fifteen-stage
  ladder.

  **The correction, and a long-open question closed by it.** Mode `0xe` is
  **Detonator**, not Zone: `Environment_LoadStageTextures`' `== 0xe` branch
  loads `Data/Tex/DetonatorMode0.gtf` (`0x008b7afc` -> `0x007b26c8`, read from
  memory) and the fall-through loads `Data/Tex/zoneMode0.gtf` (`0x008b7b7c` ->
  `0x007b2b10`); and `RaceManager->+0x2e10`'s only two non-incrementing
  writers are `SPDetonator`'s own two constructors (`0x00064470`,
  `0x000649f0`, both `li r9,0x1`). `mode-manager.md` is corrected.
  `Detonator_UpdateRace` (`0x00067b40`, confidence 75, renamed) increments
  `+0x2e10` by exactly one per event and the race ends at `15` - which is
  exactly `detonatormode.effectsettings`' stage count, so **this thread's
  "why does Detonator carry Zone's fifteen-stage ladder" is answered**: it
  has its own fifteen-step escalation and consumes the table one row per
  step. Full traces on
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-tenth-pass-all-four-0x04-source-branches-read---and-mode-0xe-is-detonator-not-zone)
  and
  [zone-environment-fallback.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md#2026-08-30-a-fifth-pass-the-threshold-table-is-read-in-full-the-inconsistency-is-a-sentinel-and-the-value-it-is-walked-against-is-the-zone-number).

  **Wired, and checked against the real Vita package**:
  `oag_title::ZoneStages`, `oag_2048::race::ZONE_STAGES`,
  `ZoneGrade::show_zone` (driven once a frame from `Scene::sync_zone_grade`,
  the way `Zone_UpdateStage` is driven from 2048's render update), plus
  2048's own key spellings in `oag_tables::effectsettings` - that file has
  no density key at all and packs the density into the fourth lane of
  `Fog.Environment Fog Colour`, confidence 74. `zone_grade_ground_truth.rs`'s
  `a_2048_zone_race_escalates_on_the_recovered_ladder` drives a real
  `ZoneMode2048.effectSettings` across every band boundary from the ladder
  alone and asserts the disc's own fog colours and densities at stages 1 and
  12.

  **What is left of this thread**, in order of tractability:

  1. **HD's `craftArray[n]->+0x640` writer.** The static search space is
     *not* closed the way `+0x00`'s was - the folded-bias trap means an
     offset sweep is not evidence of absence. The bounded next step is
     `FUN_003d0b98` read properly rather than sampled: it is HD's Speed Class
     HUD builder by its own node names, which is exactly what 2048's answer
     turned out to be.

     **2026-08-30, tried live, twice, with the actual patched build - zero
     hits, and it's a stronger negative than it sounds.** A real `Z2` watch
     on the live-resolved address caught nothing across 120s each on an
     ordinary race *and* a genuine Zone race (reached this time by actually
     driving `racebox_definition.xml`'s own `Mode` list to `Zone`, not
     inferred from the fallback branch). A watchpoint fires on any write
     regardless of value, so this means the address was never touched at
     all in either window, not just "stayed at 1." Full transcript on
     [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-an-eleventh-pass-a-live-write-watchpoint-on-craftarray0-0x640-armed-twice-zero-hits).
     Not conclusive - 120s under a slowed interpreter may not be enough
     in-game time or the right in-race event, and craft speed was never
     checked alongside the target - but `FUN_003d0b98` read properly is
     still the next concrete step, now with the exact live Zone-race
     command sequence already scripted rather than needing to be solved
     again.

     **2026-09-15: `FUN_0006c600` checked directly and ruled out.** The
     `ZONEBAR_TRANS`/`ZONEADVANCE` call site
     [`sound.md`](../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#zonebar_trans-a-call-site-found)
     found is a real candidate - it runs once per craft per tick, reads the
     craft-identifying `+0x7a60` offset, and advances a `0`-`14` wrapping
     ladder rung on the same threshold-crossing shape. Named
     `Zone_UpdateCraftClass` (`docs/ghidra/functions/ps3-hdfury-eu/
     zone-advance.md`), confidence 72. Its disassembly writes no `0x640`
     displacement anywhere, on any base register, by literal or
     immediate-fed indexed store, and its own ladder-rung array is a
     different object (the `zoneState` sub-object its caller passes in,
     indexed by racer slot) than `craftArray[n]`. `FUN_003d0b98` remains
     the next concrete step.
  2. **What sets HD's `raceState->+0x7001`** - the latch Detonator's own
     increment is gated on, i.e. what *event* Detonator counts. Ten of the
     eleven instructions touching that offset are reads; the one write clears
     it.
  3. **The cross-fade's rate, on both titles.** 2048 fades toward the *next*
     stage by `DAT_816c6bc8`, which nothing traced writes, so
     `ZoneGrade::show_zone` leaves the weight at rest and stages change
     cleanly rather than easing. Still an absence, not a choice.
  4. **The zone counter's own timing on 2048.** This engine steps
     `world.race.zone` every ten seconds, which is *Pulse's* recovered
     constant (`oag_race::zone::STEP_SECONDS`) and has never been checked
     against 2048's or HD's executables. The ladder wired here is 2048's; the
     clock feeding it is not, and a 2048 race's escalation will only be
     correctly *paced* once that is read.

  **2026-08-30, an eleventh pass, on a user observation from a live 2048 Zone
  race: "it only changes the fog, it should affect all textures and such."**
  Correct, and this pass establishes *why* rather than fixing it.

  **Found: the schema registrar, on both titles.**
  `Environment_RegisterStageSchema` - `0x003d0b98` on HD (confidence 85, the
  ~24 KB neighbour this page ruled out twice as "just memsets") and
  `0x8104714c` on 2048 (confidence 80, found from its own
  `"Zone %s.Growing Texture.Colour"` / `"Zone %s.Fog.Environment Fog Colour"`
  templates). For each stage it formats every key template against that
  stage's name and hands the result plus a destination pointer to one of nine
  typed registration helpers - the table `FwKeyedText_ParseEntry` later looks
  a parsed key up in. **This confirms the 73-key vocabulary from code rather
  than from position**, so `g_EffectSettingsSchemaKeyNames` moves 82 -> 90.

  **Two corrections to this thread's own earlier claims.** The "73-entry
  compile-time array" at `0x008b79cc` is not an array - it sits inside the
  module's **TOC**, and the registrar loads each slot individually
  (`get_xrefs_to` returns nothing and no word in the binary holds that
  address, both checked). And `0x007b26c8` sits at `0x008b7afc`, not
  `0x008b7af8`; `0x008b7af8` holds the mode-dispatch gate byte pointer.

  **Not found: the key-to-offset table, and it would not have been enough
  anyway.** HD's registrar keeps several base registers each advanced by
  `0x250` per stage, so an offset is meaningless without its base - one
  reading was withdrawn mid-pass for exactly that reason, recorded as a trap.
  `run_script_inline` is refused on this bridge
  (`GHIDRA_MCP_ALLOW_SCRIPTS` unset), so enumeration needs that variable set
  or ~700 hand-zipped instructions, and it was not spent because the offsets
  are HD's while the live trigger is 2048's.

  **The real blocker, stated plainly**: an offset says where a parsed colour
  lands in memory; it does not say what draws with it. `Colour 1..8.Colour`,
  `Window Colour N.Gradient N` and `Track Paint.*` are indexed into the
  circuit's own art and nothing recovered says which material index `3` is.
  On 2048 specifically, fog really is the ceiling today: its per-stage blocks
  author **no** `Lighting.*` keys, so `ZoneGrade::light` is a no-op on that
  title by construction. Full write-up in
  [effectsettings.md](../../docs/formats/effectsettings.md)'s `## Open`.

  **Next steps, in order of tractability:**

  1. ~~`Sky.{Horizon,Zenith} Colour` and `Background.Diffuse Colour` are
     already parsed into `StagePalette` and need no binding~~ **Read and
     implemented, 2026-08-31/2026-09-01.** The premise was half right for the
     wrong reason: HD's Zone sky is textured, but not because the engine has
     no colour input there - `Environment_LoadRaceScene` swaps in
     `Data/Tex/ZoneSky.gtf` for the circuit's own `sky.gtf` wholesale, gated
     on the same mode-id byte this thread's own next section names. Full
     trace: [zone-sky.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md).
     Wired: `crates/raceplay/src/load/environment.rs`, checked against the
     disc in `zone_sky_ground_truth.rs`. **What that page leaves open**: a
     second, genuinely computed horizon/zenith gradient sits behind the same
     gate, read but its consumer unidentified (confidence 55, below naming
     threshold) - not wired, not this project's own invention either.
  2. `"Debug.Reload Growing Textures"` (`0x81509e3c`, 2048) names a Growing
     Texture subsystem with its own debug reload - a better handle on that
     effect than anything the file side has offered.
  3. The nine typed registration helpers HD's registrar calls would settle
     the byte-versus-float storage question this thread has carried since the
     render wiring landed.
  4. The full key-to-offset enumeration, once `GHIDRA_MCP_ALLOW_SCRIPTS=1`
     makes it a script rather than a hand-zip.

  **2026-08-30, a twelfth pass: the 36-byte schema record is read, and the
  byte-versus-float question is closed.** Took the previous pass's own
  cheapest next step - read the typed registration helpers - and it settled
  more than intended. **`FwKeyedText_AddSchemaEntry`** (`0x005d3680`,
  confidence 85) is what all nine helpers wrap; it writes
  `{type tag, type tag, u16 count, destination pointer, key name}` and
  advances the registry by `0x24`, **independently confirming the 36-byte
  stride** this thread previously had only from `FwKeyedText_ParseEntry`'s
  loop bound. Three helpers read: the four `Detonator * Colour` keys
  register with count `1` into consecutive **4-byte** slots, `Airbrake
  Colour` with count `4` into a different region - **both storage domains
  exist in one file**, so the byte/float straddle `cross_fade_rgba8` and
  `fade_scalar` already implement is right rather than a hedge. It also
  **refutes** the withdrawn "`+0x00` might be a Detonator colour" reading on
  positive evidence: the cross-fade reads 16-byte fields, those colours are
  4-byte, so `r21`'s array is not the stage struct.

  **A correction to this thread's own previous next-step list**:
  `"Debug.Reload Growing Textures"` is *not* a subsystem entry point - its
  only reference is inside `Environment_RegisterStageSchema` itself, so it is
  another registered schema key. It still implies a reload hook behind that
  key's destination, but it is not the shortcut it was billed as.

  **Also closed, from the lead's parallel RPCS3 run**: the write watchpoint on
  `craftArray[0]->+0x640` catching nothing twice is a genuine test of the
  fallback branch, not an accidental Detonator race - Zone's mode id is not
  `0xe` (that is Detonator), so a Zone race reaches `+0x640` through
  `Environment_UpdateStageBlend`'s "anything else" path by construction. Two
  independent negatives now, static and runtime.

  **2026-08-30, a thirteenth pass: HD's key-to-offset table is enumerated in
  full.** `GHIDRA_MCP_ALLOW_SCRIPTS=1` went live, so the previous pass's
  "bounded but unspent job" got spent with `run_script_inline`: a symbolic
  `base + offset` walk of `Environment_RegisterStageSchema`'s loop,
  resolving each key template against the function's own TOC. **51 per-stage
  keys, confidence 88**, all landing in `g_effect_settings_stages`
  (`0x00c7efb0`, newly named).

  **It validates itself three ways** - the offsets fill the `0x250` stride
  exactly (24 padding bytes), the only two overlaps are `Scene`/`Track.EQ
  brightness` sitting in the fourth lane of their own `Texture Colour`, and
  every 16-byte key uses a 16-byte helper while every 4-byte key uses a
  4-byte one (confirming the previous pass's storage split across all nine
  helpers by geometry rather than by reading three of them).

  **It corrects a guess this thread has carried since the render wiring**:
  the three cross-faded fields are `Scene.Texture Colour` (`+0x00`),
  **`Scene.Base Colour Highlight`** (`+0x20` - the guess said `Near Colour`,
  which is actually `+0x10`) and `Scene.Base Colour` (`+0x40`). Two of three
  right. It also confirms the `Lighting.*` keys land where this project's
  name-keyed reader assumes, which had been an assumption.

  **2048's offsets are a recorded negative.** The same script recovers its
  51 per-stage key names from the executable - the first time that
  vocabulary has been read from code rather than from a shipped file - but
  its destination column is wrong (Thumb reuses `r1` for both the key
  template and the destination, and destinations come from precomputed stack
  slots), so it is **not** published. It needs a tracker that models the
  prologue's stack-slot fills.

  **Still not wired, and the reason is unchanged**: `Scene.*`/`Track.*` turn
  out to be two *named material groups* rather than indexed slots, which is
  a far better binding prospect than 2048's `Colour 1..8` - but no consumer
  of any offset was traced, and HD remains the title with no stage trigger.

  **2026-08-30, a fourteenth pass: there is no shader-parameter path - the
  stage table has a per-key getter API instead, and the `Scene`/`Track`
  colours are dead code on it.** Pulled the `Shader_InitEngineParams` /
  `ShaderRegistry_*` thread and it is genuinely cold: nothing downstream of
  the effectSettings struct touches that infrastructure. What exists is
  sixteen tiny getters at `0x003cd870`-`0x003ce120`, each reading one offset
  of `g_effect_settings_stages` - and their offsets (`iVar8 + 0x1230`, etc.)
  independently reproduce the previous pass's key-to-offset table from a
  different function.

  **Four keys reach a draw**, through TOC thunks: `Detonator Mine Colour`,
  `Mine Electricity Colour`, `Bomb Inner`/`Outer Colour` and `Airbrake
  Colour` (all five getters named, `names.tsv` updated). So the palette *is*
  consumed at draw time on HD - **per effect**, each pulling its own key.
  That is why no shader-parameter trail exists: the design never had one.

  **Seven keys reach nothing**: `Scene.Base Colour`, `Scene.Base Colour
  Highlight`, `Track.Texture Colour`, `Track.Base Colour`, `Track.Base
  Colour Highlight` and both `EQ` tints have **no branch anywhere in the
  image targeting their getters**. Confidence 85, and the method is why: a
  first test (scanning for words holding each getter's OPD address) reported
  "no hits" for the known-good controls too, so it was discarded as
  non-discriminating; the kept test resolves every `bl`/`b` flow target in
  the image plus the small TOC thunks, **with two positive controls in the
  same scan**.

  **What this means for "it should affect all textures"**: the mechanism the
  search was aimed at is ruled out - there is no material-parameter upload to
  redirect these colours into. It does **not** prove the offsets are unread:
  `FUN_003ce2c0` reads struct offsets directly rather than through the
  getters, and is the one remaining route by which `Scene.*`/`Track.*` could
  reach a draw on HD. That is the next thing to read, and it needs the same
  base-register tracking the registrar did.

  **2026-08-30, a fifteenth pass, pushing `Scene.*`/`Track.*` on the user's
  stated priority.** Three results.

  **The blend runs inside the render path, confirmed rather than assumed.**
  `Environment_UpdateStageBlend`'s only caller is `0x003aa888`, **renamed
  `Scene_PrepareFrame`** (78) - its named callees are
  `Scene_RefreshNodeMatrices`, four `Pvs_*`, three `Visibility_*` and
  `GcmContext_Callback`, i.e. scene graph, visibility and the RSX command
  context. Chain: `FUN_0067f078` -> `FUN_00757de0` (thunk) -> `FUN_003e26d0`
  -> `Scene_PrepareFrame` -> the blend.

  **`Scene.Texture Colour`'s output address is now concrete.** The fourth
  pass said the blend "copies three RGBA-shaped fields into a small
  blended-output area" without locating it. It reads
  `g_effect_settings_stages[stage] + 0x00/0x04/0x08` for two stages and
  writes the cross-fade to a fixed three-float global at **`0x00c81a5c`**
  (`iVar8 + 0x3aac`), at `0x003dc5c0`-`0x003dc5d0`. Confidence 85. So the
  consumer of `Scene.Texture Colour` is whatever reads `0x00c81a5c` - a far
  better target than "somewhere in the struct".

  **The last alternative route is closed.** The previous pass named
  `FUN_003ce2c0` as "the one remaining way `Scene.*` could reach a draw". It
  is not a route: the thunk-aware branch scan finds **no branch anywhere**
  targeting it, only its OPD descriptor. Unreached, like the seven dead
  getters.

  **No reader for `0x00c81a5c` is located**, and a displacement sweep cannot
  find one - it is base-blind, the trap this thread already recorded. Two
  candidates that read all three slots as floats (`FUN_00661f00`,
  `FUN_00661c64`) **both fail a base check** and are recorded as ruled out so
  nobody re-derives them.

  **Next step, and it is unusually cheap.** `0x00c81a5c` is a fixed address
  with a **confirmed per-frame writer** inside `Scene_PrepareFrame` - unlike
  `craftArray[n]->+0x640`, where two watchpoint runs caught nothing because
  nothing may write it at all. A **read** watch on `0x00c81a5c` during any HD
  race (the blend runs in every mode, not only Zone) names the `Scene.Texture
  Colour` consumer directly, with a guaranteed-good control built in.

  **2026-08-30, a sixteenth pass: the deciding read, and the answer is no.**
  Base-aware trace of `FUN_003ce2c0` - the last candidate consumer for
  `Scene.*`/`Track.*`: it loads `iVar8` once and **63** accesses resolve onto
  it, all at `+0x84`, `+0x32bc`-`+0x3374` or `+0x6640`. **None in the
  `+0x1000`-`+0x1250` stage-table window.** It does not read the per-stage
  struct.

  **That corrects this thread's own fourth pass**, which said this function
  re-derives the blend with "the same `+0x1000`/`+0x1004` RGBA reads" - a
  displacement read without its base, the trap this thread has now hit three
  times. Scoped honestly: what is established is "no access *via `iVar8`*";
  the tracker does not follow indexed forms or a pointer arriving as a
  parameter. The conclusion does not rest on that anyway - it rests on
  reachability, and nothing branches to this function.

  **So HD's executable, as far as static analysis reaches, does not draw
  track/scenery recolouring.** Every link of the chain is traced and the last
  two are empty: the keys are authored, registered, parsed to known offsets
  and `Scene.Texture Colour` really is cross-faded per frame into
  `0x00c81a5c` inside `Scene_PrepareFrame` - but nothing located reads that
  output, the seven `Scene`/`Track` getters have no callers **[re-run
  2026-09-15 post-lvlx-reimport: negative holds, all three routes,
  zone-effectsettings-loader.md's thirty-second pass]**, and the one
  alternative reader is unreached and reads elsewhere. Four keys *do* reach a
  draw (the Detonator mine/bomb colours and `Airbrake Colour`), so the
  mechanism is real and exercised - just not for these groups.

  **What would overturn it**: a reader reached by indirect call, or one taking
  a stage pointer as an argument - neither visible to a static branch scan. A
  **read watchpoint on `0x00c81a5c`** settles both in one run, with a
  confirmed per-frame writer as its control.

  **2026-08-30, a seventeenth pass: 2048 has a consumer, and it is a
  full-screen composite shader.** HD's side closed as a negative; 2048's does
  not, and the difference explains why HD's search kept coming up empty - the
  two titles apply this table in structurally different places.

  Chain, traced end to end: `Zone_UpdateStage` blends stage against stage+1
  and hands the 16-byte result to **`Zone_SetBlendedStageColour`**
  (`0x8103876e`, renamed, 85 - two stores and a return), which publishes it to
  **`g_zone_blended_stage_colour`** (`0x816af070`, renamed, 85). That global is
  **read** at `0x810394b2` by an 11,902-byte function calling **33 distinct
  `SceGxm_*` entry points** - a GXM draw function. Its sibling writer
  (`FUN_8103d2ae`) carries the shader-name table:
  `wo_composite_zone_fp`/`_vp`, `wo_composite_zone_hdfury_fp`/`_vp`, plus
  bloom, blur and the ordinary composite programs.

  **So the original grades the whole frame in a composite pass**, not by
  recolouring individual materials. Confidence 80. That reframes the user
  observation this work started from: "it should affect all textures" is right
  *because it is a screen-space grade*, and the "which material is `Colour 3`"
  framing - the thing that made this look intractable - was the wrong
  question. **A full-screen grade needs no per-material binding.** What it
  needs is `wo_composite_zone_fp`'s own arithmetic, a shader this project has
  not extracted.

  **Also recovered, for 2048 only**: the cross-fade rate and direction.
  `DAT_816c6bc8 = DAT_816c6bc4 * 0.0002` clamped to `1.0`, zero unless a
  transition is running; the stage commits only once the transition timer
  passes `5000.0`; and the blend runs current -> **next** (`stage + 1`,
  clamped to `0xc`), not against the previous stage as HD's does. That
  answers this thread's long-open weight question for this title, and is
  explicitly **not** transferable to HD.

  **Still not read**: 2048's key-to-offset table. The Thumb tracker was
  replaced with the decompiler's own dataflow, which fixed the template
  pairing and resolved the 16 non-stage globals plus two per-stage keys - but
  the rest come back `expr:INDIRECT`, the decompiler modelling them as
  call-clobber effects on stack-held pointers. Sharper obstacle than the
  first attempt's naive tracking, and recorded rather than worked around: no
  2048 offset table is published.

  **Next step, and it is now a different kind of job**: extract
  `wo_composite_zone_fp` from the disc and read what it does with the colour.
  That is an asset-side task, not a decompile - and it is the thing standing
  between this thread and a Zone race that grades the way the original does.

  **2026-08-30, an eighteenth pass: the composite shader is read, and it
  corrects the pass before it.** New format for this project - see
  [gxp.md](../../docs/formats/gxp.md).

  **The shaders are in the executable, not on disc.** `data.psarc`'s full
  18,430-entry directory has no shader entries; the shaders are **111 `GXP`
  blobs embedded in `eboot.elf`** from file offset `0x515f70`. Header and the
  16-byte `SceGxmProgramParameter` table are implemented against the
  Vita3K-documented layout and **all 111 parse cleanly**, which is the check
  that the reading is right. Confidence 88.

  **Blob #77 (file `0x51eac8`) is the Zone composite** - the only one of the
  111 declaring `zoneEdgeColour`, found by enumerating every uniform and
  sampler name in all of them. Parameters: `bloomFactor[4]`,
  `accumFactor[1]`, `screenTintColour[3]`, `zoneEdgeColour[3]`; samplers
  `mainTex`, `alphaTex`, `bloomTex`.

  **The correction**: the previous pass said 2048 "grades the whole frame".
  Traced instruction by instruction, the Zone stage colour lands in
  **`zoneEdgeColour`**, not in the frame-wide `screenTintColour` - which
  takes a different global (`0x816af060`) whose writer is unchased. The pass
  being full-screen stands; the stage colour being the frame's *tint* does
  not, and should not be repeated. Confidence 85 (two copies of each uniform
  name, cached resource indices matched at both bind and use site, the two
  colours staged in the order they are consumed).

  **Still unread: the USSE bytecode**, so `zoneEdgeColour`'s arithmetic is
  unrecovered. Its name says *edge*, and an edge term is not a flat tint - so
  wiring a full-screen tint off this would be an invention, and a more
  tempting one now that the plumbing either side of the shader is fully
  traced. **Nothing wired.**

  **What would close it**: a USSE decoder (Vita3K has a recompiler; this
  project has none), or observing the pass's output directly.

  **2026-08-30, a nineteenth pass: HD's side gets its own "observing the
  pass's output directly"** - a real Zone boot, driven to a confirmed race
  (Mode list step 4 screenshot checked, per the eleventh pass's dropped-tap
  trap), screenshotted at t+15s and t+60s while holding thrust. Track
  surface, side barriers and a trackside building all shift colour between
  the two shots (cyan -> purple-magenta / yellow) as the HUD's zone counter
  advances - full write-up with the exact method in
  [zone-effectsettings-loader.md's eighteenth pass](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-an-eighteenth-pass-play-evidence-overturns-the-static-no-reader-located-conclusion---hds-zone-mode-does-recolour-the-scene).
  Screenshots not committed, per leakage policy.

  **This confirms the target behaviour on HD, not the mechanism.** It does
  not identify a reader for `0x00c81a5c` or any other consumer - the
  seventeenth pass's static "no reader located" table for HD stands
  unchanged; this is independent play evidence that the behaviour exists
  regardless. **Nothing wired** - per `CLAUDE.md`, confirming the effect is
  real is not the same as having the real trigger to reproduce it with.

  **Next, and it is the same read watchpoint the seventeenth pass already
  named**: arm `Z2` (or, once available, `Z3` for a read) on `0x00c81a5c`
  during a *confirmed* Zone race - the thing every static attempt on HD's
  side has been missing so far is a verified-Zone run, not a longer window.

  **2026-08-30, a twentieth pass: tried for a tighter pair, and found out why
  one can't come from a single lap.** Zone here is one 6.536 km lap of a
  real, non-repeating circuit (`LAPS CLEARED: 1` on the Results screen), not
  a short loop - so there is no track segment a single run passes twice to
  screenshot at two different colours. Picked the closest compositional
  match available from a 25-frame burst instead (same curving street shape,
  same palm-tree silhouette corner, same HUD ladder column - `ZONE 0` cyan
  vs `ZONE 5` purple/yellow) and wrote up why that's the ceiling for this
  method, not a same-coordinates pair. See
  [zone-effectsettings-loader.md's matching section](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-closer-matched-screenshot-pair-and-why-one-lap-cant-do-better).
  Also in this window, `hd-fullscreen-tint` closed the post-process-tint
  candidate by enumeration (all five full-screen colour inputs to HD's
  resolve pass traced and accounted for, none fed from the stage table) -
  narrowing the remaining search to per-material/per-light and the one
  post-chain program that enumeration doesn't cover (`FunkLayerColour2d_fp`).

  **2026-08-31, a twenty-first pass: `Z3` (read watchpoint) built, and the
  whole read side confirmed live.** A second RPCS3 patch,
  `scripts/patches/rpcs3-gdb-read-watchpoints.patch`, extends the tracked
  Z2 one with `Z3`/`z3` - hooked into `ppu_feed_data<T>()`, the one choke
  point every PPU load (scalar and vector) passes through. Two false leads
  chased and closed along the way, both source-verified rather than left as
  hypotheses: a suspected "`Z2`/`Z3` miss vector stores" gap turned out not
  to exist (`STVX`/`LVX` route through the identical hooked templates
  scalar ops use), and a genuinely wrong stop-reply PC traced to
  `is_debugger_present()`/`Assume External Debugger` gating per-instruction
  PC tracking in RPCS3's own interpreter - a pre-existing RPCS3 property,
  fixed by a config flag, not a patch bug.

  With that settled, a `Z0` breakpoint confirmed `Scene_PrepareFrame`'s
  reader (`0x003aaf8c`) now executes at all in a genuine, screenshot-checked
  Zone race - it hadn't in an Arcade-mode control - and `Z3` on `0x00c81a5c`
  fired within 5 seconds, log-confirmed with the correct PC. **The whole
  chain - stage write, gate, read - is now live-exercised, not just
  statically reachable.**

  A follow-up, watchpoint-free poll of `0x00c49110`/`0x00c49120`/`0x00c49130`
  (once a second, 75 seconds, real Zone race) then closed the buffer-identity
  question too: `0x00c49130` moves through two full cross-fade transitions in
  sync with its neighbours, live - not just non-zero once, but actively
  carrying a per-frame blended value during real play. Buffer identity
  confidence moves 65 -> 82.

  **What is left, and it is now the last real gap**: who reads
  `0x00c49110`/`0x00c49120`/`0x00c49130` back out for rendering.
  `FUN_006ce6e0` installing `&block[0x7c00]` into `*(obj+0xd8) + 0x1b8` with
  a dirty flag is the standing lead; the object behind `*(obj+0xd8)` is not
  yet identified. That is what decides whether this is wireable into
  `oag_render` or needs a live read watchpoint on the consumer side too.
  Full write-up:
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-31-0x00c49130-moves-live-during-real-play---no-watchpoint-needed).

  **Superseded, same day, by the twenty-second pass (recorded earlier in this
  file under `## Open` and `## Next Steps` - this thread's own passes are not
  in file order, see the correction on the title above).** The consumer is
  `fogColour` (engine shader parameter 7) and `*(obj+0xd8)` is the link-time
  constant `0x00d42220` installed by `Scene_InitRenderBlock` - not an
  unidentified runtime object. The twenty-fifth pass goes further still:
  `oag_render`'s `mesh.wgsl` already draws the recovered subset. This
  paragraph's "last real gap" framing is stale; the real frontier is the four
  items the twenty-fifth pass leaves open (see `## Open`, "What that leaves
  open, all of it narrow").
