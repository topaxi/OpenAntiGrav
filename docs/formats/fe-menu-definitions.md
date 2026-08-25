# Front-end menu definitions

**Status: read, not implemented.** These are the disc's own menu screens. This
project draws [its own menu tree](../architecture/menus.md) and takes only
[presentation](../ui/menus-original.md) from them, so nothing here is built -
but reading them is what settled the presentation, and
[the menus](../architecture/menus.md) already said this page was where the
answer belonged.

They are [front-end XML](fexml.md), the `<code>`-shortened dialect, and every
trap on that page applies: the dictionary is per file, nothing is escaped, and
values live in attributes only.

## Where they are

`Data\Plugins\PI001\GUI\Skin.xml` - **in `Data.wad`, not in `FE.wad` or
`FEData.wad`** - carries a `LoadXML` list of 22 further files, each naming both
a full path and a `SrcRel` basename. Seventeen of them resolve in `Data.wad`:

`StartProfile_Definition`, `BootScreensNoHeaderBar`, `ShowUnlocks_Definition`,
`MainMenu_Definition`, `RaceBox_Definition`, `CellMode_Definition`,
`Additional_Definition`, `Selection_Definition`, `Multiplayer_Definition`,
`RecordGrid_Definition`, `Stats_Definition`, `Network_Definition`,
`Online_Definition`, `Manual_Definition`, `Demo_Definition`.

Five do not, and where they live is unread: `Controls_Definition`,
`Credits_Definition`, `Debug_Screens`, `MemoryStickBootScreens`,
`MemoryStickScreens`.

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
| `LeftLayer` | a group with a `transition`, and sometimes an `OffsetX` of its own |
| `Redirect` | `item` / `equals` / `goto`, plus a `Default` |
| `Watch` | a screen reacting to another screen's value |
| `Dialog` | `Icon`, `TextID`, `NumOptions` |
| `AreaController` | names an `area` |

`Text` widgets carry `RealGlow`, which is undecoded. So are `CalcBlur` on the
[HUD](../ui/hud.md) and `pulse`/`delay` on `BOOT_PRESS_START`.

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
`crates/pure/src/frontend.rs`'s `FALLBACK_GLOBALS` carries Pure's own measured
stand-ins for `TitleColor`, `DesignColor`, `TextColor` and `FrameLineColor` -
each sampled off a real PPSSPP capture, not derived from this table.

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
- Whether `LeftLayer`'s `OffsetX` shifts an origin or a travel. It does not
  arise on `Main Menu`, whose layer carries neither.
- What `gap` *is* for, given it demonstrably does not set the row pitch.
- Pure's menu definitions, none of which have been read for geometry.
