# The screen-layout file

**Status: built (2026-11-01, `hd-loading-layout`).** One TOML format, `schemas/v1/screen-layout.tosd`
([ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md)), for the screens this
project draws from a file of its own. Decision: [ADR-0081](../architecture/adr/0081-screen-layouts-are-files-and-a-track-may-carry-its-loading-screen.md).
Code: [`oag_hud::screen_layout`](../../crates/hud/src/screen_layout.rs) (what every screen shares),
[`oag_hud::split_hud`](../../crates/hud/src/split_hud.rs) (`screen = "split-hud"`),
[`oag_hud::loading_layout`](../../crates/hud/src/loading_layout.rs) (`screen = "loading"`) and
[`oag_game::loading::layout`](../../crates/game/src/loading/layout.rs), which draws one.

## One file, one screen

```toml
screen = "loading"                 # first key, above [toml-schema]: "split-hud" or "loading"

[toml-schema]
location = "https://oag.topaxi.com/mods/schemas/v1/screen-layout.tosd"
version = "1.0.0"

[[layout]]                         # the first layout that answers wins
name = "race-load"
grid = [1920, 1080]                # the unit of every coordinate below: the source's own pixels
modes = ["single_race"]            # oag_race::Mode::name; absent = every mode
aspect = [1.0, 3.0]                # the surface's width / height, [min, max); absent = any
```

`screen` must come before `[toml-schema]` (after it TOML reads it as `toml-schema.screen`). A file
with no `screen`, or the other screen's, is refused by name. A layout holds `group`s (split HUD) **or**
`element`s (loading), never both. The loader keeps what a schema cannot say (ranges, a `draw`'s required
keys, unique ids); the schema and the loaders are held to the same verdict on shape by
`crates/hud/tests/split_hud_schema.rs`.

## `screen = "split-hud"`

Places a title's **own named widgets** inside one view of a split screen: `views`, `aspect`, `modes`,
and `[[layout.group]]` (`widgets`, `anchor`, `offset`, `scale`). It draws no art of its own and every
number in the shipped files is **chosen, not measured**. The reference is
[split-screen.md](../rendering/split-screen.md) and `oag_hud::split_hud`'s module documentation. This
screen has no `grid`: `offset` is a fraction of the view, so one file serves the PSP's 480x272 and the
PS2's 640x448. The migration from `split-hud.tosd` was the schema name and the `screen = "split-hud"`
line; no number moved.

## `screen = "loading"`

Places the **primitives** a loading screen is made of, drawn in array order (the first is furthest back).
`grid` is required. The renderer is given the layout's own grid, so a draw's coordinates are the file's,
and the player's Safe Area scales the whole grid about its centre (the original does the same: the pair's
frame is every number under `x' = 960 + (x - 960) * 0.96` at the save's 96 %).

| `draw` | Needs | Draws |
| --- | --- | --- |
| `image` | `asset`, `at`, `size`; `uv` `[u0, v0, u1, v1]` in fractions, `colour` | one textured quad. A texture drawn more than 1.5 times its size on an axis is sampled half a texel in on that axis (the original clamps at the texture's own edge; this sheet has neighbours) |
| `text` | `at`, one of `source` and `text`; `font`, `scale`, `colour`, `align`, `wrap`, `pitch` | a run. `font` is a front-end role (`Default` = the primary face, `Title`); `scale` 1.0 is the face at its own pixels. With `wrap` and `pitch` it is a paragraph; with `wrap` alone it is elided from the left to that width |
| `label` | `at`, `asset`, `size`, `gap`, one of `string_id` and `text`; `font`, `scale`, `colour` | a bullet of `size` at `at` and the text `gap` to its right |
| `corners` | `rect` `[x0, y0, x1, y1]`, `asset`, `size`, `back`; `colour` | four brackets. The art is an `L`; a bracket on the right edge is drawn mirrored across and one on the top edge mirrored down, and a bracket on the right or bottom edge is pulled `back` from its edge (RPCS3 frame; `LoadingScreen_DrawCorner`) |
| `dots` | `asset`, `at`, `columns`, `rows`, `tile`; `fill = "progress"`, `colour`, `unlit` | a grid of tiles, the left `floor(fraction * columns)` columns in `colour` and the rest in `unlit` |
| `clip` | `source = "mode-icon"`, `at`, `size` | the mode's video clip, once it has a picture; not while the screen fades |

**Roles, not paths.** `asset` is a role the title's art table names (`rule`, `corner`, `dot`, `square`,
`title-arrow`, `subtitle-arrow`), `@feature`, the feature's picture, or `@mode-icon`, the still of the race's mode
(Omega). `colour` is `ink`, `accent`,
`trough` or `ground`, the four `FEGlobals` the original's constructor reads
(`HD_Grey`, `HD_Blue`, `HD_LightGrey`, `HD_BG`). `source` is what the screen supplies this frame:
`caption`, `feature.title`, `feature.heading`, `feature.text`, `counts`, `percentage`, `current`, `step`
(the last four only while something is counted), `mode-icon`.

