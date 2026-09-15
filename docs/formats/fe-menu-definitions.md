# Front-end menu definitions

**Status: read, and one screen's shape now feeds a page.** These are the
disc's own menu screens. This project draws
[its own menu tree](../architecture/menus.md) and takes only
[presentation](../ui/menus-original.md) from them, so most of this is read
but not built - reading them is what settled the presentation - **with one
exception**: `RecordGrid_Definition.xml`'s own per-class table is the shape
the RECORDS page draws from, see that section below.

They are [front-end XML](fexml.md), the `<code>`-shortened dialect, and every
trap on that page applies: the dictionary is per file, nothing is escaped, and
values live in attributes only.

## Where they are

`Data\Plugins\PI001\GUI\Skin.xml` - **in `Data.wad`, not in `FE.wad` or
`FEData.wad`** - carries a `LoadXML` list of **23** further files, each naming
both a full path and a `SrcRel` basename. Seventeen of them resolve in
`Data.wad`:

`StartProfile_Definition`, `BootScreensNoHeaderBar`, `ShowUnlocks_Definition`,
`MainMenu_Definition`, `RaceBox_Definition`, `CellMode_Definition`,
`Additional_Definition`, `Selection_Definition`, `Multiplayer_Definition`,
`RecordGrid_Definition`, `Stats_Definition`, `Network_Definition`,
`Online_Definition`, `Manual_Definition`, `Demo_Definition`,
`GriefReport_Definition`, `EndRace_Definition` (read in full - see
[endrace-screens.md](endrace-screens.md)), `InGame_Definition`.

Five do not, and where they live is unread: `Controls_Definition`,
`Credits_Definition`, `Debug_Screens`, `MemoryStickBootScreens`,
`MemoryStickScreens`.

**This said 22 and listed 15 until 2026-09-06**, when the list was re-derived
off the disc rather than trusted: `Skin.xml` hashes to `1f38aacf` = `Data.wad`
entry #1105, 12,389 bytes, and extracting `j="..."` off its `<e>`/`LoadXML`
elements gives 23. `GriefReport_Definition`, `EndRace_Definition` and
`InGame_Definition` were the three the count had lost. Nothing here is a
keyboard or a text-entry screen; text entry is a `TagInput` widget inside
`StartProfile_Definition.xml`, which is on the list.

## The main menu is a list, and `Grid Selection` is the hex grid

