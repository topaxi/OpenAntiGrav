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
**Not yet wired into a race** - see [Open](#open) below.

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

2048's copy is HD's ported rather than reauthored: both titles ship 15
`zoneMode*`/`zonemode*` textures beside the table (2048 doubles them into
`zoneMode*.gxt` and `zoneModeTrack*.gxt`), matching HD's 15-stage count
exactly even though 2048's own table only names 13 - see [Open](#open).

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
is the same design, a later authoring pass over the same idea.

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

Nothing draws off this yet. `oag_formats::effectsettings` parses the table;
no Zone loader on any title (Pulse's `ZoneCraft::ModelsInTeam`, HD's
`ZoneCircuit::Separate`, or 2048's `ZoneCircuit::SameCircuit`) wires a stage
lookup in, and no renderer reads a stage's fog/sky/ambient. A Zone race in
this engine currently looks identical to an ordinary race on every title
except for the HUD and the ship-model swap Pulse alone does.

## Open

- **The stage-index to zone-number correspondence is inferred from the names
  alone**, not checked against `Zone_Update`'s own 10-second-per-step timer
  ([race-modes.md](../gameplay/race-modes.md#zone),
  [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md)). Nothing
  confirms stage `N` is shown while the zone counter reads `N`, versus some
  other indexing or interpolation between stages.
- **2048's table names 13 stages but the title ships 15 textures each**
  (`zoneMode0..14.gxt` + `zoneModeTrack0..14.gxt`) - unexplained. Either two
  stages are unnamed/implicit, the extra two are unused leftovers from the HD
  port, or the stage-to-texture correspondence is not 1:1 to begin with.
- **Why `detonatormode.effectsettings` carries Zone's own 15-stage ladder is
  unread.** Detonator has no known "zone number" that ramps the same way Zone
  does, so either the table is reused wholesale regardless of relevance, or
  Detonator has an escalation mechanic of its own that has not been looked
  for.
- **Whether Pulse or Pure carry an analogous table has not been checked.**
  Their Zone circuit is plain `Skycube`/`fogCube` geometry
  ([skycube.md](skycube.md), [vex.md](vex.md)) with no obvious sibling file
  spotted in either disc's `Data\Environments\` tree, but no targeted search
  was done - the PSP titles' assets are hash-named in their WAD, unlike the
  two PSARC titles' plain paths.
- **`Growing Texture.Scale Bias`/`.Factors`** (4 floats each) presumably
  drive whichever `zoneMode*.gxt` is shown and how it is tiled/scrolled, but
  no `.gxt`/`.gtf` in the set has been opened to see what the texture depicts.
- **Wiring a stage's values into a race** - the next concrete step once the
  zone-number correspondence above is settled. HD is the best-measured
  target, since its `.envsettings` reading and drawing path already exists.

## See also

- [envsettings](envsettings.md) - the sibling format this reuses the
  tokeniser from, and the circuit-wide light rig this table overrides stage
  by stage during Zone