**An element whose asset, source or string the title does not resolve is skipped**, never replaced by a
stand-in. A title whose layout file is absent or refused draws the ground alone and says so in the loader
report; nothing is drawn from a guess. Provenance sits in the file's comments beside each number:
`[address, confidence]` for a number read off the executable, "chosen, not measured" for one that was not.

### The HD race load

`assets/fallback/Data/XML/LoadingScreen_HD.toml`, named by `oag_title::Loading::layout`, is measured off
`LoadingScreen_Draw` (`0x002b61c8`) and checked against an RPCS3 frame; the addresses are on
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md), section "2026-11-01: the race
load's layout". What the file does **not** carry because it is unread: the second `Title` pass at the
caption's point, the tournament screen (type 1) and kind 8. The four rows below the bar (`counts`,
`percentage`, `current`, `step`) are this build's own waits, **chosen, not measured**.

Two facts the pair taught that are not in the executable: the wrap of the paragraph breaks a word when
the line **and the space after it** would pass the width (780 from x 975; "with" would end 5 px inside
the edge and the original breaks before it), and the feature picture's window is texel rows `0..480` of a
1024x512 texture (the code's float is `0.0625`, counted from the bottom).

## A mod overrides a layout by path

No new mechanism: the override layer keys every entry by path
([asset-layers.md](../architecture/asset-layers.md)). `overrides/wipeout-hd/Data/XML/LoadingScreen_HD.toml`
(searched in `data/overrides/`, beside the program, `<data dir>/oag/overrides/`, `$OAG_OVERRIDES`)
replaces the fallback's file, and, unlike the fallback, is searched **ahead of the disc**. A mod folder
ships the same path under its `overrides` tree and `oag-mod.toml` needs no key. A layout is replaced
whole; there is no merge. Art roles resolve the same way
(`overrides/wipeout-hd/Data/FE/Images/line.gtf` replaces the rule).

## A mod track carries its own loading screen

The track manifest's optional `[loading]` table
([track-file.md](track-file.md), `schemas/v1/track.tosd`), every key optional, files relative to the
manifest and inside its folder (no `..`, never absolute):

```toml
[loading]
layout          = "asteroid-run.loading.toml"  # a screen-layout file, screen = "loading"
caption         = "ASTEROID RUN"               # the circuit's name on the caption line (else track.title)
feature_image   = "asteroid-run.png"           # drawn by the layout's `@feature` image, sized to its box
feature_title   = "NO WALLS"
feature_heading = "WARNING"
feature_text    = "The outer ring has no barrier."
```

Resolution, per thing, first hit wins; what a track does not supply stays the title's:

1. **layout**: the manifest's file, else the title's (override, disc, fallback, in that order), else none.
2. **feature**: if the track names a picture or any of the three texts, it **is** the feature: the title's
   random draw is not run, and a key left out draws blank rather than another feature's text. Otherwise
   the title's draw stands.
3. **caption**: the manifest's `caption`, else the circuit's name the load reports.
4. **everything else** (the mode icon, labels, palette, fonts, art roles) is the title's. A track cannot
   change them, and cannot supply code, fonts or sounds.

A layout or picture that will not read (or a layout of another `screen`) is named in the loader report
and that one thing falls back; the picture is a PNG, decoded to RGBA8 and placed under the title's sheet.
The picture fills the layout's `feature-image` box (896x480 in HD's file), so an author makes it that
shape. A label's `string_id` in a track's layout resolves only ids the title's own layout names (the
strings are read once, with the title); a track uses `text` for anything else. A track raced on another
title (a Race Remix) uses its own `[loading]` first and the **running** title's layout second.

Try it: `oag-game <image> --loading-screen 0/1 --loading-step race --track-file <manifest> --ticks 60
--screenshot out.png` draws the screen with the manifest's table laid over the title's.

## The pair and `--loading-draw`

`oag-game --loading-screen ... --loading-draw N` seeds the feature draw (the position in the mode's deck of
the seed's first number; default 0). The original draws a feature at random each time the screen goes up
(`Libc_Rand`), so a pair names the draw its frame showed: `scripts/rpcs3-hd-menu-pair.py --walk loading`
reads the original's `Feature type == N` from `TTY.log` and runs ours for the first seed in `0..16` whose
logged feature slot equals it. Method and baseline: [compare-page.md](../tools/compare-page.md), "Menu
pairs: HD/Fury and Omega".

## Cross-title check (CLAUDE.md: 2048 and Omega)

**HD, Omega and 2048 carry a loading layout.** Pure names no `Loading` at all, and Pulse's names no layout (`oag_title::Loading::layout` is `None`), so their screens are still the code's. The HD law is **checked, not applicable yet**:
Omega's front end is HD's, but no Omega loading frame exists and shadPS4 race shots are untrusted
(`docs/formats/omega-status.md`); 2048's `LoadingScreen_2048.toml` carries HD's operands, compared in 2048's draw function ([2048-loading.md](2048-loading.md)). The split-HUD half is unchanged on all
four titles (`SplitHud_*.toml` each carry `screen = "split-hud"`).
