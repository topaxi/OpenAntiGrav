# Wipeout HD / Fury: the in-race HUD

**Status: the layouts are read, confidence 90.** HD/Fury's HUD is the same
authored data Pulse's is - [front-end XML](fexml.md) carrying exact rectangles,
atlas sub-rectangles, named colours, font roles and alignment, plain here rather
than shortened - and it goes through the reader Pulse's HUD already had, with
two changes that the lineage forced. All **eighteen** shipped layouts compose
with **nothing missing and nothing skipped**, into 2,320 widgets.

Pinned by `crates/raceplay/tests/hd_hud_ground_truth.rs`; the entry names are
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
    -E 'test(/^hd_hud_ground_truth::/)'
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
deliberately not in `ROOTS`, which is the single-screen set. A two-view race reads
`splitscreen_hud_{0,1}.xml` and `splitscreen_elimination_hud_{0,1}.xml` as
`oag_hd::hud::SPLIT`: whole-screen 1920x1080 layouts, the second the first moved
down 540, see [split-screen.md](../rendering/split-screen.md#hud). **`splitscreenzone_hud/` is not wired, and is probably dead art (2026-10-16, confidence
75).** The HD EU executable (`/hdfury/EBOOT-ps3-hdfury-eu.elf`) names every other split path
as a literal (`Data\XML\SplitScreen_hud\...` and its `Vert_` twins, three skins, the four
`Duel_HUD\Duel_SplitScreen_HUD\*` files at `0x0077dc20`-`0x0077dcd8`) and has **no string
containing `splitscreenzone`** (case-insensitive search, and no `Zone`/`Split` pair at all);
the only Zone HUD paths are the three `Zone_HUD.xml` skins, loaded by `SPZone_RaceManager`
(`0x008a724c`). The race managers it names are MPArcade, MPElimination, MPNitro,
MPTimeTrial, MPTournament and the SP set: **there is no MPZone**, so split-screen plain
Zone is not a mode HD has. The files themselves are a Zone HUD (`Zone Txt`, `ZoneDamage.gtf`,
`zone_hud.xml` + `zone_hud1.xml` = player 0 and 1, plus 2097/wo3 skins) with nothing
Zone-Battle-specific; Zone Battle is the Nitro manager, whose HUD literals are
`Data\XML\MPTag_HUD.xml` (4 sites near `0x0077b408`). 75 not higher: a path assembled at run
time is not excluded (the only format strings are `%s\screen_zone.xml` and
`InGame->Redirect SplitScreen %i`). Unwired; the port has no Zone Battle `Mode` either.
Omega and 2048 ship the same directory (16 files each): **checked, not checkable** without
their executables' path tables here.
The one to know
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

## HD's speed bar fill is a `<CurveBar>`, and it draws

**2026-10-16.** A maintainer playing HD saw the speed bar's frame and its km/h
label and no fill. The fill was authored all along: `HUD_speed_bars.xml` (in the
arcade, time trial and speed lap layouts) has `SpeedBarBG`, `ThrustBarBG` and
`SpeedBarOverlay` as `<Image>`s and **`SpeedBar` and `ThrustBar` as
`<CurveBar>`**, an element `Layout::collect` did not know and walked straight
past, so neither reached `sprites` or the loader report's `skipped`. Nothing
else was wrong: `SpeedBar` was never on `oag_hd::hud::ALWAYS_ON` because no
sprite of that name existed to list.

What a `<CurveBar>` carries, off the disc's own XML:

| Attribute | `SpeedBar` | `ThrustBar` |
| --- | --- | --- |
| `x`, `y`, `width`, `height`, `U`, `V`, `TxtrWidth`, `TxtrHeight` | the same as `SpeedBarBG`'s | the same as `ThrustBarBG`'s |
| `maxWidth` | 452 (= `width`) | 393 |
| `CurveStartPos` | 290 | 284 |
| `color1`, `color2` / `color3`, `color4` | `0xFFFFFF0C` yellow / `0xFFFF7F0C` orange | `0xFFB7D1FF` light blue / `0xFF072476` dark blue |

