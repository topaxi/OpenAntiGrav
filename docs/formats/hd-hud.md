# Wipeout HD / Fury: the in-race HUD

**Status: the layouts are read, confidence 90.** HD/Fury's HUD is the same
authored data Pulse's is - [front-end XML](fexml.md) carrying exact rectangles,
atlas sub-rectangles, named colours, font roles and alignment, plain here rather
than shortened - and it goes through the reader Pulse's HUD already had, with
two changes that the lineage forced. All **eighteen** shipped layouts compose
with **nothing missing and nothing skipped**, into 2,320 widgets.

Pinned by `crates/game/tests/hd_hud_ground_truth.rs`; the entry names are
[`oag_hd::hud`](../../crates/hd/src/hud.rs), the reader is
[`oag_hud::compose`](../../crates/hud/src/compose.rs). Measured
2026-08-17 on `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`.

90 rather than higher because **no executable path has been read**: every number
is static reading plus exact agreement with shipped data, and the two rules
recovered below ([offset
composition](#the-rule-offsetxoffsety-translate-xy-place) and the [texture
extension](#the-layouts-name-source-art)) are inferred from the data agreeing
with them rather than from code that implements them. See the [confidence
rubric](../reverse-engineering/confidence-rubric.md). The textures themselves
**do** decode - [gtf](gtf.md), landed the same day - and every sprite's source
rectangle is checked against them, which is what a score of 90 rests on rather
than the layout arithmetic alone.

**Since 2026-08-25 it also reaches a shader**, and has been compared against a
frame of the running original for the first time - see [the HUD draws, and each
sprite out of its own
texture](#the-hud-draws-and-each-sprite-out-of-its-own-texture) for what that
turned up and [what is not done](#what-is-not-done) for the four things the
frame shows that this build still does not.

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
    -E 'binary(hd_hud_ground_truth)'
```

## The headline

**HD's HUD is Pulse's HUD, composed out of fragments instead of written in one
file.** Nothing about the widget model changed: `<Image>` with a `Src` is a
sprite, without one a solid fill, `<Text>` is a label, `U`/`V`/`TxtrWidth`/
`TxtrHeight` index an atlas, `FEConst->Name` resolves against a `<Variable
global=>` table. What changed is *where the widgets are*.

| | Pulse (PSP/PS2) | HD / Fury (PS3) |
| --- | --- | --- |
| Layouts | 5, one per mode | **18**: 8 modes, 3 skins |
| Files per layout | 1 | **1 to 17** |
| Dialect | shortened, `<code>` dictionary | plain `<?xml` |
| Coordinate space | 480x272 | [1920x1080](hd-frontend.md#the-coordinate-space-is-1920x1080) |
| Widgets holding widgets | **0** | 242 |
| Elements carrying an offset | 26, all `<Item>` | 218, 116 of them widgets |
| Atlas | one `.mip` per layout | up to 6 [`.gtf`](gtf.md) per layout |
| Nameless widgets | 0 | 106 |

## The eighteen roots

`/data/xml/[<skin>/]<mode>_hud.xml`, read off the manifests rather than off the
executable. Sprite/fill/label counts are the composed layout; `files` counts the
root itself.

| Mode | Files | Sprites | Fills | Labels | wo3 | 2097 |
| --- | ---: | ---: | ---: | ---: | :-: | :-: |
| `arcade_hud.xml` | 17 | 138 | 4 | 70 | yes | yes |
| `elimination_hud.xml` | 15 | 132 | 2 | 70 | yes | yes |
| `timetrial_hud.xml` | 14 | 51 | 1 | 43 | yes | yes |
| `speedlap_hud.xml` | 15 | 50 | 1 | 40 | yes | yes |
| `zone_hud.xml` | 5 | 36 | 1 | 32 | yes | yes |
| `detonator_hud.xml` | 8 | 71 | 2 | 40 | - | - |
| `mptag_hud.xml` | **1** | 0 | 0 | 8 | - | - |
| `duel_hud/duel_hud.xml` | 9 | 62 | 1 | 52 | - | - |

`wo3` and `2097` are the Wipeout 3 and Wipeout 2097 retro skins, under
`/data/xml/wo3_hud/` and `/data/xml/2097_hud/`. The same three-way split appears
in `skin.xml`'s font slots - `HUD`, `wo3HUD`, `2097HUD` - which
[hd-frontend](hd-frontend.md) records. **Detonator, Duel and MPTag have no
skinned variant**, which is what you would expect of the two Fury modes and a
multiplayer mode that the retro skins predate.

`mptag_hud.xml` is the one HD HUD shaped like a Pulse one: no includes, eight
text widgets, nothing else - the same floating name tags
`oag_pulse::hud::layouts::MP_TAG` is.

**A split-screen family also ships** - `splitscreen_hud/`,
`splitscreenzone_hud/`, `duel_hud/duel_splitscreen_hud/` and a `vert_` variant
of most of it, inside the 312 archive entries with `hud` in their path - and is
deliberately not
enumerated, because nothing in this project draws two viewports. The one to know
about is `/data/xml/splitscreenzone_hud/zone_hud.xml`: same basename as the Zone
root, one directory across, and a looser rule picks it up. The ground-truth test
asserts it exists *and* is excluded, so the exclusion is a decision on the record
rather than a rule that happens to miss it.

## A mode's HUD is a shell plus a dozen fragments

`Data\XML\Elimination_HUD.xml` is 1,856 bytes and holds **three** widget
elements, two of which are the `ForwardHUD`/`ReverseHUD` groups that wrap
everything else:

```xml
<Screen>
  <Screen name="HUD" transition="0">
    <Image name="ForwardHUD">
      <LoadXML><Values SrcRel="HUD_colours.xml"    DirectEmbed="true"></Values></LoadXML>
      <LoadXML><Values SrcRel="HUD_sights.xml"     DirectEmbed="true"></Values></LoadXML>
      <LoadXML><Values SrcRel="HUD_ready_go.xml"   DirectEmbed="true"></Values></LoadXML>
      ...
```

Composed, it is **15 files and 204 widgets**. Read the shell alone and you get
two empty rectangles, with nothing reported as lost - because nothing *was*
lost: the widgets were never in that file. That is the failure mode this page
exists to close, and it is a quiet one, because a HUD with two rectangles in it
looks like a rendering problem rather than a reading one.

Three properties of the include, each counted over every `<LoadXML>` in every
HUD file on the disc:

| | Count |
| --- | ---: |
| `<LoadXML>` elements | 720 |
| carrying `SrcRel` | **720** |
| carrying `Src` | **0** |
| carrying `DirectEmbed="true"` | **720** |

Unanimity in shipped data is not evidence about what an attribute *selects*, so
`DirectEmbed` is read by nothing here, and the `Src` branch of the resolver is
exercised by unit tests and by no file on this disc. Both are recorded as
untested paths rather than as decisions.

### `SrcRel` resolves against the including file's directory

This is what makes the three skins distinct rather than aliases:
`/data/xml/elimination_hud.xml` and `/data/xml/wo3_hud/elimination_hud.xml` both
name `HUD_elim_line.xml`, and they mean two different files. Resolving from a
fixed root instead loads the default skin's fragments for all three - which
produces a HUD that draws, and the wrong one.

**And `..` ships.** `Data\XML\Duel_HUD\Duel_HUD.xml` reaches four of its
fragments as `SrcRel="..\HUD_colours.xml"` and the rest by bare name. A reader
that concatenates asks for `/data/xml/duel_hud/..\hud_colours.xml`, is told
there is no such entry, and silently loses the colour table, the countdown, the
info text and the time-difference readout - a quarter of that mode's HUD, absent
with an explanation that reads like a missing file. Duel goes from 5 files to 9
once the segment is applied.

## The rule: `OffsetX`/`OffsetY` translate, `x`/`y` place

Two coordinate attributes with two different meanings, and telling them apart is
the whole of the geometry change. `OffsetX`/`OffsetY` move the element **and
everything under it**; `x`/`y` place the element inside whatever translation it
inherited. So offsets **compose** down the tree and positions do not.

`oag_hud` read an `<Item>`'s offset as *absolute* until 2026-08-17, with
the note "nothing in the shipped data nests them, so composing would be untested
code". True of Pulse, and false of the lineage:

| | Pulse | HD / Fury |
| --- | ---: | ---: |
| elements carrying an offset | 26, **all `<Item>`** | 218 |
| of those, widgets rather than `<Item>` | 0 | 116 |
| widgets holding a widget | **0** | 242 |
| offsets carried inside `<Values>` | 0 | 0 |

Composing is therefore a **strict no-op on Pulse** - nothing there ever inherits
a non-zero offset, and all nine of `hud_layout_ground_truth`'s assertions pass
unchanged - and is required to read HD at all.

**An element's own offset applies to itself, not only to its children**, and
`HUD_Elim_info_text.xml` is what settles that half:

```xml
<Text name="Info1" OffsetX="600" OffsetY="330" transition="0" SDOffsetY="100">
  <Values string="info1" font="HUDSmall" align="centre" vertalign="middle" x="0" y="20" .../>
  <Image name="FrameMiddle1"><Values centred="true" x="0"    y="20" width="240" .../></Image>
  <Image name="FrameRight1"> <Values centred="true" x="160"  y="20" width="52"  .../></Image>
  <Image name="FrameLeft1">  <Values centred="true" x="-160" y="20" width="40"  .../></Image>
</Text>
```

The text lands in the middle of its own three-piece frame only if the element's
`x`/`y` and its children's are read in the *same* translated space. Read the
element's own position in the parent's space instead and the caption sits at
(0, 60) while its frame sits at (600, 350).

The corroborating case is `HUD_Elim_positions.xml`, where four sibling
`PosTag0..3` at `y` 90, 170, 250, 330 each hold a background `<Image>` at 62,
142, 222, 302 - a constant −28 that **tracks the parent** rather than being
−28 relative to it. Parent-relative would put all four children at −28.

`SDOffsetX`/`SDOffsetY` appear on 31 elements and are **not applied**: they are
the standard-definition variant of the same offset, and nothing here targets a
576-line output.

### What this does *not* pin

The composed extents run x −1048..2208 and y −872..1096, so the layouts do not
close on 0..1920 / 0..1080 by themselves, and this page does not re-derive the
coordinate space - it cites [hd-frontend](hd-frontend.md#the-coordinate-space-is-1920x1080)'s
reading of `skin.xml` (confidence 90). Widgets outside the box are the expected
kinds: the runtime-anchored position tags, the reverse-view mirror set, and
parked off-screen elements. `oag_hud::inside_screen` is still PSP-only and
**a screen-space check for HD is not written**; doing it needs the same
`x`-is-not-one-thing distinction [ui/hud.md](../ui/hud.md) records for the PS2.

## The layouts name source art

A HUD sprite's `src` is the **exporter's input**, not the shipped file. Across
the eighteen composed layouts there are twelve distinct references and 1,029
sprites:

| Uses | Reference | On the disc |
| ---: | --- | --- |
| 338 | `Data\HUD\Textures\HUD_Components.gtf` | DATA02 |
| 148 | `Data\HUD\Textures\hdHUD.mip` | **as `hdhud.gtf`**, DATA02 + DATA03 |
| 105 | `Data\HUD\Textures\fury_hud.gtf` | DATA00 |
| 104 | `Data\XML\wo3_hud\texture\wo3_hud.gtf` | DATA03 |
| 97 | `Data\XML\2097_hud\texture\wo2097_hud.gtf` | DATA00 + DATA03 |
| 63 | `Data\HUD\Textures\HUD_Components_01.gtf` | DATA02 |
| 56 | `Data\FE\Images\voiceCom.gtf` | front end |
| 44 | `Data\HUD\Textures\detonator_hud2.tga` | **as `detonator_hud2.gtf`**, DATA00 |
| 36 | `Data\HUD\Textures\missile_reticule.gtf` | DATA02 |
| 24 | `Data\HUD\Textures\HUD_Components_02.gtf` | DATA02 |
| 12 | `Data\HUD\Textures\nitro_hud.gtf` | DATA00 |
| 2 | `Data\HUD\Textures\ZoneDamage.gtf` | DATA02 |

Ten spell themselves `.gtf`; the two that do not are the two with the most uses
between them. **There is no `.mip` anywhere on this disc**, and the only `.tga`
is a smoke ramp under `data/ribboneffects`. Both resolve when the extension is
replaced with `.gtf`, so the rule is *replace the extension* -
`oag_hd::hud::texture_entry` - and with it twelve of twelve references resolve
to a shipped entry rather than ten of twelve. Read literally, 192 of the HUD's
1,029 sprites have no texture, which is exactly how it looked before this was
measured.

**Confidence 90** on the rule: twelve of twelve resolve and the two exceptions
carry 192 sprites, so it is not a coincidence of one file. 2026-08-25: widened
past the HUD's own twelve references to every `Src`/`ImageSrc`-shaped image
reference across all of HD's XML - 133 distinct values spanning all eighteen
layouts, the front end, ships and skins - and **129 of 133 (97%) resolve
under this same rule**. The four that do not (`teaser_firedup.mip`,
`teaser_medievil.mip`, `teaser_wipeout.mip`, `default_texture.mip`) do not
exist on the disc under any name or extension at all - cut content and an
unbacked fallback name, not counter-examples. Still short of the
arithmetic-invariant ceiling (94) for that reason, and because **the literal
HUD code path is still unread**; an engine that tries the literal name, fails,
and falls back would be indistinguishable from here. What raised it off 85:
chasing this rule through the executable found a hardcoded instance of the
identical convention - a ship-thumbnail
loader builds `%s\fe\miniBW.gtf` from a bare directory, discarding entirely
that the XML authors the same asset as `Data\Ships\<Ship>\fe\miniBW.tga`. That
is what a blind extension-replace with no fallback looks like from inside the
executable, for one asset kind - not a trace of the HUD's own code path, but
no longer an inference from the file set alone either. A second spelling
exists on this disc and is *not* what the HUD uses:
`/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf` **appends**.

**One reference is not a HUD texture at all.** `Data\FE\Images\voiceCom.gtf` is
a front-end image, reached from the position-list fragment for the voice-chat
indicator - so a HUD's texture set is not confined to `data/hud/textures/`, and
a loader that assumed a directory gets 56 sprites wrong.

## A nameless widget is drawn, not broken

Pulse names every widget in all five layouts, so the layout reader required a
`name` and recorded "an `<Image>` with no name" in its skipped list otherwise.
HD authors **106** nameless widgets against 2,214 named, most of them the
`BackgroundLayer="1"` panels sitting behind a named readout. Dropping those
leaves the numbers with nothing behind them.

They are kept with an empty name. Nothing in the draw list's tables is named
`""`, so an anonymous widget carries geometry and can never be selected by
name - which is what an unaddressable widget should do.

## The same path ships in more than one archive

Six of the eighteen roots exist in two or more archives **at different sizes**:
`/data/xml/elimination_hud.xml` is 1,856 bytes in `DATA00` and 5,666 in
`DATA02`; `/data/xml/zone_hud.xml` is in `DATA02`, `DATA03` and `DATA06` at
10,681, 10,979 and 10,891; `speedlap_hud.xml` is in four.

`oag_assets::Archives` resolves those by this project's own precedence -
`DATA00`, then `DATA02`, then the rest, per
[`oag_hd`](../../crates/hd/src/lib.rs)'s candidate lists - and **whether the
original picks the same copy is not read.** The counts in this page and in the
ground-truth test are therefore the composition *this build* performs. A change
to the archive order moves them legitimately, which is why the test pins them
per root rather than in aggregate: the table is where that shows up, instead of
in a screenshot nobody compares.

Worth knowing before spending time on it: the executable's own
`Hud_LoadDefinition` call sites pass whole paths as literals, so the archive
choice is not made by composing a name at runtime. See
[race-hud.md](../ghidra/functions/ps3-hdfury-eu/race-hud.md).

### The precedence mechanism, read precisely

`Archives::holder_of` (`crates/assets/src/source.rs`) checks the bulk archive
(`data`, which `oag_hd`'s `DATA_CANDIDATES` pins to `DATA00` alone) first, then
the companion archive (`fe`, pinned to `DATA02` alone), then `extra` in
candidate order, and returns the **first** match - `read_name` never sees the
other copies. So `/data/xml/elimination_hud.xml` and `/data/xml/speedlap_hud.xml`
resolve to `DATA00`, which has both. `/data/xml/zone_hud.xml` has no `DATA00`
copy at all, so the `data` check misses and the `fe` check (`DATA02`) wins over
`DATA03` and `DATA06`, which are only ever reached through `extra` - behind
`DATA02` in every case a `DATA00` copy is absent.

### Diffing the three copies directly: three different relations, not one

2026-08-26. Every prior pass at this open question read the executable; nobody
had extracted and diffed the actual bytes. `scripts/psarc.py cat` against
`hdfury-ps3-eu-dec.iso` pulls a specific archive's copy of a path without
guessing at precedence, and the three multi-copy roots turn out to disagree in
three different ways - not one bug repeated three times.

**`elimination_hud.xml` (`DATA00`, 1,856 B vs `DATA02`, 5,666 B): two different
designs, not a superset and a subset.** `DATA00`'s copy composes entirely from
twelve `HUD_Elim_*`-prefixed fragments private to Elimination mode, including
`hud_elim_positions.xml` - an 8-slot position list (`PosTag0`-`PosTag7`), each
slot carrying a `VoiceCom` icon and an `MPWeaponIcon`, textured from
`fury_hud.gtf`. `DATA02`'s copy instead reuses the *generic* race-HUD
fragments other modes share (`HUD_lap_counters.xml`, `HUD_damage_indicator.xml`,
`HUD_pickups.xml`, ...) and adds one inline block - not a fragment - defining a
`KillsText` counter and six `PosTag0`-`PosTag5` slots on `HUD_Components.gtf`/
`HUD_Components_01.gtf`, with no voice-chat or per-opponent weapon icons. Every
fragment either copy names resolves on the disc; this is not a missing-file
defect in either one.

**`speedlap_hud.xml` (four copies: `DATA00`/`DATA06` both 2,635 B, `DATA05`
2,637 B, `DATA02` 2,392 B): a real superset/subset, plus a disabled feature
caught in the act.** `DATA00` and `DATA06` are byte-identical. `DATA02` is
that same file with exactly two `<LoadXML>` blocks removed - the
`HUD_lap_ghost.xml` include and the `SplitScreen_hud\SplitScreen_other_player_tag.xml`
include - and nothing added; every line `DATA02` has, `DATA00` also has.
`DATA05` is `DATA00`'s content with the ghost-lap block's tag misspelled,
`<aLoadXML>`/`</aLoadXML>` instead of `<LoadXML>`/`</LoadXML>` - a
one-character-per-tag change that accounts for `DATA05`'s extra 2 bytes
exactly. `LoadXML_Item`'s reader keys on the literal tag name (see this page's
own `Open` history and `oag_tables::fexml`), so a renamed tag is not a
fragment that fails to resolve - it is a fragment that is never looked for at
all, authored to be skipped without deleting the reference. This project has
not previously documented this project's own disc doing that as a way to
disable a feature; it is worth knowing the shape exists, next time a HUD or
front-end oddity looks like a parser bug.

**`zone_hud.xml` (`DATA02` 10,681 B, `DATA03` 10,979 B, `DATA06` 10,891 B): the
copy precedence actually serves is the one missing a widget the other two
share.** `DATA03` and `DATA06` both carry a `<Text name="Zone Perfect Txt"
idstring="ER_PERFECT" .../>` that `DATA02` does not have at all;
`ER_PERFECT` resolves to the string `"PERFECT"` in
`/data/plugins/languages/english/entries.xml`. `DATA03` and `DATA06` differ
from each other only in how eleven ring-of-dots widgets spell the same colour
attribute - symbolic (`color="FEGlobals->HD_Blue"` in `DATA03`) versus literal
(`color="0xFF8AC0CA"` in `DATA06`) - which direction that went, or whether the
two actually resolve to the same value, is not checked here. Per the
precedence mechanism above, **`zone_hud.xml` resolves to `DATA02`** - the one
copy of the three missing the "PERFECT" widget - because `DATA03` and `DATA06`
are never reached; a `DATA00` copy does not exist so the `fe` archive
(`DATA02`) wins outright. `hd_hud_ground_truth.rs`'s pinned zone_hud row (`36,
1, 32, 5`) is this composition; resolving to `DATA03` or `DATA06` instead
would add exactly one `Label` (a `<Text>` with no children), making it `36, 1,
33, 5`.

The concept the missing widget names is not cut art: `/data/sound/speech_zone.bnk`
(`DATA00.PSARC`, confirmed again in `DATA01.PSARC` at a different byte count -
an eighth multi-copy root, unread beyond this) ships a `PERFECT_LAP` announcer
cue in its `zone_vo` bank, the same cue Pulse's `Data.wad` carries under the
identical name. A voice line is not the same claim as an on-screen label, and
`PERFECT_LAP` names a lap rather than `ER_PERFECT`'s zone, but it rules out
"the widget is unused leftover text" as the reason `DATA02` might be the
correct pick.

**This is not a new failure mode - it is the same one `Archives::locations`'s
own doc comment already names for a different file.** Wipeout HD's front-end
plugin definition (`Data\Plugins\frontend\definition.xml`, five copies,
[`oag_hd::names::FRONT_END_PLUGIN_DEFINITION`](../../crates/hd/src/lib.rs))
disagrees the same way: `DATA02`'s copy numbers the circuits the way the base
game did, `DATA06`'s the way the Fury-era front end does, and blind precedence
landed on the wrong one for eight circuit names until `oag_game::boot::load_circuit_names`
was written to pick the copy that agrees with a corroborating source instead
of the one mounted first. `DATA02` is the base game's copy there too, by the
same reasoning that page states outright: "presumably the base game's, from
before Fury added four." Three independent files now show `DATA02` as the
emptier, pre-Fury copy of something `DATA00`, `DATA03` or `DATA06` also ships,
fuller - which is corroborating, not proof: nobody has watched a PS3 read any
of them, so "which copy the original loads" per root stays exactly as open
as it was, sharpened rather than settled. `elimination_hud.xml` above is the
counter-example worth keeping in view before generalising this further: its
two copies are not "one fuller than the other," they are two different
designs, and "prefer the richer copy" would be the wrong rule to encode
outright even here.

### The seven archives are content-release layers, not two eras

2026-08-27. Every prior pass framed the open question as a single dichotomy -
"is `DATA02` the pre-Fury copy of this one file." Listing and diffing all
seven archives' full path sets disc-wide (`scripts/psarc.py list` per
archive, no per-file guessing) shows the split is finer than that, and gives
the `DATA02`-is-older reading independent corroboration beyond the three
files already cited above.

**The team roster alone settles part of it.** `/data/ships/*` splits cleanly:
`DATA02` carries exactly Wipeout HD's original eight teams (`ag_systems`,
`assegai`, `egx`, `feisar`, `goteki`, `piranha`, `qirex`, `triakis`) plus
non-team entries (`drone`, `flame_test`, `ship_damage`, `zone`); `DATA03`
carries exactly Fury's four added teams (`auricom`, `harimau`, `icaras`,
`mirage`) and nothing else; `DATA06` carries all twelve, each with `_c1`/`_n1`
variant suffixes (an extra class per team `DATA02`/`DATA03` do not have).
`DATA00`'s only `/data/ships/` entries are `detonator` and `zone` - it is not
a team archive at all. This is a clean, disc-verified split matching Wipeout
HD/Fury's known release history (eight launch teams, four added by the Fury
expansion) and needs no interpretation: `DATA02` is demonstrably the pre-Fury
roster, `DATA03` is demonstrably Fury-added, independent of any HUD file.

**Per-track content sharpens rather than contradicts this.** All eight base
circuits' directory names appear in `DATA00`, `DATA02` and `DATA03` alike, so
tracks are not split base-vs-Fury by name - but their *contents* are:
`DATA02`'s `01_vineta_k/` carries the bulk per-track assets (`hd_textures/`,
`fe/preview.bik`, `audioconfig.xml`, the `.bnk`); `DATA03`'s carries only
`lmaps_dlc/` (extra lightmaps) and `materials_dlc/`, plus
`track_reversed.{envsettings,probes,pvs,pvspatch,rcsmodel,vex}` and
`start_grid_reversed.vex`/`stats_reversed.xml` - the reverse-direction
variant, a documented Fury feature; `DATA00`'s carries `aurora.xml`,
`padreplacement_reversed.{rcsmodel,vex}` and `fe/trackselectemblem_fury.gtf`.
Every one of `DATA00`'s and `DATA03`'s track-level files is additive
(reversed variants, Fury-branded art, lightmap/material overrides) layered on
top of `DATA02`'s base set, never a replacement of it. `DATA00` also holds
content no other archive does: the `aurora` circuit and `new_mode_pads`
(unread further this pass), both Fury additions by name, plus the three
`2097_hud`/`wo3_hud`/`duel_hud`/`splitscreenzone_hud` skin directories and a
fully redesigned `elimination_hud.xml` fragment set (`hud_elim_*`) - so
`DATA00` reads as *Fury's own scaffold-and-skins layer*, not a generic
"newest wins" archive.

**`DATA01`, `DATA04` and `DATA05` are a third kind of layer, not aligned to
either era.** `DATA01`'s `/data/sound/xfship_*.xfx` set already includes all
twelve teams' engine audio, Fury's four alongside the base eight in one
un-split archive - so ship audio was never partitioned base/Fury the way
ships and tracks were. `DATA04` is a localisation patch (Chinese/Korean
fonts, all fifteen `/data/plugins/languages/*/entries.xml`, three novice/
skilled trophy tiers) with no gameplay content at all. `DATA05` is a
front-end asset patch (rank badges, PlayStation Store banners, on/off toggle
icons) carrying its own narrow HUD fixes (`hud_lap_ghost.xml`,
`hud_positions.xml`, plus the `2097_hud`/`wo3_hud` skin folders again).
Neither maps to "base" or "Fury" - both read as later, narrower patch
archives on top of whichever of `DATA00`/`DATA02`/`DATA03` they touch.

**What this changes about the open question, and what it does not.**
`zone_hud.xml`'s `DATA02` copy (missing the "PERFECT" widget) is now
corroborated as the pre-Fury base by a *fourth* independent line of evidence
(team roster, track content, and the two prior HUD-file diffs), and `DATA03`'s
copy sits inside an archive independently confirmed - by its team roster
alone, with no HUD-specific reasoning at all - to be Fury-added content. That
raises confidence that the "PERFECT" widget is a real Fury-era addition and
`DATA02`'s precedence pick is the stale one, but it is still corroboration,
not proof: nobody has watched a PS3 resolve any of these paths, and the
seven-archive picture shows enough distinct content domains (base, Fury
scaffold, Fury new-teams-and-reverse, and three further unaligned patch
layers) that "prefer the later-numbered archive" is not a rule this evidence
licenses either - `DATA06`'s fuller roster and `DATA05`'s narrower one are
patches over different, specific gaps, not a strict chronological ladder.
Changing `Archives::holder_of`'s precedence on this evidence alone would
still be encoding a guess, per this page's own rule above. What it does give
a future fix is a second corroborating source, independent of the front-end
plugin definition's circuit numbering, for the specific claim that `DATA02`
is HD's pre-Fury layer - useful if `load_circuit_names`'s corroboration
approach is ever generalised to the HUD roots.

## What a race reads today

**Since 2026-08-18 an HD race reads HD's own root**, not Pulse's:
`oag_title::HudLayouts` is the axis (`arcade`, `time_trial`, `speed_lap`,
`zone`), each title package fills it in, and `oag_raceplay::hud_layout` maps
the mode. Before that every title was served `oag_pulse::hud::layouts` and HD
worked only because PSARC normalisation folds `Data\XML\Arcade_HUD.xml` onto
`/data/xml/arcade_hud.xml` - finding S2 of that day's review.

Two things changed measurably on the disc, both visible in the race's own load
report:

- **`load_hud` composes.** It read only the root before, which for HD meant
  *one fill, no sprites, no labels* - the empty rectangles this page's own
  introduction predicted. `just play hd --race` now reports
  `/data/xml/timetrial_hud.xml: 51 sprite(s), 1 fill(s), 43 label(s), composed
  from 14 file(s)` and decodes `HUD_Components.gtf` as its atlas. A Pulse or
  Pure layout includes nothing and composes to itself, so their reports are
  unchanged to the widget.
- **Speed lap is HD's own file.** Neither PSP disc ships a `SpeedLap_HUD.xml`
  (the name hashes to `1af0a646` and no entry carries it), so both draw the time
  trial's; HD ships one, and it was unreachable. `just play hd --race --mode
  speed_lap` now reports `/data/xml/speedlap_hud.xml: 50 sprite(s), 1 fill(s),
  40 label(s), composed from 15 file(s)`.

The default skin only, because which skin a race picks is still unread - see
below.

## The HUD draws, and each sprite out of its own texture

**Since 2026-08-25 an HD race draws its HUD**, checked against a frame of the
running original rather than against the layout alone. Three things were wrong
at once, and they were one mistake repeated: `oag_hud` held Pulse's answer
as a `const` and served it to every title. All three are now
[`HudArt`](../../crates/title/src/hud.rs) rows, which the title packages fill in.

| | Pulse / Pure | HD / Fury |
| --- | --- | --- |
| `texture_extension` | `None` - the layouts name shipped entries | `.gtf` |
| `always_on` | seven widget names | fifteen |
| `pickup_backdrop_colour` | `HudBGColour` | `None` - drawn as authored |

**A sprite is offset by its own texture's placement.** `Layout::atlas()` returned
*the* texture a layout samples - true of all nine Pulse and Pure layouts, still
pinned by `the_layouts_name_at_most_one_texture`, and false here. The sheet held
that one texture and every sprite took its origin, so on the arcade HUD the 45
sprites naming one of the other five sampled `HUD_Components.gtf` at coordinates
meant for a different image: the right rectangle out of the wrong picture, which
reads as art rather than as an error. The pickup icon came out as a grey box, in
the right place, at the right size. `Layout::textures()` now names all six, the
loader builds one sheet from them, and `sprite_draw` looks a sprite's own `src`
up in it - drawing **nothing** for a texture the sheet does not hold, rather than
falling back to `(0, 0)`.

`a_drawn_sprite_lands_inside_the_texture_its_own_layout_names` checks the
*composed* uv of all 1,029 sprites against its own texture's placement in the
sheet. The older check on the authored rectangle passes either way, which is
exactly how this hid.

**The extension rule reaches the loader.** `oag_hd::hud::texture_entry` was read
by the ground-truth test and by nothing on the load path, which read the declared
name only. `hdHUD.mip` and `detonator_hud2.tga` carry 192 sprites between them
and are not on the disc under those names.

**Fifteen widgets are drawn whenever the HUD is up**, listed with the frame each
was identified in on [`oag_hd::hud::ALWAYS_ON`](../../crates/hd/src/hud.rs). The
discriminator is *visible in a capture of the original*, so what a single frame
cannot separate - "always on" from "on in this state" - is left out and named
there rather than guessed at.

Two smaller ones, both the same shape of mistake:

- **The pickup backdrop is not retinted.** Pulse's is a filled white hexagon
  under a white icon and needs the substitution `oag_pulse::hud::PICKUP_BACKDROP_COLOUR`
  records; HD's is a hexagon **outline**, and Pulse's quarter-alpha black turned
  it into a smudge.
- **The speed value drops its unit when the layout spells the unit out.** HD
  authors `SpeedBarText`'s placeholder as `string="0 kmh"` - inherited XML - and
  then a `SpeedBarTextKMH` beside it at `idstring="RC_KMH"`, so taking the
  placeholder at its word put `0 kmh KM/H` on screen. Confidence 75; the frame
  crops the speed readout off its right edge, so the recovered text has not been
  read back off the original.

## Zone's ladder draws, and the widget it is missing is missing for a reason

**2026-08-31.** The column down the left of a Zone race - the current zone, the
ten coming up, and the class the current one runs at - draws now. It was not a
rendering bug but three separate gaps, none of which is visible without the
other two:

1. **The sprites were not in the always-on set.** `ZoneBG`, `CurrentZonePanel`
   and `ZonePlusLight0`-`ZonePlusLight10` were named in
   [`oag_hd::hud::ALWAYS_ON`](../../crates/hd/src/hud.rs)'s "deliberately left
   out" list, because the frame that set was read off is a **speed lap** and
   authors none of them. The maintainer supplied a Zone frame
   (a maintainer capture, not kept in the repository) and all thirteen are up
   in it. They cost the other seventeen layouts nothing: only the three Zone
   roots author any of them, asserted by
   `every_zone_ladder_widget_is_authored_by_the_zone_layouts_alone`.
2. **The labels were not in the text allow-list.** `ZonePlus0`-`ZonePlus10`
   carry no `idstring`, so `oag_hud::draw_list` dropped every one. They
   are `zone + N`: the widget name says it, both reference frames read the
   current zone and the ten above it, and **the authored placeholders are the
   zone-`0` frame spelled out** - `ZonePlus0` carries no `string` at all and
   `ZonePlus1`-`ZonePlus10` carry `1` through `10`, which is `zone + N` at
   `zone = 0` with the current row blank. Confidence 88.

   That last point is also what bounds the gate. `Zone` and `Score` are off
   until the first ten-second step, and extending that to the whole ladder left
   the column empty for the first ten seconds of every run; the disc's own
   placeholders say the rows ahead are numbered there and only the current one
   is not.
3. **`RotationTheta` was not modelled at all, and the shader sheared it when it
   was.** `ZoneBG` is a 676x153 bar authored at a quarter turn, which is what
   stands it up as the 153x676 column; the eleven ticks carry small per-row
   tilts. `oag_game::frontend::Draw::RotatedSprite` already existed for the
   lock-on reticle, and `ui.wgsl` turned the **unit square** before scaling by
   the rectangle - `scale . rotate`, a shear on anything that is not square.
   Every previous caller was 8 pixels square, so this had never shown. Now in
   pixels, which is `rotate . scale`.

`Layout::sprites` gained a `rotation` field for it and `sprite_draw` returns the
rotated variant when it is non-zero.

### The two retro skins carry fewer widgets, and that is the disc's

`wo3_hud` and `2097_hud` write `<Image name="CurrentZonePanel">` with **no
`<Values>` at all** - a bare grouping element holding the eleven rows - where the
default skin authors it as a 476x54 patch of `HUD_Components_01.gtf`; `2097_hud`
does the same to `ZoneBG`. So the retro ladders are ticks and numbers with no
column behind them and no highlighted current row. Nothing turns on it, since
this build draws the default skin only, and the counts (13 / 12 / 11) are
asserted so a silent rise would flag the parser reading a bare group as a sprite.

### The speed class, and the table that says which one

The disc names fifteen rungs three times over and they agree one for one:
`zonemode.effectsettings` keys its palettes `0 Start`, `1 Sub Venom` ..
`14 Supersonic`; the language plugin carries each as a HUD string
(`MSC_SVENOM` = `SUB-VENOM` .. `IG_HUD_SUPSON` = `SUPERSONIC`, with
`IG_HUD_MACH1` = `MACH 1` filling `13 Mach 1`); and the reference frame reads
`SUB-VENOM` with the string table's hyphen rather than the palette key's space,
so the HUD is drawing those strings. That table is
[`oag_hd::hud::ZONE_SPEED_CLASSES`](../../crates/hd/src/hud.rs), indexed by rung.

**Which zone is on which rung is now recovered too**, out of the executable:
`g_ZoneSpeedClassTable` at `0x00860d44`, fourteen records of
`{ u32 zoneThreshold, u32 stringIdPointer }` descending to zero, walked by
`Hud_UpdateZoneSpeedClass` (`0x00049718`). It is
[`oag_hd::race::ZONE_STAGES`](../../crates/hd/src/race.rs) and the evidence is
[zone-speed-class-table](../ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md).
Bands `0`-`1`, `2`, `3`-`4`, `5`-`6`, `7`-`11`, widening to fifteen at the top -
which is what the maintainer's play describes ("not every zone is a bump",
"zone 2 should already be Venom, kind of the exception") and what got a first
reading of one-rung-per-zone corrected.

**And the same function closed a different question.** Its last act is
`stw r3, 0x640(r29)` with `r3 = 14 - i`, which is the writer of the per-craft
Zone stage index that
[zone-effectsettings-loader](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)
recorded as unfound - so **HD's colour grade escalates now**, on the same table,
and the HUD's class name and the circuit's palette move together because they
are the same index. That is 2048's architecture: the HUD widget drives the grade.

### The next class rides the row its zone starts at

`NextSpeedClassBG` and its label are authored at `x="0" y="0"` inside
`CurrentZonePanel`'s group - on top of the current row, name to the left of that
row's own number - which is this dialect's runtime placeholder, the same idiom
the lock-on reticle uses at `(-960, 540)`. The reference frame shows the bar a
row lower and its name clear to the right of that row's digit.

**The rule is the maintainer's**: an upcoming class is named beside the zone
number it starts at. So the bar sits on that zone's ladder row and slides down
as the bands widen.

**A second capture, at zone 8, says exactly where.** It reads

```text
 8  SUB-RAPIER
 9
10
11
12  RAPIER
13
```

- `RAPIER` is **level with its own `12`** - one line, the same font and size as
  the digit beside it, not offset the way `SpeedClass` is offset from the much
  larger `ZonePlus0`. So the vertical answer is the row's own `y`.
- Its horizontal inset is the one `SpeedClass` already has from `ZonePlus0`, 66
  layout units, which measures at ~39 screen pixels on that capture against 42
  predicted.

`oag_hud::draw::zone_next_row` is those two sentences; the bar takes the
same row and the panel's own horizontal inset, keeping its authored size.
Everything in it is read out of the layout, and no constant here was chosen to
match a picture. **Confidence 80**, up from 72: the zone-1 frame put two
competing readings within six screen pixels of each other and could not separate
them, and a four-row gap does.

**And the same frame is a third confirmation of the threshold table** - zones
`7`-`11` on Sub Rapier with the bump at `12` - on a band five zones wide, which
is the case no one-rung-per-zone reading could ever produce.

## The per-lap history draws, off a new `RaceState`/`Standing` field

**2026-09-07.** `Lap1Image`-`Lap4Image`, the four rows in
`HUD_lap_times.xml`, draw now. This page used to record the whole group as
undrawable because there was nothing behind it to draw: `oag_race::RaceState`
and `oag_race::Standing` tracked a running `best_lap_ticks` only, no per-lap
history, so `Readout` had nothing to carry even once the HUD side read the
widgets. That prerequisite is `oag_race::RaceState::lap_splits` and
`oag_race::Standing::lap_splits` (`crates/race/src/state.rs`,
`crates/race/src/standing.rs`) - a `[Option<u32>; MAX_RECORDED_LAPS]`, not a
`Vec`, per [ADR-0003](../architecture/adr/0003-no-ecs.md).

**`MAX_RECORDED_LAPS` is four because the disc's own layout is**: every copy
of `HUD_lap_times.xml` checked (root, `2048_hud`, `2097_hud`, `wo3_hud` -
2048's `data.psarc` ships one identical to HD's, byte for byte) composes
exactly `Lap1Image` through `Lap4Image` and no fifth row, so the array is
sized to the one consumer that exists rather than to a guess. A lap beyond the
fourth stops being recorded, and a Phantom single race or time trial - both
five laps, per [`Mode::SINGLE_RACE_LAPS_BY_CLASS`] and
[`Mode::TIME_TRIAL_LAPS_BY_CLASS`] - are the cases that reach it: the fifth
lap is still counted and still ends the race, only its split time is dropped.
Widening the array would invent a fifth row no measured title authors, and would
change `World`'s size and so the committed state hash to store a number nothing
can display.

Each `<Image name="Lap{n}Image">` carries two `<Text>` children,
`Lap{n}Text` and `Lap{n}Time`, both authored `string=""` in every copy - so
neither half comes from the layout itself. `oag_hud::lap_splits` reads
the family the same way `zone_plus` reads `ZonePlusN`: one function per half
rather than eight match arms.

- **The row's own number, in `Lap{n}Text`.** Read off which named slot is
  populated, not off any authored digit - `Lap1Text` is definitionally lap 1's
  row. This is a structural fact about the naming, not a guess, but no
  reference frame has this cluster on screen to confirm the reading against;
  see below.
- **The row's time, in `Lap{n}Time`.** `format_lap_time(_, Precision::
  Hundredths)` - the same rule already measured for `BestTime`, reused rather
  than chosen fresh: both are a clock that has stopped, where `CurrentTime`'s
  `Precision::Tenths` is one still running.
- **The row draws nothing until its lap is recorded**, background included -
  `Lap{n}Image` is not in [`oag_hd::hud::ALWAYS_ON`], and is instead drawn
  conditionally by `oag_hud::lap_splits::lap_split_sprites`, gated on
  `RaceState::lap_splits[n - 1]` being `Some`. The alternative - drawing every
  row and letting the blank ones show an empty box - is the "no value" versus
  "not wired up" confusion this page's `text_for` allow-list already rejects
  everywhere else, so the row's own background follows the same rule as the
  text it holds.

**Confidence 70 on the two bulleted readings**, not the 90 the rest of this
page carries: everything about the fragment's shape (four rows, both halves
blank, the source rectangles, the offsets) is read straight off the disc, but
*what those two blank widgets are for* is inferred rather than checked against
a frame - see [What is not done](#what-is-not-done).

**2048 ships the identical fragment and does not reach it from a played
race.** Checked directly against `data/extracted/vita/PCSF00007/base/PSP2/
data.psarc`: the bare-root `TimeTrial_HUD.xml` loads `HUD_lap_times.xml`, but
that root is the one [`2048-hud.md`](2048-hud.md) already found is reached
only by `DemoRaceManager_Construct` - the attract-mode demo, not a race a
player starts. None of the seven played `2048_hud\` roots (`Arcade`,
`Elimination`, `Zone`, `MPTag`, `SpeedLap`, `SpeedLap_TimeTrial`, `Zombie`)
load it; `SpeedLap_TimeTrial_HUD.xml` - the file a real time trial reads - pulls
in `HUD_lap_counters.xml` and `HUD_target_time_total.xml` instead, neither of
which is this cluster. So the fragment is genuine 2048 data, but wiring it
there today would draw a widget group no played 2048 race ever loads; left
for whoever next holds `crates/2048`'s HUD reading, alongside filling in its
still-empty [`oag_2048::hud::ALWAYS_ON`].

**Pulse and Pure author no equivalent at all.** Every one of Pulse's five HUD
layouts (`Arcade`, `Elimination`, `TimeTrial`, `Zone`, `MPTag`) and Pure's
`TimeTrial_HUD.xml` were read directly off their discs (`oag-wad cat
--expand`) for a `Lap[0-9]` name and none carries one - not a gap in what this
build reads, the two PSP titles simply never authored a per-lap history
widget.

[`Mode::SINGLE_RACE_LAPS_BY_CLASS`]: ../../crates/race/src/mode.rs
[`Mode::TIME_TRIAL_LAPS_BY_CLASS`]: ../../crates/race/src/mode.rs
[`oag_hd::hud::ALWAYS_ON`]: ../../crates/hd/src/hud.rs

## Three matched-state frames replace the one this page was written from

**2026-09-13.** Every prior section above rests on one frame (speed lap,
stationary on the grid) plus a separate Zone frame. `data/reference/hd-capture/
talons-matched/{00,01,03}.png` are three new frames of the running original,
1280x720, all the same team (Feisar, `concept1` hull) and circuit (Talon's
Junction), all a Fury-campaign single race rather than speed lap: `00` at the
grid (full shield, lap 1/3, position 8/8), `01` mid-race at 529 km/h (heavy
motion blur, still lap 1/3, position 7/8), `03` mid-race and damaged (shield
98, position 7/8, a weapon pickup held). Three findings came directly out of
comparing them against this build's own render at the matching state.

### Two widgets drew that no frame ever shows, and one dropped a suffix no frame ever carries

`oag_hud::draw::text_for`'s fallback arm draws **any** widget carrying an
`idstring` as a static caption, with no gate - right for a genuine always-on
caption, wrong for two widgets this corpus never checked against a frame
before:

- **`PositionTxt2`.** `HUD_positions.xml` authors *two* widgets sharing
  `idstring="IG_HUD_POS"`: `PositionTxt` (gated on `place_shown` already) and
  `PositionTxt2`, buried inside the `PosTag0`-`7` head-to-head cluster whose
  sprites (`VoiceCom0`-`7`) are already excluded from every title's
  `ALWAYS_ON`. Nothing gated the *text* the same way, so every HD race drew a
  second `POS` label floating beside the real one. No `talons-matched` frame
  shows one.
- **`PickupDamageTxt`/`PickupAbsorbTxt`** (`HUD_pickups.xml`, idstring
  `MSC_DAMAGE`/`MSC_ABSORB`) - the `DAMAGE`/`Absorb` words flanking the shield
  hexagon. `00.png`/`01.png` (full shield, no recent hit) show neither;
  `03.png` (after a hit) shows both, each with a real number beside it
  (`PickupDamage`/`PickupAbsorb`, both authored `string=""` and already
  correctly drawing nothing - they carry no `idstring` so the catch-all never
  reached them). Nothing in `oag_race::RaceState`/`Standing` tracks a recent
  hit's damage-taken or damage-absorbed total, the same prerequisite gap the
  per-lap history had before 2026-09-07, so both words are `None` outright now
  rather than a half-wired gate - fixed for `00`/`01`, still open for `03`'s
  populated state (see below).

Fixed by naming both cases explicitly in `text_for` rather than widening or
narrowing the catch-all, so no other idstring-bearing caption is affected.

**`ShieldBarText` carried Pulse's `%` on a title that never shows one.** All
three frames read a bare `100`/`100`/`98`, never `100%` - and Pulse's own
reference frame is the one that reads `100%` in the first place, so this was
Pulse's answer served to HD by default, the same shape this page's own
"headline" section already found for the atlas, the always-on set and the
pickup backdrop. `oag_title::HudArt::shield_percent: bool` is the new axis:
`true` for Pulse/Pure/2048 (Pure and 2048 unmeasured, kept at Pulse's prior
reading rather than guessed changed), `false` for HD. No widget authors a
`%`-suffix companion the way `SpeedBarTextKMH` does for `SpeedBarText`'s unit,
so this cannot be read off the layout - it has to be a title fact, checked in
`oag_hud::draw::text_for`'s `the_shield_readout_drops_the_percent_on_a_
title_that_measures_none` test.

### `ShieldBarText`'s colour is sharper now, and still not implemented

**Settled 2026-09-23 by the executable, and the candidate below was right:**
`0x1664FF`, opaque, written every tick. See
[the executable section](#the-executable-writes-the-shield-the-number-and-the-arcs);
what follows is the frame evidence that pointed there.

The ["what is not done"](#what-is-not-done) section below already recorded
`ShieldBarText` as "translucent red authored, critical-threshold override
unread." Sampling the actual rendered pixels of `00.png`'s `100` and `03.png`'s
`98` - the two least-compressed, least-motion-blurred digit fills across all
three frames - narrows that claim rather than closing it:

| Frame | Shield | Dominant fill (RGB) |
| --- | ---: | --- |
| `00.png` | 100% | `(47, 109, 233)` |
| `03.png` | 98% | `(42, 108, 231)` / `(40, 103, 228)` |

Both are a consistent, near-opaque **blue**, not the authored translucent red
(`0x94FF0000`, which alpha-blended over any plausible pill background would
read as pink/orange, never blue) and not grey either. **This rules out "red
whenever not full" outright** - 98% is not full and still draws blue - and
narrows "grey only above a critical threshold" to "this colour is drawn at
every shield level checked so far, full and near-full alike," which is a
different claim than the prior section's "one frame at full shield cannot
distinguish always-grey from critical-only" reasoning assumed.

The hue is in the same family as the disc's own "HD blue," authored twice
elsewhere in this exact composition: `HUD_pickups.xml`'s `PickupAbsorbTxt`/
`PickupAbsorb` (`Color="0x941664FF"`, next to the `98` in the same `03.png`
row) and `zone_hud.xml`'s alternate `DamageBar` state
(`color="0xFF1664FF"`). This is more than a hue-family resemblance, checked
with a same-frame control: an alpha-blend back-solve (observed colour, local
background sample, the widget's own authored alpha) on `DAMAGE`/`15`
(authored `0x94FF0000`) recovers `(188, ~0, ~0)` - pure red, confirming the
method - and the same back-solve on `Absorb` (authored `0x941664FF`, `(22,
100, 255)`) recovers `(0, 64, 188)`, undershooting R and G in the same
direction and by a comparable margin. `Absorb`'s raw sample and `98`'s raw
sample are both blue-dominant with the same channel ordering, in the same
frame, same row, same font, same authored alpha (`0x94`) - and `98`'s raw
sample sits *closer* to `0x1664FF` than `Absorb`'s own back-solve does,
consistent with `98` sitting over a brighter backdrop than `Absorb`'s flat
pill. Read together this is a specific, positive candidate - **the runtime
most likely substitutes the disc's own `0x1664FF` for `ShieldBarText`** -
not merely "somewhere in the blue family," though a clean isolate of `98`'s
own backdrop (the striped `DamageBarBg` pattern plus the hexagon's glass glow,
not a flat pill the way `PickupAbsorbTxt`'s is) was not possible from a
screenshot alone, so this stops short of an exact-value confirmation.

**Not implemented.** Recolouring `ShieldBarText` to `0x1664FF` on this
evidence would still be tuning a colour to match a screenshot - the
*substitution rule* (which widget, which state, why) is unread, and a
specific candidate is not the same thing as a confirmed one. Recorded
precisely (both frames' dominant samples, the same-frame control that
validates the method, and the specific candidate it points to) so the next
pass with the Ghidra bridge starts from "does the runtime substitute
`0x1664FF` for `ShieldBarText` outright" rather than from "what colour is
this."

### `PosBar0`-`7`'s lit count tracks place; `LapBar0`-`6`'s is confounded

**Corrected 2026-09-23.** Two readings here were wrong: `00.png` shows one
`PosBar` segment at 8th, not zero, and the segments are baked **yellow**, not
white - the probe sampled the atlas upside down. The executable settles both
counts. See
[the executable section](#the-executable-writes-the-shield-the-number-and-the-arcs).

Both rings' segments are `<Image>` widgets over `HUD_Components.gtf` with no
`color=` attribute in any copy (established earlier on this page) - so a lit
segment has always needed a runtime tint. What the three new frames add is the
first cross-check of *how many* segments light, decoupled from *what colour*:

- **`PosBar0`-`7`** (eight segments, `HUD_positions.xml`): `00.png` (place
  8/8, last) shows **zero** lit; `01.png` and `03.png` (place 7/8) both show
  **exactly one**, at the same screen position each time - the ring's
  bottom-most segment. Two data points at one field size (8) cannot separate
  `lit = ships - place` from any other monotone mapping, but they do
  establish that the count moves with **place**, not with a static property
  of the race (field size is 8 in both states).
- **`LapBar0`-`6`** (seven segments, `HUD_lap_counters.xml`): all three
  frames show the same **three** lit, and all three frames are lap 1 of 3
  laps total - so this data cannot tell "encodes total laps" (constant 3,
  unrelated to progress) from "encodes current lap" (constant 1, unrelated to
  total) from coincidence. A frame on a different lap, or a race with a
  different total, would settle it; recorded rather than guessed at.

`cargo run -p oag-game --example hd_hud_bar_pixels -- data/images/
hdfury-ps3-eu-dec.iso` decodes `HUD_Components.gtf` directly and averages the
RGB inside each `PosBar*` widget's own authored `U`/`V`/`TxtrWidth`/
`TxtrHeight` rectangle: every one comes back **pure white** `(255, 255, 255)`,
confirming there is no baked-in colour a "lit" segment could just be a
different sprite for - the tint is applied at runtime, to a segment that is
always drawn. This means the lit-count finding above is not yet actionable
either: adding `PosBar0`-`7` to `ALWAYS_ON` today draws eight white segments
over an already-white ring outline and changes no pixel, so it would be
landing untested logic rather than a visible fix. Left unimplemented, and
recorded as the sharpest lead yet for whoever next holds the Ghidra bridge -
sharper than the prior pass's "no frame shows the arcs lit at a known state"
because two states now do.

## The executable writes the shield, the number and the arcs

**2026-09-23, and it closes the three tints this page left open for a month.**
Every one of them is written by HD's per-tick HUD update, not authored, and
all three functions are now read -
[hud-readouts.md](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md) carries
the addresses, the constants and the branch table. `oag_title::HudArt::runtime`
holds the values (`oag_hd::hud::RUNTIME`) and `oag_hud::runtime` applies
them; `crates/hud/src/runtime/tests.rs` pins each rule and
`crates/game/tests/hd_hud_runtime_ground_truth.rs` checks the widgets it acts on
are where it looks on the disc's own layouts.

- **`DamageBar` is a fill, not a second state.** `Hud_UpdateShieldReadout`
  (`0x000866c8`) crops it from the top to the shield fraction with the bottom
  edge fixed - height, `y`, source `V` and source height all moved together -
  and paints it opaque `0x1664FF`, the "HD blue" this page's frame sampling
  had already singled out. The earlier "cropping it would be an invention"
  below was the right call without the executable and is wrong with it.
  Confidence 82.
- **`ShieldBarText` takes the fill's colour, opaque, and no `%`.** Same
  function, same colour word, written through `Text_SetColour`
  (`0x00200468`); the digits are `(int)pct` built by hand, up to three, with
  no suffix - so `HudArt::shield_percent: false` is now read as well as
  measured - and **truncated**, which this build now matches rather than
  rounding. The translucent red `HUD_damage_indicator.xml` authors is never
  seen.
- **`DamageBarBg` is forced white, and flashes red.** At or under 20 % shield,
  or through a one-second window after the whole percentage drops,
  `DamageBarBg` turns `0xFFFF0000` on every other eighth of a second, the
  number blinks fully transparent on the off phases, and the fill stays
  solid. The red only shows where the fill has drained, since the fill is
  drawn over it.
- **The colour is white in five modes.** A byte `Hud_BindWidgets` sets from
  the mode id picks white over blue in modes 8, 13, 14, 20 and 21 - Eliminator
  (single and multiplayer), Detonator and two unassigned ids. This build
  races Eliminator, where the readout draws white; its `DamageBar` is a
  different image from its `DamageBarBg`, so no fill is found and none drawn.
- **The arcs are shown and hidden, never tinted.** `Hud_UpdateLapCounter`
  (`0x00096ef0`) shows `LapBar_k` while `k >= laps - lap`, and
  `Hud_UpdatePositionCounter` (`0x00096088`) shows `PosBar_k` while
  `k <= 8 - place`, the `8` a literal. **The yellow is the atlas's own**:
  `HUD_Components.gtf` bakes the segments yellow (`cargo run -p oag-game
  --example hd_hud_bar_pixels` now reads `(255, 218, 0)` and
  `(255, 255, 100)`). The "baked pure white" reading above sampled the atlas
  upside down - a `.gtf`'s rows run bottom-up, which `crate::sprite`'s
  `decode_gtf` flips for the sheet and that probe did not - and averaged
  transparent texels, whose RGB is white.

**Checked against the running original.** The frame counts the section above
recorded were partly misread and now agree with the executable: `00.png`
(lap 1 of 3, 8th) shows `LapBar2`-`LapBar6` yellow and exactly one short
`PosBar0` segment - not zero - and `03.png` (7th) shows `PosBar0` and
`PosBar1` side by side along the bottom edge, which read as one. This build at
the same state (`just play hd --race --mode single_race --team feisar
--ticks 30`) draws the same five and the same one, a full blue hexagon and a
blue `100`.

**A second mode agrees, independently.** the zone-8 capture (not kept in the repository),
a Zone frame of the running original (Zone mode's id is not one of the five
white ones), shows both brackets **red above and blue below** with a blue `42`
over them - the fill cropped from the top to 42 %, drawn over a background on
a flash's red phase, and the number in the fill's colour. This build now draws
Zone the same way: `zone_hud.xml`'s `DamageBar` shares its `DamageBarBg`'s
source rectangle, so the same rule reaches it. (That the Zone HUD's update runs
with bit `0x02` of `hud+0x44` set is read off this frame, not off the
executable.)

**Checked against the running original at low shield, 2026-10-02.** The
rate, the threshold, the colours, the blinking number and the absorb row were
all compared with a Recompiler boot of the Fury campaign's Talon's Junction,
the shield poked through the GDB stub and the HUD recorded at 30 fps; the
numbers, the cells poked and the method are on
[hud-readouts.md](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md#what-the-running-original-does-at-low-shield).
Where this build differed it is fixed:

- **The post-hit window arms on a drop of the truncated whole percentage**,
  not on any drop: 60.9 to 60.2 % arms nothing and 60.2 to 59.9 % does (3 reps
  in each of three boots, read off the HUD's own `+0x110`). `Race::advance_shield_flash_whole`
  is the twin of Pulse's step with `fctiwz`'s test.
- **The phase a flash starts in is the original's accumulator** (`+0x1e8`,
  kept between flashes, wrapped past 1.0) and not the race clock:
  `Readout::shield_blink_phase_whole`. The cycle itself was already right, at
  3.91-4.03 cycles a second over six stretches in two boots.
- **The third flash condition is wired for its absorb half.** Within a second
  of an absorb the fill and the number blink together and the background stays
  white - row `c` of the table - from `Readout::shield_absorbing`, the same
  one-second window the hull overlay draws off. The craft-side LeachBeam flag
  (`ship+0x6958`) is not wired: which of two craft it marks is unread.

`oag-game --force-shield TICK:PERCENT` sets the player's shield for a headless
capture, so both sides can be looked at in the same state: this build at 20.0 %
flashes the red bracket and the number on and a pale bracket and an empty
plate off, four frames each at a 2-tick step, like the recording; at 20.5 %
after a drop from 100 it flashes for about a second and then reads a steady `20`;
`--force-shield 0:60 --give rocket --hold cross --press circle` absorbs and the
fill and the number blink over a white bracket.

**Still chosen, not read:** that a place of zero draws no place arc (the
original never meets one).

## What is not done

**Re-checked 2026-09-07, still true, and with a stronger negative result than
before.** `crates/game/examples/hd_hud_shield_census.rs` and
`hd_hud_tint_census.rs` walk every one of the eighteen composed layouts and
every raw fragment file each one reads (`cargo run -p oag-game --example
hd_hud_shield_census|hd_hud_tint_census -- data/images/hdfury-ps3-eu-dec.iso`),
through the same archive precedence a real load uses
(`Archives::holder_of`, above). Because `DamageBarBg`, `HUD_lap_counters.xml`
and `HUD_positions.xml` are exactly the kind of multi-copy path this page's
[precedence section](#diffing-the-three-copies-directly-three-different-relations-not-one)
warns can disagree, every fragment below was additionally pulled **archive by
archive** with `scripts/psarc.py cat` and diffed by hand, not just read through
the one copy precedence serves - so this is not "no constant happens to
match", it is "no copy on the disc authors a `color=` attribute on the
relevant widget", checked directly rather than inferred from resolved colours
alone.

- **The shield readout's second flash input, `ship+0x6958`**, is the
  LeachBeam manager's flag (`0x0013d7a8`), and which of the two craft it marks
  is unread, so it never fires here; the absorb half of the same condition is
  wired. See
  [hud-readouts.md](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md#what-c-is-the-absorb-window-and-a-leach-flag).
  Everything else the 2026-09-07 re-check above listed as needing the
  executable is read and drawn now; see
  [the section above](#the-executable-writes-the-shield-the-number-and-the-arcs).
  That re-check's layout census stays true - the layout authors none of it.

- **The per-lap history has no reference frame.** [The section
  above](#the-per-lap-history-draws-off-a-new-racestatestanding-field) draws
  `Lap1Image`-`Lap4Image` now, but the digit-and-time reading it draws from was
  never checked against a running race: no capture of the original shows more
  than one lap completed with this cluster on screen. Confidence 70 on the two
  inferences that section names, against 90 for the rest of this page - a
  frame with two or more laps down would settle both at once.
- **No skin selection.** Three skins ship; which one a race picks is a branch in
  a race-manager constructor and is unread.
- **The split-screen family**, above.
- **`SDOffsetX`/`SDOffsetY`**, and what the SD presentation is.
- **Label outlines.** 54 widgets carry a `BorderColor` and the renderer has no
  outline pass.
- **The rotation's sign is passed through, not verified.** Nothing recovered
  says which way the original turns a widget; `ZoneBG`'s quarter turn covers the
  same pixels either way and the ticks are too small to read off the frame.
- **The emulator check is one frame of one mode**, plus the Zone frame this
  page's Zone section is written from. `just rpcs3-race` on speed lap, Talon's
  Junction, craft stationary on the grid, no pickup held, no assist and no
  warning up. The always-on set outside Zone still rests on that single state,
  which is why it is deliberately short.

## See also

- [hd-status](hd-status.md) - the format layer this sits on top of
- [hd-frontend](hd-frontend.md) - `skin.xml`, the coordinate space, the fonts
- [ui/hud](../ui/hud.md) - Pulse's HUD, and the reader both titles share
- [fexml](fexml.md) - the XML dialect
- [psarc](psarc.md) - the container
- [gtf](gtf.md) - the textures, and the reading this page needed
- [race-hud](../ghidra/functions/ps3-hdfury-eu/race-hud.md) - `Hud_LoadDefinition`
- [hud-readouts](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md) - what
  the per-tick update writes over the shield hexagon and the two arcs
