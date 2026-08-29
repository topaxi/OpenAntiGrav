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

**2026-08-28, later still: with a Ghidra project open on `vita-2048-eu-v104`,
2048's own loader confirms the mechanism - and reveals the four ported HD
`zone_N` circuits get no palette at all.** Named `Environment_Load`
(`0x8102f6d0`, confidence 80,
[zone-environment-fallback.md](../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md)),
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
[zone-environment-fallback.md](../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md).
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
[zone-environment-fallback.md](../docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md)).
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
[zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-28-a-fifth-pass-the-live-project-had-reverted-and-once-restored-the-files-own-content-turns-out-to-be-parsed-after-all).

## Open

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
  decoding both ways and looking at the picture).

  **2026-08-28, later the same day: decoded.** `oag_formats::gxt::Format::Argb8888`,
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
  section and `crates/formats/tests/gxt_ground_truth.rs`'s
  `the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages`, whose
  renders (`data/shots/2048_zone_track_stage{0,7,14}.png`) this rests on.
  **Still open**: what the shader does with `Growing Texture.Colour`/`.Scale
  Bias`/`.Factors` against this art - no HD executable has been read for its
  Zone shader, so wiring this into a render is unstarted.

## Next Steps

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
- Wire the current stage's fog/sky/ambient into the renderer for one title as
  a proof of concept, once the two items above settle - HD is still the
  best-measured *rendering* target, since its `.envsettings` reading and
  drawing path already exists ([envsettings.md](../docs/formats/envsettings.md))
  and this is the same shape, even though the clearest recovered *selection*
  logic so far is on 2048.
- ~~Add `SceGxmTextureBaseFormat` `U8U8U8U8` to `oag_formats::gxt`~~ **Done,
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
  [zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
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
  ([mode-manager.md](../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md))
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
  [zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-the-schema-table-is-read-and-it-is-the-full-recognised-vocabulary-not-a-guess).
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
  [zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-later-the-same-day-who-reads-ivar8--0x1000---two-candidates-found-one-ruled-out-one-strong-and-unconfirmed).
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
  returns nothing - so it has no located caller, corrected from a first
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
  [zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead).
  **Nothing wired into Rust from this pass**: the stage-selection write
  itself is still unfound and the `+4`/`+0x74` mechanism has two
  unreconciled producers, both short of `CLAUDE.md`'s bar for firing an
  effect on a recovered trigger. **Next step**: trace who sets the `+0xc`
  flag byte that gates `FUN_003da540`'s own `+4` write - a single byte at
  a known offset, more tractable than a further blind sweep for the
  `+0` write.