`MainMenu_Definition.xml` is 2,927 bytes and holds two screens. The first is
`Main Menu`: a title, a `Menu` of seven `Entry` elements, seven `helptext`
widgets, and a `Redirect` block mapping each entry to a destination screen.
Confirmed against a capture - see [the original's menus](../ui/menus-original.md).

This corrects a conflation.
[ppsspp-debugger](../reverse-engineering/ppsspp-debugger.md) says "Pulse's menus
are a hex grid whose state name stays `Cell Selection` across every cell". Both
halves are true of *a* screen and neither is true of the main menu: the hex grid
is `CellMode_Definition.xml`'s `Grid Selection`, reached from `RACE CAMPAIGN`
via `TournamentLoad`. **The main menu is a plain left-aligned vertical list.**

### The seven rows and their help text, read in full

**Confidence 95** - `Data\Plugins\PI012\entries.xml` (the USA disc's English
language plugin) read directly with `oag-wad cat ... --expand`, 2026-09-15.
`Main Menu`'s `<Menu>` names seven `<Entry idstring>`s in this order, each
paired with a `helptext`*N* widget - only the selected row's is drawn, per
[the original's menus](../ui/menus-original.md). The English text is the
`idstring`'s own `<Entry ID=... String=...>` in `entries.xml`:

| # | `idstring` | Row text | `helptext` id | Help text |
| --- | --- | --- | --- | --- |
| 1 | `FE_RACE_CAM` | RACE CAMPAIGN | `FE_HELP_RC` | The definitive WipEout® single player experience |
| 2 | `FE_RACEBOX` | RACEBOX | `FE_HELP_RB` | Customize your own races and campaign grids |
| 3 | `FE_MP` | MULTIPLAYER & SHARING | `FE_HELP_MP` | Race online or over Ad Hoc, and share data |
| 4 | `FE_WIPEOUT_DOT_COM` | WIPEOUT-GAME.COM | `FE_HELP_WOC` | Visit the official site for downloads and more |
| 5 | `FE_PROFILE` | PROFILE | `FE_HELP_PRO` | Stats, records and profile management |
| 6 | `FE_OPT_PLUS` | OPTIONS | `FE_HELP_OPT` | Change settings, controls, and the music playlist |
| 7 | `FE_EXTRAS` | Extras | `FE_HELP_EXT` | Software manual, staff credits and more |

Row 1 (`FE_RACE_CAM`) is the default cursor - confirmed independently by
`docs/ui/campaign-screens.md`'s own capture. This project's tree has no row
for `FE_MP`, `FE_WIPEOUT_DOT_COM` or `FE_EXTRAS` - there is nothing behind
any of the three yet - so `assets/ui/menu.toml`'s own `main` page keeps only
the two that map onto something this project builds: `RACE CAMPAIGN` first,
and the existing custom-race page renamed to `RACEBOX` second, matching both
the order and the naming above. See [the menus](../architecture/menus.md) for
why the tree past those two rows is still ours rather than a transcription.

This also settles this page's own prior "Open" item: the help text's content,
not only its layout, is now read. What draws it is
[the row subtitle](../architecture/menus.md#a-per-row-subtitle), which is
worded in this project's own English rather than lifted from the table
above - see that section for why.

## `RecordGrid_Definition.xml` is four record screens, one per mode

**Read 2026-09-09, for the RECORDS page - see
[the menus](../architecture/menus.md)'s own RECORDS section for what this
project builds from it.** `Data\Plugins\PI001\GUI\RecordGrid_Definition.xml`
is a `Screen type="FE_Default" name="Record Grid"` holding four child
screens, each carrying a `RecordsController` naming the task it shows:

| Screen | `RecordsController` task | Shape |
| --- | --- | --- |
| `Speed Lap Records` | `Speed Lap` | A `Track` list, no class picker - four rows, one per speed class (`Venom`/`Flash`/`Rapier`/`Phantom`), columns `PRO_TIME`/`PRO_TAG`/`ER_TEAM` plus a boost icon |
| `Time Trial Records` | `Time Trial` | `Track` and `Class` lists, six rows (`IG_HUD_1ST`..`IG_HUD_5TH`, `IG_HUD_TOTAL`), same three columns plus a perfect-lap icon per row |
| `Race Records` | `Race` | Identical shape to Time Trial Records |
| `Zone Records` | `Zone` | A `Track` list only, four rows (`IG_HUD_1ST`..`IG_HUD_4TH`), columns `PRO_POS`/`PRO_TAG`/`IG_HUD_ZONES`/`PRO_STATS_PZONE`/`IG_HUD_SCORE` |

Extracted with `oag-wad cat <image>:PSP_GAME/USRDIR/Data.wad
"Data\Plugins\PI001\GUI\RecordGrid_Definition.xml" --expand`, confirmed
against both `pulse-psp-usa.chd` and read for structure only (no bytes
reproduced here) - confidence 92: the screen names, the `RecordsController`
task strings and the column layout are read directly off the file, and the
only inference is what `PRO_TIME`/`PRO_TAG`/`ER_TEAM`/`PRO_POS`/
`PRO_STATS_PZONE` mean, which is not settled beyond what their names and
column position suggest.

**Two facts drive this project's own RECORDS page, and both are about what
the disc's shape assumes that this project's own persisted schema does not
have:**

- **Speed Lap's shape - one row per class under a single track picker - is
  the one whose data this project actually has**, since
  `oag_game::records::Record` is already one row per `(track, mode, class)`.
  Time Trial/Race's own richer shape - a *chosen* class and six ranked rows
  - implies either several attempts per key or a multi-entrant board; this
  project's schema keeps exactly one best lap and one best total per key, no
  ranking and no per-lap breakdown. So RECORDS draws Speed Lap's shape
  (track picker, one row per class) for every mode, picking whichever of
  `best_lap_ticks`/`best_total_ticks` fits the mode - see
  `crate::records_page`'s own module doc in `crates/game/src/main/
  records_page.rs`.
- **TAG, TEAM and the boost/perfect-lap icons are left off entirely**, since
  nothing in this project's persisted schema carries a pilot identity, a
  per-lap breakdown or a boost/perfect-lap flag - `oag_game::records::Record`
  and its own module doc say so directly. Drawing invented values in those
  columns would be exactly the "plausible-looking stand-in" `CLAUDE.md`'s own
  rule forbids; the honest choice is the column not appearing at all.

`Stats_Definition.xml` was extracted the same way and is a `screen type=
"FE_Default" name="stats holder"` with a `PRO_STATS` title - not read past
that title, since nothing in this project's schema yet has anything to put
on a stats screen (`oag_game::records::Record` carries no aggregate count a
"stats" page would show, career or otherwise). Left for whoever grows that
schema next.

## `Top FE Screen->FE Screen` is the one exception

Every menu screen's frame - light angled top bar, two footer strips - is
authored on `Skin.xml`'s own `Top FE Screen->FE Screen`, not in a `LoadXML`
include, so it needed no following to reach. Unlike the rest of this page it
**is** built: see [the original's menus](../ui/menus-original.md)'s own
section on it.

## Elements this reading relied on

Not an exhaustive schema - only what was needed, and named because
[fexml](fexml.md) lists "how layout and anchoring are expressed" as an open
question.

| Element | What it carries |
| --- | --- |
| `Menu` | a row list: `x`, `y`, `gap`, `align`, `font`, `color`, `focus`, `GSDisableEntriesBitField` |
| `Entry` | one row, by `idstring` |
| `LeftLayer` | a group with a `transition`, and sometimes an `OffsetX`/`OffsetY` of its own - an **origin** for its children, settled below |
| `Item` | a bare positioned group: `OffsetX`/`OffsetY` and nothing else. `Selection_Definition.xml`'s info panel is one per stat row |
| `Image` with `Color1`..`Color4` and no `src` | a gradient fill, the four being its corner colours (`1`/`2` left, `3`/`4` right); every rule on the selection screens is two mirrored halves of one |
| `Redirect` | `item` / `equals` / `goto`, plus a `Default` |
| `Watch` | a screen reacting to another screen's value |
| `Dialog` | `Icon`, `TextID`, `NumOptions` |
| `AreaController` | names an `area` |

`Text` widgets carry `RealGlow`, which is undecoded. So is `CalcBlur` on the
[HUD](../ui/hud.md).

**`pulse`/`delay`, censused 2026-09-05 across all 17 GUI files, carry no
oscillation parameters.** `delay` is a reveal-offset timer - it appears on
`<Animation>`, `<TextInfo>` and `<Text>` alike (`Skin.xml`'s footer strips and
news ticker, values `0` to `1.6`), the same role `transition`/`enabletransition`
play for a page change, not a period. `pulse="true"` is a bare boolean, with no
numeric value anywhere it appears - `BOOT_PRESS_START` and equivalent blinking
prompts in `CellMode_Definition`, `Selection_Definition`, `Network_Definition`
and `Demo_Definition`, but **never on `MainMenu_Definition.xml`'s own
`Menu`/`Entry` elements**. So the selected row's own highlight pulse (see
[menus-original.md](../ui/menus-original.md)) is not sourced from this flag at
all - its period and depth were read off the running original instead, the
same way `BOOT_PRESS_START`'s own blink still needs to be.

## `transition` and `enabletransition`

Counted across the files read. Both are **seconds** - confirmed for `transition`
by timing a page change against a 30 Hz front end, not assumed.

| Value | `transition` | `enabletransition` |
| --- | --- | --- |
| 0 | 17 | 11 |
| 0.2 | 1 | - |
| 0.4 | - | 2 |
| 0.5 | 47 | 6 |
| 0.7 | 7 | 24 |
| 1 | - | 17 |

`enabletransition` is undecoded. What `transition` *does* is on
[the original's menus](../ui/menus-original.md): a zoom and crossfade, not a
slide.

## `FEGlobals`

`Skin.xml` declares roughly forty. The layout ones:

| Name | Pulse | Pure |
| --- | --- | --- |
| `MenuXOffset` | 50 | 21 |
| `MenuScale` | 1.0 | 1.15 |
| `TitleXOffset` | 50 | 21 |
| `TitleYOffset` | 0 | 20 |
| `TitleScale` | 1.0 | 0.97 |
| `MSWarningScale` | 1.0 | 0.8 |

And a palette Pure declares none of: `TextColor` `0xFF33A6B9`, `TitleColor`
`0xFF000000`, `DesignColor` `0xFF5FDBF6`, `DesignColor2` `0xFF99D9E8`,
`MenuHighLightArrowColor` `0xFFED4796`, `MenuScrollArrowColor` `0xFF37CAEB`,
`InternalLineColor` `0xFF0FC3FF`, `FrameLineColor` `0xFE99C9D8`,
`CrosshatchColor` `0xFFD9E3E4`, plus medal, tab, stat and record colours. These
are Pulse's own values - **Pure's copies are not the same constants**, where
measured: `TitleColor` is `0xFFED4896` on Pure against black here, so nothing
in this table should be borrowed as a Pure default without checking.
Pure authors its own four in the *style* skin its plugin definition activates
beside the UI one (`Data\Skins\Default\Skin.xml`): `TitleColor` `0xFFED4796`,
`DesignColor` `0xFF5FDBF6`, `TextColor` `0xFF11ACD0`, `FrameLineColor`
`0xFE99C9D8`. Those were pixel-sampled stand-ins in
`crates/pure/src/frontend.rs`'s `FALLBACK_GLOBALS` until 2026-09-10, and three
of the four samples were wrong - see [race-setup.md](race-setup.md). **Zero
declarations in a title's front-end root does not mean undeclared on the disc
when that title is skinnable.**

The mechanism that reads these is
[fe-globals](../ghidra/functions/ps2-pulse-eu/fe-globals.md), including the trap
that an `FEGlobals->` field holds a hash in the slot an int or float otherwise
occupies.

## Pure differs in kind, not only in value

| | Pulse | Pure |
| --- | --- | --- |
| `Skin.xml` dialect | `<code>`-shortened | plain `<?xml` |
| menu definitions | `MainMenu_`, `RaceBox_`, `CellMode_`, ... | `Teaser_`, `Options_`, `Selection_`, `Multiplayer_` |
| `MainMenu_Definition.xml` | present | absent |
| `LeftLayer` | 17 files | none at all |
| menu `font` roles | `menu` | `Title`, `Stats`, `scroll` |
| `gap` values | 0, 10, 15 | 0, 25 |

That Pure's `Skin.xml` is plain rather than shortened matches
[fexml](fexml.md)'s finding that not one of Pure's 291 XML entries begins
`<code>`: **shortening is Pulse-era only**, and it is per file rather than per
platform.

## Open

- The five files that do not resolve in `Data.wad`.
- `RealGlow`, `enabletransition`, `GSDisableEntriesBitField`.
- ~~Whether `LeftLayer`'s `OffsetX` shifts an origin or a travel.~~ **An
  origin, settled 2026-09-09**: `Track Creation`'s `<LeftLayer OffsetX="290"
  OffsetY="25">` holds an info panel whose own `x` is absent, and the panel
  sits at x=290 on the capture - see
  [selection-screens.md](../ui/selection-screens.md). Nested `Item`s add
  theirs on top, and `oag_ui::screen::Screens` sums them.
- What `gap` *is* for, given it demonstrably does not set the row pitch.
- Pure's menu definitions, none of which have been read for geometry.
