# Wipeout HD / Fury: the in-race HUD

**Status: the layouts are read, confidence 90.** HD/Fury's HUD is the same
authored data Pulse's is - [front-end XML](fexml.md) carrying exact rectangles,
atlas sub-rectangles, named colours, font roles and alignment, plain here rather
than shortened - and it goes through the reader Pulse's HUD already had, with
two changes that the lineage forced. All **eighteen** shipped layouts compose
with **nothing missing and nothing skipped**, into 2,320 widgets.

Pinned by `crates/game/tests/hd_hud_ground_truth.rs`; the entry names are
[`oag_hd::hud`](../../crates/hd/src/hud.rs), the reader is
[`oag_game::hud::compose`](../../crates/game/src/hud/compose.rs). Measured
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

`oag_game::hud` read an `<Item>`'s offset as *absolute* until 2026-08-17, with
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
parked off-screen elements. `oag_game::hud::inside_screen` is still PSP-only and
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
chasing this rule through the executable
([the handover thread](../../handover/wipeout-hd-furys-hud-reads-and-the-reader.md))
found a hardcoded instance of the identical convention - a ship-thumbnail
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
own `Open` history and `oag_formats::fexml`), so a renamed tag is not a
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

## What a race reads today

**Since 2026-08-18 an HD race reads HD's own root**, not Pulse's:
`oag_title::HudLayouts` is the axis (`arcade`, `time_trial`, `speed_lap`,
`zone`), each title package fills it in, and `oag_game::race::hud_layout` maps
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
at once, and they were one mistake repeated: `oag_game::hud` held Pulse's answer
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

## What is not done

- **Two runtime tints are unrecovered**, and both are visible in the frame:
  - `DamageBarBg`'s striped hexagon is **saturated blue** on the original and
    pale blue-grey in the atlas, with no layout constant matching. Its sibling
    `DamageBar` carries the **same rect and the same source rectangle** and
    differs only in colour (red), which is this dialect's own "at most one of
    these is live" idiom - so it is a second *state* of the shield readout, not
    a bar over a background, and cropping it the way Pulse's `ShieldBar` is
    cropped would be an invention. Neither is drawn.
  - `LapBar0`-`LapBar6` and `PosBar0`-`PosBar7`, the progress arcs around the
    two panels, are white in the layout and yellow in the frame.
- **`ShieldBarText` shows `100%` where the original shows `100`**, in the red the
  layout authors where the original shows grey. Both halves are Pulse's answer
  applied here and neither has a data-side signal to key off: **both discs
  author the placeholder as `string="+0"`**, so the `%` is an engine-side
  measurement off a Pulse reference frame, and the red is HD's own authored
  colour with the runtime override unread. Recorded rather than fixed on a
  guess.
- **`Lap1Image`-`Lap4Image`**, the per-lap time rows, appear as laps are set -
  their labels are authored as empty strings - and nothing drives them.
- **No skin selection.** Three skins ship; which one a race picks is a branch in
  a race-manager constructor and is unread.
- **The split-screen family**, above.
- **`SDOffsetX`/`SDOffsetY`**, and what the SD presentation is.
- **Label outlines.** 54 widgets carry a `BorderColor` and the renderer has no
  outline pass.
- **The emulator check is one frame of one mode.** `just rpcs3-race` on speed
  lap, Talon's Junction, craft stationary on the grid, no pickup held, no assist
  and no warning up. Everything above rests on that single state, which is why
  the always-on set is deliberately short.

## See also

- [hd-status](hd-status.md) - the format layer this sits on top of
- [hd-frontend](hd-frontend.md) - `skin.xml`, the coordinate space, the fonts
- [ui/hud](../ui/hud.md) - Pulse's HUD, and the reader both titles share
- [fexml](fexml.md) - the XML dialect
- [psarc](psarc.md) - the container
- [gtf](gtf.md) - the textures, and the reading this page needed
- [race-hud](../ghidra/functions/ps3-hdfury-eu/race-hud.md) - `Hud_LoadDefinition`
