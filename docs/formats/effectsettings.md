# `.effectSettings`/`.effectsettings`: the per-stage table that reskins Zone

**2048 races Zone on whatever circuit the player picked - it has no dedicated
Zone circuit at all.** HD/Fury's own Zone races one of four ported
`/data/environments/zone_N/` circuits instead
([race-modes.md#zone](../gameplay/race-modes.md#zone)), so it is not this
title that needed a stand-in. What both titles need, dedicated circuit or not,
is a way for the *look* to escalate as the race goes on - the disc's own
`MSC_EVENT_ZONE` text says the top speed rises every ten seconds, and this
file is the palette that rises with it: a title-wide colour-grade table with
one full palette per Zone speed-class stage, read against the zone number the
race is currently on and layered over whichever circuit is racing. Detonator
mode ships the same mechanism under its own file name.

Implemented in
[`oag_formats::effectsettings`](../../crates/formats/src/effectsettings.rs),
checked against the disc by
[`effectsettings_ground_truth.rs`](../../crates/formats/tests/effectsettings_ground_truth.rs).
**Wired into a Zone race since 2026-08-30, driven by an explicit stage index
and blend weight and by nothing else** - see [What is read and what is
not](#what-is-read-and-what-is-not). The cross-fade `cross_fade_rgba8`
performs is the one HD/Fury's own executable performs between two adjacent
stages (recovered independently from `FUN_003da540` and `FUN_003ce2c0`, both
`ps3-hdfury-eu`, confidence 80 -
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead));
**what moves the stage during a race is still unrecovered**, so a race rests
on stage `0` and nothing advances it.

```sh
just psarc cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    /data/environments/zonemode.effectsettings
```

## The files

| Title | Path | Size | Scope |
| --- | --- | --- | --- |
| HD/Fury | `/data/environments/zonemode.effectsettings` | 21,160 B | Zone, one file, title-wide |
| HD/Fury | `/data/environments/zonemodedlc3.effectsettings` | 43,417 B | A second, larger revision of the same table |
| HD/Fury | `/data/environments/detonatormode.effectsettings` | 43,240 B | Detonator, reusing Zone's own stage ladder |
| HD/Fury | `/data/environments/detonatormodedlc3.effectsettings` | 43,230 B | Detonator's DLC3 revision |
| 2048 | `Data\art\published\environments\<circuit>\ZoneMode2048.effectSettings` | 43,812 B | Zone. Ships in all 10 base circuits, **byte-identical** across every one |

**2048's own loader code names five more files, none of which ship** - see
[Environment_Load](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md):
a title-wide `ZoneMode2048default.effectSettings`, and four
`data/ZoneEnvironmentHDFury/{ZoneMode,DetonatorMode}HDFury[DLC3].effectSettings`
paths reached for on the four `zone_N` circuits HD's own DLC ports into this
title. Checked against all three packages (base, DLC1, DLC2): none exist.

2048's copy is HD's ported rather than reauthored: **both titles ship the
same two texture sets, fifteen files each - corrected 2026-08-28, HD's own
`zonemodetrack{0..14}.gtf` was missed on the first pass and only 2048's
doubled naming was noticed.**

| Title | "General" set | "Track" set |
| --- | --- | --- |
| HD | `/data/tex/zonemode{0..14}.gtf` | `/data/tex/zonemodetrack{0..14}.gtf` |
| 2048 | `data/Tex/zoneMode{0..14}.gxt` | `data/Tex/zoneModeTrack{0..14}.gxt` |

Both counts match HD's 15-stage ladder exactly, even though 2048's own table
only names 13 - see [Open](#open). **The two sets are not equivalent**: the
"general" one is a single blank texture duplicated fifteen times on both
titles, and the "Track" one carries fifteen genuinely distinct images - see
[Open](#open) for the decode.

## The format is `.envsettings`'s tokeniser, with a stage prefix

Same syntax [`envsettings`](envsettings.md) already parses - one quoted,
dotted `"key"=floats` per line, no sections, no escaping, no comments:

```text
"0 Start.Lighting.Sun colour"=1.000000 1.000000 1.000000 0.000000
"Zone 0 Start.Colour 1.Colour"=1.000000 1.000000 1.000000
```

so `EffectSettings::parse` reads the whole file through `EnvSettings::parse`
rather than a second tokeniser. What is new is that most keys carry a **stage
prefix** before the first `.` - HD writes `"<n> <Name>"`, 2048 writes
`"Zone <n> <Name>"` - and a handful of title-wide keys (2048's `"Sky Radius"`,
`"Max Draw Distance"`, HD's `"Texture U scale"`/`"Texture V scale"`) carry
none at all and read straight off `EffectSettings::table`.

## The stage ladders

**HD's 15 stages, shared verbatim by all four HD files measured:** `Start,
Sub Venom, Venom, Sub Flash, Flash, Sub Rapier, Rapier, Sub Phantom, Phantom,
Super Phantom, Zen, Super Zen, Subsonic, Mach 1, Supersonic`.

**2048's 13 stages** are the same list with `Sub Venom`/`Venom` removed and
every later index shifted down by exactly 2: `Start, Sub Flash, Flash, Sub
Rapier, Rapier, Sub Phantom, Phantom, Super Phantom, Zen, Super Zen, Subsonic,
Mach 1, Supersonic`.

`detonatormode.effectsettings` and its DLC3 revision carry HD's own 15-stage
ladder verbatim, not a Detonator-specific one - see [Open](#open) for why that
is still unexplained.

## Each stage's own key groups

**2048** (`ZoneMode2048.effectSettings`): `Window Colour {1,2,3}.{Gradient
1-3, Emissive}`, `Colour {1-8}.{Colour, Emissive}`, `Cube Animation Colour 1`,
`EQ.{Colour A/B/C, Mid-Band Position, BG Colour A/B/C, BG Mid-Band Position}`,
`Track Paint.{Primary,Secondary} Colour`, `Growing Texture.{Colour, Scale
Bias, Factors}`, `Background.Diffuse Colour`, `Sky.{Horizon,Zenith} Colour`,
and two independent fog blocks - `Fog.Environment Fog Colour` and
`Fog.Track Fog Colour`, each its own RGB plus its own density.

**HD** (`zonemode.effectsettings`): `Scene.{Base Colour, Base Colour
Highlight, Texture Colour, EQ brightness}`, `Track.{the same four}`,
`Lighting.{Constant Ambient Colour, Sun colour, Sky reflection colour,
Fog/Alt Fog/Track Fog colour+density, Prelit Colour Power/Scale}`, `Sky
{horizon,zenith} colour`, `EQ {colour,analogue colour} tint`. The vocabulary
differs from 2048's but the shape - per-stage scene/track/fog/sky/EQ blocks -
is the same design, a later authoring pass over the same idea. **This is
what `zonemode.effectsettings` itself exercises, not the executable's full
vocabulary.** `zonemode.effectsettings` is the smaller of HD's four files
and uses only the plain `Scene`/`Track`/`Lighting` subset above (no
gradients, no `Aniso`/`Near Colour`/`Luminance Power`, no Detonator/Aurora/
Airbrake/Radial Bloom keys); `detonatormode.effectsettings` (and both DLC3
revisions) exercise nearly everything else the schema recognises. Checked
directly against all four shipped files
(`just psarc cat ... | grep -o '"[^"]*"' | sort -u`, stage prefix
stripped): only **8 of the schema's 73 entries never appear in any of
them** - the five title-wide keys (`ZoneMode`, `Override game control`,
`Target zone level`, `Transition start speed`, `Transition acceleration`)
and three per-stage ones (`Scene.Luminance Power`, `Track.Luminance Power`,
`Radial Bloom Intensity`) - see
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#the-full-73-entry-vocabulary-in-order)
for the full 73-entry list and the per-file verification.

`EffectSettings::stage_vec4` exists specifically for HD's `Lighting.Sun
colour`/`Fog colour` family: they are floats with an alpha, written with
decimal points, so `EnvSettings::rgba8` (which only answers the byte-encoded
case `.envsettings`' `Sky colour` needs) refuses them.

## Confidence

**85** on the syntax and the stage-prefix shape. All five files measured -
HD's four and 2048's ten identical copies - parse in full, and every stage
name in each recovers correctly (`effectsettings_ground_truth.rs`). Two legs
short of the 85-94 band's ceiling rather than at it: the corpus is five
distinct files (2048's ten are one file duplicated ten times, not ten
independent authorings), and nothing here reads either title's executable -
see [Open](#open) for what that leaves unsettled.

## What is read and what is not

**2026-08-30: the table now grades a Zone race, from an explicit stage.**
A Zone race on a title that ships a table loads it and lays the stage that
is showing over the circuit's own `.envsettings` fog and light rig - the
same `mesh_render::Fog`/`Light` path
[`envsettings`](envsettings.md) already drives:

| Piece | Where |
| --- | --- |
| Where each title keeps its table | `oag_title::ZonePalette` - `TitleWide` on HD/Fury, `BesideCircuit` on 2048, `None` on Pulse and Pure |
| One stage's palette, and the blend of two | `oag_formats::effectsettings::{StagePalette, EffectSettings::blended_palette}` |
| The runtime stage state, and what it does to fog and rig | `oag_game::race::zone_grade::{StageBlend, ZoneGrade}` |
| Read at race load, applied per frame | `oag_game::race::load::environment::staging`, `race/scene/frame.rs` |
| Checked against the real image | `crates/game/tests/zone_grade_ground_truth.rs` |

`StageBlend` is the recovered struct's own three fields and no others: a
current stage (`+0x00`), a requested one (`+0x04`) and a cross-fade weight
(`+0x18`), with `ZoneGrade::commit` reproducing
`Environment_UpdateStageBlend`'s request/commit gate. **The transition effect
that function draws is not drawn** - the call it makes (`FUN_0067a7d8`) is
unidentified, so nothing is fired in its place.

Three readings this project made where the data alone does not decide, all
reversible and none of them a substitution for authored values:

- **A stage authoring `Fog density`=`0` leaves the circuit's own fog
  standing** rather than switching fog off. `Start` authors exactly that, and
  the guard matches `envsettings_fog`'s own `density <= 0.0`.
- **The `Fog` block is used; `Alt Fog` and `Track Fog` are parsed and not
  applied**, because what selects between the three is unread - the same
  choice `.envsettings`' reader already makes about its own alternate pair.
- **A stage tints a light rig, never aims one.** This schema has no `Sun
  direction` key at all, so a circuit that authors no `.envsettings` rig is
  left with the stand-in rather than given an invented direction to hang a
  stage colour on.

**What still draws off nothing**: `Sky horizon/zenith colour` (this engine's
HD sky is the `sky.gtf` cubemap, which has no horizon/zenith term), the
`Scene.*`/`Track.*` colours, the EQ tints and the `Growing Texture`
parameters. All are parsed and reported; none is given an invented consumer.

**And nothing moves the stage.** A Zone race rests on stage `0` - which is
where HD's own loader leaves it, `Environment_LoadStageTextures` resetting
`+0x00` to `0` on every load - because the mapping from a live race onto a
stage index is unrecovered on both titles that ship a table. See
[Open](#open).

**2026-08-28: HD's own executable is now confirmed to parse this file's text,
not just carry its path around** - see
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-28-a-fifth-pass-the-live-project-had-reverted-and-once-restored-the-files-own-content-turns-out-to-be-parsed-after-all).
`Environment_LoadStageTextures` opens `ZoneMode.effectSettings`
(`FwFile_OpenByPath`), reads it whole (`FwFile_ReadChunked`), and tokenises it
with a generic, effectSettings-agnostic keyed-config reader
(`FwKeyedText_ParseBuffer`/`FwKeyedText_ParseEntry`, six call sites elsewhere
in the binary) into a real per-stage struct table - 15 stages, `0x250` bytes
each, arithmetic that checks out three independent ways against the
already-found "reversed circuit" `memcpy` block. Checked directly against
HD's own stage 0 (`just psarc cat ... /data/environments/zonemode.effectsettings`):
its `Lighting.*` keys are spelled differently from `.envsettings`'
(`"Sun colour"` vs `envsettings::SUN_COLOUR`'s `"Sun color"`, no `Sun
direction` key at all) - close enough to look like the same field, not close
enough to reuse, and one field short of what a full light rig needs.

**2026-08-29: settled with evidence, not a guess.** The schema table
`FwKeyedText_ParseEntry` looks a parsed key up against is read -
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-the-schema-table-is-read-and-it-is-the-full-recognised-vocabulary-not-a-guess)
has the full 73-entry vocabulary (`g_EffectSettingsSchemaKeyNames`,
`0x008b79cc`). HD's executable genuinely recognises the British-spelled,
`%s`-prefixed family this file's own contents already showed
(`"%s.Lighting.Sun colour"`, `"%s.Lighting.Fog colour"`/`"...density"`,
`"%s.Lighting.Alt Fog colour"`/`"...density"`) - `.envsettings`' American
`"Sun color"` is confirmed to be a different table's own vocabulary
entirely, not a second spelling this schema also accepts. The 15-stage
ladder below is independently confirmed from the executable side too: it is
compiled in verbatim as the `%s` fill-in values, not just present in the
shipped files. **New key groups this schema recognises that no file-side
read had surfaced**: `%s.Scene.*`/`%s.Track.*` (texture/near/base colour,
aniso power/curve, luminance power, three RGB gradients on Scene, no
gradients on Track), and a trailing group covering EQ tint, sky
horizon/zenith colour, radial bloom intensity, aurora colour, airbrake
colour, and four Detonator-only mine/bomb colours - one schema serving both
modes, which is itself a partial answer to why `detonatormode.effectsettings`
reuses Zone's own ladder (see [Open](#open)). **Still not found: who reads
`iVar8 + 0x1000`** - the schema settles *what the file can say*, not what
stage currently applies or what draws off it; wiring a Rust-side light-rig
override still needs that write side traced first, per `CLAUDE.md`'s rule
that a trigger needs recovering before an effect is wired.

**2026-08-29, later the same day: a strong candidate found, not confirmed.**
`FUN_003da540` reads a per-entity field, uses it (and that value minus one)
to index two adjacent stages of this exact table by the same `0x250` stride,
and copies three RGBA-shaped fields from each into a small blended output
area - structurally the same shape as 2048's own `Zone_UpdateStage` (a
per-entity index driving a cross-fade), the first time that shape has
turned up anywhere in HD's own executable. Not yet established: what the
per-entity index actually indexes (craft, camera, something else), or what
consumes the blended output. Full trace, including a ruled-out sibling that
turned out to be a table-clearing `memset`, on
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-later-the-same-day-who-reads-ivar8--0x1000---two-candidates-found-one-ruled-out-one-strong-and-unconfirmed).

## 2048's own loader confirms the mechanism - and reaches for files that never shipped

**2026-08-28, read with a live Ghidra project on `vita-2048-eu-v104`.**
`Environment_Load` (`0x8102f6d0`, confidence 80 -
[zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md))
is 2048's own per-race environment loader, called once per race from the
track loader. On the ordinary path it builds exactly the two files this page
already documents, `<circuit>\ZoneMode2048.effectSettings` and a second,
title-wide `ZoneMode2048default.effectSettings` (a **fourth, previously
unknown file**), and loads both independently into a shared resource cache
(`Environment_LoadEffectSettingsFiles`, confidence 78) - not
primary-then-fallback, and a missing file is a silent no-op for that one
insert rather than an error.

**When a circuit has no `art\published\environments\<circuit>\` tree of its
own, the loader assumes it is one of HD's four ported `zone_N` circuits and
reaches directly for HD's own title-wide files** -
`data/ZoneEnvironmentHDFury/ZoneModeHDFury[DLC3].effectSettings` or the
`DetonatorMode` pair, picked the same way this page's own key names are:
Detonator vs Zone by a mode flag, `DLC3` vs base by whether the circuit's own
path names `Zone_1`-`Zone_4`. **Verified reachable, not dead code**: DLC2
ships those four circuits under `data/art/published/DLC1/environments/zone_N/`
- the extra `DLC1` segment is exactly what makes the primary path probe fail
for them, which is exactly what this fallback needs to fire. **And verified
absent**: none of the five files this section names -
`ZoneMode2048default.effectSettings` and the four `ZoneEnvironmentHDFury\*`
paths - exist anywhere on the disc, checked directly against all three
packages (base, DLC1, DLC2).

**So the code path that would supply a palette on the four ported `zone_N`
circuits reaches for four files, all of them absent.** This is not a gap in
this project's reading; it is the original game's own code reaching for data
its own disc does not carry.

## `Zone_UpdateStage` (`0x81044cfc`) is the real per-stage selection - a second pass, same day

**This is what the "stage-index to zone-number correspondence" bullet below
was asking about.** `Environment_Load` calls it once at load time, and it is
also called every frame from 2048's main render-update loop - both call
sites pass `0`, the local player's own craft. Confidence 80, full evidence
on [zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md):

- Reads a per-craft struct field at `+0x634`, clamps it to `0xc` (**12**) -
  an exact match against 2048's own `ZoneMode2048.effectSettings` table,
  which names exactly thirteen stages, `0`-`12`.
- Indexes a table (`DAT_816c4890`, stride `0x59` words = 356 bytes, one row
  per stage) by that clamped value and **cross-fades** it against the next
  stage's row using a separate blend factor - so the visual transition
  between stages is a smooth interpolation, not a hard cut.
- Fires `"ZONE_Wave"`/`"ZONE_Pulse"` named cues when the stage (or a related
  `+0x638`-derived counter) changes.
- **Keeps running every frame on the four unshipped-palette circuits too** -
  the per-frame call is gated on whether *any* environment path was probed
  successfully at load time, not on whether the effectSettings file itself
  loaded. So there is no crash and no skip on those circuits: the blend math
  runs against whatever is already resident at `DAT_816c4890`, which nothing
  traced this pass ever writes for them specifically - plausibly
  zero-initialised `.bss` on a cold boot, not verified.

**What this does not settle**: the struct `+0x634` belongs to was not
identified, so its relationship to a `RaceState::zone`-shaped counter this
project already tracks for the other three titles
([zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md)) is open.
**Searched directly, a third pass, 2026-08-28**: `Zone_UpdateStage`'s own two
callers, `Zone_InitStageState`, the craft state machine (`FUN_811c711e`) and
an EMP-bar HUD setup function were all fully decompiled and none writes
`+0x634` - see
[zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md#dat_816c4890-is-built-not-the-raw-file---and-0x634s-writer-is-still-unfound)
for exactly what was ruled out. **`DAT_816c4890`'s provenance is sharpened,
not settled, same pass**: `Environment_LoadEffectSettingsFiles` writes it
directly - not a passive read of the raw-file cache - inside a block that
reverses a table in place, gated on a flag read off the current circuit
record (plausibly a reversed-circuit flag, unconfirmed). So `DAT_816c4890`
is a *built* table, not the parsed file itself, but how the raw bytes get
into it in the first place is still unread.

## The four ported circuits get their own subsystem - parallel, not feeding, `Zone_UpdateStage`

**2026-08-28, a separate pass run alongside the HD one below.** The DLC
fallback's own `FUN_8102493c` (called with the raw HDFury effectSettings
path - see above) is now decompiled and named
`Environment_LoadHDFuryContent` (confidence 75, full evidence on
[zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md#environment_loadhdfurycontent-0x8102493c-confidence-75---a-separate-subsystem-not-a-shared-one)).
It loads the effectSettings-shaped resource into its own cache (not
`Environment_LoadEffectSettingsFiles`'s), preloads all fifteen files of
whichever "Track" texture set the active mode names (confirming
`zonemodetrack{0..14}.gtf`/`DetonatorModeTrack{0..14}.gtf`'s identification
from executable control flow, not just the byte-pattern inference above),
and builds what reads as procedural geometry - plausibly the `Growing
Texture` mesh itself, unread past recognising the shape. **It touches none
of `Zone_UpdateStage`, `DAT_816c4890`, or `+0x634`/`+0x638`** - checked
against its full 1,664-line decompile, not sampled.

**Confirmed against `Environment_Load`'s own full decompile**: the DLC
branch that calls this function returns without ever reaching the label
that calls `Zone_UpdateStage` at load time. So on the four ported circuits,
`Zone_UpdateStage` runs **only** from its per-frame caller - the earlier
"plausibly zero-initialised `.bss`" hedge is now a confirmed mechanism, not
a guess: nothing in either `Environment_Load` branch ever populates
`DAT_816c4890` for these circuits, so the per-frame blend runs against
whatever it already held, every frame, with certainty.

## HD's own load site is found; a Ghidra database defect blocks the rest

**2026-08-28, a fourth pass, on HD/Fury's executable this time** (priority
order: Pulse, Pure, HD/Fury, 2048 - 2048's own thread is bottom of that
list, so this closes the HD/Fury half instead of digging further into
2048's dead end). Full evidence:
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).

HD reads its own `Data/Environments/ZoneMode.effectSettings` from a
function (`0x003f3fb0`) independently identified by an *earlier, unrelated*
session as per-race environment setup (it also loads `sky.gtf`/`skycube`,
per `envsettings.md`) - TOC-verified by hand, not trusted from Ghidra's own
(wrong) resolution, since this binary has a known, documented TOC defect
(`docs/ghidra/functions/ps3-hdfury-eu/memory.md`). That function passes the
loaded path to a second function, `0x003d6dc8` - and **there the trail
stops**: that function's own real TOC also differs from what Ghidra
assumes, so its decompiled body cannot be trusted (the same failure mode
`memory.md`'s own worked example demonstrates - a plausible-looking wrong
string, not an error). Closing it needs either a full database re-import
with the TOC fix applied, or a much larger manual verification pass than
this one attempted. **Nothing here reaches a stage-selection field the way
2048's `Zone_UpdateStage` does**, and nothing claims HD works the same way
2048 does - the two mechanisms have not been compared, only both partially
traced.

## Open

- **The stage-index to zone-number correspondence is inferred from the names
  alone**, not checked against `Zone_Update`'s own 10-second-per-step timer
  ([race-modes.md](../gameplay/race-modes.md#zone),
  [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md)). Nothing
  confirms stage `N` is shown while the zone counter reads `N`, versus some
  other indexing or interpolation between stages. **Sharpened, not settled,
  2026-08-28**: HD's own `speech_zone.bnk` names fourteen `MR_*` cues at
  consecutive indices 26-39 - `MR_SVE`, `MR_VEN`, `MR_SFL`, `MR_FLA`,
  `MR_SRA`, `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`,
  `MR_Z_SUB`, `MR_Z_M1`, `MR_Z_SUP` - which read as Sub Venom/Venom/Sub
  Flash/Flash/.../Subsonic/Mach 1/Supersonic in that exact order: a 14/14
  match, one cue per non-`Start` stage of this table's own 15-stage ladder,
  contiguous rather than two separate blocks (the `MR_Z_*` prefix on the last
  three is a naming quirk, not a positional break - read directly with
  `oag-wad sounds` against the extracted `.bnk`, not assumed from
  `psp-audio.md`'s prose list, which undercounted it as "eleven ... plus
  three"). That is strong evidence the effectsettings ladder **is** the same
  thing `SpeedClass`/`NextSpeedClass` names and this announcer voices - see
  `psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`
  and `crates/hd/src/race.rs`'s `ZONE_ANNOUNCER`. **Sharpened further, same
  day, on 2048 rather than HD**: `Zone_UpdateStage`
  (`0x81044cfc` -
  [zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md))
  reads a per-craft field, clamps it to `12`, and that clamp is an exact
  match against 2048's own thirteen-stage table - real selection logic,
  found and named. **One avenue closed**: the DLC-specific loader,
  `Environment_LoadHDFuryContent`, was checked in full and does not touch
  `+0x634`, `Zone_UpdateStage`, or `DAT_816c4890` either - it is a separate
  subsystem for the four ported circuits, not a second writer to look for
  the field in.

  **2026-08-30: both titles' write sites found, independently, same day.**
  2048's `+0x634`/`+0x638` writer is `Hud_UpdateZoneSpeedClassWidget`
  (`0x81197d6c`) - the Zone-mode HUD's own Speed Class number widget, not a
  race-progress timer, driven by a percentage-shaped value against a
  17-entry Mach-number threshold table
  ([zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md#2026-08-30-a-fourth-pass-0x6340x638s-writer-is-found---its-the-zone-mode-huds-own-speed-class-widget-not-racephysics-logic)).
  HD's own field (a two-entry array's `+0x00`/`+0x04`, a different struct
  shape from 2048's per-craft one) is written by
  `Environment_UpdateStageBlend` (`0x003da540`), a self-contained
  request/commit pair: `+0x04` holds a requested stage sourced from
  `g_GameState.mode`-dependent tables, committed into `+0x00` when the two
  differ
  ([zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-ninth-pass-the-0x00-write-is-found-environment_updatestageblend-at-0x003da540-confidence-85)).
  **Per `CLAUDE.md`'s rule that a trigger needs recovering before an effect
  is wired, the mechanism itself is now recovered on both titles** - what
  is not recovered is the exact *numeric* mapping from live ship speed to a
  stage index: 2048's own 17-entry threshold table read inconsistently on
  direct decode (a per-record string-pointer field didn't line up 1:1 with
  its own threshold, unresolved rather than force-fit), and HD's
  `g_GameState.mode`-dependent source tables were traced to their existence,
  not to their contents. **So the format/blend/render-application plumbing
  can be wired now** (the recovered fields and cross-fade math are enough
  to drive it from an explicit stage+weight input, the same way
  `.envsettings` already drives static fog/sky), **but the live
  speed-to-stage trigger itself is not** - inventing those thresholds would
  be exactly the fitted-constant-as-recovered-fact `CLAUDE.md` forbids.
- **2048's table names 13 stages but the title ships 15 textures in each
  set** - still unexplained, though sharpened by the texture-content finding
  below: the "Track" set is where the real per-stage art lives on both
  titles, so this is a mismatch in the *content* set that matters, not in
  spare duplicates of a placeholder. Either two stages are unnamed/implicit,
  the extra two are unused leftovers from the HD port, or the
  stage-to-texture correspondence is not 1:1 to begin with.
- **Why `detonatormode.effectsettings` carries Zone's own 15-stage ladder is
  unread.** Detonator has no known "zone number" that ramps the same way Zone
  does, so either the table is reused wholesale regardless of relevance, or
  Detonator has an escalation mechanic of its own that has not been looked
  for. **Partial answer, 2026-08-29**: HD's own executable schema
  (`g_EffectSettingsSchemaKeyNames`, above) is a single, mode-agnostic
  vocabulary shared by both `ZoneMode` and `DetonatorMode` files - it was
  never authored with a separate Detonator ladder to draw on in the first
  place, which explains the reuse without yet explaining whether Detonator
  needs one. **One data point, not a resolution**: 2048's `Environment_Load`
  treats Zone and Detonator as genuinely separate tables even in its
  fallback path (`ZoneModeHDFury.effectSettings` vs
  `DetonatorModeHDFury.effectSettings`, picked by a mode flag) - so the
  engine's own code does not assume they are the same table, whatever the
  shipped files' shared stage names might suggest.
- **Checked, 2026-08-28: neither Pulse nor Pure appears to carry an
  analogous table, though a hash-named WAD cannot be proven empty, only
  searched.** `scripts/mine-names.py`'s own candidate generation (executable
  strings, template combinations, the plugin definition's real track
  locations - 1,409 candidates for Pulse, 1,509 for Pure) was matched against
  each disc's `Data.wad` directory. Every `zone`-named entry either disc
  resolves is geometry (`zone_track.vex`/`_reversed`), the shared hull
  (`Zone.vex`/`zonewreck.vex`), audio (`ship_zone.bnk`, `speech_zone.bnk`,
  Pulse's own `ZONE_ENV.bnk`) or HUD/UI (`Zone_HUD.xml`,
  Pulse's `screen_zone.xml` - read directly: a front-end track-select
  carousel layout, its only colour a `0xCF000000` UI drop-shadow constant,
  not a palette). Pure's `Data\Zone\0N_Zone\TrackStartup.xml` is 162-163 B,
  too small to carry fifteen palettes. A further **322 hand-guessed**
  candidates (stage-name fragments against both titles' own path
  conventions) added nothing either. Their Zone circuit is plain
  `Skycube`/`fogCube` geometry ([skycube.md](skycube.md), [vex.md](vex.md))
  with no obvious sibling. **Not proof of absence** - a hash-named WAD can
  always hide an entry no candidate list reaches - but two independently
  generated candidate sets agreeing on nothing is stronger than "not
  checked."
- **Which texture depicts `Growing Texture` is settled for HD, 2026-08-28;
  what the parameters do to it is not.** The "general" `zonemode{0..14}.gtf`
  set is a red herring: all fifteen are **byte-identical** (one 87,552 B
  DXT4/5 256x256 file, `md5sum` confirms it) and decode
  (`oag_formats::gtf`) to flat, uniform `[255, 255, 255, 255]` on every one
  of the 65,536 texels of every one of the fifteen files - no picture at
  all. **The "Track" `zonemodetrack{0..14}.gtf` set is where the real art
  is**: fifteen genuinely distinct DXT4/5 files (confirmed both by decoded
  pixel content and by raw `md5sum` - no two match), each a greyscale image
  with a varying alpha channel, and the pattern visibly changes shape
  between stages sampled (0, 1, 7, 14) - blocky interlocking shapes at
  `Start`, fine vertical stripes partway through the ladder, a dense small
  grid at `Supersonic`. That is consistent with `Growing Texture`'s own name
  - an escalating visual keyed to speed class - but **what the shader does
  with `Growing Texture.Colour`/`.Scale Bias`/`.Factors` against this art is
  still unread**; no HD executable has been read for its Zone shader, so
  whether the four `Scale Bias`/`Factors` numbers pick a UV tile, drive a
  scroll, or something else is not established, only that there is now a
  real picture for them to apply to. **2048's own copy could not be checked
  the same way**, but its byte pattern already corroborates the same
  two-set split: `zoneMode{0..14}.gxt` are as byte-identical to each other
  as HD's "general" set, while `zoneModeTrack{0..14}.gxt` are fifteen
  distinct files, exactly matching HD's "Track" set - checked from the raw
  bytes alone, without needing to decode them. Both sets carry format byte
  `0x0c` - **`SceGxmTextureBaseFormat U8U8U8U8`, corrected 2026-08-28 from an
  earlier `U4U4U4U4` mislabel** (`gxt.md`'s own history of the correction)
  - one of the six format codes
  [gxt.md](gxt.md#the-format-byte-names-a-scegxmtexturebaseformat) already
  catalogues (99 of 9,910 files disc-wide) but does not decode. The raw
  `format`, `0x0c001000`, bitwise-matches `U8U8U8U8 | SWIZZLE4_ARGB` exactly
  against the public vitasdk headers, so the channel order is now a
  well-evidenced hypothesis rather than a blind guess - but **the tiling
  order is not**: whether the four raw bytes read raster or the same
  Morton/twiddle order `.gxt`'s own `UBC2` reader needs is unset by anything
  in the file, the same ambiguity that format's own reticle-texture decode
  had to settle by trying both and looking at a recognisable picture. This
  format's own corpus has offered nothing that recognisable yet, so decoding
  it here on the channel order alone would still risk the tiling half of the
  "plausible-looking stand-in" `CLAUDE.md` warns against. What 2048's own
  "Track" art actually looks like is still open.
- ~~**Wiring a stage's values into a race**~~ **Done, 2026-08-30** - see
  [What is read and what is not](#what-is-read-and-what-is-not). The
  format/blend/render-application plumbing is in and checked against the real
  image; **what remains open is only the live trigger**, which is the numeric
  mapping neither title has yielded (2048's 17-entry threshold table read
  inconsistently, HD's mode-keyed source tables were traced to their
  existence rather than their contents). The seam a recovered trigger
  attaches to is `ZoneGrade::request_stage`/`set_weight`, which today no race
  calls.
- **Which quantity `+0x18` actually is, and which way it runs.**
  `cross_fade_rgba8`'s own recovered doc reads `1.0` "at the moment a stage
  becomes current"; `Environment_UpdateStageBlend` writes `+0x18 = 0` at
  exactly that moment. Both cannot be the same quantity with the same
  meaning, and nothing recovered says what advances the field afterwards.
  This build keeps `cross_fade_rgba8`'s convention and zeroes on commit the
  way the traced store does, which is visibly consistent but not evidence.
- **Which schema keys the three blended runtime fields are.** The cross-fade
  reads three 16-byte fields per stage; the correspondence to
  `Scene.Texture/Near/Base Colour` is positional and explicitly unconfirmed,
  so the Rust side blends fields chosen by the *file's own key names*
  instead. A read of the schema's destination-pointer-to-struct-offset
  mapping would settle it - and would also settle whether the byte-domain
  blend applies to keys this project currently fades in floats, which it does
  because three of `zonemode.effectsettings`' own colours exceed `1.0` and
  byte quantisation would send them all to white.

## See also

- [envsettings](envsettings.md) - the sibling format this reuses the
  tokeniser from, and the circuit-wide light rig this table overrides stage
  by stage during Zone
