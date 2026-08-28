# 2048 and HD's `.effectSettings` table now parses; wiring it into a race is what remains

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

**2026-08-28, later the same day: a parser landed.** `oag_formats::effectsettings`
([`effectsettings.rs`](../crates/formats/src/effectsettings.rs),
[docs/formats/effectsettings.md](../docs/formats/effectsettings.md)) reads all
five files above plus 2048's ten identical copies, validated against the real
disc/PSARC in `effectsettings_ground_truth.rs`. It reuses `EnvSettings::parse`
for the tokeniser and adds a stage index/name read off each key's own prefix.
**Nothing is wired into a race yet** - see Open and Next Steps below, both
trimmed to what is still actually open.

Plain text, same `"Key.Subkey"=float [float...]` shape `oag_formats::envsettings`
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
(`oag_formats::gtf`) to a flat, uniform white texture with nothing in it -
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

## Open

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
  **Still open**: what selects a row at runtime, and whether the index is
  `RaceState::zone` directly (uncapped past 14, per `crates/race/src/zone.rs`)
  or something derived from it - no HD executable read exists for this, so
  per `CLAUDE.md` it stays unwired rather than guessed.
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
  of its own that has not been looked for.
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
  (`oag_formats::gtf`) to a flat white texture with nothing in it. **The
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
  decoding both ways and looking at the picture). What 2048's own "Track"
  art depicts is still open.

## Next Steps

- Cross-reference the stage-name ladder against `Zone_Update`'s recovered
  timer logic to settle whether stage `N` is live exactly while the zone
  counter reads `N`, before wiring anything that assumes it.
- Wire the current stage's fog/sky/ambient into the renderer for one title as
  a proof of concept, once the timer cross-reference above settles - HD is
  the best-measured target, since its `.envsettings` reading and drawing path
  already exists ([envsettings.md](../docs/formats/envsettings.md)) and this
  is the same shape.
- Add `SceGxmTextureBaseFormat` `U8U8U8U8` (byte `0x0c`, `ARGB` swizzle - now
  identified, see above) to `oag_formats::gxt` to see what 2048's
  `zoneModeTrack{0..14}.gxt` actually depict - the raw bytes already say
  they are not blank, matching HD's own "Track" set, so this closes a real
  picture rather than confirming another placeholder. The channel order is
  now a well-evidenced starting point; **the tiling order (raster vs
  Morton/twiddle) is the part still genuinely unread** and needs settling
  the same way `UBC2`'s reticle texture was - decode both ways and see which
  looks like a picture - ideally against several of the format's other 97 real files
  rather than one alone.
- Read HD's Zone shader (needs Ghidra RE on `ps3-hdfury-eu`, a much larger
  undertaking than this thread on its own - see
  `handover/wipeout-hd-furys-executable-is-26100-functions-with.md`) to
  settle what `Growing Texture.Scale Bias`/`.Factors` actually do to the
  now-identified "Track" art, and what selects an effectsettings row from
  `RaceState::zone` at runtime.
