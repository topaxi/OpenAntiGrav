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
reauthored:**

| Title | Textures | Count | Size each |
| --- | --- | --- | --- |
| HD | `/data/tex/zonemode0..14.gtf` | 15 | 87,552 B |
| 2048 | `data/Tex/zoneMode0..14.gxt` + `zoneModeTrack0..14.gxt` | 15 + 15 | 349,568 B |

15 textures on both, matching HD's 15-stage count exactly (2048's own table
only names 13 stages but still ships 15 textures - see Open). 2048's doubled
set (`zoneMode*` and `zoneModeTrack*`) likely pairs with the `Track Paint`
key block having its own colour pair distinct from the general palette, but
that is a guess from naming, not confirmed.

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
  or interpolation between stages.
- 2048's table names 13 stages but the title ships 15 `zoneMode*`/`zoneModeTrack*`
  textures each - unexplained. Either two stages are unnamed/implicit, the
  extra two textures are unused leftovers from the HD port, or the
  correspondence between stage and texture index is not 1:1 to begin with.
- Why `detonatormode.effectsettings` carries Zone's own 15-stage speed-class
  ladder is unread. Detonator has no "zone number" that ramps the same way
  Zone does (or does it? - unconfirmed), so either the table is reused
  wholesale regardless of relevance, or Detonator has an escalation mechanic
  of its own that has not been looked for.
- Whether Pulse or Pure carry any analogous data-driven visual escalation for
  Zone has not been checked. Their Zone circuit is plain `Skycube`/`fogCube`
  geometry (`docs/formats/skycube.md`, `docs/formats/vex.md`) with no obvious
  sibling file spotted so far in either disc's `Data\Environments\` tree, but
  no targeted search for a similarly-named per-stage table was done - the PSP
  titles' assets are hash-named in their WAD, which makes a blind directory
  listing unlike the two PSARC titles' plain paths.
- The `Growing Texture.Scale Bias`/`.Factors` keys (4 floats each) presumably
  drive whichever `zoneMode*.gxt` is shown and how it is tiled/scrolled, but
  no `.gxt`/`.gtf` in the set has been opened to see what the texture even
  depicts (a warp tunnel, a vignette, a scrolling grille - unknown).

## Next Steps

- Cross-reference the stage-name ladder against `Zone_Update`'s recovered
  timer logic to settle whether stage `N` is live exactly while the zone
  counter reads `N`, before wiring anything that assumes it.
- Wire the current stage's fog/sky/ambient into the renderer for one title as
  a proof of concept, once the timer cross-reference above settles - HD is
  the best-measured target, since its `.envsettings` reading and drawing path
  already exists ([envsettings.md](../docs/formats/envsettings.md)) and this
  is the same shape.
- Decode one `zoneMode0.gtf` (or `.gxt`) to see what the texture actually is;
  that settles what `Growing Texture` is for.
- Look for an analogous per-stage table on Pulse and Pure before concluding
  they have none - check the disc's own string table / name-mining output
  for anything matching `zone` + a stage-name fragment (`venom`, `flash`,
  `phantom`, `zen`, `subsonic`, `supersonic`) rather than assuming the
  mechanism is HD-lineage-only.
