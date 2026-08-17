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

90 rather than higher because **nothing here has reached a shader and no
executable path has been read**: every number is static reading plus exact
agreement with shipped data, and the two rules recovered below ([offset
composition](#the-rule-offsetxoffsety-translate-xy-place) and the [texture
extension](#the-layouts-name-source-art)) are inferred from the data agreeing
with them rather than from code that implements them. See the [confidence
rubric](../reverse-engineering/confidence-rubric.md). The textures themselves
**do** decode - [gtf](gtf.md), landed the same day - and every sprite's source
rectangle is checked against them, which is what a score of 90 rests on rather
than the layout arithmetic alone.

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

**Confidence 85** on the rule: twelve of twelve resolve and the two exceptions
carry 192 sprites, so it is not a coincidence of one file - but no code path in
the executable has been read for it, and an engine that tries the literal name,
fails, and falls back would be indistinguishable from here. A second spelling
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

## What is not done

- **The pixels exist and nothing draws them.** [`.gtf` is read](gtf.md) as of
  2026-08-17, so all twelve of the HUD's textures decode and every one of the
  **1,029 sprites' source rectangles has been checked against the dimensions of
  the texture it names** - a check that tests both readings at once, and one that
  turned up the disc's own single overhang (`VoiceCom0`-`VoiceCom7` sampling a
  62x64 patch from `V="1"` of a 64x64 image, 32 of 56 copies). Per `CLAUDE.md`'s
  "never invent what the assets already author", no stand-in atlas exists and
  none should.
- **One atlas per layout, and HD has six.** This is the blocker on drawing an HD
  HUD, and it is in `oag_game::hud` rather than in the renderer.
  `Layout::atlas()` returns *the* texture a layout samples, and
  `the_layouts_name_at_most_one_texture` measured that as true of all nine Pulse
  and Pure layouts. HD names **twelve across eighteen layouts, up to six in
  one**, so `Assets::atlas_origin`, `sprite_draw`'s single origin and `Overlay`
  all inherit an assumption that does not hold. A sprite has to carry which
  texture it samples through to the draw call. `oag_render` also has no `.gtf`
  upload path, but that is the second problem, not the first.
- **No draw list.** `oag_game::hud::draw_list` selects widgets by Pulse's own
  names (`SpeedBar`, `Lap`, `PosTag0`). HD's names differ and nothing has
  checked which are live when. The geometry is read; what drives it is not.
- **No skin selection.** Three skins ship; which one a race picks is a branch in
  a race-manager constructor and is unread.
- **The split-screen family**, above.
- **`SDOffsetX`/`SDOffsetY`**, and what the SD presentation is.
- **No emulator check.** Nothing on this page has been compared against the
  running original, which is the ceiling on every score here.

## See also

- [hd-status](hd-status.md) - the format layer this sits on top of
- [hd-frontend](hd-frontend.md) - `skin.xml`, the coordinate space, the fonts
- [ui/hud](../ui/hud.md) - Pulse's HUD, and the reader both titles share
- [fexml](fexml.md) - the XML dialect
- [psarc](psarc.md) - the container
- [gtf](gtf.md) - the textures, and the reading this page needed
- [race-hud](../ghidra/functions/ps3-hdfury-eu/race-hud.md) - `Hud_LoadDefinition`