- **Parsed as a sprite with a gradient.** `oag_hud::Sprite::gradient` holds the
  left and right edge colours (`color1`/`color2` the left, `color3`/`color4` the
  right, each pair meaned - the reading `Fill::draw` gives the same four
  attributes) and `oag_ui::frontend::Draw::GradientSprite`, drawn by
  `ui.wesl`'s mode 6, grades the sprite's own texels across the quad. The fill
  is then the existing `bar_fraction`/`crop_horizontally` path, with the grade
  kept on the whole bar so a part-full bar ends part way along it.
- **Frames of the running original agree** (`data/reference/hd-capture/talons-matched/`,
  1280x720, Feisar `concept1` on Talon's Junction). At 529 km/h (`01.png`) a yellow
  strip fills the bar's lower edge to about 82-88% of its length, beneath the
  light-blue thrust bar and its `91%` label; at 74 km/h (`00.png`) a short yellow
  stub of about 9-12%. The thrust bar at 98% is **dark blue toward its tip**, so
  the grade spans the whole `maxWidth` and a crop shows the part up to the value
  rather than squeezing the whole grade into the cropped width. Those two
  readings fit a full scale between 600 and 700 km/h within the frames' motion blur; the executable's own 1/700 (`Hud_UpdateSpeedReadout`) is the figure used for HD and Omega, as per-title data (2026-10-16, `hd-thrust-bar`: the yellow ends at 82% of the width at 529 km/h against 76% predicted by 700 and 88% by 600, and at 6% at 74 km/h against 11% and 12%).
- **Read off the executable, frames agree loosely:** the full-scale speed
  (`oag_title::HudArt::speed_full_kmh`, 700 for HD and Omega; Pulse, Pure and 2048 keep 600). **Chosen, not measured:** that the fill is a straight horizontal
  crop of the art up to `CurveStartPos` and past it (the frames cannot tell a
  crop from a path-length fill where the strip rises), and the angle of the fill's
  cut end. `CurveStartPos` is parsed by nothing: it is not read.
- **`ThrustBar` draws, cropped to the craft's throttle (2026-10-16, `hd-thrust-bar`).** It is the
  light-to-dark-blue bar above the speed strip with the `ThrustBarText` percentage. The
  executable crops it to `record[+8] * 0.01` of its width (`maxWidth` 393 = the sprite's own
  width, so the plain horizontal crop is right) and prints `(int)record[+8]` and `%`, with no
  chase of its own; `record[+8]` is `craft+0x30c * 100`, the throttle after Pilot Assist's scale -
  99 or 92 on a Venom with the assist on, 0 with no throttle. See
  [the evidence page](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md#hud_updatespeedreadout-0x00086e88-and-what-thrustbar-shows).
  `oag_hud::Readout::thrust_percent` carries `ship.physics.thrust` (which this engine already
  scales by `PilotAssist_ThrustScale`), `ThrustBar` is on `oag_hd`'s and `oag_omega`'s
  `ALWAYS_ON`, and `ThrustBarText` is the `"{}%"` of the truncated value. **Chosen, not
  measured:** the bar is clamped to full where the original is not, and the grade, as the speed
  bar's, is kept whole across the crop (the 98% frame is dark blue toward its tip).
  2048's `thrust_chase_percent` is the other dialect: 2048's `Hud_Update` chases, HD's does not.
  **Omega: ported**, the three race layouts author the same `ThrustBar`/`ThrustBarText` and the
  feed is title-agnostic; the Omega executable's own writer is not read (no PS4 capture path), so
  the value law is HD's by lineage. Drawn on single screen and both split cuts for HD and Omega
  (in the headless `--window-seats 2h,2v` run with `--hold cross` only seat 0 holds throttle; another seat read 0% while moving because seat 0 rammed it from behind at tick ~340 and shoved it along at its own speed - no wrong-slot bug, and under `--autopilot` every seat reads its throttle, 99.6 to 100).
- **`SpeedBarMark` is not HD's.** No HD layout authors it, so the slide question
  in [the HUD page](../ui/hud.md) is Pulse's alone and stays open.
- **Omega: ported.** Its `Data\XML\{Arcade,TimeTrial,SpeedLap}_HUD.xml` author the
  same pair, `SpeedBar` is on `oag_omega`'s `always_on`, and
  `omega_hud_ground_truth::the_speed_bar_fill_is_authored_in_the_three_race_layouts`
  pins it. A race at 376 km/h draws the fill.
- **2048: checked, differs.** The skin 2048 plays (`2048_hud`) authors its bars as
  plain `<Image>`s (`ThrustBar`, `SpeedBar0`-`4`); only the bare root skin, kept
  for the demo, carries HD's two `<CurveBar>`s, and nothing plays it. Pinned by
  `vita_2048_hud_ground_truth::only_the_unplayed_bare_root_authors_curve_bars`.
- **Pulse and Pure** author `SpeedBar` as an `<Image>`, unchanged.

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
`crates/hud/tests/hd_hud_runtime_ground_truth.rs` checks the widgets it acts on
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

## The lap hexagon: `CurrentTimeTxt` is the lap number

**2026-10-21, `hud-leftovers` (lap 1) and `hd-lap-hexagon` (later laps), confidence 85, watched live.**
`HUD_lap_times.xml` authors `<Text name="CurrentTimeTxt"><Values string="1" font="HUD" align="centre" x="66" y="195">`
over `TimeIcon`'s hexagon, beside `CurrentTime`. Every HD Time Trial compare pair shows that `1` in the
original and an empty hexagon in ours: the reader drew a label only for a widget with an `idstring` or a
computed arm, and this one has only a literal. Pulse authors an `idstring` caption there and is unchanged.

Read in the executable (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, `scripts/ps3-toc.py attrib 0x0077f880`): only
`Hud_BindWidgets` (`0x0009bb30`) names the widget, at TOC slot `0x008a7ac4`, which is the `hud+0x298` pointer
(`-0x1685` in that function). `Hud_UpdateTimeCluster` (`0x00094598`) writes it only in the lap-change branch, **as the
current lap number, one ASCII digit (`0x30 + lap`), for laps 2 to 5**; lap 1 is the authored `1` and a lap past 5 holds
the `5`. The branch runs only where the layout binds the lap rows and has no `BestTime` text, and not in the Arcade
clock layout (`hud+0xac`), which authors `string=""` here and shows `ClockIcon` and a `BEST` row instead. Full read and
the live values (`hud+0x294` = 2, 3 at the lines; the hexagon reading `1`, `2`, `3`):
[hud-readouts](../ghidra/functions/ps3-hdfury-eu/hud-readouts.md), "The lap-change branch". An earlier version of this
section read the branch's string as unread, and before that `record+0x34` as a held time; `record+0x34` is the finished
lap's time and goes to the lap row's time text, while the digit comes from `record+0xc`.

`oag_hud::draw::text_for` draws the authored literal on lap 1 and the lap number after, capped at the history's
four rows plus one. A layout whose literal is empty draws nothing, as before. The compare pairs are all lap 1 (and the
Arcade one lap 2 on a layout that authors no digit), so no pair's pixels moved; the check is the live frames, hexagon crops
of the original at laps 1, 2 and 3 against ours rendered from a pair's flags with `--hud-state 1,<lap>,...`, which agree to
the digit, its yellow and its size.

Omega: **not checkable here**, no Omega HUD XML is extracted on this tree and there is no PS4 emulator; it carries HD's
front-end plugin and HUD fragments, so the same `string="1"` and branch are expected, and the reader is shared. 2048's
`HUD_lap_times.xml` is byte-identical but no played 2048 mode loads it ("The per-lap history draws").

## The `100` box is `PickupFarBackground`'s texels, and the original draws it translucent

**2026-10-21, `hd-lap-hexagon`, open (confidence 70 on the identification, nothing wired).** The grey rounded box behind
the shield `100` on every HD pair is not part of `DamageBarBg` (that sprite, `HUD_Components.gtf` `U=770 V=186`, is the
two striped hexagon halves and nothing else). It is the slot at the top middle of **`PickupFarBackground`**
(`HUD_pickup_background.xml`, `U=146 V=380`, 878x116): an opaque grey texel (178, alpha 255) in a bar whose body is
alpha about 0.3. Modesto Heights pair, 1920x1080, after the pickup dim landed (the hexagon and pills now agree): the box
reads a flat 178 in ours and about 150 in the original, with the sky and the dark ring behind it showing through, so the
original draws it at roughly 0.8 over the background (a one-pair estimate with a guessed background, **chosen
reading, not a measured constant**). `Hud_BindWidgets` binds `PickupFarBackground` at `hud+0x5c0`; the only function that
reads that slot is `FUN_0008fbf0`, which shows or hides it with the five widgets around it (bit `0x4` of `+0x34`), and no
`Image_SetVertexColours` is called on it, so the translucency is not a per-tick colour write. The layout authors
`Vertices="10"` on it, an attribute ours does not read (`rg Vertices crates/hud` finds nothing); that a 10-vertex strip
carries per-vertex alpha is the open lead. The original's whole HUD also sits 20 px lower than ours in this frame, which is
the cockpit sway (`hd-cockpit-lean`'s), not this. Not wired: ours keeps the authored opaque texel.

## The whole HUD slides with the drawn lean (`hd-hud-sway`, 2026-10-21)

`Hud_Update` writes `(-15 * ship+0x6a58, 15 * ship+0x6a5c)` into the translation of the `"HUD"` screen
every tick, and every race layout *is* that screen (seventeen of the eighteen roots; `mptag_hud.xml`'s is
`MPTag_HUD`), so the bars, clocks, messages, pickup slot and reticle move as one. A held full stick
moved a 1280x720 frame by 13.3 px (the follower is the one the hull is drawn with), a half stick by
2.7 px, all regions alike. **Ported (x only)**: `oag_title::HudArt::sway`, applied at the end of
`oag_hud::draw_list`; HD measured, Omega and 2048 inherited, Pulse and Pure none. The y term
(`15 * ship+0x6a5c`, the far camera rig's spring lag, up to `+/-20` px on a rough stretch, the
"20 px lower" of the frame above) is read but drawn as zero: two of its inputs are unread. The
evidence, the writer and the measurements are in
[ps3-hdfury-eu/ship-lean.md](../ghidra/functions/ps3-hdfury-eu/ship-lean.md#the-hud-sway-read-and-measured-2026-10-21-hd-cockpit-lean-hd-hud-sway).
The same instance drew the whole HUD at about `0.966` of the frame about its centre (the widgets sat `18` px
nearer the middle than ours): that is HD's `Safe Area Setting`, see the next section.

## The whole HUD is drawn at the Safe Area Setting (`hd-hud-leftovers`, 2026-10-21)

**The option.** `additional_definition.xml` (DATA06's copy, the one the live Game Options screen is) authors
`<Slider name="Safe Area Setting" global="Safe Area Setting" save="true" default="90%">` with `minSlide="80"
maxSlide="100" incSlide="2"`, labelled `Screen Size` (`OPT_SCREEN_SIZE`). The disc's default is **90**.
**The save is at 96.** Restoring `hd-timetrial-talons-harimau-classic-start` and `hd-race-talons-venom-pack`
on a private RPCS3 and reading the option's global (the string `Safe Area Setting` is at `0x0077a680`; the
TOC is `0x008ad4d8`, read off the `.opd` entry of `0x00029f08`; the slot `-0x7924(r2)` is `0x008a5bb4`, a
pointer to a pointer to an object whose `+0x104` and `+0x108` hold `96.0` as big-endian floats, in both
saves) gives `96`. The maintainer's profile therefore ran at 96, and the correlation of the earlier
lane (`0.964`-`0.968`) is `96 / 100` to within its noise. Confidence of "the HUD is drawn at value / 100":
**70**, one value of one setting; the consumer is **unread** (poking `90.0` into both floats and running
Restart Race did not move the HUD, so either the HUD scale is fixed at race load from a copy elsewhere or the
global is not what it reads; a float-pair scan for `0.96`, `0.966` and `0.954` found nothing, `0.9` pairs are
too common to tell).

**Ours.** `oag_title::hud::SafeArea` on `HudArt::safe_area` (HD **measured** off the disc XML; Pulse, Pure,
2048 and Omega `None`), `[graphics] hud_safe_area` in the settings file (percent, `None` = the title's
default 90, held to the slider's range and step; no menu row, taken at race load), and one scale about the
centre of the layout's grid at the end of `oag_game::hud_overlay::Overlay::draw` (`safe_area_frame`).
**Default 90, the disc's**: an HD player's HUD is now 10% smaller than before, as the original's is on a fresh
profile. The comparison profile (`scripts/compare-environment.toml`, `hd-fury`) pins `hud_safe_area = 96`, the
save's value, so a pair draws ours at what the original ran with. Not applied to a split screen
(`draw_views`: the original's split layouts are not measured at a safe area).

**Result on the thirteen HD time-trial pairs plus `hd-race/talons-pack`** (1920x1080, the RPCS3 resume toast
masked, grey RMSE of the region against the original, before to after; whole-frame figure is the pair tool's
mean grey):

| pair | lap | record | clock | speed | frame (pair tool) |
| --- | --- | --- | --- | --- | --- |
| anulpha-pass | 72.9 to 30.9 | 72.1 to 48.2 | 63.2 to 30.1 | 63.6 to 27.2 | 23.49 to 19.54 |
| chenghou-project | 70.3 to 21.5 | 56.4 to 24.9 | 61.9 to 16.9 | 66.2 to 14.7 | 13.57 to 9.00 |
| talons | 62.9 to 18.4 | 70.1 to 29.6 | 57.6 to 19.6 | 52.3 to 11.3 | 15.97 to 12.19 |
| vineta-k | 68.0 to 43.2 | 83.7 to 66.7 | 64.4 to 17.4 | 59.3 to 14.7 | 23.17 to 19.65 |
| hd-race/talons-pack | 87.6 to 87.9 | 101.9 to 96.4 | 73.5 to 75.9 | 101.2 to 91.7 | 53.04 to 53.02 |

(Every pair's mean grey fell, 14 of 14; the full table is in the lane's report.) The pack race's HUD regions
barely move: the frames differ in scene and in what the HUD shows (the original is mid-race with a weapon and damage readout, ours at the grid), and its save holds 96 as well. One
caveat on the signed frame mean (ours minus the original's): it moved by more than `0.1` on three pairs
(`anulpha-pass` `-9.11` to `-9.31`, `sol-2` `-2.06` to `-2.28`, `vineta-k` `-2.06` to `-2.19`) because a
smaller HUD covers less of a bright scene; that is the scene, and the absolute grey difference fell on all of
them.

**Not the top plate.** The earlier claim that the top plate and the pickup hexagon's outline draw "opaque and
near-black" in ours, where the original's are light translucent grey, **is false on the current build**: a
crop of the `talons` pair at 2x shows both translucent grey with the scene through them on both sides, and
the visible gap was the scale and the position. The `ring.png` comparison it came from was a different
scene on an older build.

**Cross-title.** Omega: **checked, differs** - its patch front end (`data09.psarc`
`additional_definition.xml`) authors no `Safe Area Setting` slider, so `safe_area` is `None` and its HUD is
unscaled. 2048: **checked, differs** - its Vita packages carry no `additional_definition.xml` (a different
front end), `None`. Pulse and Pure: no option. The front end's own `0.904` scale
([campaign-screens](../ui/campaign-screens.md)) is **not** explained by this: a front end at the value / 100
rule would read 0.96 on the same save, so either the setting differed on that earlier day's profile (90
reads `0.904`) or the front end scales differently; open.

## Right- and centre-aligned text ends on the ink (`hd-sharp-hud`, 2026-10-22)

**Measured.** Template matching (grayscale gradient phase correlation, sub-pixel peak) of fixed crops of
`theirs.png` against `ours.png` on the 13 cold `hd/*` pairs (1920x1080, Resolution Scale 100), borders and
fixed labels only, never digits. Offset is ours minus theirs, in px, mean over the pairs:

| element | before (dx, dy) | after (dx, dy) | worst pair before | worst pair after |
| --- | --- | --- | --- | --- |
| lap ring | -0.06, +0.04 | +0.00, +0.02 | 0.28 | 0.26 |
| `LAP` label (right) | -2.84, -0.00 | +0.22, -0.01 | 2.93 | 0.25 |
| lap-count `3` box | -0.44, +0.25 | +0.06, +0.21 | 2.53 | 0.50 |
| RECORD box | +0.23, +0.04 | +0.21, +0.04 | 0.38 | 0.36 |
| `RECORD` label (right) | -3.60, -0.06 | -0.02, -0.04 | 4.03 | 0.37 |
| timer box | +0.07, -0.07 | +0.07, -0.07 | 0.23 | 0.22 |
| KM/H frame | -0.02, -0.03 | -0.02, -0.03 | 0.13 | 0.13 |
| `KM/H` label (left) | -0.03, -0.08 | -0.03, -0.08 | 0.22 | 0.22 |
| top hexagon | +0.10, +0.08 | +0.15, +0.09 | 0.54 | 0.36 |
| `100` in the hexagon (centred) | -1.68, +0.17 | +0.13, +0.14 | 1.76 | 0.36 |

(`RECORD` on `sebenco-climb`, and the `3` box on `vineta-k`, are failed matches on both sides of the change and
excluded from the means; the worst-pair columns still include the latter.) Every frame, bar, box and left-aligned
label was already within 0.5 px; the "few pixels off in a few labels" were the **right-aligned** and **centred**
strings only, 1.7 to 4 px left of the original's, identically on all 13 pairs.

**The law.** The same one Pulse's PSP list showed for right-aligned text (above): a string aligned right or
centre uses its **ink** width, the advances less the last glyph's trailing bearing (advance less cell width).
`oag_title::HudArt::ink_aligned_text` (HD `true`, measured; Omega `true`, inherited from HD's HUD engine and
font, unmeasured; Pulse and Pure keep the narrower rule on `raster`; 2048 `false`, unmeasured) drives
`Renderer::push_text`. No per-label constant. Whole-frame mean grey fell on 13 of 13 pairs by 0.08 to 0.13
(the HUD text only). Cross-title: Pulse **ported earlier** for right-aligned text, centred text there still
unmeasured; 2048 **checked, applies, not wired**: same engine, no capture of its text edge.

## The HUD digits' size and softness (`hd-hud-digits`, 2026-10-22)

**Measured, no code changed.** The 13 cold `hd/*` pairs (1920x1080, Resolution Scale 100, hull shake patched out), the
lap `1` (`Lap`, scale 1.0 x 0.96 safe area), the lap-count `3` (`Lap Outof`, authored scale 0.6, `Centre`/`Bottom`, colour
`0.39` grey) and the speed `0` (`SpeedBarText`, scale 1.0). The HUD is screen-fixed, so none of this depends on the
craft pose. Three measures, each on a fixed window around the glyph, ours against theirs:

- **Ink box**: the 50 % crossings of the glyph's own coverage (sub-pixel), needs a clean background.
- **Scale and shift fit**: ours resampled about the glyph centre over a grid of scale and shift, maximising the
  normalised correlation of the two gradient-magnitude images. Works on all 13 where the ink box saturates (a bright
  sky behind the white `0`).
- **Edge width**: the median over the middle rows of `1 / max slope` of the coverage normalised to 0..1. It hides
  contrast by construction, so it says nothing about how bright the glyph is (see the opacity below).

| digit | ink box w x h, theirs / ours (mean of 13) | fit scale, ours to theirs | shift of theirs vs ours | edge width, theirs / ours |
| --- | --- | --- | --- | --- |
| lap `1` | 12.44 x 56.16 / 12.41 x 56.26 | 1.000 | +0.02, +0.04 | 1.13 / 1.21 px |
| speed `0` | 16.6 x 20.5 / 16.5 x 20.4 (10 pairs with a clean background) | 0.999 (13 of 13) | +0.00, +0.08 | 1.33 / 1.30 px (10 pairs) |
| lap-count `3` | spans below | 1.018 (1.01 to 1.03) | -0.52, -0.42 (up and left) | 1.35 / 1.28 px |

The lap `1` and the speed `0` **match the original within 0.4 px on every pair**: the stem is 12.4 px wide in both,
the strokes' edges rise over the same 1.1 to 1.3 px, and no pair is off by more than 0.25 px in position. The
glyph size, the atlas filtering and the layout scale are right for both.

The lap-count `3` is **not** within 1 px on the width. Edge-to-edge spans of the 3 (50 % crossings along the top
bar, the bottom bar and the left stem, mean of 13, theirs / ours): top bar 23.1 / 21.7 px, bottom bar 22.2 / 21.1 px,
height 34.3 / 33.5 px, so ours is narrower by 1.1 to 1.35 px and shorter by 0.7 px on every pair (range -0.5 to
-1.8). The pixel rows show where: the bars' left ends begin about one pixel further left in the original (column 231
against 232 on the `talons` pair), with the bottom-right corner within 0.3 px, so it reads as a 0.5 px shift of the
whole glyph plus about 1.2 % size. The cause is **unfound**. Read and ruled out: the layout scale (0.6 x 0.96 =
0.576, cell 43x80, the same font and engine as the `1`, which matches exactly); a pen rounding (the `0` and `1` would
move too, and they do not); a missing outline (both sides draw the `HudBGColour` 25 % black halo, `125` against `128`
at the bar's end). Nothing wired: a nudge for one label is what the lane's rules forbid.

**What the player sees as "smaller and softer" is opacity, not size or filtering.** The original draws the lit digits
and labels at about **0.8 alpha**: the lap `1` solves to alpha 0.800 (0.791 to 0.803 on every pair), with
`theirs = 0.8 * ours + 0.2 * background` on each channel. Its core reads `(218, 218, 15)` against ours
`(254, 254, 0)`, and the 15 in blue is the blend, not a tint (the authored colour has no blue). The white speed `0`,
the `LAP`, `KM/H` and `RECORD` labels, the lap ring and the timer read 0.85 to 0.90 of ours at their brightest pixels
(0.95 on the thin `KM/H` frame line, where bilinear sampling lowers the peak anyway), consistent with the same 0.8 over
a mid-grey scene. The dark `3` (`100` against `103.7` over a translucent box) is the same law within its noise. The
authored colour of every one of these labels is alpha 1.0, so the factor is not in the layout. It is **not** the Safe
Area Setting (that scales position, above) and **not** wired: no source for it is found, and a 0.8 constant would be
a nudge. It is probably the same thing as "the original draws the `100` box at roughly 0.8" in the section above
(`hd-lap-hexagon`): per-vertex alpha on a 10-vertex strip, or a HUD-wide fade the RSX state carries. Open.

- **Omega**: checked, applies, not checkable. It runs HD's HUD engine and font (`ink_aligned_text` is inherited the
  same way), so the digit sizes are expected to hold and its opacity is expected to be the same; no PS4 capture path
  exists to measure either.
- **2048**: not checked, unmeasured (a different HUD dialect; no HD-style capture of its digits).

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
