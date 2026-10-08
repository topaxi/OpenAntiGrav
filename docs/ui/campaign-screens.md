# The Race Campaign's two screens: `Grid Selection` and `Cell Selection`

**Status: both screens draw, off the disc's own `CellMode_Definition.xml` and
the real 236-cell campaign, and confirming a cell launches it.** The player
can page through all sixteen grid tiers, drill into a tier's own hex grid,
and see a selected cell's detail panel - track, class, laps, weapons,
points, medal, and (where the disc shows them) the three medal targets.
Confirming a cell in one of the six modes this engine implements
(`Race`/`Time Trial`/`Speed Lap`/`Zone`/`Elimination`/`Tournament`) opens
`Team Selection` and launches the cell's own race (a Tournament cell opens
its own first leg - see `docs/gameplay/race-modes.md#tournament`); a cell in
one of the three this engine does not (`Head2Head`/`Custom Grid`/`AI Race`)
logs why and stays on `Cell Selection`. See "Confirming a cell launches" below and
`docs/architecture/persistence.md` for the medal it earns and where it is
kept. **Both screens answer a mouse and a finger too, since 2026-09-14** -
hovering a tier tile or a hex selects it, a second click on the selection
confirms it, the paging arrows are clickable, and the secondary button (or
a click/tap while `Cell Help` is open) backs out - the same vocabulary
`docs/architecture/menus.md`'s "A mouse and a finger" already gives every
other front-end screen. See
[`oag_ui_screens::campaign::pointer`](../../crates/ui-screens/src/campaign/pointer.rs) for
the targets and the two-tap idiom, and its own module doc for why a hex is
hit-tested as a hexagon rather than its bounding box - the staggered grid's
neighbours overlap at their corners otherwise. Implemented in
[`oag_ui_screens::campaign`](../../crates/ui-screens/src/campaign.rs) (model, layout, draw),
[`oag_game::campaign`](../../crates/game/src/campaign.rs) (the shared read
off an open source), `crates/game/src/main/campaign_stage.rs` (what the
session holds open) and `crates/game/src/main/session/campaign.rs` (the
flow: `RACE CAMPAIGN` opens it, confirm and back move between the two
screens, and a confirmed cell launches). Captured headlessly with
`--menu-page grid-select` / `--menu-page cell-select`, on `pulse-psp-eu` -
the drawing alone; the launch itself was verified live under Xvfb with
`xdotool` driving the mouse on 2026-09-14 - hover, click, `Team Selection`,
an autopiloted race to its results table and back onto `Cell Selection` all
confirmed by screenshot (see the `campaign-pointer` lane's own report) -
not captured against the running original in this pass (see [Open](#open)).

This page is the *picture* half of
[`docs/formats/race-setup.md`](../formats/race-setup.md)'s "The Race
Campaign" section and
[`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md)'s
decompiled law: those pages read the XML and the executable, this one says
what this build draws and why. The frame around both screens - the tab, the
footer, the row colours - is [menus-original.md](menus-original.md)'s and is
not repeated. `docs/formats/race-campaign.md` and
`crates/tables/src/race_campaign.rs` are the parser this reads its 236 cells
through; nothing here re-derives that schema.

## What is authored, and where

`Data\Plugins\PI001\GUI\CellMode_Definition.xml` in `Data.wad`, dictionary-
shortened like every other Pulse front-end file. Two screens:
`<Screen type="GridSelection" name="Grid Selection">` and
`<Screen type="CellSelection" name="Cell Selection">`, both reachable from
`Main Menu`'s `FE_RACE_CAM` entry via `TournamentLoad` (a loading/autosave
dialog this build does not draw - see [Open](#open)).

## `Grid Selection`: sixteen tiers, four hexes at a time

**The `honey` counter's own formula settles the page shape without a
capture.** `GridSelection_Update` binds it to `"%d-%d / %d"` off
`index*4+1, index*4+4, max*4+4` - with sixteen grids that only closes if
`max` counts *pages* of four (`3*4+4 = 16`), not grids (`16*4+4 = 68`) or
tiers directly. So the `GridController name="Grid"` widget
(`MaxX="4" MaxY="1"`) is the current page's four tiers - a **row**, not a
flat wrapping list. `oag_ui_screens::campaign::GridSelection` implements it as one
flat `index` over every grid, `page()` and `slot()` derived from it,
`counter()` reproducing the formula exactly.

**Superseded, 2026-09-25: up/down and left/right are two separate controls,
not one.** This section used to read the `up arrow`/`down arrow` at
`(117, 84)`/`(117, 167)` as cycling all sixteen tiers one at a time, on the
reasoning that `CellMode_Definition.xml` authors no left/right arrow image -
and, following that same reasoning, `GridSelection::update` left `Left`/
`Right` unbound entirely. Both are wrong: a maintainer playing this build
reported left/right dead on this screen, and measuring live against PPSSPP
settled it the other way - `Up`/`Down` page by a full four-hex row and
`Left`/`Right` step one tile within the page, and **neither wraps**, both
clamp at their own boundary (the deck's own ends for `Up`/`Down`, the current
page's own ends for `Left`/`Right`). See "Measured against PPSSPP,
2026-09-25" below for the full walk. `GridSelection::page_step`/`tile_step`
(`crates/ui-screens/src/campaign/pointer.rs`) carry the corrected arithmetic;
`GridSelection::step` (wrapping, one tile) is kept only for HD/Fury's own
one-tile-per-page flyer pager, which reuses this same model with
`per_page` set to `1` - see [Wipeout HD/Fury: `Grid Selection` is not a hex
grid of tiers](#grid-selection-is-not-a-hex-grid-of-tiers---it-is-a-flyer-pager)
below.

| Thing | Where | Authored as |
| --- | --- | --- |
| up / down arrow | `(117, 84)` / `(117, 167)` | same `pulse_assets.mip` sub-rects `Track Creation` uses |
| honey counter | `(135, 167)` | `"{first}-{last} / {total}"`, template `"1/1"` |
| four hex tiers | staggered diagonal, `(65,145)`/`(95,126)`/`(125,107)`/`(155,88)` | `Medal_0_0`..`Medal_3_0` / `Outline_0_0`..`Outline_3_0`, `GridController MaxX="4" MaxY="1"` |
| detail panel | `(270, 93)` area, `204x110` backing | `BGgradient`/`BGhexgrid`, `Title`/`Medals`/`Points`/`Required` |

Measured off the real `CellMode_Definition.xml` (`just wad cat
"data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"
'Data\Plugins\PI001\GUI\CellMode_Definition.xml'`, expanded through
`oag_tables::fexml::text`): the four hex positions are a diagonal stair, not
a row - `+30x`/`-19y` per step, `19` being half the hex's own `38` vertical
pitch. Confidence 95, direct read.

### Two `GridController`s, and only one is drawn

The XML authors **two** `GridController`s at the same position, both naming
`Medal_0_0`..`Medal_3_0` identically: `Grid` (`focus="true"`, its four hexes
carry a staggered `delay="0.2"/"0.4"/"0.6"` reveal) and `Grid1`
(`startenabled="false"`, otherwise byte-identical minus the `delay`
attributes). `oag_ui::screen::Screens::collect_widgets` gained a
`"gridcontroller"` arm for this file - **the tag was previously unhandled
and every widget inside one, on either screen, was silently dropped** - and
that arm skips any controller whose own `startenabled` reads `"false"`.

**Why skip rather than merge**: `screen.images` is a flat `Vec` with no
notion of which controller a name came from, so collecting both would have
one tier's hexes silently overwrite the other's under the same key. Which
of "crossfade buffer for paging" and "the settled state after `Grid`'s own
entrance animation finishes" `Grid1` actually is was not resolved - neither
matters to what is drawn, since only the enabled one ever is. Confidence 60
on "harmless to skip"; open at 50 on what `Grid1` is *for*.

### Medal/points figures, 2026-09-14

**No longer always the fresh-profile reading.** `GridSelection_Update` binds
`Medals` to `Grid_CountMedalsAtLeast(grid, 0) / Grid_CellCount(grid)` and
`Points` to `Grid_PointsEarned / Grid_PointsPossible` - both read a saved
record, which now exists (`records.toml`'s `[[campaign]]` table, see
`docs/architecture/persistence.md`). `oag_ui_screens::campaign::GridSummary::from_grid_with_medals`/
`CellSelection::with_medals` take a `Fn(&str) -> Option<Medal>` keyed on a
cell's own `name`, and `crates/game/src/main/campaign_stage.rs` feeds it
from `Session::records` - a snapshot read once when the screen opens, the
same "read once" choice the sixteen grid files themselves already make.
`GridSummary::from_grid`/`CellSelection::new` still exist for a caller with
no store to read - `crates/game/src/capture/menu_page.rs`'s headless
`--menu-page grid-select`/`cell-select` capture, which draws a page in
isolation with no `Session` behind it - and draw the fresh-profile numbers
exactly as before: `"00/{cell_count:02}"` and `"000/{max_points:03}"`.
`Required` was always real - `grid.required_points`, or `FE_NA` on
`grid15`'s own `0` - since it is authored on the grid itself, not derived
from a save.

**Superseded, 2026-09-14: `Title` resolves through a real per-grid
idstring, not `grid->name` drawn raw.** The reading above had it backwards -
`GridSelection_Update` binding `Title` to `grid->name` directly is still
what the decompile shows, but the disc's own English `entries.xml` carries
one idstring per grid, spelled `Grid0`..`Grid19` (capitalised, matching
`grid->name`'s own spelling with its first letter upper-cased): `Grid0`
answers `"Grid 1"`, `Grid4` answers `"Grid 5"`, and - not derivable from a
`"Grid {n+1}"` formula - **`Grid12`..`Grid15` answer `"Phantom Grid
1"`..`"Phantom Grid 4"`**, not `"Grid 13"`..`"Grid 16"`. `Grid16`..`Grid19`
exist too (`"Download grid 1"`..`"4"`), for DLC grids this build's sixteen
shipped ones never reach. `oag_ui_screens::campaign::draw::grid_title` looks the
capitalised name up directly (`strings.get(&id)`), falling back to the raw
`grid->name` on a miss - the same visible-absence rule every other label on
this screen already follows. This also very likely settles `race-campaign.md`'s
own open `Group="1"` field on `grid12`..`grid15`: it is plausibly what
selects the `"Phantom Grid N"` naming rather than `"Grid N"`, though that
binding itself was not traced this pass - the string table entries were
found by inspection, not decompilation, so this is a strong correlation
(`Group` is set on exactly the four grids with the alternate name) rather
than a proven mechanism.

## Grid tiers and cells lock and unlock, 2026-09-14

**Superseded.** This section used to say every tier draws open because this
build had no per-cell save to sum `Grid_PointsEarned` from - that save now
exists (`records.toml`'s `[[campaign]]` table, see the "Medal/points
figures" section above and `docs/architecture/persistence.md`), and
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Unlock rules, cell
and tier" traced the actual glyph rule in full, on both screens:

```
tier lock glyph visible  ⟺  grid.locked != 0
                        AND  this grid's own Grid_PointsEarned == 0
                        AND  NOT (previous tile's earned >= previous tile's required)

cell lock glyph visible  ⟺  cell.locked (absent defaults to true) != 0
                        AND  Cell_BestMedal(cell) == 0xff
                        AND  no hex-adjacent cell has a medal either
```

`oag_ui_screens::campaign::GridSelection::tier_shows_lock`/`CellSelection::cell_shows_lock`
implement both exactly, the six-neighbour cell check against
`g_anCellNeighbourOffsets`'s own two-parity table included, and
[`Lock_x_y`/`Lock_n_0`](#lock_x_ylock_n_0-now-draw-three-linen-titles-still-do-not)
now draws under it on both screens - see that section for which widgets
still do not.

**Confirming a locked tile also refuses, on both screens - chosen, not
measured.** `crate::main::session::campaign::handle_campaign` reuses the
identical glyph predicate to swallow a `Confirm` on a locked tier or cell,
logging why rather than opening `Cell Selection`/launching. This is *not* a
decompiled mechanism: no PI001 function this project has read
(`CellSelection_CommitSelection`, `GridSelection_CommitSelection`,
`StateMachine_TransitionTo`) ever refuses the transition on either `Locked`
byte - only a live PPSSPP capture measured that confirming a locked tile
does nothing (see "Grid-tier locking is not cosmetic in the original"
below), and the real refusal was narrowed to an un-decompiled function
(`0x088c8a10`) upstream of the state machine, not found in this pass. Reusing
the glyph's own predicate reproduces the *measured behaviour* without
claiming to have found the *mechanism*.

## `Cell Selection`: the 32-slot staggered hex grid

`GridController name="Grid"` at `(35, 40)`, `MaxX="7" MaxY="5"` - the bounding
box, not the cell count. Columns sit `30px` apart (`x = 35 + 30n`), odd
columns staggered `19px` down (half the `38px` row pitch), and the authored
column heights are `5, 4, 5, 4, 5, 4, 5` - 32 hex positions total, filled to
whichever 8-16 of them the selected grid's own cells actually name (parsed
off each cell's `grid0_x_y`-shaped name, `Cell::grid_coords`).

`oag_ui_screens::campaign::CellSelection` reads the selected `(x, y)` position
straight off the disc-resolved `Medal_{x}_{y}`/`Outline_{x}_{y}` widgets -
no stagger formula is reimplemented in Rust, since the widget's own already-
offset-resolved `x`/`y` *is* the pixel position. Movement (up/down/left/
right) is a nearest-neighbour search over the *occupied* cells' own `(x, y)`
grid coordinates (not screen pixels), picking whichever other cell has the
most positive dot product with the pressed direction and, among ties, the
smallest perpendicular offset. **Chosen, not measured**: the original's own
directional adjacency on a staggered hex grid was not traced, and this
reduces to "the nearest cell roughly that way" without needing the stagger
spelled out a second time.

### The detail panel

**`Title`/`Track Line` resolve through the disc's own string table,
2026-09-14 - superseding the raw-string reading this table used to give
both.** `docs/ui/campaign-screens.md`'s own PPSSPP measurement (below) found
the previous reading backwards: a `Race` cell reads `"SINGLE RACE"`, not the
raw enum spelling, and `Track Line` reads the circuit's display name, not
the raw `NN_Track` id. `oag_ui_screens::campaign::draw::cell_title` mirrors
`crate::menu::mode_label`'s own `MSC_EVENT_*`-head-before-the-colon reading
for the six modes this engine implements, plus `MSC_EVENT_HTH` for
`Head2Head` (present on disc, resolved the identical way, but unmeasured
against a live frame - no capture reaches that mode). `Custom Grid`/`AI
Race`/an HD `Other` mode still fall back to the raw spelling, since neither
authors an `MSC_EVENT_*` entry at all.
`draw::track_line` calls `strings.get_or_id(&cell.track)` directly - Pulse's
single copy of the string table needs no `CircuitNames` fold to reach it,
unlike Wipeout HD's (see `oag_ui::language::CircuitNames`'s own doc).

| Widget | Source | Shown for |
| --- | --- | --- |
| `Title` | the localised mode name (`cell_title`) - raw spelling only for `Custom Grid`/`AI Race`/`Other` | always |
| `Track Line` | the circuit's own display name (`strings.get_or_id(&cell.track)`), or `"{n} Races"` off `tournament_tracks.len()` for `Tournament` | always |
| `Line1` | `cell.class` (raw string - `"Zone"` on a Zone cell, not a speed class) | all but Zone |
| `Line1 Title` | `RC_SC` (`"Speed class"`) | all but Zone |
| `Line2` | `cell.laps`, or `RC_INF` when absent/zero/Zone | always |
| `Line2 Title` | `RC_LAPS` (`"Laps"`) | always |
| `Line3` | `FE_ON`/`FE_OFF` off `cell.weapons` | Race / Head2Head / Tournament only |
| `Line3 Title` | `RB_WEAP` (`"Weapons"`) | same as `Line3` |
| `Line6` | the selected cell's own saved points over `Medal::Gold.points()`, `"0/3"` with no saved medal | always |
| `Line6 Title` | `ER_POINTS` (`"Points"`) | always |
| `Line7` | `IG_HUD_GOLD`/`SILVER`/`BRONZE` for a saved medal, suffixed `"(<rung>)"` when the medal's own difficulty is known, `MSC_NONE` otherwise | only when `Target0..2` are not (see below) |
| `Line7 Title` | `IG_HUD_BEST` (`"Best"`) | same as `Line7` |
| `Target0..2` | `cell.gold`/`silver`/`bronze`, formatted as `M:SS.CC` for Time Trial/Speed Lap, a plain number otherwise | Time Trial / Zone / Elimination / Speed Lap only |
| `Target0..2 Title` | `IG_HUD_TARGET`, resolved | same as `Target0..2` |
| `Target0..2 Image` | the gold/silver/bronze swatch (`0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`) | same as `Target0..2` |

**`Line7`/`Target0..2` are mutually exclusive, not additive, 2026-09-14** -
see "A cell's detail panel: `Best`/`Target` looks mutually exclusive" below,
whose finding this now implements: `Line7`/`Line7 Title` draw only when
`targets_visible` is false, sharing the row `Target0..2` would otherwise
sit in.

**`Line7`'s difficulty suffix, `pulse-cellsel` lane, 2026-09-28.** Pulse
cells never author `PI_Cell.difficulty_targets`, so this build's own
`Store::record_campaign` used to gate recording a difficulty on that field
existing at all - `None` on every Pulse cell by construction, since the
medal a Pulse cell earns never varies by difficulty. `CellSelection_CommitSelection`
(`0x088d6138`) decompiles to something narrower and unconditional: it
persists the screen's own *browsed* rung (`Profile_SetDifficultyRC`) on every
Confirm, on every cell, as metadata alongside whichever medal that race
earns - never a gate on the medal itself. `Session::handle_campaign` no
longer filters on `difficulty_targets`, and `CellSelection_PopulateDetail`'s
own `"%s (%s)"` format (`0x088d68d8`) is reproduced by
`oag_ui_screens::campaign::draw::medal_line` - `"Gold (Medium)"`, not a bare medal
word, once the saved row knows its own rung. See
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
`record+8`/`record+9` write" and "The `DifficultyRC` persisted rung"
sections for the full decompile, including the corrected tie-break
(`Store::record_campaign`'s own "harder rung wins outright" reading was
wrong on a strict medal improvement, which overwrites the stored difficulty
unconditionally - even downward - rather than comparing it at all).

**The row dividers (`line bg1`/`2`/`3`, drawn as a `Fill` pair per stripe)
draw one per *visible* row, not four unconditionally, 2026-09-14** - a
`Race` cell shows three (`Line1`/`2`/`3` all visible), a `Time Trial` cell
two (`Line3` hidden). `line bg4` never draws: nothing ever populates `Line4`
to sit above it.

`targets_visible`/`weapons_visible` reproduce
`CellSelection_PopulateDetail`'s own gating exactly:
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md` documents the target
column hidden for `Race`/`Tournament`/`Head2Head` (a finishing position is
implicit there) and shown for the four modes that need an explicit number.

**A regression caught by looking at the capture, not by a pre-existing
test**: the `Target0..2 Image` swatches used to draw *unconditionally*, in
the general per-widget pass that also draws the hex grid and the panel's
rules - nothing excluded them there, only the second, gated pass that draws
them properly. A `Race` cell showed all three medal-colour hexes regardless.
Fixed; `crates/ui-screens/src/campaign/tests.rs`'s
`race_draws_no_target_swatch_even_though_the_sprite_resolves` is the
regression test, built with a sprite closure that actually resolves a
`Placed` rather than the other tests' `&|_| None` - which hid the bug in
the first place, since an unconditionally-drawn widget and a correctly-
gated one look identical when neither ever reaches a real sprite draw.

### `Lock_x_y`/`Lock_n_0` now draw; three `Line{n} Title`s still do not

**`Lock_x_y`/`Lock_n_0` moved out of this list, 2026-09-14.** `race-campaign.md`'s
own "what is not determined" scored `Locked` at 50 and said not to implement
a lock from it - a later pass traced its actual consumer in full (see "Grid
tiers and cells lock and unlock" above), so this build now draws it under
that measured rule on both screens rather than leaving every cell and tier
open.

- **`Line1`/`Line2`/`Line3`/`Line6`/`Line7 Title`, on `Cell Selection`, also
  moved out of this list, 2026-09-14.** They resolve to real idstrings -
  `RC_SC`/`RC_LAPS`/`RB_WEAP`/`ER_POINTS`/`IG_HUD_BEST` - found by reading
  the disc's own English `entries.xml` for the exact label text the
  PPSSPP frames show (`"Speed class"`, `"Laps"`, `"Weapons"`, `"Points"`,
  `"Best"`), not by decompilation: `CellSelection_PopulateDetail`'s own
  table (`race-campaign.md`) still names `Line1`..`Line8` as *values* only,
  never their `Title` companions, so which idstring belongs to which row is
  this build's own reading of the string table's contents, not a traced
  binding. `RC_SC`/`RC_LAPS` sit in the same `RC_`-prefixed family as
  `Medals`/`Points`/`Required Title` already confirmed on `Grid Selection`
  (`RC_GM`/`RC_TP`/`RC_PN`); `RB_WEAP` is corroborated independently -
  `docs/formats/race-setup.md` already names it the `Single Player` screen's
  own `Weapons` row, same `FE_ON`/`FE_OFF` values; `ER_POINTS` is the one
  plain `"Points"` entry in the whole table with no `RC_`-family
  alternative. `Line7 Title` (`Best`) draws only when `Target0..2` do not -
  see "A cell's detail panel: `Best`/`Target` looks mutually exclusive"
  below, whose finding this implements.
- **`Line4`.** Never appears in `PopulateDetail`'s own table at all. Still
  blank.
- **`Line5`/`Line8`.** Both `Cell_SavedRecord` - the saved best this build
  has no record for - sharing one offset (`Item OffsetX="260" OffsetY="180"`,
  the same swap idiom `race-setup.md` documents for `Single Player`'s
  `Zone`/`DifficultyNaText`). Still blank, and so are their own `Title`
  companions - no idstring was found for either.

## Confirming a cell launches, 2026-09-14

The launch path traced (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
"how a campaign event launches") and wired the same afternoon:
`session::campaign::handle_campaign` reads the confirmed cell and hands it to
`Session::launch_campaign_cell`, which resolves the cell's own track and
mode against the open source, refuses a mode this engine cannot run
(`Tournament`, `Head2Head`, `Custom Grid`, `AI Race` - logged, `Cell
Selection` stays open), and opens `Team Selection` the same way the RACE
page's own START row does - every authored cell carries `ShipChoice="Yes"`.
Confirming a team launches the race with the cell's own track/mode/class/
laps/kill target, and a finished or escaped run's medal is evaluated against
the cell's own gold/silver/bronze targets and folded into `records.toml`'s
new `[[campaign]]` table - see `docs/architecture/persistence.md`. See that
page and `crates/game/src/main/session/campaign.rs` for the full mode-by-
mode detail; this page stays about what draws.

**Not launched from a cell: the AI difficulty curve.** A campaign cell's own
`skill`/`skillEasy`/`skillHard` (`AI_ResolveSkillScale`) is not
implemented, so a campaign race still uses the ordinary `[ai] difficulty`
setting. Chosen, not measured, and logged when it happens.

**A locked tile refuses `Confirm` outright, 2026-09-14** - `handle_campaign`
checks `GridSelection::selected_is_locked`/`CellSelection::selected_is_locked`
before either arm above runs, logging why and leaving the screen where it
was rather than opening `Cell Selection` or `Team Selection`. See "Grid
tiers and cells lock and unlock" above for the predicate and why this is
chosen, not measured, on the exact refusal mechanism.

**Omega launches through the same function, 2026-09-30.** The mapping above is
`oag_game::campaign::launch::plan_cell` (cell and a circuit resolver in; mode,
circuits, class, laps, kill target out) and `Session::launch_campaign_cell`
applies its plan. A disc-backed test holds Omega's 167 parsed cells to it; see
[`omega-status.md`](../formats/omega-status.md#campaign-confirming-a-cell-starts-its-race-2026-09-30)
for what a walk found (no `Team Selection`, no EndRace screens yet).

`Cell Help` (`triangle`) is modelled as a boolean toggle
(`CellSelection::help_open`) that suspends movement while open, matching
the disc's own `Watch` element redirecting every directional press back to
`Cell Help` while it is the present screen - but the overlay itself is not
drawn yet; see [Open](#open).

## The hex look, 2026-09-14: an outline always, a medal-colour swatch only where earned

**Superseded: this build used to draw `hex_filled.mip` on every hex, on both
screens, unconditionally.** `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
`CellSelection_PopulateGrid`/`GridSelection_PopulateTiles` pseudocode names
two widgets at every slot with two different jobs: `Outline_x_y` (base hex,
always drawn) and `Medal_x_y` (a colour swatch, drawn only where a cell/tier
has actually earned a medal or points - `draw_medal_colour`/`if pointsEarned
!= 0`). `oag_ui_screens::campaign::draw::{grid_draw_list, cell_draw_list}` now
follow that: `Medal_x_y` is skipped entirely (`Outline_x_y` still draws)
unless `CellSelection::medal_at`/the tier's own `points_earned` says
otherwise, and the swatch that does draw is tinted per the medal tier
(`medal_argb`) or a single "earned" tint on `Grid Selection`
(`medal_tint`).

**The swatch's own colour is chosen, not measured.** No PPSSPP capture this
project holds exercises it - every frame under
`data/reference/psp-campaign-screens/` is a zero-medal profile, so the
`gold`/`silver`/`bronze[medal]` array `CellSelection_PopulateGrid`'s
pseudocode names was never read off the executable or seen on screen.
`medal_argb` reuses the panel's own measured `Target0..2 Image` swatches
(`0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`) as the closest available evidence -
the same three-tier colour concept, authored on the same screen - not an
independent reading of the hex fill itself. `Grid Selection`'s own
`medal_tier_colour(pointsEarned, ...)` mapping was not traced at all, so its
tint is always gold's own swatch regardless of how much of the tier is
actually earned.

**`Outline_x_y`'s own colour now resolves through `FEGlobals`, 2026-09-14.**
`Cell Selection`'s copy authors `i="FEGlobals->CM_HEX_Outline"`
(`Data\Plugins\PI001\GUI\Skin.xml`: `0x7F34ACC2`, a semi-transparent teal);
`crate::campaign::load` parsed `CellMode_Definition.xml` on its own before
this pass, with no fallback globals, so that reference never resolved and
the outline drew opaque white instead. `load` now takes the front-end
root's own globals as a fallback (`shell.globals` live,
`frontend.screens().globals` in a `--menu-page` capture) - the same
`Screens::from_xml_with_fallback_globals` idiom `crate::boot::screens::load_included_screens`
already uses for every other `LoadXML` include. `Grid Selection`'s own copy
of `Outline_x_y` authors no `FEGlobals->` reference at all, so its colour is
unaffected by this fix and stays the authored default.

## Captures

`--menu-page grid-select` / `--menu-page cell-select`
(`data/images/pulse-psp-eu.chd`, 480x272, not committed - game content):

- `grid-select`: `"CAMPAGNE DE COURSES"` (the disc's own French `FE_RACE_CAM`
  text - this machine's source offers no English table, so every screen
  falls back to French), four staggered hex tiers, honey `"1-4 / 16"`,
  panel showing `grid0`'s own live numbers - `"MÉDAILLES D'OR 00/08"`,
  `"TOTAL POINTS 000/024"`, `"POINTS REQUIS 12"` (`RC_GM`/`RC_TP`/`RC_PN`,
  resolved off the disc's own string table).
- `cell-select`: the 8-cell hex grid for `grid0` (its own `PI_Grid` names 8
  cells), selected on `grid0_2_1` - the *first* cell `grid_00.xml` lists,
  which this capture has no way to choose otherwise. Panel: `"RACE"` /
  `"16_TRACK"` / `"VENOM"` / `"3"` / `"OUI"` (weapons on) / `"0/3"` /
  `"NÉANT"` (no medal) - matching `grid0_2_1`'s own authored values
  (`track="16_Track" mode="Race" class="Venom" Weapons="on" ... laps="3"`)
  digit for digit against `race-campaign.md`'s own reading of the file.

### Recaptured after this pass's fixes, 2026-09-14

Same two flags, `data/images/pulse-psp-usa.chd` (English), `--size
1440x816`, kept at `/tmp/oag-drive/picture/ours-{grid,cell}-select.png` (not
committed - see the frontend handover thread for the durable copy's path).
Every string, lock and hex-fill change this page documents above is visible
side by side with `data/reference/psp-campaign-screens/`'s own frames on the
same fresh, zero-medal profile:

- `grid-select`: `"RACE CAMPAIGN"`, `"GRID 1"` (not `"GRID0"`), `"GOLD
  MEDALS 00/08"` / `"TOTAL POINTS 000/024"` / `"POINTS NEEDED 12"`, three of
  the four visible tiles showing a lock glyph over a plain outline hex (no
  fill) and the selected tile (`grid0`, `Locked="false"`) showing neither -
  matching `grid-selection-page1-grid0-unlocked.png` widget for widget.
  Still missing: the tip ticker and the button-legend footer row, neither
  implemented this pass (see [Open](#open)).
- `cell-select`: `"SINGLE RACE"` / `"Talon's Junction White"` (not `"RACE"`
  / `"16_TRACK"`), row labels `"SPEED CLASS"` / `"LAPS"` / `"WEAPONS"` /
  `"POINTS"` / `"BEST"`, all six non-selected occupied hexes locked (plain
  outline, padlock glyph) and the selected one (`grid0_2_1`, no `Locked`
  attribute - defaults locked) also showing its own lock glyph under the
  selector - matching `cell-selection-grid0-default-cell.png` on every text
  row. The `HELP`/`CHANGE DIFFICULTY` footer draws here (a mechanism outside
  `oag_ui_screens::campaign` this pass did not need to touch) but without the
  `Confirm`/`Back` legend the reference shows - not chased further this
  pass. The occupied-hex layout is still the two-column zigzag
  `crates/tables/src/race_campaign.rs`'s `grid_coords` parse produces from
  `grid0`'s own cell names, not the original's diamond - see "The 8-cell hex
  layout is a diamond" below, unchanged by this pass since the coordinates
  are the disc's own, not a rule this build could choose differently
  without inventing one.

## Measured against PPSSPP, 2026-09-14

**PPSSPP v1.20.4 (SDL build), `pulse-psp-usa.chd`, Xvfb `:97` at 960x544 (twice
the PSP's 480x272, the `selection-screens.md` convention), `import -window
root`.** The memory stick already carried a profile from an earlier session on
this machine (state went `Show Logo` -> `RemoveMemoryStickWarning` ->
`AutoLoadProfileCheckScreen` -> `Main Menu` with no name/tag entry), so
**`TournamentLoad`'s autosave dialog (`MSC_SQ_MSG7`) never appeared** - a
legitimate negative result, not a skipped step: the SDL build persists
`~/.config/ppsspp/PSP/SAVEDATA/`, and this stick's is not fresh. The profile
carries no campaign medals, so grid1 onward reads as freshly locked regardless.
Fifteen frames, kept under `data/reference/psp-campaign-screens/` (gitignored,
not committed): `grid-selection-*.png`, `cell-selection-*.png`,
`cell-help-overlay-*.png`, and this build's own `ours-grid-select.png` /
`ours-cell-select.png` (`--menu-page grid-select` / `cell-select` on the same
`pulse-psp-usa.chd`, matching the emulator rather than `pulse-psp-eu`).

**State names, confirmed live**: `Grid Selection`, `Cell Selection`, `Cell
Help` - exactly the three names this page and `race-campaign.md` already
assumed. `Main Menu`'s row 1 (the default cursor) is `RACE CAMPAIGN`, matching
`scripts/psp-drive.py`'s own `tap(dbg, "down")  # RACE CAMPAIGN -> RACEBOX`
comment - a single `cross` from a fresh `Main Menu` enters the campaign
directly.

### The `Selector` misalignment, measured off the assets directly

**A user playing this build reported "the selected hexagon is misaligned" on
`Cell Selection`; this section is that report turned into numbers.** The
mechanism: `oag_ui_screens::campaign`'s `hex_position` (`crates/ui-screens/src/campaign.rs`)
draws the `Selector` widget with its own top-left placed exactly at the
selected `Medal_x_y` hex's own resolved top-left. That is only correct if
`Selector` and `Medal_x_y`/`Outline_x_y` are the same size - they are not,
and the XML says so directly.

**The two source textures, read from the disc, header bytes and pixel
content both** (`just wad cat ... 'Data\FE\Images\hex_filled.mip'` /
`hex_outline.mip`, decoded by hand against `crates/texture/src/texture.rs`'s
own documented header layout - 4bpp/8bpp indexed, GE-swizzled per `+0x07`
bit 0, unswizzled with `oag_formats::swizzle::unswizzle`'s exact algorithm
before palette lookup):

| Asset | Native size | Non-transparent content bbox | Content size | Content centre (from its own top-left) |
| --- | --- | --- | --- | --- |
| `hex_filled.mip` | 32x32, 4bpp | `(0,2)`-`(32,30)` | 32x28 | `(16, 16)` - exact canvas centre |
| `hex_outline.mip` | 32x32, 8bpp | `(0,2)`-`(32,30)` | 32x28 | `(16, 16)` - exact canvas centre |
| `Selector` (crop of `pulse_assets.mip` at `U=114,V=75`, both screens author the identical `x="30" y="95" TxtrWidth="42" TxtrHeight="43"`) | 42x43, not swizzled | `(5,9)`-`(37,37)` | 32x28 - **the same shape and size as the hex art**, just embedded in a bigger canvas | `(21, 23)` |

**The glow art is the same 32x28 hex, just padded into a larger canvas -
the padding is the entire bug.** `Selector`'s own visible content-centre
sits `(21-16, 23-16) = (+5, +7)` PSP-native pixels right and down from its
own canvas's top-left-aligned position relative to a hex's, because both
widgets are placed at the same screen point (`hex_position`'s return value)
but `Selector` carries 10 extra pixels of width and 11 of height that this
build's code does not account for. Confidence **92** - both numbers come
directly from the shipped texture bytes via the project's own documented
decode (swizzle flag, palette, indices), not a screenshot estimate, and the
two hex sources (filled and outline) agree on the target's own geometry
to the pixel.

**Reproduced exactly in this build's own captures, both screens.** Crop
`ours-cell-select-selector-misaligned.png` and
`ours-grid-select-selector-misaligned.png` (in
`data/reference/psp-campaign-screens/`, alongside
`asset-hex_filled-32x32.png` / `asset-hex_outline-32x32.png` /
`asset-selector-crop-42x43.png`, the three decoded textures above): the
selected tile shows two visibly offset hexagons, a larger filled one at the
authored top-left and a smaller outlined one shifted down-right of it -
the `Selector` sitting exactly where the maths above predicts. Confidence
95 - the direction and rough magnitude match the asset arithmetic on sight,
on two independently-captured screens.

**The original's own frames show no such artifact** - a single clean
glowing hex, both on `Grid Selection` and `Cell Selection`
(`cell-selection-grid0-default-cell.png`,
`grid-selection-page1-grid0-unlocked.png`). Since the same oversized
`Selector` sprite is what the original draws too (same `.mip`, same 42x43
crop - this is one shared asset, not a per-build difference), the original
must be compensating for the size delta somehow before drawing it -
**most likely by centring `Selector` on the target hex rather than
matching top-left corners**, which only needs an offset of `(-5, -7)`
applied to what `hex_position` returns today to close the gap exactly.
This is inferred from the asset geometry and the absence of the artifact,
**not runtime-traced**: no PPSSPP breakpoint was taken on the native
positioning code this pass (out of this pass's scope - the breakpoint
budget went to the launch trace in `race-campaign.md`), so whether the
original centres on the sprite's own canvas, on its content bbox, or does
something else that happens to land in the same place is open. Confidence
**60** on "centring by half the size delta" as the *mechanism*, separate
from the 92/95 above on the *measurement*, which does not depend on this
guess being right.

**Grid Selection's own `Selector` node is authored twice**, at the same
`(30, 95)` position/size both times - once inside the enabled `Grid`
`GridController` (spelled `<aImage c="Selector">` in the dictionary-decoded
XML, an odd tag name worth a second look some other pass) and once more as
a `LeftLayer`-level sibling of both `GridController`s. `Cell Selection`
authors only the second form. Since both instances carry identical
numbers, this does not change the finding above, but it is the same
duplicate-declaration shape `campaign-screens.md` already flagged for
`Grid`/`Grid1`, now seen on a third widget.

### Grid origin, pitch and stagger - corroborated, not re-measured

The hex origins/pitch this page already documents (`Grid Selection`'s
`(65,145)/(95,126)/(125,107)/(155,88)` diagonal, `Cell Selection`'s `(35,
40)` `GridController` origin with 30px columns and a 19px stagger) come
from the XML directly and are re-derivable from `crates/ui-screens/src/campaign/
tests.rs`'s own miniature fixture, which mirrors the real file's container
nesting exactly - `Item`/`GridController` offsets accumulate the way
`oag_ui::screen::Screens::collect_widgets` sums them: for `Medal_0_0`,
`LeftLayer(0,0) + Item(35,50) + Item(30,19) + Image's own (0,76)` = `(65,
145)`, matching the table exactly once the innermost `Image`'s own `x`/`y`
(added by `image_from_node`, not a further `OffsetX`/`OffsetY`) are summed
in too. A rough visual cross-check against both PPSSPP
captures (hex centres eyeballed to the nearest few pixels, not
edge-detected) is consistent with a ~30px column pitch on both screens;
a pixel-exact re-measurement of the *original's* own render was not done
this pass beyond that, since the XML numbers are already a direct read
of the same file the original executes, not a separate guess this build
could disagree with independently of the `Selector` bug above.

### The three headline differences from this build

1. **`Grid Selection`'s `Title` reads `"GRID 1"`, `"GRID 5"`, `"GRID 9"` -
   never the raw `grid->name`.** `grid-selection-page1-grid0-unlocked.png`
   shows `"GRID 1"` for `grid0`, `grid-selection-page2-grid1-4-locked.png`
   shows `"GRID 5"` for `grid4` (page `5-8 / 16`, i.e. flat index 4, 1-based
   in the title), and the page-3 burst shows `"GRID 9"` for `grid8`. This
   build's `ours-grid-select.png` reads `"GRID0"` (the raw name, capitalised
   by the font, the `0` reading as an `O`) - the exact gap
   `campaign-screens.md`'s own "`Title` is the grid's own raw name" section
   already flagged as unmeasured. **The reading there was backwards**: the
   game does show a friendlier `"GRID N"` (1-based), not the raw name - the
   XML's own `string="GRID 1"` template is not a runtime placeholder pattern
   after all, or a second, unlocated formatting/lookup step turns
   `grid->name` into it. Confidence 90 the *display* is `"GRID N"` (three
   separate grids read on screen, digit for digit); confidence 50 on *how*,
   since no breakpoint was taken on `GridSelection_Update` this pass (out of
   scope - the priority breakpoints were `CellSelection`'s, per the
   assignment).
2. **`Cell Selection`'s `Title` is a localised mode name, not
   `cell->mode`'s bare enum spelling.** A `Race` cell
   (`cell-selection-grid0-default-cell.png`) reads `"SINGLE RACE"`; a `Time
   Trial` cell (`cell-selection-grid0-time-trial-cell.png`) reads `"TIME
   TRIAL"`. `"TIME TRIAL"` is consistent with either reading (it's the enum
   name, capitalised), but `"SINGLE RACE"` is not - `g_mode_name_table` names
   value 3 `"Race"`, and `"Single Race"` is the *Racebox's* `RACE TYPE`
   label for the same mode (`selection-screens.md`/the front-end capture
   never named this screen's own copy of the string). This build's
   `ours-cell-select.png` reads `"RACE"` - the raw table entry,
   uppercased by the same font, per `campaign-screens.md`'s own documented
   binding. Confidence 88 - two cells read on two different discs' worth of
   mode text disagree the same way runtime-vs-static already does elsewhere
   in this doc, and `"Single Race"` matching the Racebox's own wording is a
   second, independent corroboration it's a shared localised string rather
   than a coincidence.
3. **The `Track Line` shows the circuit's own display name, not the raw
   XML string.** The default cell resolves to `grid0_3_1` (`track="03_Track"
   mode="Race" class="Venom" Weapons="on" laps="3"`, matched digit for digit
   against `grid_00.xml`) and the panel reads `"Moa Therma White"` - `03_Track`
   is `Moa Therma` per `ppsspp-debugger.md`'s own name-to-directory note.
   Moving down one row selects `grid0_3_2` (`track="16_Track" mode="Time
   Trial" ... Gold Target="11500"`) and the panel reads `"Talon's Junction
   White"` - `16_Track` is Talon's Junction, this project's own reference
   track. This build's `ours-cell-select.png` reads `"16_TRACK"`, the raw
   string, exactly as `campaign-screens.md`'s existing captures section
   already documented for `grid0_2_1`. Confidence 92 - two tracks on two
   different cells resolve to their known display names exactly, with no
   name-to-directory ambiguity on either.

### Corroborated, not contradicted

- **The three stat rows on `Grid Selection` match this build's own numbers
  exactly**, both grids read: `grid0` shows `Gold medals 0/8`, `Total points
  000/024`, `Points needed 012`; `grid4` (`"GRID 5"`) shows `0/16`, `000/048`,
  `028` - both pairs match `race-campaign.md`'s table
  (`Cells`/`Max points`/`RequiredPoints`) and this build's own
  `ours-grid-select.png` reading for `grid0`. Confidence 95.
- **The honey counter's formula is right.** `"1-4 / 16"` on page 1, `"5-8 /
  16"` and `"9-12 / 16"` on the pages reached by `down`, matching
  `index*4+1, index*4+4, max*4+4` exactly.
- **`down`/`up` on `Grid Selection` move a full page (+/-4 index); `left`/
  `right` move one tile within the current page.** Not previously measured:
  a `down` from `grid0`'s page landed directly on `"GRID 5"` (index 4), not
  `"GRID 2"`, and a `right` from `grid0` landed on `"GRID 2"` (index 1). This
  settles which control is the paging one and which is the per-tile one -
  both were assumed to be up/down in the prose above the "Two
  `GridController`s" section, which is not what was pressed. Confidence 90,
  directly observed. **`left`/`right` were never wired to anything in this
  build at all until 2026-09-25** - see "Measured against PPSSPP,
  2026-09-25" below for the fix and the fuller walk (whether either
  direction wraps, and whether a page turn keeps the same slot).
- **No visible crossfade at a page turn, down to the earliest captured
  frame (~50 ms after the press).** Four bursts, one as tight as `wait 0.05`
  before the first shot, all show the new page already fully rendered - no
  half-opacity, no partial reveal. This is *evidence toward* "`Grid1` is a
  pre-settled buffer swapped instantly" over "a crossfade", but does not
  fully rule out a transition faster than this capture's ~50 ms floor.
  Confidence 60 on the reading, 90 on the observation itself.
- **`Cell Help`'s overlay is static.** Two frames a second apart
  (`cell-help-overlay-t0.png` / `-t1s.png`) are pixel-identical apart from the
  independently-scrolling tip ticker bleeding through underneath - no
  scroll animation fired in this window, consistent with this build's
  planned "a static overlay would be honest" approach in [Open](#open)
  below. It draws as a **semi-transparent panel over the still-visible,
  darkened `Cell Selection` screen**, not a full-screen replacement, and
  the button-icon glyphs at the bottom render as plain circles rather than
  face-button icons in this PPSSPP build - almost certainly a font/icon
  substitution specific to the emulator's UI font, not a reading of real
  hardware, and not a claim about the original console's own rendering.
- **`AI difficulty` (`square`) cycles Easy/Medium/Hard and changes nothing
  else in the panel** (`cell-selection-difficulty-hard.png` against the
  `-t0`/`-t1s` pair) - only the bottom-bar label text changes
  (`"AI difficulty (Medium)"` -> `"(Hard)"`).

## Measured against PPSSPP, 2026-09-25: `left`/`right` on `Grid Selection`

**A maintainer playing this build reported left/right dead on the first
campaign screen** - `oag_ui_screens::campaign::GridSelection::update` left `Left`/
`Right` entirely unbound, and bound `Down`/`Up` to a single-tile wrapping
step, both on the reasoning (struck above) that no left/right arrow image is
authored. PPSSPP v1.20.4 (SDL build), `pulse-psp-usa.chd`, Xvfb, the same
disc this build already reads, driven from the existing (non-fresh)
`~/.config/ppsspp/PSP/SAVEDATA/UCUS98612P0000` profile - `Gold medals 0/8`
on `grid0`, matching the 2026-09-14 session's own zero-medal reading for
that grid, so this is the same starting state, just not a first boot.
Fourteen frames captured with `import -window <id>` after moving the SDL
window onto Xvfb's own visible root (the window opens off-screen by
default under a bare Xvfb, no window manager to place it), one `press` at a
time, state read off the honey counter and the `Title` panel each time
(`docs/reverse-engineering/ppsspp-debugger.md`'s `Debugger.press`/
`state_name`). Not committed (`data/` is gitignored); reproducible with the
same recipe.

The walk, all from `grid0` (`"GRID 1"`, page `"1-4 / 16"`) unless noted:

1. `right` -> `"GRID 2"`, page unchanged (`index 0 -> 1`).
2. `right` x2 more -> `"GRID 4"`, page unchanged (`index 1 -> 3`, the
   page's own last slot).
3. `right` once more from `"GRID 4"` -> **stays on `"GRID 4"`**, `1-4 / 16`
   unchanged. Does not cross into `grid4`'s own `"GRID 5"` on the next page.
4. `left` x3 from `"GRID 4"` -> back to `"GRID 1"` (`index 3 -> 0`).
5. `left` once more from `"GRID 1"` -> **stays on `"GRID 1"`**. The deck's
   own first slot clamps the same way the page's own last slot does.
6. `down` x3 from `"GRID 1"` -> `"PHANTOM GRID 1"` (`grid12`, `"13-16 /
   16"`, the last page - `Grid12` is the first of the four grids whose
   idstring reads `"Phantom Grid N"` rather than `"Grid N"`, see the
   `Title` section above).
7. `down` once more from `"PHANTOM GRID 1"` -> **stays on `"PHANTOM GRID
   1"`**, `13-16 / 16` unchanged. Does not wrap to `"GRID 1"`.
8. `up` x4 from `"PHANTOM GRID 1"` -> `"GRID 1"` (`index 12 -> 0`, all in
   one page-sized step per press, landing exactly on the deck's own start).
9. `up` once more from `"GRID 1"` -> **stays on `"GRID 1"`**. Symmetric
   with (5)/(7): no direction on this screen wraps.
10. `right` once from `"GRID 1"` (-> `"GRID 2"`, `index 1`), then `down`
    once -> `"GRID 6"` (`grid5`, `index 5`), **not** `"GRID 5"` (`grid4`,
    `index 4`). A page turn keeps the pressed-in slot; it does not reset to
    the new page's own first tile.

So: `Down`/`Up` page by a full four-hex row and `Left`/`Right` step one tile
within the page (already measured 2026-09-14, restated here); **and, new
this pass: neither direction ever wraps**, both clamp at their own boundary
(the current page's own ends for `Left`/`Right`, the deck's own ends for
`Up`/`Down`), and a page turn preserves the slot pressed into rather than
resetting to the new page's own first tile. Confidence 90 - ten separate
presses in one session, each read digit-for-digit off the honey counter and
the `Title` panel, with the two boundary tests ((3)/(5)/(7)/(9)) each showing
the *identical* frame before and after the press, not merely "a plausible
neighbour" - a clamp reads as literally nothing moving, which a wrap could
not produce by coincidence.

`oag_ui_screens::campaign::GridSelection::page_step`/`tile_step`
(`crates/ui-screens/src/campaign/pointer.rs`) implement this; `GridSelection::update`
(`crates/ui-screens/src/campaign.rs`) binds `Down`/`Up`/`Left`/`Right` to them.
`GridSelection::step` (the old wrapping, single-tile method) is kept
unchanged and still used by HD/Fury's own one-tile-per-page flyer pager
(`GridSelection::step`'s own doc, and see "`Grid Selection` is not a hex
grid of tiers" below) - a new `per_page` field, `4` by default and
overridden to `1` at HD's own construction sites in `campaign_stage.rs`,
is what tells `page_step` which shape it is turning a page for, and
`page_step` itself delegates straight back to the old wrapping `step` when
`per_page` is `1`, so HD's own navigation is unchanged by this pass -
verified in code review, not re-measured, since HD's own campaign is a
different lane's own thread.

The up/down and left/right arrow **click** targets (`GridSelection::pointer`
in `crates/ui-screens/src/campaign/pointer.rs`) now call the same `page_step` the
pad's `Up`/`Down` does, so a mouse/touch player sees the identical clamped,
slot-preserving paging a pad player does - no click target exists for a
single-tile left/right step, since the four hexes are individually
clickable already and reach the same place in one tap.

### New: a scrolling tip ticker, not documented anywhere before this pass

Both `Grid Selection` and `Cell Selection` carry a **scrolling marquee of
one-line tips and stats** along the bottom, above the button row: `"Why don't
you try out the Speed Lap events?"`, `"You've listened to a total of 1
songs"`, `"Throughout the game you've travelled a total of 0.00 km"`, `"You
haven't raced in a single Zone mode event yet - why not?"`. This build draws
an empty bar in the same position (`ours-grid-select.png`,
`ours-cell-select.png`) - not a difference this pass's captures previously
knew to look for, since no earlier capture of either screen exists. Not
traced to a widget name or a string table this pass; flagged in
[Open](#open) below for whoever picks up the ticker.

### Grid-tier locking is not cosmetic in the original

Paging to `"GRID 2"` (`grid1`, `Locked="true"`, page `1-4 / 16` via `right`)
and pressing `Confirm` **does nothing** -
`grid-selection-confirm-on-locked-grid-inert.png` is pixel-identical to
`grid-selection-grid2-locked-selected.png` taken immediately before it, and
the state name stays `Grid Selection`. This build's own "Grid tiers are not
locked" section documents *drawing* every tier open as a deliberate choice,
not a measurement - this is the measurement that choice was waiting on, and
it says the original gates entry on `Locked`/points, not just the lock glyph.
**Not breakpoint-verified** (no execution breakpoint was armed on
`GridSelection`'s own confirm handler this pass - out of scope, since the
assignment's five priority breakpoints are all `CellSelection`'s); confidence
70 on "confirm is gated" from the behavioural evidence alone, capped below
the runtime-trace band for exactly that reason. Left open for whoever picks
up the grid-tier locking thread next, not implemented here.

### A cell's detail panel: `Best`/`Target` looks mutually exclusive, not additive

The `Race` cell's panel (`cell-selection-grid0-default-cell.png`) shows
`Points 0/3` then `Best None` and no `Target` rows. The `Time Trial` cell's
panel (`cell-selection-grid0-time-trial-cell.png`) shows `Points 0/3` then
three `Target` rows (gold/silver/bronze, each with its own coloured hex
swatch) and **no `Best` row at all** - not `Best` alongside the targets, not
`Best` blanked to nothing in the same position, just absent. `race-campaign.md`
reads `Line7` (`Best`/medal) as shown "always" and `Target0..2` as shown only
for the four modes that need one, which this contradicts unless the two
share screen space the same way `Line5`/`Line8` are already documented to
(`Cell_SavedRecord`'s "same offset" swap idiom). Confidence 65 - one cell of
each shape, not a census, but a clean and repeatable behavioural
observation.

**Target times are formatted with periods, not a colon**: `"1.55.00"` /
`"1.58.00"` / `"2.03.00"` for `grid0_3_2`'s gold/silver/bronze
(`11500`/`11800`/`12300` centiseconds - 115.00 s, 118.00 s, 123.00 s exactly),
not `"1:55.00"`. `race-campaign.md`'s own reading names the format
`"M:SS.CC"`; the separator is measured wrong there. Confidence 95 - direct,
legible digits on screen matching the authored centisecond values to the
centisecond.

### The 8-cell hex layout is a diamond, not a static full-page grid, but all 8 do show

`cell-selection-grid0-default-cell.png` shows all 8 of `grid0`'s cells at
once (2+3+1+2 in a diamond: two locked corners, a row of
locked-selected-locked through the middle, one unlocked-unselected cell
below centre, two more locked corners at the bottom) - not fewer, and not
panned. This build's `ours-cell-select.png` also shows all 8, in a two-column
zigzag rather than a diamond. **Both show every authored cell**, so this is
a layout-shape difference, not a windowing/panning one as first suspected
mid-capture - struck here rather than left as a live misreading. The
exact per-cell pixel positions were not measured to sub-pixel precision this
pass (no widget dump against this specific screen was taken, unlike
`Grid Selection`'s table above, which is measured off the XML directly); the
diamond shape is a visual read of the capture, confidence 70.

### After a campaign race, measured

**Played `grid0_3_2` (`Time Trial`, `16_Track`/`Talon's Junction White`,
Venom, 3 laps, targets gold `1.55.00`/silver `1.58.00`/bronze `2.03.00`) to
completion in PPSSPP**, confirmed through `Team Selection` and flown three
laps with `scripts/psp-autopilot.py` against a freshly generated
`oag-trace track` spline (none existed in the repo for either candidate
track before this pass). **Not `grid0_2_1`** (the `Race` cell this session's
own assignment first proposed, on the reasoning that a position-based medal
is easier to earn than a time target): confirmed *locked* in this same
session (see `race-campaign.md`'s "Runtime-verified 2026-09-14
(deliverable 1)" - `grid0_2_1` carries no `Locked` attribute at all in
`grid_00.xml`, defaults locked, and its confirm never reached
`CellSelection_CommitSelection`). `grid0_3_1` (`Race`, `03_Track`/Moa
Therma White, the other `Locked="false"` cell) was also available but not
used: `psp-autopilot.py` follows one fixed craft address with no per-craft
filter (see `ppsspp-debugger.md`'s "a race updates every craft from the
same function"), and an 8-craft `Race` was judged too likely to produce a
garbled capture within this pass's time budget. `grid0_3_2` was chosen as
the reliable, already-proven case; a slow or medal-less finish was
explicitly acceptable going in.

**Result: no medal, an honest miss.** `3` laps in `3.11.76` total
(`1.32.49` / `0.49.34` / `0.49.93`, the first lap slow from the autopilot's
own startup/aim-acquisition cost, the next two close to the reference
capture's own `~52 s`/lap pace) against a bronze target of `2.03.00` -
comfortably over. Frames (`data/reference/psp-campaign-screens/`,
gitignored):

- `ingame-start.png` - the in-race HUD at the green light: `Lap 1/3`, `GO`
  banner, `Best 0.00.00`/`Current 0.10.7` corner clock, energy bar.
  **No medal-target indicator anywhere on this frame** - consistent with
  `race-campaign.md`'s own "the in-race HUD's medal tier is a separate,
  still-unread value" open item, though this is one frame at race start,
  not a full-race capture, so "never shown at all" is not established here,
  only "not shown at the green light." Confidence 60 on the negative
  (a HUD element could still appear once the cell's own targets need a
  passing reference, e.g. a ghost/split - not checked this pass).
- `results-01.png` - **`EndRace Results`**, a new state name not
  previously documented: `RESULTS` / `TIME TRIAL COMPLETE!`, a four-row
  table (`Lap`/`Time`/a pennant-icon column reading `6`/`9`/`10`, total
  `25`) with the per-lap and total times above. The pennant column's
  meaning was not identified this pass (not a lap count, not a position -
  candidates left for whoever picks this screen up next).
- `results-02.png` - **`EndRace Rewards`**, also new: a `REWARDS` header,
  an explicit **`"No medal awarded"`** row with a hexagonal dash glyph (the
  authored "no medal" icon, distinct from a gold/silver/bronze hex), and
  `Assegai Loyalty: 90 Points` / `Total loyalty: 90` with a filled bar.
  Confirms a loyalty counter is written even on a run that earns no medal
  and no campaign points - loyalty and medal are separate rewards, not one
  gated on the other.
- `results-03.png` - **`EndRace Menu`**, also new: `RETURN TO GRID` /
  `RACE AGAIN` / `VIEW RESULTS AGAIN` / `SAVE GHOST` / `DELETE DATA`, and
  below the ship render, `NO GHOST TIME FOUND` / `NEW GHOST RECORD:
  0.49.34` - the best of the three laps just driven, not the total.
  `RETURN TO GRID` is the default cursor position, confirming the campaign
  flow returns to the hex screens rather than the front end's `Racebox`
  menu tree a custom race would return to.

**What changed on `Cell Selection` and `Grid Selection` after returning**
(`back-to-cellselect-postrace.png`, `gridselect-postrace.png`):

- **`grid0_3_2`'s own detail panel gained a `Campaign record` row reading
  `3.11.76`**, positioned above `Points 0/3` - a row this same cell's panel
  did not show before the race (`cellselect-grid0_3_2.png`, captured
  earlier this pass, shows only `Points`/`Target0..2`, no record row at
  all: `Cell_SavedRecord` has nothing to format before a first attempt).
  This refines `race-campaign.md`'s `CellSelection_PopulateDetail` table:
  the row it names `Best` (`Line7`, the *medal*) is not what a `Time Trial`
  cell shows on a medal-less run - `Campaign record` (`Line8`,
  `Cell_SavedRecord`'s own time) is what appears instead, once a
  record exists, corroborating the existing table's own "`Line5`/`Line8`
  share one offset with `Line7`" reading from a live, positive case rather
  than an absence.
- **`Points` stayed `0/3`, and no lock glyph moved anywhere on the
  hex** - all six previously-locked `grid0` cells are still locked, exactly
  matching a zero-medal, zero-points outcome under both the cell rule
  (`Cell_BestMedal(cell) == 0xff`, still true - no medal was ever earned)
  and the neighbour-fade rule (no adjacent cell gained a medal either).
- **`Grid Selection`'s `GRID 1` tier panel is byte-for-byte unchanged**:
  `Gold medals 0/8`, `Total points 000/024`, `Points needed 012` - the same
  three numbers this pass's own earlier capture read before the race. A
  medal-less run banks nothing towards `RequiredPoints`, as
  `Grid_PointsEarned`'s own definition (`sum of Cell_MedalPoints`, `0` when
  no medal) already predicts.
- **`DAT_08b30ffc` (the campaign cell pointer) is unchanged across the
  entire round trip**: read directly before the race (`0x08f61490`) and
  again after returning to `Cell Selection` (`0x08f61490`, identical) -
  the same value the 2026-09-14 launch-trace pass recorded for this exact
  cell on an earlier boot of the same disc, which is corroboration that
  `PI_Cell` records sit at fixed, parse-time addresses rather than
  per-session heap ones (the grid/cell table is built once at load and
  never reallocated). Not cleared by `Race End Photo` -> results ->
  `EndRace Menu` -> `Return to Grid`, so a reimplementation's own
  equivalent state does not need to null this out on the way back to the
  hex, only on actually leaving `Cell Selection` for a different one (or
  leaving the campaign entirely) if it does at all - not itself confirmed
  either way this pass.
- **The profile's gold/silver/bronze counters (`+0x160`/`+0x164`/`+0x168`)
  were not read.** No global "current profile" pointer is documented
  anywhere in `docs/ghidra/functions/psp-pulse-usa/*.md`, and finding one
  needs the Ghidra bridge (not held this pass, PPSSPP-only per this
  session's lane). Left as a genuine gap rather than skipped silently:
  the `EndRace Rewards` screen's own loyalty counter is the closest
  substitute this pass could read, and it is a per-ship value, not the
  profile-wide medal tally `race-campaign.md` describes.

## Wipeout HD/Fury: the same two screen names, off a different file, 2026-09-14

**Draws, off the real disc, both screens.** HD authors `Grid Selection`/`Cell
Selection` too, but in `Data\Plugins\Frontend\Gui\CellMode_Definition.xml`
([`oag_hd::campaign::SCREEN_ENTRY`](../../crates/hd/src/campaign.rs)) rather
than Pulse's `Data\Plugins\PI001\GUI\CellMode_Definition.xml`, at 1920x1080
rather than the PSP's 480x272, and `Grid Selection` **nests** `Cell
Selection` inside it in the XML rather than sitting beside it as a sibling.
`oag_ui::screen::Screens::collect` already flattens a nested `<Screen>` into
its own entry regardless of depth, so
[`oag_ui_screens::campaign::Layout::read_authored`](../../crates/ui-screens/src/campaign.rs)
reaches both by the same two names unmodified - the discriminating question
this thread opened with ("does `FlyerSelection` combine what Pulse splits
into two?") turned out to be no: read whole, the file is the same two-screen shape, just
authored differently inside each screen. Implemented in
[`oag_ui_screens::campaign::hd`](../../crates/ui-screens/src/campaign/hd.rs), wired through
`oag_game::campaign::load` (title-dispatched to a Wipeout HD/Fury branch the
same way the rest of this build picks a title's tables),
`crate::main::campaign_stage::CampaignStage::is_hd`/`circuit_names`/
`cell_grid_summary`, and a draw/pointer dispatch in `crate::main::menu_stage`/
`crate::main::session::pointer`. **The confirm/launch path is untouched** -
`Screen::Grid`/`Screen::Cell` and `oag_ui_screens::campaign::Event` are shared
between the two titles, so `session::campaign::handle_campaign`/
`launch_campaign_cell` need no HD arm of their own; an HD cell launches
through the identical code Pulse's already does. Captured headlessly with
`--menu-page grid-select`/`cell-select --size 1920x1080` against
`hdfury-ps3-eu-dec.iso` (German, this machine's settings language) - kept at
`/tmp/oag-drive/hd-campaign-grid-screen.md`'s own paths, not committed (game
content).

### `Grid Selection` is not a hex grid of tiers - it is a flyer pager

HD's own `<Screen type="FlyerSelection" name="Grid Selection">` pages **one**
tier at a time through `Flyer Left Arrow`/`Flyer Right Arrow` beside a 3-D
flyer model, not Pulse's four-hexes-per-page `GridController`. There is no
lock glyph per tile either - one `Flyer Pad Lock` overlays the whole flyer,
gated the same three-term way Pulse's own tile lock is
(`GridSelection::selected_is_locked`), reused unchanged and labelled
**"assumed to transfer to HD, unmeasured"** on the exact glyph, the same
qualifier `crate::campaign` module doc already carries for `Cell
Selection`'s own lock rule. `oag_ui_screens::campaign::GridSelection`'s model is
reused as-is - one flat `index`, `step`, `selected_is_locked` - since a
one-tier-at-a-time pager is exactly that shape; only the *draw* differs
(`hd_grid_draw_list`, not `draw::grid_draw_list`).

Measured off the widget names and the panel's own numbers:

| Thing | HD widget | Reads |
| --- | --- | --- |
| page counter | `EventNum`/`GridNum` | `"Event {:02}/{:02}"`, 1-based - the literal template both widgets author |
| points achieved | `Medals Title` (`RC_POINTSACH`) / `Points` | a bare number, `0` on a fresh profile. **Corrected 2026-09-30**: this row said `Grid_CountMedalsAtLeast`/`Grid_CellCount`, `"00/06"` - read off the XML's own placeholder shape, not a frame. Every settled RPCS3 frame of an unlocked tier reads a bare `0`. This build draws `points_earned`; the widget is named `Medals Title` and a fresh profile reads `0` under either, so which of the two it is stays unmeasured |
| points available | `Points Title` (`RC_TOTPOINTSAV`) / `TotPoints` | a bare `max_points`: `18` on `grid0`, `21` on Fury's `grid8` (RPCS3), no `000/` prefix |
| unlock reason | `Required`/`Required Previous` | two texts for two distinct lock reasons - see below |
| flyer lock | `Flyer Pad Lock` | gated on `selected_is_locked()` |

**`Required`/`Required Previous` are the unlock box's two texts, and since
2026-09-30 which one shows is measured, not chosen** - on eight settled RPCS3
frames of a fresh profile: an **unlocked** tier (`grid0`) shows `Required`
(`RC_POINTSTOUNL`, `"%d MORE POINTS NEEDED TO UNLOCK:"`) over the **next**
grid's logo; a **locked** one shows `Required Previous` over the **previous**
grid's, with `RC_POINTS_NEEDED` (`"%d MORE POINTS NEEDED IN:"`, and
`RC_POINT_NEEDED`, `"1 MORE POINT NEEDED IN:"`, which the string table ships
beside it) - the widget's authored `string="more points needed from previous"`
is a placeholder the code overwrites. The figure is the **previous** tier's
`RequiredPoints` less what it earned (on `grid0`, its own): `10`, `10`, `13`,
`16`, `19`, `22`, `25`, `30` over `grid0`..`grid7`, which are exactly the
eight `RequiredPoints` read one tier back. A Fury frame on a profile with three
points reads `9` where a fresh one reads `12`, so the subtraction is real. A
locked tier also draws **no** `POINTS ACHIEVED`/`TOTAL POINTS AVAILABLE` and
neither bullet arrow. **Still chosen, not measured**: an unlocked tier whose
own requirement is already met, and the last tier, show no box (only the
fresh-profile frames exist); `RC_POINT_NEEDED` is selected for a figure of 1.
Pinned by `oag_ui_screens::campaign::hd::unlock::tests`.

**Drawn since 2026-09-30** - see "The flyer behind `Grid Selection`" below.
The earlier reading that the per-grid `flyerlogo` path was "the same literal
string on every grid with no per-grid attribute to pick a different one" was
wrong: the literal is a default, and the executable builds the real path from
each grid's own `FlyerName`.

### The flyer behind `Grid Selection`, 2026-09-30

**A "flyer" is a promotional card, not a craft**: a wordmark, a stripe, a ship
silhouette and sponsor marks on a flat rectangle, with a reflection under it on
RPCS3. Every grid has one, the base campaign's flat and Fury's built from
layers. Drawn by `oag_game::flyer`, through the mesh-in-menu seam below, for
**all sixteen grids**; Wipeout HD's `Grid Selection` only.

**The mechanism is one, and it already existed in part.** `oag_game::preview`
drew a circuit's outline ribbon and (on Pulse) a team's hull behind the race
box's two screens, but only from a PSP or PS2 `.vex`: HD's own race-setup
picker does **not** draw its `<Model name="ShipModel">` (`preview_meshes` is
`false`, `oag_ui_screens::picker::hd` reads the pose and stops). Two changes made it
general, and `ShipModel` is unblocked by them but not wired:

- `preview::model` now reads a PS3 `.vex` + `.rcsmodel` pair, with every
  material and `.gtf` through the same `Archives` a race uses
  (`preview::model_named` also answers each texture slot's path);
- `Preview::draw_matrices` is `draw_mode3d`'s second half with the camera
  passed in, so a menu that owns a pose draws through the one pass.

**Layering.** `Preview` composites over a finished frame, which would bury
`Flyer Pad Lock`. `oag_game::flyer::render_list` splits the screen's draw list
at its backdrop: backdrop, then the card, then chrome and widgets. Live and
`--menu-page` share it. Headless check: `--menu-page grid-select-hd@3
--screenshot out.png --size 1920x1080 --no-audio` (base HD tier 3;
`grid-select@N` is Fury's tier `N`).

#### What the disc authors

| What | Value | Confidence |
| --- | --- | --- |
| widget | `<Flyer name="FlyerModel">`, top level (a sibling of the screens), `OriginX/Y` `960`/`540`, `nearZ` `1`, `farZ` `1000`, `obeySafeZone` `true`, `x` `80`, `y` `-33.3`, `z` `-200`, `RotX` `0`, `RotY` `1.5`, `RotationCentreOffsetX` `-60`, no `orthoScale` (the two `Campaign Selection` flyers carry `0.75` and `RotY` `0.6f`/`-0.6f`) | 95 |
| which model | the widget's `Src` is always `Data\FE\Flyers\00_flyer.vex`, a 1.6 KB placeholder (`cardShape`, `card_reflectShape`, two dummy textures the runtime swaps for the flyer's picture). The executable carries `Data/FE/Flyers/%s/`, `%sflyer.vex`, `%sflyer_Back.vex` and `Data\FE\Flyers\%s\Logo.gtf`, and every grid authors `FlyerName="01_uplift"` and so on: that is the `%s`. The class is `Flyer_Item` (`Flyer_Item.cpp`), a `Model_Item` (`Model_Item.cpp`); its loader `0x001a2ba8` formats the two paths and stores the front and back models at `+0x224`/`+0x228` | 85 (the formatting call site is read, the loop through it is not) |
| the flyer's own camera | every `flyer.vex` carries a `Transform` `camera1` parenting a `Camera` `cameraShape1` (`oag_vex::camera`): a translation `(0, 0, z)` with `z` `92.4957` on the eight base grids and `12.0` on Fury's eight and on both campaign cards. `Flyer_Item`'s loader also stores the camera node at `+0x22c`/`+0x230`, and its render function (`0x001a2510`) puts that node's matrix in the view stack. The `Camera` payload's `+0x1c` word is the card's aspect ratio to four digits (`1.5380` base, `1.5389` Fury, `1.0833` campaign) | 90 |
| the widget's field of view | `Flyer_Item`'s render function `0x001a2510` builds a perspective matrix from `tanf(0.5)` (a literal at `0x008acf4c`, the half angle) and the aspect global `1.7778`: a vertical field of view of **1.0 rad**. A planar fit of four base frames with the focal length left free lands on 0.90 to 1.03 rad independently. `Model_Item`'s own render function (`0x001d35e8`) takes `tanf(field * scale)` from a widget field instead | 90 |
| materials | the base campaign's are `basicnonalpha` (`TEX` then `MOV`, alpha 0, opaque), `basicalpha` and `scrollingalpha` (colour from a swatch `.gtf`, alpha from an elements atlas's red channel times a parameter): unlit. **Fury's are `simpletexture`, `simpletextureandtexturealpha` and `simpletextureandtexturealphauvoffsetscale`**, which compute `(constantAmbientColour + saturate(N.L) * directionalLight0Colour) * texture`, the light direction and both colours patched by name hash (`0x02df31e5`, `0x2dba643d`, `0x81db67ea`) | 90 (`ps3-microcode.py fp-file`) |
| loop | `Flyer_Item`'s constructor (`0x001a2060`) holds `6.0` at `+0x234` and `3.0` at `+0x238`, and the render function wraps a time past `6.0` back by `3.0`: the card loops over `[3, 6)`. Each node keeps its own `LoopEnd` (`6.0` on the base cards, `4.0` on Fury's) | 80 |
| animation | 33 `Anim Transform` nodes on `01_uplift`, 34 on `09_blitzed`; elements grow in (scale keys from 0.05) and slide to poses that overhang the frame | 90 |

Addresses are read in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`) and **not
renamed**: none has a `names.tsv` row.

#### What a card is: the flyer's camera image on a flat rectangle

**The card is flat, and the flyer scene is flattened through its own camera
before it is turned on the screen.** Four things in the settled RPCS3 frames
say so, and a fifth is that it reproduces them:

1. **Every frame starts at the same column.** Whatever the card is made of,
   on all fourteen settled frames, base campaign and Fury alike, the card's
   left edge is at raw pixel 742 to 746 (the dark `07_dropzone` needs a lower
   brightness threshold to read the same), which is authored 748 to 752. The
   stripes are cut mid-hatch there and the reflection under the card starts
   at it too, on flyers whose geometry has nothing else in common.
2. **The elements that overhang a flyer's frame are not there.** One node's
   last key puts a bar 90 units left of a base card and another a strip 160
   to its right; a Fury card's dark layers and its parked glitch panel sit
   outside its frame. RPCS3 shows a clean rectangle on all sixteen.
3. **Layers show no parallax.** A Fury card's wordmark is 8 units nearer the
   camera than its frame. Turned by the settled yaw as a 3-D stack it would
   slide 2 units sideways against the frame; measured on `impact`, the
   wordmark starts 82 px from the frame's edge in RPCS3, 80 px in a flat warp
   of the head-on image and 68 px in the fitted 3-D stack (two-thirds-scale
   comparison crops).
4. **The camera's image is the card.** The camera's `+0x1c` word is the
   card's aspect to four digits, and `hd_campaign`'s background is 22.2 by
   20.4 units at 32 from its camera, which at the window found below is
   exactly the picture's height.
5. **A flat card reproduces it.** One pose for all sixteen (below) puts the
   card's left edge at authored column 747, where the frames put it (750), and
   fits six frames at a mean correlation of 0.91.

So `oag_game::flyer::clip::flatten` puts every vertex where its own camera
sees it (`u = x / -z / window_tan * card_height / 2`), `clip::clip` cuts the
result to the card's rectangle, and the card stands in front of the widget's
camera at a pose. Every layer of a flyer is a quad at one depth, parallel to
the picture, so the flattening is affine per triangle and texture coordinates
need no correction. The nodes are posed first, at one moment
(`clip::bake`).

#### What is chosen, not measured

No confidence score: each was fitted to settled RPCS3 frames of `Grid
Selection` (base tiers 0, 1, 4, 5 and Fury tiers 0, 1; `1600x1200` raw, mapped
to the authored grid by the padlock's own bounding box, `0.94` and offset
`(39, 28.6)`), by warping the head-on render of the card's own camera image
onto the frame and maximising the correlation of blurred redness (Fury) or
luminance (base), inside the padlock's and the left column's exclusions.

| What | Value | Fitted |
| --- | --- | --- |
| pose (`GRID_POSE`) | yaw `-0.289`, centre `22.0` units right and `111.1` in front of the widget's camera, none up; in card units, one pose for all sixteen | four starts converge on yaw `-0.288..-0.298`, `tx 21.96..21.99`, `dist 111.3..111.6` |
| card height (`CARD_HEIGHT`) | 66.6 units, its width the camera's `+0x1c` times that (102.4) | the placeholder's `cardShape` spans 102.4 by 66.6 |
| window (`HD_WINDOW`, `FURY_WINDOW`) | tangent of half the vertical field of view the card shows of its camera's image: `0.346` on the base grids, `0.321` on Fury's and on the two `Campaign Selection` cards | fitted; the camera's `+0x20` word moves with the first two (`0x4f15`, `0x4a2c`; proportionality gives `0.344`, `0.323`) and **not** with the campaign cards' (`0x3621` would give `0.236`, where `hd_campaign`'s own geometry says `0.319`), so it is not what sets them |
| moment (`SETTLED_SECONDS`) | 3.0 s | the widget's own loop start, `[3, 6)`: after the elements are in, before the glitch a Fury card flashes at `3.63..4.0` s of its four-second loop, and with `10_impact`'s ship silhouette assembled (at 2.5 s it is still in pieces); the base cards differ from their old "latest key" moment by about 1 percent of pixels |

The widget's own `x y z`, `RotY` and `RotationCentreOffsetX` are **read and
not applied**: `RotY="1.5"` read as radians is an 86 degree turn, which is
what RPCS3 shows mid-transition, and the native code that settles it
(`Model_Item`'s update, `0x001d3f08`, reads `x y z` at `+0xac..+0xb4`, three
rotations at `+0xb8..+0xc0` and the pivot at `+0xc4`) is AltiVec-heavy and
unread. Trying them as start values needs that function.

**The window differs by 8 percent between the base grids and Fury's.** The base
camera frames the artwork (on `04_vertigo` the elements run to `+-48` units and
the window is `+-48.9` wide at the picture's own depth) and Fury's frames its
background plane exactly (`0.321` against `0.319` from geometry). The
proportionality with the camera's `+0x20` word that two points suggested fails
on the campaign cards, so the two windows are constants per campaign.

#### The shell: `00_flyer.vex` is the card, not a stand-in

The widget's `Src` is drawn by `Flyer_Item` as the card itself: `cardShape` (a
102.4 by 66.6 rectangle with a chamfered top-left corner and a few notches
along its left and right edges, 6,783.6 of the rectangle's 6,819.8 square
units, at `y` `0` to `66.6`) and `card_reflectShape` (`y` `-33.3` to `0`, the
lower half mirrored, its vertex alpha `0x4c` at the card's bottom edge, `0x26`
a third of the way down and `0` at the bottom). `oag_game::flyer::shell` reads
both: the front face's twenty-five triangles cut the flattened picture to the
card's outline (`clip::clip_to_shape`), and `clip::reflect` adds the mirrored
copy, alpha-blended and faded by the recovered alpha. The layout of the inline
chunks they sit in is on `docs/formats/rcsmodel.md`. The earlier lane's "two
chunks read at stride 0" was one of three surfaces: the scene builder already
drew `cardShape`'s front face (114 vertices) and the reflection; the 26-vertex
back face is the one with no recoverable stride.

#### Not drawn, and said so

- **The card's body colour.** The shell's front face is textured with
  `flyer_dummy_front.gtf`, a 128 by 128 green debug picture the runtime swaps
  for the flyer's picture (`flyer_dummy_` is a format string in the executable);
  ours draws no body under a card, so a flyer whose artwork does not cover its
  face shows black there where RPCS3 shows the mid-transition grey.
- **The glow around the card and the bloom on Fury's reds.** RPCS3's Fury
  frames are brighter and saturated where ours are the texture's own colour.
- **The light on Fury's cards.** Its materials are lit (above) and the three
  parameters are written by code nobody has found: their names sit in a table
  at `0x007b3658`/`0x007b36b8`/`0x007b36e0` whose only reference is another
  table, and the front end loads no circuit settings file. Drawn unlit
  (`lit = 0`), the ship silhouette on `10_impact` is flat white where RPCS3
  shades its facets.
- **The effects a Fury card plays over itself**, hidden by texture name
  (`UNREAD_EFFECTS`: `loops`, `flashes`, `noise_bar`, `failscreen`,
  `crash_screen`): a glitch panel with noise bars, rings and flashes, each a
  quad whose UV offset and scale or alpha a native parameter animates. Drawn
  at rest they came out as solid white discs, white bars and a brown static
  panel where RPCS3 shows nothing at three different moments.
- The flip to `flyer_back.vex` (its camera is 97.15 from the card on
  `01_uplift`, not 92.5), the elements animating in on a page change, the
  idle loop, and the tier change's card swing.
- `Campaign Selection`'s two cards draw through this path since the same day -
  see "The two cards".

Fury's cards were "not drawn" until 2026-09-30 on the reading that they were
"a lit box at a third of the scale": they are layered scenes framed from 12
units where the base cards' are framed from 92.5, and once each is seen through
its own camera the two families share one card.

**Omega: same assets by name, not drawable yet.** Omega's base package carries
`Data/fe/flyers/01_uplift/flyer.vex`, `flyer.rcsmodel`, `flyer_back.*` and
`logo.gnf` (353 flyer entries in `data00.psarc`), but its `.rcsmodel` opens with
`0xedad5cca`, PS4 little-endian, which HD's reader rejects (`version
0xedad5cca, expected 0x000a0000`; `crates/game/examples/omega_flyer_probe.rs`).
Its campaign screens also draw through Pulse's path, not `hd_grid_draw_list`. Not
wired: the Omega lane owns that reader.

**The unlock box and points figures were corrected on the same frames**, see the
paragraphs under the screen's widget table above.

Evidence: RPCS3 frames `data/scratch/hd-flyer/rpcs3-raw/grid-hd-t0..t7-settled`
and `grid-fury-t0/t1-settled` (Xvfb `:91`, silent config, untrimmed, 1600 by
1200); ours `data/scratch/hd-fury-cards/final/` and side by side
`data/scratch/hd-fury-cards/exp/cmp-m-*.png`; the joint fit
`data/scratch/hd-fury-cards/fit/joint_m2.py`. Pinned on the disc by
`crates/game/tests/hd_flyer_ground_truth.rs` (all sixteen cards decode, each
authors its camera, the widget's values); the flattening and the pose's left
edge by `oag_game::flyer`'s unit tests. The pose, the window and the moment
are chosen and are not pinned to the frames.

### `Cell Selection` is the same 32-slot staggered hex grid Pulse's is

`GridController name="Grid" OffsetX="170" OffsetY="230"`, `MaxX="7"
MaxY="5"` - the identical bounding shape and widget-name scheme
(`{Bg,Outline,Lock,Medal}_{x}_{y}`) Pulse's own `Cell Selection` authors,
just with a fourth layer (`Bg_x_y`, `Hexagon_HD_OUTLINE.mip`/`.gtf` - see
below) under Pulse's three. `oag_ui_screens::campaign::CellSelection` and
[`hex_rect`](../../crates/ui-screens/src/campaign.rs) (which already searches for a
"Medal_" or "Outline_" widget, both of which HD authors) are reused
unchanged; `Bg_x_y`/`Outline_x_y` both draw for every occupied slot the same
way Pulse's single `Outline_x_y` does, `Lock_x_y`/`Medal_x_y` gated the same
way. Confirmed by the capture: the hex grid, its padlocks and the selected
hex's medal-colour gate all draw correctly.

**The detail column and the target row are the same shape as Pulse's
`Line1`..`Line8`, spelled with real names instead of numbers** - `Event`/
`Track`/`Speed Class`/`Weapons` (each in its own `*Emblem` container) and
`RC Laps`/`Record`/`Points`/`Best` (a plain column, `RightColumnText`), plus
`Target0`..`2` under a `Target Title`, exactly matching Pulse's own naming
and gating (`targets_visible`/`weapons_visible`, the identical mode sets).
Unlike Pulse's screen, HD's own column shows the target row **and**
`RC Laps`/`Record`/`Points`/`Best` simultaneously, at non-overlapping
offsets - no `Line7`/`Target` mutual-exclusion swap idiom is needed here,
since nothing shares a position.

| Widget | Source | Notes |
| --- | --- | --- |
| `Event` | `crate::campaign::draw::cell_title` (reused) | the localised mode name |
| `Track` | [`hd_track_line`](../../crates/ui-screens/src/campaign/hd.rs) | through `oag_ui::language::CircuitNames`, not `strings.get_or_id` directly - see below |
| `Speed Class` | `cell.class` | shown for every mode, not gated on Zone the way Pulse's `Line1` is - nothing shares its slot |
| `Weapons` | `FE_ON`/`FE_OFF` | Race/Head2Head/Tournament only, same gate as Pulse |
| `Target0..2` | `Cell::targets_for_difficulty(model.difficulty())` | the selected difficulty rung - see below |
| `RC Laps`/`Points`/`Best` | reused Pulse helpers (`laps_line`/`medal_line`) | identical formatting |
| `Record` | not drawn | `Cell_SavedRecord`, no saved record kept - same absence as Pulse's `Line5`/`Line8` |

**A track's display name goes through `CircuitNames`, which Pulse's own
`track_line` doc says it never needs.** This is the screen that fold was
for: HD's circuit name table is a different archive copy from its circuit
*list* (`docs/formats/hd-frontend.md`), so `strings.get_or_id(&cell.track)`
alone would show the wrong name on eight circuits. `hd_track_line` folds
through `CircuitNames::get`, falling back to `strings.get_or_id` and then
the raw id. It is **not** `oag_raceplay::catalogue::label`: that function
appends `FE_REVERSE` where a reversed circuit shares its forward twin's name,
which the original never does on HD (`docs/formats/hd-frontend.md`: the drawn
name never carries a direction, and Track Select here draws the bare name
plus a `ReverseIcon`), so a reversed Fury cell (`grid9`..`grid14` carry
dozens) reads `TALON'S JUNCTION`, not `TALON'S JUNCTION REVERSE` - checked
2026-09-29 by comparing `label` with the bare fold over every cell's track
ids. The live session (`session::campaign::open_campaign`) and the headless
`--menu-page cell-select` capture both pass `Shell::circuit_names` (the
capture passed `CircuitNames::default()` until 2026-09-29 and so drew the raw
`17_Track` where the disc names `TALON'S JUNCTION`).

**A three-rung difficulty toggle, `DifficultyButton` - new state, backed by
an existing function.** `oag_tables::race_campaign::Cell::targets_for_difficulty`
already existed (added alongside the schema itself); nothing there changed.
What is new: `CellSelection::difficulty`/`cycle_difficulty`, a `0..=2`
field defaulting to `1` (medium - the rung `gold`/`silver`/`bronze` already
mean on a cell with no `difficulty_targets`, so it is inert on every Pulse
cell) and a wrapping step, bound to `Square` on the pad (free on this
screen in this build) and to a click on `DifficultyButton`'s own text via
[`difficulty_button_rect`](../../crates/ui-screens/src/campaign/hd.rs) - **an
invented rect**, since the widget is a `Text` with no authored width or
height, sized generously around its baseline rather than measured off a
capture.

### The archive's `.mip` naming is dead; the real files are `.gtf`

`CellMode_Definition.xml` spells every hex texture `src="Data\FE\Images\Hexagon_HD_OUTLINE.mip"`
- the PSP-era extension [`oag_hd::frontend::names::MENU_STRIP_CURSOR`]'s own
doc already found for `cursor.mip` - but **no archive on this disc carries a
`.mip` by that stem**. Measured directly: `oag_assets::psarc::Archive::paths`
over all seven archives on `hdfury-ps3-eu-dec.iso` lists every one of the
five hex textures as `.gtf`, lower-cased (`data/fe/images/hexagon_hd_outline.gtf`
and siblings), never as `.mip`. `oag_assets::psarc`'s own path normalisation
folds case and backslashes for the read, but `oag_hud::sprite::Sheet::get`
keys its placements by an exact string match against a widget's own
`image.src` - so `oag_hd::campaign::HEX_TEXTURES` is `(widget src, archive
path)` pairs: read the `.gtf` off the archive, shelve the decoded blob under
the `.mip` spelling the widget actually asks for at draw time.
`OTHER_TEXTURES` (the bullet arrows, the padlock) needs no such pairing -
every one of those is already spelled `.gtf` on both sides.

### Two shared-parser gaps this pass found and fixed, verified as no-ops elsewhere

Both in `oag_ui::screen::Screens::collect_widgets`
(`crates/ui/src/screen.rs`), both found by a garbled or missing picture on
the real capture rather than by reading the parser cold, and both re-run
against the full `oag-ui` test suite (331 tests, up from 322, zero
regressions) to check neither changes anything Pulse/Pure/HD's *other*
screens already depended on:

1. **A `Text` authoring its own `OffsetX`/`OffsetY` directly, with no `x`/
   `y` and no wrapping `<Item>`, used to position only its *children* by
   that offset, not itself.** `Medals Title`/`Points Title`/`EPoints Title`
   are the only three widgets in either HD screen shaped this way - every
   other positioned label sits inside an `<Item OffsetY="...">` instead,
   which already worked. The bug was invisible until this screen: two
   labels landing at `(x, 0)` read as one garbled overlapping string at the
   top of the frame. Fixed by positioning a `Text` at `inner` (its own
   offset folded in) rather than `offset` - a no-op wherever a `Text`
   authors neither, which is every widget measured before this pass.
2. **An `Image` used as a positioned container for widgets of its own was
   silently dropped, children and all.** HD's detail column
   (`<Image name="Event Emblem" OffsetX="630" OffsetY="340">`, nesting a
   `Text`/`Image`/`Text` triple) is authored the same way Pulse's own
   `Item`/`LeftLayer` containers are, but `collect_widgets`'s `"image"` arm
   had no recursion into a widget's own children at all - unlike every
   other container tag (`"text"`, `"item"`, `"leftlayer"`,
   `"gridcontroller"`, `"viewport"`, `"animation"`, `"screen"`). The whole
   detail column (`Event`/`Track`/`Speed Class`/`Weapons`) was simply
   absent from the capture until this was fixed, not merely mispositioned.
   Fixed the same way `"text"` already recurses: push the image itself,
   then walk its children at `inner`.

`crates/ui/src/screen/tests.rs` pins both
(`a_text_that_authors_its_own_offset_with_no_wrapping_item_positions_itself_by_it`,
`an_image_used_as_a_positioned_container_collects_its_own_children`).

### Which archive copy of a shared HD grid file is read: kept, and a real disagreement found

**Kept: this project's own `oag_assets::Archives::read_name` precedence**,
unchanged - `DATA02.PSARC`'s flat copy of `grid_00.xml`..`grid_07.xml`, the
same reading `crates/hd/src/campaign.rs`'s own module doc already
documents. The driving brief asked this be settled by measurement where
possible, RPCS3 otherwise; this pass took the offline half of that (diffing
the archive copies directly) and found something worth recording before
deciding whether to spend an RPCS3 boot on it:

**`DATA02`'s flat `Gold`/`Silver`/`Bronze` values equal `DATA04`/`DATA06`'s
per-difficulty copies' `hard` rung, not the `medium` one.** Four
non-dummy-valued cells checked in `grid_00.xml` (`grid0_1_2`, `grid0_2_2`,
`grid0_3_2`, `grid0_4_2`) all agree: e.g. `grid0_2_2`'s flat targets are
`11100`/`11400`/`12000`, matching `DATA04`/`DATA06`'s own `hard` rung
(`11100`/`11400`/`12000`) exactly, **not** `medium`
(`11400`/`11700`/`12300`). This contradicts
`oag_tables::race_campaign::Cell::gold`'s own doc comment, which reads
"on a cell with `difficulty_targets`, this is that rung's own `medium`
value" - true for a cell *authored* with the per-difficulty shape, but
`DATA02`'s flat copy of the same grid is a **separate file**, and its own
numbers are the hard rung's, not medium's. Not itself a bug to fix (both
readings are correct for the file each one parses); a genuine, measured
disagreement between the disc's own three copies of "the same" grid, left
for whoever picks up the RPCS3 half of this question. **Not RPCS3-verified
this pass** - which rung the real screen shows by default (if the flat and
per-difficulty copies really do disagree this way generally, "default" may
not even be `medium`) needs a boot this pass did not spend on it, per its
own budget; see [Open](#open) below.

### The stray hex outline was `Selector`, not a seventh cell - fixed 2026-09-21

**Not the sixth cell's own hex, the hypothesis this thread opened with -
`Selector` centred on the wrong rect.** `grid0`'s six cells
(`grid0_3_1`/`grid0_5_1`/`grid0_2_2`/`grid0_4_2`/`grid0_1_2`/`grid0_3_2`,
read directly off the real disc's `grid_00.xml` through
`Cell::grid_coords`) all resolve to one cohesive staggered cluster -
`grid0_5_1` sits one column right of the selected `grid0_3_1`, exactly
where `GridController`'s own per-column `<Item OffsetX>` places it, not
isolated. The stray outline sat well past that cluster, over the detail
column's own dead space, and moved with `Selector`'s own semi-transparent
`Hexagon_HD_THICK_OUT.mip` colour (`0x3fffffff`) - confirmed by
instrumenting the draw call directly rather than by reading the XML cold.

**Root cause: `hex_rect` (`crates/ui-screens/src/campaign.rs`) tried `Medal_{x}_{y}`
before `Outline_{x}_{y}`, and HD's `Medal_` widget is not hex-sized.** Both
prefixes are interchangeable on Pulse - `hex_filled.mip`/`hex_outline.mip`
are both a plain 32x32 hex, so whichever the function found first gave the
same rect. HD's own `Medal_{x}_{y}` sources `Hexmedal_HD.mip`, a shared
1024x256 multi-colour atlas, not a single hex, and authors no
`width`/`height` of its own to say otherwise - so `hex_rect` returned the
whole atlas's size, `[470, 332, 1024, 256]` for `grid0_5_1`'s own slot, as
"the hex's rect". `centred_selector_draw` then centred a 128x64 sprite
inside that box, landing it near `(805, 430)` - visually detached from
every real hex, which all sit inside `x∈[230,540], y∈[330,400]` on a
1920x1080 capture. **HD authors a `Medal_{x}_{y}` widget for every grid
slot in the 7x5 template, occupied or not** (`Bg_0_0`..`Bg_6_4` and
siblings, all declared unconditionally in `CellMode_Definition.xml`), so
this fired for every selected HD cell, not just this one - `grid0_2_2` and
every other cell tried during the RPCS3 pass below would have shown the
same detached cursor.

Fixed by swapping `hex_rect`'s own prefix order to `["Outline_",
"Medal_"]` - `Outline_` is always present and always single-hex-sized on
both titles, so it is the reliable source of a hex's own rect; `Medal_`
stays as a fallback for the same reason it was tried at all. Confirmed by
re-instrumenting the same draw call after the fix: `Selector` now resolves
to `hex=[350, 332, 128, 64]`, drawn at the identical rect - exactly
`Outline_3_1`'s own position and size, since `Selector`'s widget authors no
width/height of its own either and so takes the hex's exact size, making
the centring term zero. Confidence 95: reproduced directly (before/after
capture, `data/scratch/lane-hd/before-cell-select.png` vs
`after-cell-select.png`, this session's own files, not committed - game
content) and pinned by `hex_rect_prefers_outline_over_an_oversized_medal_atlas`
(`crates/ui-screens/src/campaign/tests.rs`), a synthetic fixture asserting
`hex_rect` resolves to `Outline_0_0`'s 32x32 rect rather than a
neighbouring `Medal_0_0`'s 1024x256 one at the same slot. Not full
confidence only because the real disc's `Selector` alpha blend was not
independently re-verified against RPCS3 in this pass - the position fix is
geometric and does not depend on that.

**Also affects HD's own hit-testing, not just the draw**: `hex_rect` is the
same rect `crate::campaign::pointer::hit` tests a click against
(`pointer::grid_targets`/`cell_targets`), shared with Pulse. The fix
shrinks every HD cell's own clickable box from the atlas's 1024x256 down to
the real 128x64 hex - a correctness fix there too, not just cosmetic;
before this, adjacent HD cells' giant hitboxes would have overlapped each
other's whole detail column. Pulse's own pointer tests are unaffected: 
their fixture returns the same 32x32 `Placed` for every `src`, so the
prefix swap is a no-op for them (`crates/ui-screens/src/campaign/tests.rs`'s own
353-test run stayed green). Nobody has clicked an HD cell in this build
before this pass - step 3 below is the first live exercise of this path.

**A related, not-yet-visible latent bug the same investigation found and
left open**: `tinted_medal_draw`/`sprite_draw` feed the same unauthored
`Medal_{x}_{y}` widgets through `image.width.unwrap_or(placed.width)` when
an actual medal *is* drawn (`model.medal_at(x, y).is_some()`), which on HD
means the same 1024x256 atlas rather than a cropped single-colour hex -
untested this pass because a fresh profile earns no medals, so
`tinted_medal_draw` was never reached for any HD cell captured. Whoever
next drives a podium finish on an HD cell (this thread's own step 3, or a
future pass) should check the medal glyph's own size once one exists to
earn - if it renders oversized the same way `Selector` did, `Hexmedal_HD.mip`
likely packs each colour as a horizontal frame `sprite_draw`'s `u`/`v`/
`texture_width`/`texture_height` fields would need to select, not something
`hex_rect`'s own fix touches.

### That latent bug was real and is fixed - 2026-09-27

**Confirmed by seeding a `records.toml` with earned medals rather than
waiting for a live podium finish** - `--menu-page cell-select` reads that
file the same "read, never write" way a live session does
(`crate::capture::campaign_page::campaign_page`'s own `records` parameter),
so three synthetic `[[campaign]]` rows (`grid8_2_1` gold, `grid8_4_1`
silver, `grid8_3_1` bronze - Fury's own first tier, the campaign this build
defaults to) put a real medal glyph on screen with no race actually run.
Before this fix, the whole 1024x256 `Hexmedal_HD` atlas - every tier, every
frame of what turned out to be a many-frame rotation strip - drew stretched
across each occupied hex and multiplied by a flat swatch on top,
`before-cell-select.png`'s own smear of overlapping medal renders spilling
past the hex grid into the detail column's own text. Not committed (game
content); reproducible with the same `--menu-page cell-select` capture.

**Root cause, precisely**: `hd_cell_draw_list`'s own `Medal_{x}_{y}` arm
called `tinted_medal_draw(image, placed, medal_argb(medal))`, and neither
half of that was right for HD. `tinted_medal_draw` (still correct for
Pulse's own plain-white `hex_filled.mip`) draws at `image.width.unwrap_or(placed.width)`
- the unauthored widget's own XML gives no size, so this fell back to the
*placed* size, which for `Hexmedal_HD.mip` is the whole atlas, not a hex.
And `medal_argb` multiplies a flat tier swatch over whatever draws - correct
for a plain white hex, actively wrong for an atlas frame that already
carries the tier's own baked-in colour.

**The fix, and the evidence pinning its numbers**: `oag_ui_screens::campaign::hd::hd_medal_frame`
crops one 60x60 frame of the correct tier and `hd_tinted_medal_draw` draws it
at the widget's own authored `image.x`/`image.y` (no tint) - unchanged from
the pre-fix code's own position, which was always right; only the size and
crop were wrong. **An earlier draft of this fix centred the crop on
`Outline_{x}_{y}`'s own hex instead**, the same `hex_rect` idiom `Selector`
already uses - plausible, but wrong: `CellMode_Definition.xml`'s own `<Item>`
grouping shows every `Medal_{x}_{y}` sits in its own item, offset a constant
`(+7, +2)` from `Bg_{x}_{y}`/`Outline_{x}_{y}`'s own item at the same slot,
checked across all seven columns. That is the disc's own registration
between the medal layer and the hex layer, authored once, not a per-column
tune a runtime centring formula could reproduce - the centred draft's own
`(+34, +2)` overshot the real `(+7, +2)` by 27px in `x` on every column
(`y` only agreed by coincidence: both `+2`, for unrelated reasons - the
disc's own constant on one side, half a 4px size delta on the other).
Reverted in favour of the authored position; see `hd_tinted_medal_draw`'s
own doc for the seven offset pairs and the visible before/after. The crop
rect is not a guess - `DATA06.PSARC`'s own
`Cell Selection` variant authors the identical crop on its own `Target0/1/2
Medal` widgets, the only place either archive authors a `u`/`v`/`TxtrWidth`/
`TxtrHeight` sub-rect of this texture at all: `width="60" height="60" u="0"`
on all three, `v="0"`/`"61"`/`"122"` for `Target0`/`1`/`2`, which this file's
own `hd_cell_draw_list` already reads as gold/silver/bronze respectively.
Confidence 90 on the crop rect (a real widget's own authored attributes for
the identical texture, on the same title's own screen family); confidence
95 that this was the actual bug (before/after capture, see below).
Cross-checked independently by decoding `Hexmedal_HD.gtf` directly
(`oag_texture::gtf::Gtf::parse`) and reading the raster back: the two
readings only agree once a Y-flip between this project's own raster order
and the GPU's `V` convention is accounted for (unmeasured which side is
"backwards" - not worth a second GTF reader to settle when the disc's own
widget already gives the numbers a caller needs). See
`crates/ui-screens/src/campaign/hd.rs`'s own `hd_medal_frame` doc for the full
reconciliation, and `crates/hd/tests/campaign_selection_ground_truth.rs`'s
`target_medal_widgets_author_the_hexmedal_atlas_crop_hd_medal_frame_reads`
for the disc-backed pin.

**Which rotation frame is "the" icon**: `u=0` matches `Target0 Medal`'s own
choice, and two independent RPCS3 captures agree it does not visibly
animate over the timescale a capture script's own button presses span -
`data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png` and `01-down.png`
(a different moment in the same drive script, `grid8_3_2`'s own `TARGET
200 (NOVICE)` row in both) show pixel-indistinguishable gold/silver/bronze
icons once cropped to the same window. Most of that directory's other
frames (`00-default.png`, `10-default.png`, `11-down.png`) are still too
interlace-corrupted mid-transition captures to use. **Not full
confidence**: those two comparable frames are themselves low-resolution
and compression-softened, so a flat, simple hexagon is all either shows -
neither clearly resolves the swirl/ribbon detail this section's own decoded
atlas frame carries, so this is consistent with a static `u=0` read but
does not independently confirm it is *this* atlas rather than some other
plain medal glyph. Left as "chosen, with two frames' worth of static
corroboration" rather than fully measured - still the same "one endpoint"
idiom `SELECTOR_TINT` uses, just less uncertain than before.

**Position correction, same day**: a first draft of this fix centred the
crop on `Outline_{x}_{y}`'s own hex rect (`hex_rect`, the same idiom
`Selector` already uses) rather than the widget's own authored position.
Wrong: `CellMode_Definition.xml`'s own `<Item>` grouping shows every
`Medal_{x}_{y}` sits in its own item, offset a **constant** `(+7, +2)` from
`Bg_{x}_{y}`/`Outline_{x}_{y}`'s own item at the same slot, checked across
all seven columns - the disc's own registration between the medal layer and
the hex layer, authored once, not a per-column tune. The centred draft's own
`(+34, +2)` (half the 60x60 crop's own delta from the 128x64 hex) overshot
the real `(+7, +2)` by 27px in `x` - `y` only agreed by coincidence (both
`+2`, for unrelated reasons: the disc's constant on one side, half a 4px
size delta on the other). `hd_tinted_medal_draw` now draws at the widget's
own `image.x`/`image.y` instead, matching the pre-fix code's own position
(which was always right - only the crop and tint were wrong).

**A second, real bug found while re-verifying against that same RPCS3
frame - fixed the same pass**: `EPoints Title` (`hd_cell_draw_list`) read
`format!("{:02}/{:02} POINTS", grid_summary.gold_medals, grid_summary.cell_count)`,
confused with `Grid Selection`'s own, different `Points`/`Medals Title`
field (which genuinely is `gold_medals`/`cell_count`). The disc's own
`<Values string="00/16 POINTS">` is a dummy placeholder, not the runtime
format; `02-square.png` (`grid8_3_2` selected, fresh zero-medal profile)
reads `"0/21 POINTS"` - not zero-padded, and `21` is `grid_summary.max_points`
(`3 * cell_count`), not `cell_count` itself. Fixed to
`format!("{}/{} POINTS", grid_summary.points_earned, grid_summary.max_points)`.
Confidence 90 on the **denominator and the padding** - one live capture on
the exact grid this thread already had a decoded `grid_08.xml` for
(`NitroElimElite/Skilled/Novice Target="200"` on `grid8_3_2` matches
`02-square.png`'s own `"TARGET 200 (NOVICE)"` exactly), giving `21` as
`max_points` unambiguously and confirming no zero-pad. **Lower on the
numerator's own field**: the capture is a fresh, zero-medal profile, so
`points_earned` and `gold_medals` both read `0` there and the frame alone
cannot tell the two apart - the choice of `points_earned` over `gold_medals`
rests on the `"POINTS"` label itself (`gold_medals` is a medal *count*, not
a point total) and on `Grid Selection`'s own sibling field `TotPoints`
already reading `points_earned`/`max_points` the same way, not on a second
capture with an earned medal. Not yet cross-checked against a second grid's
own numbers either way.

**A third finding, real but left open rather than fixed this pass**: that
same RPCS3 frame's `TARGET 200 (NOVICE)` row shows three lit, shaded medal
icons beside `1ST`/`2ND`/`3RD` - not the plain grey `Subtitle_Arrow_HD.gtf`
bullet `hd_cell_draw_list` draws today for `Target0/1/2 Image` (gated on
`targets_visible`, drawn unmodified via `image_draw`). An earlier read of
this pass's own attributed those icons to `MedalModelGold`/`Silver`/`Bronze`
(the 3-D trophy `<ImageModel>` `docs/formats/hd-endrace-screens.md`
documents for `Results`, already known **not collected** by this build's
screen parser at all) - wrong, caught by checking directly:
`CellMode_Definition.xml` (either archive's copy) authors no `ImageModel`/
`Trophies`/`MedalModel` anywhere on `Cell Selection` at all
(`grep -n 'ImageModel\|Trophies\|MedalModel'` over both, no match) - so
these are not 3-D trophies. **What they are instead is inferred, not
independently confirmed by the pixels themselves**: every `.xml` this
project holds an extracted copy of was greped for `Hexmedal`
(`data/scratch/lane-hd-sel/*.xml` and `data/scratch/hd-rewards/*.xml` -
not a full disc-wide sweep, only the front-end screens this project has
already pulled a copy of), and the *only* widgets that source it anywhere
on either archive's `Cell Selection` are `Medal_{x}_{y}` (both archives, no
crop authored) and `DATA06`'s own `Target0/1/2 Medal` (`width="60"
height="60" u="0"`, `v="0"/"61"/"122"` - the exact numbers `hd_medal_frame`
already reads as evidence, see above); nothing else on either screen names
a medal-shaped 2-D asset at all. The captured icon itself, zoomed, is too
compression-softened to independently confirm it carries the same
swirl/ribbon detail this section's own decoded atlas frame does - a plain,
flat hexagon is all either resolves - so the identification rests on
process of elimination (not a trophy, nothing else on the disc is a
plausible source) rather than a pixel match. Taken together with `DATA02`'s
own `Target0/1/2 Image` (`Subtitle_Arrow_HD.gtf`, no `Hexmedal_HD` reference
at all), this is first-party evidence the real PS3 renders `Cell Selection`
off `DATA06`, not `DATA02` - the same open question

**A second screen checked and ruled clean - `EndRace Results`,
2026-09-27**: `Target0/1/2 Image` on `EndRace Results`
(`data/scratch/hd-rewards/endrace-0{2..6}.xml`, every archive copy this
project has extracted) **does** author a real crop of `Hexmedal_HD.mip` -
`width="60" height="60" U="0"`, `V="0"/"61"/"122"` for `Target0`/`1`/`2`,
matching `hd_medal_frame`'s own numbers exactly (a second independent
confirmation, on a second screen - `DATA06`'s `Cell Selection` `Target0/1/2
Medal` was the first). A fourth widget on the same screen, `Single Target
Image` (for a mode with one target rather than three), authors the
identical `60x60`/`u=0`/`v=0` crop too - a third confirming instance of the
gold row specifically.

But none of it ever draws: **this is not a runtime skip, it is a
compile-time-fixed list**. `oag_hd::endrace::EXTRA_TEXTURES`
(`crates/hd/src/endrace.rs`) is the complete set of textures
`oag_game::endrace::load_hd` extends the front-end's base sheet with for
this screen, and it names exactly one file - `Title_Arrow_HD.gtf` -
`Hexmedal_HD` is not in it, on any code path: the same constant builds the
sheet whether the caller is a live session (`crate::main::session::endrace`,
which passes `shell.sprites` - the plain front-end sheet, not the
campaign-extended one `renderer_set_sprites` only ever uploads to the GPU
transiently while campaign screens are up) or `--menu-page endrace-results`.
So `hd_results_draw_list`'s own image loop (`crates/ui-screens/src/endrace/hd.rs`)
finds no sprite for any of these four widgets and skips them (`sprites(&image.src)`
answers `None`), the same absence the sibling `Target0`/`1`/`2` *text*
widgets are already explicitly given. Net effect: the whole target row
draws nothing on `Results`, in any session, not a wrong render - an honest
absence (`--menu-page endrace-results`, checked directly,
`data/scratch/drive-2026-09-27/hd-medals/endrace-results.png`), not a
second instance of this bug. `EndRace Rewards`'s own XML has no `Hexmedal`
reference at all, on any copy - nothing to check there.
`docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`"
section below already measured for that screen alone, now with a second,
independent data point pointing the same way for `Cell Selection` too.
**Adopted, 2026-09-27**: `oag_game::campaign::load_hd` now reads `DATA06`'s
copy of `CellMode_Definition.xml` for `Grid Selection`/`Cell Selection` too,
not only `Campaign Selection`/`Grid Selection Fury` - see "Wipeout HD/Fury:
the TARGET block reads `DATA06` too" below for the full argument (a general
last-wins archive-overlay rule plus this section's own per-widget RPCS3
confirmation) and what changed once the switch landed (`bBg_x_y`, the
repositioned `GridController`, the `Target0/1/2 Medal` icon row, the
reflowed detail column, and the mode-dependent `TARGET` header text a fresh
capture round found once the medals were finally visible to check against).

**Reproducing any of this without a real podium finish**: `--menu-page
cell-select` reads `<config dir>/oag/records.toml` the same "read, never
write" way a live session does. Point `XDG_CONFIG_HOME` at a scratch
directory with its own `oag/records.toml` (`dirs::config_dir()` honours it)
rather than editing the real, shared `~/.config/oag/records.toml` - e.g.:

```sh
mkdir -p data/scratch/hd-medals/scratch-cfg/oag
cat > data/scratch/hd-medals/scratch-cfg/oag/records.toml <<'EOF'
[[campaign]]
title = "wipeout hd"
cell = "grid8_2_1"
best_medal = "gold"
best_difficulty = "hard"
EOF
XDG_CONFIG_HOME=data/scratch/hd-medals/scratch-cfg cargo run -q -p oag-game -- \
  data/images/hdfury-ps3-eu-dec.iso --menu-page cell-select --no-audio \
  --screenshot data/scratch/hd-medals/cell-select.png
```

## One shape per difficulty, not just one colour per tier - confirmed 2026-09-28

**The maintainer's own play observation - HD medals reflect three
difficulties, and each difficulty's own medal has a different icon shape -
is confirmed, measured directly off the disc's own texture data, not
inferred from play alone.**

### The atlas itself: two archives, two different heights

`Data\FE\Images\Hexmedal_HD.gtf` (and its `.mip`-spelled sibling
`Medal_{x}_{y}` sources - see [`hd_medal_frame`](../../crates/ui-screens/src/campaign/hd.rs)'s
own doc) exists on two archives that disagree, the same shape
`CellMode_Definition.xml`'s own `DATA02`/`DATA06` split already established
for the screen XML:

| Archive | Size | Decodes to | Content |
| --- | --- | --- | --- |
| `DATA02.PSARC` | 262,272 bytes | `1024x256` | One icon shape (a swirl/spiral emblem), three tier rows (gold/silver/bronze, `v=0/61/122`, `TxtrHeight=60`) - the flat-schema archive's own copy. |
| `DATA04.PSARC` | 786,560 bytes | `1024x768` - **exactly 3x** | Three `1024x256`-shaped blocks stacked, each with its own three tier rows at the same `v=0/61/122` spacing *within its own block* - the per-difficulty archive's own copy, carried alongside its `grid_00.xml`..`grid_07.xml` (`docs/formats/race-campaign.md`'s HD archive table). |

Measured directly: `oag-unpack extract` off `hdfury-ps3-eu-dec.iso`, then
`oag-assets`' own `psarc_cat` example against each archive's
`/data/fe/images/hexmedal_hd.gtf`, then `oag-texture`'s own `gtf_to_png`
example to decode and view both. `DATA00`/`DATA06` carry **no** copy of
this file at all - only `DATA02` and `DATA04` do, which is what makes
`oag_assets::Archives::read_name`'s ordinary precedence (searches the bulk
archive first) matter: it lands on `DATA02`'s shorter copy, not `DATA04`'s.

The three blocks hold visibly different icon **shapes**, not only colour -
read off `DATA04`'s own decoded atlas, row-content bands measured
programmatically (alpha-per-row) rather than eyeballed:

- A **plain, unadorned hex** - the block nearest the raster's own bottom
  edge, `v=0..182` after the Y-flip `hd_medal_frame`'s own doc already
  reconciles for the shorter atlas.
- A **hook/"cane"-shaped emblem** - the middle block, `v=183..365`.
- A **swirl/spiral emblem** - the block farthest from the raster's bottom
  edge, `v=366..548`. This is the *only* shape `DATA02`'s shorter, flat
  atlas carries.

Block pitch (`183` - three `60`px tier rows at `61`-pitch each, `3*61 =
183`) is measured off the decoded atlas's own content bands, not an
authored crop: no widget on either archive's `CellMode_Definition.xml`
sources a `v` past `122`, so there is nothing to cross-check this specific
number against the way the `61` tier spacing is already cross-checked
(`Target0/1/2 Medal`'s own authored `width="60" height="60" u="0"
v="0"/"61"/"122"`). Confidence 60 on the `183` pitch itself.

### Which block is which difficulty: measured on RPCS3, 2026-09-28

**Falsifier check, explicit**: the observation would be falsified if the
atlas held only three frames (it holds many more - see the existing
"many-frame rotation strip" finding below) or if a live screen showed the
same icon at every difficulty. Neither happened - the atlas genuinely
carries three shapes, and this project's own render (below) shows three
different shapes drawn for three different stored difficulties.

**Confirmed on a live RPCS3 frame.** `easy/novice = plain hex (v-block 0)`,
`medium/skilled = cane (v-block 1)`, `hard/elite = swirl (v-block 2)` - the
mapping this section previously reasoned to from convergent evidence alone -
is now a direct capture read: `browse --screen "Cell Selection" --button
triangle --steps 6` (recipe and full navigation notes in
`docs/reverse-engineering/rpcs3-capture.md`'s "Cell Selection:
`DifficultyButton` toggle, per-rung icon shape" section) landed on `grid8_3_1`
(`Fury`, `Race`, Talon's Junction, Venom, weapons on, 3 laps - the
first-reached cell on a default `Main Menu` -> `Cell Selection` walk with no
d-pad input) and cycled its `Target0/1/2 Medal` row through two full
`NOVICE -> SKILLED -> ELITE` cycles. Reading is by the frame's own `AI
DIFFICULTY (<rung>)` footer text, not press count (face-button presses drop
at ~9 fps and leave no `TTY.log` line):

| Rung (footer text) | Icon shape | Frames |
| --- | --- | --- |
| `NOVICE` | plain hex | `03.png`, `06.png` |
| `SKILLED` | cane/hook | `01.png`, `04.png` |
| `ELITE` | swirl | `02.png`, `05.png` |

Two independent frames per rung, one boot, one cell, one mode (`Race`) -
confidence 90, not higher, per `hd_medal_frame`'s own doc comment. This
project's own render of the identical cell at its fixed default browsed rung
(`--menu-page cell-select`, `CellSelection::new`'s `Difficulty::Medium`
default) draws the same cane shape at `TARGET (SKILLED)` - an internal
consistency check, not a second independent capture. **`00.png` (the
unpressed frame) is not usable evidence of anything**: it is a comb-artifact
mid-transition capture (`browse` screenshots it with no settle at all - see
`rpcs3-capture.md`'s own account), and inferring its rung from the three-rung
cycle's own periodicity against `01.png` would assume the first
`DifficultyButton` press did not drop, which is exactly the kind of
press-count reasoning this same page's own "pair by footer text, not press
count" rule exists to rule out. The pre-press rung on this boot is simply
unknown. **Not reproduced this pass**: a second boot, or an
`Elimination`/`NitroBattle` cell (whose own target triple is separately
named `Novice`/`Skilled`/`Elite` rather than `1st`/`2nd`/`3rd` - see
"measured on RPCS3" above); `browse`'s own `--nav` plan is dead once
`--screen` is reached (breaks the walk loop before `navigate()` fires), so
reaching a *different* cell than the default needs a different driver than
the one used here.

**A second, unplanned finding: the footer's `DIFFICULTY` prompt is
mode-dependent text, not the fixed string the 2026-09-25 section below
assumed.** Every frame from this pass's own `grid8_3_1` boot (`Race`,
`TARGET (<rung>)`, `1st`/`2nd`/`3rd`) reads **`AI DIFFICULTY (<rung>)`**, not
the bare `DIFFICULTY (<rung>)` the 2026-09-25 section records - and that is
not a crop that clipped an `AI` off: re-checking that section's own source
frames (`data/scratch/lane-hd/rpcs3-grid0-3-2/02-triangle.png`,
`data/scratch/drive-2026-09-25/hd-footer-glyphs/difficulty-icon-zoom.png`)
directly, both are `grid8_3_2` (`Eliminator`, The Amphiseum, `TARGET 200
(<rung>)`), and both genuinely read the bare `DIFFICULTY (<rung>)` with no
`AI` - a wider crop of the same frame confirms nothing is cut off to its
left. A third, independent, pre-existing capture of `grid8_3_1` itself -
`data/reference/hd-capture/talons-matched/screen-Cell-Selection.png`
(2026-09-13, a different boot entirely) - also reads `AI DIFFICULTY
(NOVICE)`, agreeing with this pass's own `grid8_3_1` reading rather than the
2026-09-25 `grid8_3_2` one. So the pattern across three boots is consistent
with the prompt depending on the cell's own mode (`AI DIFFICULTY` for
`Race`, bare `DIFFICULTY` for `Eliminator`) rather than being one fixed
string - not itself confirmed against a third mode family, and not chased
further this pass, but recorded here rather than silently generalised from
either single-mode reading. **Left for the lead to route**: this bears
directly on `pulse-cellsel`'s own `DIFFICULTY (<rung>)` runtime-prompt work,
which this section's own 2026-09-25 write-up (below) is the source this
pass found the mismatch against.

The convergent-evidence reasoning that predicted this mapping before any
capture existed: `DATA02`'s shorter, flat-schema copy - the one every
pre-Fury cell effectively used before per-difficulty targets existed -
carries *only* the swirl shape, and
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
`SaveData_MigrateCellMedalsToHardElite` credits exactly that pre-existing,
difficulty-less medal at `HARD`/`ELITE` when migrating an old save. The
swirl icon and the hardest rung were the two things that pre-existed the
difficulty split, and the migration equates them - a reading two independent
facts agreed on, now corroborated rather than merely argued for.

**This pass's own boot did not run on a fresh profile and says nothing about
the default rung.** An existing (non-fresh) save was on disk - moving it
aside was refused by this session's own permission classifier as an edit to
the user's real `~/.config/rpcs3` outside the repo - and `00.png` (before
any `DifficultyButton` press) is a comb-artifact frame per the note above,
unreadable regardless of the profile. A separate, pre-existing capture does
carry a clean arrival reading of this same cell: `data/reference/hd-capture/talons-matched/screen-Cell-Selection.png`
(2026-09-13, a settled `--nav-shots` frame taken before any
`DifficultyButton` press) reads `AI DIFFICULTY (NOVICE)` on `grid8_3_1` -
see `docs/reverse-engineering/rpcs3-capture.md`'s own account for why that
frame is trustworthy where this pass's `00.png` is not. Whether *that*
profile was itself fresh is unverified, so this corroborates rather than
settles the earlier "fresh-profile default reads `NOVICE`" finding.
`CellSelection::difficulty`'s own `Difficulty::Medium` (`SKILLED`) default
versus RPCS3's `NOVICE` stays open, per the 2026-09-25 section below - not
this pass's to fix.

### The fix, and this project's own render as an internal consistency check

Two bugs, both in `oag_game::campaign::load_hd` (`crates/game/src/campaign.rs`),
kept this build drawing `DATA02`'s shorter atlas everywhere despite the
taller one being on disc:

1. **`Target0/1/2 Medal`** (spelled `.gtf`, read through `OTHER_TEXTURES`)
   and **`Medal_{x}_{y}`** (spelled `.mip`, read through `HEX_TEXTURES`) are
   two different shelvings of the *same* archive file, in two different
   loops - fixing the archive precedence in one and not the other still
   left one of the two widgets drawing off the wrong copy. Both loops now
   share one `read_hd_texture` helper that asks `DATA04` for this one path
   by name (`Archives::read_every_name`, filtered by archive label) rather
   than taking whichever archive `read_name`'s ordinary "bulk archive
   first" precedence happens to try first.
2. `oag_ui_screens::campaign::hd::hd_medal_frame` took only a medal tier, always
   cropping `v-block 0` - the atlas's *height* changed but nothing selected
   a different block. It now takes a [`Difficulty`](../../crates/tables/src/race_campaign.rs)
   too: the currently-browsed one for `Target0/1/2 Medal`'s own crop (that
   row is a live target-threshold indicator, not a saved result), and the
   cell's own earned difficulty for `Medal_{x}_{y}`'s (the grid overview
   badge, keyed on `oag_game::records::CampaignRecord::best_difficulty` -
   see that field's own doc for the storage model this project chose, and
   why it is chosen rather than measured).

**Not independent confirmation of the mapping** - this project's own crop
math matching the atlas's own content bands proves the *code* is internally
consistent, not that the original draws Easy as the plain hex - but it is a
useful end-to-end check that the archive fix, the block arithmetic and the
difficulty plumbing agree with each other. Seeding
`data/scratch/hd-medals/scratch-cfg/oag/records.toml` with
`best_difficulty = "easy"`/`"medium"`/`"hard"` on `grid8_2_1` and rendering
`--menu-page cell-select` (recipe above) drew the plain hex, the cane and
the swirl respectively on the `Medal_{x}_{y}` grid badge (keyed on the
seeded `best_difficulty`) - not committed (game content, `data/scratch/` is
gitignored), reproducible with the recipe above.
**Correction, 2026-09-28**: the `Target0/1/2 Medal` row does **not** vary
with the seeded record the way the paragraph here previously claimed.
`model.difficulty()` is `CellSelection`'s own single browsed-rung field, and
`--menu-page cell-select` has no session to cycle `DifficultyButton` through
at all - `CellSelection::new` hardwires it to `Difficulty::Medium`
(`crates/ui-screens/src/campaign.rs`), so every `--menu-page cell-select` render
draws `Target0/1/2 Medal` at the cane shape regardless of what
`best_difficulty` a scratch `records.toml` seeds. Only `Medal_{x}_{y}` reads
the seeded value. This pass's own render
(`data/scratch/hd-campaign-live/ours-cellselect-default.png`) is the
`Target0/1/2 Medal` row's actual internal-consistency check - one rung
(`Medium`/`SKILLED`), matching the live RPCS3 read above at that same rung.

## Wipeout HD/Fury: the TARGET block reads `DATA06` too, 2026-09-27

The maintainer's own "medals render off/wrong" report had a second cause
flagged but not chased by the pass above: `Cell Selection`'s `Target0/1/2
Image` drew a plain grey `Subtitle_Arrow_HD.gtf` bullet on every
`TimeTrial`/`Zone`/`Elimination`/`SpeedLap` cell, needing no earned medal to
see - unlike the hex-badge bug, visible on every fresh profile. This pass
closes it by switching `Cell Selection`'s own screen source, not by grafting
one widget onto the old screen.

### Archive precedence, settled on paper before RPCS3 booted

`CellMode_Definition.xml` is one path on two archives (`DATA02`, `DATA06`).
`oag_hd::campaign::SCREEN_ENTRY`'s own doc already had `DATA06` as a later,
Fury-era build (measured: it alone carries `Campaign Selection`/`Grid
Selection Fury`), but stopped short of the general rule that would extend
that to `Cell Selection` too. This section's own "`DATA00`'s copy is the
live one" measurement gives it: `TTY.log`'s own archive load order is
`data01, data02, data03, data04, data05, data06, data00` - `DATA06` loads
after `DATA02`, and `DATA00`'s own load-last, its-own-copy-wins behaviour
(confirmed by a `sys_fs_stat` probe naming a file only `DATA00`'s `skin.xml`
has) is evidence of a **last-wins overlay**, not just a discovery order.
`CellMode_Definition.xml` has no `DATA00` copy at all, so among the two
archives that do carry it, `DATA06` - loading after `DATA02` - wins the
same way. Confidence 85, from two converging lines: this general rule, and
a direct per-screen confirmation - the medal icons below only exist on
`DATA06`'s own `Target0/1/2 Medal` widget, and a live RPCS3 frame shows
them.

`oag_game::campaign::load_hd` (`crates/game/src/campaign.rs`) now reads
`DATA06`'s copy for `Grid Selection`/`Cell Selection` directly (by archive
label, the same way it already did for `Campaign Selection`/`Grid Selection
Fury`), one parse serving all four screens. Not a merge: `DATA06`'s `Cell
Selection` draws *whole* - its own `bBg_x_y` layer, its own repositioned
`GridController` (`OffsetX="240" OffsetY="370"`, was `"170"`/`"230"`), its
own `Event`/`Track`/`Speed Class`/`Weapons` emblem layout and
`RightColumnText` panel - not a graft of `DATA06`'s medal icons onto
`DATA02`'s otherwise-unchanged screen (`oag_ui_screens::campaign::hd`'s own module
doc, "The winning archive", has the complete widget diff). Every
`OffsetX`/`OffsetY` difference draws correctly with no code change at all:
`oag_ui::screen`'s widget collector already folds offsets generically, and
the new `<Bracket>` borders around the emblems and the target row have no
parser arm (the same "silently dropped, not drawn wrong" rule
`oag_ui_screens::campaign::selection` already established for `Campaign
Selection`'s own `Bracket`).

**Left unswitched, deliberately**: the sixteen `grid_00.xml`..`grid_15.xml`
grid files still read through the unchanged archive precedence (`DATA02`'s
flat-schema copy for `grid_00`..`07`), even though the same rule argues
`DATA06`'s per-difficulty copy is equally live - see "A base-HD grid's own
precedence, not settled this pass" below. Switching a screen's widgets and
switching the campaign's own medal-law numbers are different-sized changes;
only the first was in this lane's scope.

### What actually needed fixing beyond the archive switch

Three real bugs surfaced once the medal icons were finally visible to check
against a live frame, all in `oag_ui_screens::campaign::hd::hd_cell_draw_list`
(`crates/ui-screens/src/campaign/hd.rs`) - see that file's own doc comments on
`hd_target_title`/`hd_target_value`/`hd_format_centiseconds` for the full
capture-by-capture evidence:

1. **`Target0/1/2 Medal` looked up its own texture and found nothing.** The
   widget spells its `src` as `Data\FE\Images\Hexmedal_HD.gtf`, but this
   build's sprite sheet only shelved that file under the `.mip` spelling
   `Medal_{x}_{y}` uses (`oag_hd::campaign::HEX_TEXTURES`'s own rewrite).
   Fixed by adding the `.gtf` spelling to `oag_hd::campaign::OTHER_TEXTURES`
   too - a second, deliberate shelving of the same decoded texture under
   the second spelling a widget actually asks for.
2. **`weapons_visible`/`targets_visible` excluded modes the real screen
   shows them on.** `grid8_3_1` (`Race`) shows a `TARGET (NOVICE)` row the
   pre-switch code's own mode list excluded; `grid8_3_2` (`Elimination`)
   shows a `WEAPONS ON` row a different mode list excluded. Three of three
   sampled cells across three different modes (`Race`, `Speed Lap`,
   `Elimination`) show both rows, and every cell on the disc authors both
   fields regardless of mode - both gates are unconditional now.
3. **The TARGET header and its three values are mode-dependent text, not
   one idstring plus a bare number.** `grid8_3_1`/`grid8_4_2`/`grid8_3_2`
   read `"TARGET (NOVICE)"`/`"TARGET LAP TIME (NOVICE)"`/`"TARGET 200
   (NOVICE)"` and `"1ST"/"2ND"/"3RD"` beside gold/silver/bronze icons - none
   of which the pre-switch `target_value`/a bare `IG_HUD_TARGET` read could
   produce. `hd_target_title`/`hd_target_value` compose it from the mode,
   the difficulty rung and (for `Elimination`/`NitroBattle` -
   `campaign_grids_ground_truth.rs`'s own
   `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`
   ground-truths that pairing against the real disc) the cell's own nitro
   target.

### Captures

`data/scratch/drive-2026-09-27/hd-targets/`: `fury-race-3-1.png` (`grid8_3_1`,
`Race`), `fury-speedlap-4-2.png` (`grid8_4_2`, `Speed Lap`) - both fresh RPCS3
captures this pass took (own config, Xvfb `:92`, audio off); `grid8_3_2`
(`Elimination`) reuses the earlier pass's own
`data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png`, not recaptured. This
build's own before/after: `after-cell-select.png` (medal icon missing, the
`.gtf`/`.mip` bug above, on `grid8_2_1`/`NitroBattle`) through
`after-cell-select-4.png` (final: `TARGET 15 (SKILLED)` header, `NitroElimSkilled
Target="15"` on `grid_08.xml` matching exactly, three medal icons with
`1ST`/`2ND`/`3RD`) - the four-shot sequence pins each fix in turn rather than
only the end state. Not committed (game content).

### A base-HD grid's own precedence, not settled this pass

`grid_00.xml`'s `DATA06` copy (per-difficulty `Time Trial`/`Zone`/`Speed
Lap` targets, e.g. `EasyGold Target="12000"` on `grid0_2_2`) was read
directly this pass to compare against `DATA02`'s flat copy - the same
"which archive a base-`Wipeout HD` grid actually reads" question
`oag_hd::campaign::SCREEN_ENTRY`'s own doc leaves open for the grid files.
No base-HD `Time Trial` cell was captured on RPCS3 this pass (only Fury's
own `grid8` cells were reached), so this remains unmeasured, exactly where
the archive-precedence pass before this one left it - reported, not acted
on, per this lane's own scope.

### Pointer

`Cell Selection`'s own hex click targets (`oag_ui_screens::campaign::pointer::cell_targets`)
read the same generically-parsed `Outline_x_y`/`Bg_x_y` widget positions
this section's own screenshots confirm draw correctly at their new
`DATA06`-authored offset - the geometry is shared between drawing and
hit-testing, not duplicated, so nothing here changes independently of what
the captures above already show. The existing pointer test suite (95 tests
under `oag_ui_screens::campaign`) is unchanged and green. Not independently
exercised with a live mouse click this pass - `--press right` produced no
visible navigation in a `--menu-page cell-select` capture, which reads as a
capture-tooling gap (this debug flag's own d-pad mapping on this screen, not
exercised anywhere this project's own docs confirm working) rather than a
regression, since nothing pointer-specific changed in this lane's own code.

## Wipeout HD/Fury: `Campaign Selection`, 2026-09-21

**Modelled and driven, off the disc's own XML.** The 2026-09-21 RPCS3 pass
under "Open" below found the screen exists and defaults to `Fury`, but left
its own widgets unread and its toggle to the base `Wipeout HD` campaign
unmeasured. This pass read the screen directly, confirmed the toggle on
RPCS3, and wired `oag_ui_screens::campaign::selection` + `crate::campaign_stage`
ahead of `Grid Selection` for HD only - Pulse's flow is untouched, and its
own ground-truth tests still pin a straight `RACE CAMPAIGN` -> `Grid
Selection` boot.

### Where the screen actually lives: `DATA06`, not the archive this build already reads from

`oag_hd::campaign::SCREEN_ENTRY`'s own doc already recorded two disagreeing
copies of `CellMode_Definition.xml` - `DATA02` (42,548 bytes, the one
`oag_assets::Archives::read_name`'s precedence reaches) and `DATA06` (59,361
bytes, "presumably a later build", undiffed until now. Reading both off
`hdfury-ps3-eu-dec.iso` directly (`archives.read_every_name`, the same
"choose the copy that actually carries what's needed" idiom
`oag_game::boot::load_circuit_names` already uses for the same kind of
disagreement on a different file) settles it:

- **`DATA02`'s copy has no `Campaign Selection` screen at all**, and its own
  `Grid Selection`'s `flyerlist` carries eight `<Entry IDString="BLANK">`
  placeholders rather than real grid names - consistent with being the
  pre-Fury base game's own copy.
- **`DATA06`'s copy has all four screens**: `Campaign Selection`, `Grid
  Selection` (real `<Entry String="grid0">`..`"grid7">`), `Grid Selection
  Fury` (`"grid8">`..`"grid15">`), `Cell Selection` - confirmed structurally
  (`Screens::collect` reaches all four by name off this file, including
  through the two-top-level-`<Screen>` shape `Campaign Selection` and the
  anonymous wrapper around the other three sit in) and empirically: RPCS3's
  own `TTY.log` prints `Switching Screen "Campaign Selection" to "Grid
  Selection Fury"` on this exact disc, a screen name that only exists in
  `DATA06`'s copy. **The running PS3 reads `DATA06`'s copy of this file**,
  which is measured, not chosen.
- **`DATA06`'s own `Grid Selection` (base) is otherwise widget-identical to
  `DATA02`'s** - same widget names, same positions, same sizes, diffed
  line-by-line. The only differences are two `FEGlobals->` colour
  indirections in place of `DATA02`'s literal/differently-named ones
  (`TitleColor` vs `HD_Grey`, which `crates/ui/src/menu/skin.rs`'s own doc
  already records as agreeing on this title, both `0xFF646464`; a literal
  `0xffdedede` vs `FEGlobals->HD_LightGrey`) and the real `flyerlist` entries
  noted above, which this build never draws (`y="-500"`, authored off
  screen). This pass (2026-09-21) read `Campaign Selection` and `Grid
  Selection Fury` from `DATA06` and left `Grid Selection`/`Cell Selection`
  on `DATA02` - **chosen, not measured**, on the safe side of a real, larger
  divergence in `Cell Selection` (an extra `bBg_x_y` background layer under
  the hex grid; `Target0/1/2 Image` + `Target0/1/2 Title` replaced by one
  shared `Target Title` header and per-rung `Target0/1/2 Medal` icons
  sourcing `Hexmedal_HD.gtf`; plus new `GridTopBar`/`chooserace
  Arrow`/`NextPoints Arrow`/`Track Reverse` widgets).
  **Superseded, 2026-09-27**: `hd_cell_draw_list` reads `DATA06` for both
  screens now - see "Wipeout HD/Fury: the TARGET block reads `DATA06` too"
  below for the archive-precedence argument that closed this out and what
  the divergence turned out to mean once it actually drew.

### The screen itself

`<Screen name="Campaign Selection" type="CampaignSelection">`, read via
[`oag_ui_screens::campaign::selection`](../../crates/ui-screens/src/campaign/selection.rs):

| Widget | Reads |
| --- | --- |
| `ScreenTitle` | idstring `FE_RC_SELECT` - `"CAMPAIGN SELECT"`, confirmed on an RPCS3 frame |
| a `MiniText` at `(160, 140)` | idstring `FE_CAMPSEL_MODES` - `"CAMPAIGN MODES"`, confirmed on the same frame |
| `Bracket` | `x="160" y="170" Width="1595" Height="780"`, the frame both flyers sit inside |
| `campaignList` (`List`) | idstring `FE_CAMPAIGNLIST`, two `<Entry>`s: `String="FE_RC_FURY"` then `String="FE_RC_HD"` - **Fury first**, matching the measured default. The list's own `<Values>` sits at `y="-500"`, off screen (the file's own comment on that line reads `<!-- -500 for y so it's off screen!! -->`) - that position drives the widget's internal selection state only, never meant to be drawn there. **The two entries' own names are drawn anyway**, at a chosen position - see "Drawing the two entries' own names" below. |
| `<Redirect>` | `campaignList == FE_RC_FURY -> "Grid Selection Fury"`; `== FE_RC_HD -> "Grid Selection"`; `<Default goto="To Be Done">` - a third branch that is evidence about the build (an unfinished fallback) even though nothing observed here can fire it |
| `FuryCampaignFlyerModel` / `HDCampaignFlyerModel` (`Flyer`) | `OriginX="640"`/`"1280"` - Fury left, `Wipeout HD` right, both `Src="Data\FE\Flyers\00_flyer.vex"`. **Not the per-campaign models the disc otherwise ships** (`/data/fe/flyers/fury_campaign/flyer.vex`, `/data/fe/flyers/hd_campaign/flyer.vex`) - this screen's own XML points both widgets at the same placeholder, as `Grid Selection`'s does, and the executable's `Flyer_Item` loader replaces it (`Src` is the dummy, the campaign's own folder the model). Drawn - see "The two cards". |
| `MedalImageFury`/`MedalImageHD` | `Hexmedal_HD.mip`, plus `FuryGoldMedalsMiniText`/`HDGoldMedalsMiniText` (idstring `RC_GM`, `"GOLD MEDALS"` - the same idstring `Grid Selection`'s own `Medals Title` already resolves) and `NumMedalsTextFury`/`NumMedalsTextHD` (idstring `RB_EVENT_TYPE` - a reused/generic idstring this build does not trust as content, the same "idstring is a placeholder slot, not the text" reading `Line{n}` already gets elsewhere in this module). **Closed, 2026-09-25** - see "Wipeout HD/Fury: the `GOLD MEDALS` denominator" below: RPCS3 reads `"0 / 87"` on the `Wipeout HD` side and `"0 / 80"` on `Fury`'s, and both are exactly the campaign's own total cell count, now drawn as `{earned} / {total}` by `selection::draw_list`. |

**The subtitle and both `GOLD MEDALS` labels are hand-placed, not read
through `oag_ui::screen::Screens::collect`.** All three are `<MiniText>`,
and adding a `MiniText` arm to that parser was tried first - it made this
build's own screen reachable, but a scratch survey
(`rg -c "<MiniText" data/scratch/lane-hd-sel/*.xml`) found `<MiniText>` on
`MainMenu_Definition.xml`, `RaceBox_Definition.xml`,
`Additional_Definition.xml` and `RecordGrid_Definition.xml` too - collecting
it generically would have started drawing widgets on every title's every
screen this pass never measured, not only this one. Reverted; the three
positions/colours in the table above are read directly off the file and
kept as plain constants in `oag_ui_screens::campaign::selection`
(`SUBTITLE_POSITION`/`FURY_GOLD_MEDALS_LABEL_POSITION`/`HD_GOLD_MEDALS_LABEL_POSITION`),
the same "grounded in a real, measured number, not through the generic
parser" precedent `BRACKET_RECT` already sets for the `Bracket` above.

Confidence 90: every widget above is read directly off `DATA06`'s own XML,
cross-checked against an RPCS3 frame for the two visible strings
(`ScreenTitle`, the subtitle) and the medal fraction's numerator shape.

### Drawing the two entries' own names, and the selector, 2026-09-21

**Superseded 2026-09-30 for a source that draws the two flyer cards** - see
"The two cards" below: with the cards drawn, the outline and the names are not
(`draw_list`'s `cards` flag), and only the selected campaign's medal counter
is. What follows is what a source with no card still draws.

The first pass at this screen (above) left `campaignList`'s own two entries
undrawn, on the same "drives selection, never drawn" reasoning `Grid
Selection`'s own `flyerlist` genuinely earns - but that reasoning does not
hold here: `flyerlist`'s sixteen names are redundant with the grid tiles
already drawn from `Definition.xml` directly, where `campaignList`'s two
entries are the *only* text on this build's screen that says which half is
Fury and which is `Wipeout HD` at all. A player looking at the first
headless capture saw two raw ids, a bare `0`, and one `GOLD MEDALS 0` -
nothing naming either campaign and nothing marking which was selected.

**The names**: `campaignList`'s own `<Entry String="FE_RC_FURY">`/`<Entry
String="FE_RC_HD">` resolve, off `DATA06`'s own
`Data\Plugins\Languages\English\entries.xml` (confirmed directly against
`hdfury-ps3-eu-dec.iso`), to `"FURY CAMPAIGN"` and `"HD CAMPAIGN"` -
[`oag_ui_screens::campaign::selection::Campaign::entry_id`]. These are disc data, not
invented: the disc's own string table names each entry, this build had just
never drawn it. Drawn at `FURY_ENTRY_NAME_POSITION`/`HD_ENTRY_NAME_POSITION`
(`(558.75, 610)`/`(1356.25, 610)`), `Align::Centre` - **chosen, not
measured**: the list's own authored position is off screen by design (see
the widget table above), so there is no authored on-screen position to read.
Each `x` is the centre of its own half of `Bracket`'s own rect (the same
split `CampaignSelection::pointer`'s own click targets and
`selector_outline` both use), and `y` sits above the gold-medal labels,
inside the Bracket, in the space the 3D flyer card fills where one is drawn. **The first version of this fix left-aligned each name at the
gold-medal label's own `x` (755/1395)** - at that anchor `"FURY CAMPAIGN"`
runs past the Bracket's own midpoint and is cut by `selector_outline`'s own
border, caught by looking at the resulting capture rather than measured off
anything; centring on each half fixes it structurally, not by nudging a
number.

**The selector**: this screen authors no `<Image name="Selector">` at all,
unlike `Grid Selection`/`Cell Selection`'s own (`oag_ui_screens::campaign::draw`'s
`centred_selector_draw`) - the real game's own equivalent is presumably the
selected 3D flyer's own scale/emphasis (an RPCS3 frame shows the selected
card larger and centred; see the "not drawn" note above for why this build
draws neither flyer at all). With no selector widget and no flyer to make the
choice visible, [`oag_ui_screens::campaign::selection::draw_list`] draws a plain
four-sided white outline (`selector_outline`, `SELECTOR_BORDER`/`SELECTOR_COLOR`)
around the selected half of `Bracket`'s own rect, split at its own midpoint -
the same split `CampaignSelection::pointer`'s own click targets already use.
**Chosen, not measured**: a stand-in for the disc's own animation, not a
reproduction of it, so the choice is visible and the pointer has something to
hover.

**The `--menu-page` capture path resolves the same four ids the live session
does.** Before this pass, only `crate::main::session::campaign::open_campaign`
(the live path) overlaid `DATA06`'s own copy of `FE_RC_SELECT`/`FE_CAMPSEL_MODES`
onto `strings` - `crate::capture::campaign_page` (`--menu-page
campaign-select`) had no such overlay at all, so a capture showed all four
ids (title, subtitle, both entry names) as raw text where a live session
showed the title and subtitle resolved and the two entry names not yet drawn
at all. Both gaps close together: `oag_game::campaign::hd_selection_string_overlay`
is now the one place either id set is resolved (all four ids, not two), and
both `open_campaign` and `campaign_page` call it - confirmed directly against
`hdfury-ps3-eu-dec.iso`, screenshot below.

Confidence 85 for the two entry names (a direct disc-string read, not yet
cross-checked against an RPCS3 frame showing this build's own screen next to
it) and confidence 50 - **chosen, not measured** - for the selector outline's
own look, which stands in for an animation this build does not attempt to
reproduce.

### The toggle: measured on RPCS3, `right` reaches the base campaign

Three RPCS3 boots this pass, Xvfb `:77`, own `--config` copy under
`data/scratch/lane-hd-sel/` (not committed - game content), `just
rpcs3-preflight` clean beforehand:

1. **Boot 1** (`data/scratch/lane-hd-sel/drive-campaign-selection.py`): at
   `Campaign Selection` with no prior d-pad input (matching the earlier
   pass's own default-Fury finding), `left` then confirm still lands on
   `Grid Selection Fury` - consistent with `left` being a no-op at the
   list's own first entry. `right` then confirm lands on `Grid Selection`
   (the base campaign) - the first direct measurement of a path to it.
   `up`/`l1`/`r1` then confirm also land on `Grid Selection`, but this run
   never returned the list to `Fury` between attempts, so those three are
   **confounded by whatever `right` already left selected** and are not
   independent evidence of their own - see boot 3.
2. **Boot 2** (fresh boot, same script's shape): `right` then confirm lands
   on `Grid Selection` again - reproduces boot 1's own finding on a
   completely separate boot, the same two-boots-agree bar the original
   Fury-default finding was held to.
3. **Boot 3** (`data/scratch/lane-hd-sel/drive-persistence-check.py`),
   built to settle the boot-1 confound directly: `right` -> confirm ->
   `Grid Selection` -> `circle` (back, two presses needed - the first was
   dropped, matching `RACE_WALK`'s own doc comment on ~9fps dropped presses
   here) -> back at `Campaign Selection` -> confirm with **no further d-pad
   input at all** -> `Grid Selection` again, not `Fury`. The selection
   persists across a back-and-return within one boot; it is not reset on
   re-entry. This is what makes boot 1's `up`/`l1`/`r1` results
   explainable without those three being toggles of their own, and it is
   also what makes `right` itself trustworthy as *the* toggle rather than a
   coincidence of a screen that resets to a random entry.

**Confidence 85 that `right` is the toggle to the base `Wipeout HD`
campaign** (three boots, two independent reproductions of `right` and one
dedicated persistence check, all consistent, all reading the state-machine's
own `TTY.log` screen name rather than an eyeballed frame). **Confidence ~50,
not independently isolated, that `left`/`up`/`down`/`l1`/`r1` are true
no-ops rather than a second working toggle that happens to leave a two-entry
list looking unchanged** - `down`'s own no-op reading carries over from the
earlier pass, `left`'s is boot 1's own clean (unconfounded) result, but
`up`/`l1`/`r1` were only ever tested against an already-persisted selection
and would need their own boot 1-style clean run from a reset `Fury` default
to rule out being a second, redundant toggle direction.

### What is modelled, and what is not

[`oag_ui_screens::campaign::selection::CampaignSelection`](../../crates/ui-screens/src/campaign/selection.rs)
is a flat two-entry list - `Fury` then `Hd`, matching `campaignList`'s own
document order and the measured default - stepped by `left`/`right` (pad) or
a click on one of two invented halves of the screen's own `Bracket` rect
(left half Fury, right half `Wipeout HD`, the same "grounded in a real rect,
not a free-standing invention" idiom `oag_ui_screens::campaign::hd::hd_grid_targets`
already uses for `Grid Selection`'s own confirm region) - two-tap, hover
selects and a second click on the already-selected half confirms, the same
idiom every other campaign screen in this crate uses. `Up`/`down` are left
unbound pending the confidence-~50 question above rather than guessed at.

Confirming opens `Grid Selection` (`Wipeout HD`, `grid0`..`grid7`) or `Grid
Selection Fury` (`grid8`..`grid15`) - `crate::campaign_stage::CampaignStage`
slices the sixteen grids [`read_grids`](../../crates/game/src/campaign.rs)
already reads whole at `grid0`/`grid8`, the same split
`crates/hd/src/campaign.rs`'s own module doc already documents by grid-name
convention, not a new measurement. `Back` on either `Grid Selection` screen
now returns to `Campaign Selection` rather than closing the campaign outright
- the previous, pre-this-pass behaviour, kept for Pulse and Omega (neither
ever builds a `Selection` screen, so their own `Back` still closes the
campaign the way it always has - `crates/ui-screens/src/campaign/tests.rs` pins both
titles' own flows).

**Drawn since 2026-09-30**: both `Flyer` widgets - see "The two cards". Not
drawn: the medal fraction's own denominator (see the widget table above), the
cards' reflection, glow and light, and the cards' own animation on a selection
change.

### The two cards, 2026-09-30

`FuryCampaignFlyerModel` and `HDCampaignFlyerModel` draw the two flyers the
disc ships for them (`Data/FE/Flyers/fury_campaign/flyer.vex` and
`hd_campaign/flyer.vex`), through the same flat-card path `Grid Selection`'s
sixteen use ("What a card is" in "The flyer behind `Grid Selection`"), each
seen through its own camera (both at 12 units, `+0x1c` `1.0833`).

**The widgets' own numbers place them, with `orthoScale` as a scale on
everything**: a card stands `z / orthoScale` in front of the camera (`70 /
0.75 = 93.3` units, at the same vertical field of view of 1.0 rad), its pivot
is `RotationCentreOffsetX / orthoScale` (`-40` on Fury's, `+40` on `HD`'s) and
its `x` and `y` scale the same way. `oag_game::flyer::campaign_pose` computes
the pose:

- **Face on**, `RotY` `0`: centre `(0, -11.1, -93.3)`. The `-11.1` is the
  widget's `y="-33.3"` over `0.75` (`-44.4`) plus half the card's height
  (`33.3`): the placeholder's `cardShape` runs from `0` up by `66.6`, and `y`
  is what centres it - so `Grid Selection`'s `y="-33.3"` is the same fact, and
  its fitted `ty` of `0` agrees. Measured on RPCS3, the selected Fury card is
  centred on `OriginX` (`639.4` against `640`), `212` down from the top and
  `93.4` units away by its height (`710` pixels for `66.6` units at 1.0 rad).
- **Turned**, `RotY` `+-0.6`: the card swings about its pivot, which puts it
  22.6 units farther back and 7.0 units toward its pivot. Measured, `HD`'s
  left edge sits at authored column 1105 with Fury selected, where the
  authored numbers put it within 40.

**Chosen, not measured**: that the selected card is the one with `RotY` `0` -
the widget authors only the turned pose, and an RPCS3 frame with Fury selected
shows Fury facing front and `HD` turned, the reverse with `HD` selected; that
the composition is a pivot turn with the model moved after it; and
`CAMPAIGN_STRETCH`, `1.048`. The cards stand 800 by 710 authored pixels
(aspect 1.13) where the camera says 1.083, and the wordmark on Fury's is 5
percent wider than the camera's aspect makes it, so the picture is stretched
rather than shown wider. The window (`FURY_WINDOW`, 0.321) is Fury's: it
matches `hd_campaign`'s own geometry at 0.319.

`selection_shows` gives both cards to `flyer::render_list`, between the
backdrop and the widgets, and `--menu-page campaign-select@1` selects `HD`.
The selected campaign's `GOLD MEDALS` counter is the only one drawn (an RPCS3
frame with Fury selected has none beside `HD`'s turned card, and one with `HD`
selected none beside Fury's).

Pinned on the disc by `campaign_selections_cards_land_where_rpcs3_shows_them`
in `crates/game/tests/hd_flyer_ground_truth.rs` (both selections, against the
measured columns), and by `oag_ui_screens::campaign::selection`'s tests for the stand-ins
and counters. Evidence: RPCS3 `campaign-settled-a`/`-b` and `campaign-right` in
`data/scratch/hd-flyer/rpcs3-raw/`; ours and the comparisons
`data/scratch/hd-fury-cards/exp/cmp-cs-sel0.png`, `cmp-cs-sel1.png`.

### Verification

- Headless `--menu-page campaign-select` against `hdfury-ps3-eu-dec.iso`,
  `--size 1920x1080` (before 2026-09-30; now the two cards, see "The two
  cards"): draws the screen's own title, subtitle, both entries'
  own names (`"FURY CAMPAIGN"`/`"HD CAMPAIGN"`), a white selector outline
  around the default-selected `Fury` half, and both campaigns' `GOLD MEDALS`
  reading - `Fury`'s own label is genuinely invisible (black text on black
  background, the disc's own authored colour, presumably legible against the
  undrawn flyer card in the real game; `Wipeout HD`'s own label is
  `FEGlobals->HD_Grey` and visible), each entry name centred in its own half
  of the Bracket rather than cut by the selector border. Screenshot:
  `data/scratch/lane-hd-sel/shots/campaign-select-v5.png`, not committed
  (`v4` was the left-aligned version the advisor caught, kept alongside it
  for the before/after).
  Command:
  ```sh
  cargo run -p oag-game -- data/images/hdfury-ps3-eu-dec.iso \
    --menu-page campaign-select --size 1920x1080 \
    --screenshot /tmp/campaign-select.png
  ```
  **The disc path is a positional `[SOURCE]`, not `--race <path>`** - `--race`
  is a bare flag ("skip the front end and go straight to a ship on a track")
  and combining it with `--menu-page` here launches the default race instead
  of drawing the still, the same trap `--screenshot --press cross --until`
  fell into below for a different reason.
- **Closed 2026-09-28** - see "Wipeout HD/Fury: walked live, end to end,
  2026-09-28" below. What follows is the earlier attempt's record.
  **A live, interactive walk (`Main Menu` -> `RACE CAMPAIGN` -> click a
  campaign -> `Grid Selection`/`Grid Selection Fury` -> `Cell Selection`) was
  attempted and not completed this pass** - left open, see below. Two
  approaches were tried, both under Xvfb `:93`:
  - `xdotool` mouse/keyboard against a windowed `cargo run`, with an
    explicit `XSetInputFocus` (python-xlib) beyond `xdotool
    windowfocus`/`windowactivate` - the same fix `docs/architecture/menus.md`'s
    own 2026-08-19 finding needed. `XGetInputFocus` confirmed the right
    window held focus, but a `std::fs::write` sentinel placed at the top of
    `Session::open_campaign` (the live handler `RACE CAMPAIGN`'s own action
    fires) never appeared on disk across several tries, meaning the
    synthetic key never actually reached the app's own input handling
    despite X reporting the correct window focused - a deeper winit/Xvfb
    input-delivery gap than the focus fix alone, not chased further.
  - `--screenshot --press cross --until "Team Selection"` (headless,
    no X11 at all): this does **not** test menu navigation the way it looks
    like it would - a `--screenshot` capture with no `--menu-page` reaches
    the boot chain's own `"Launch Game"` state in 3 ticks and launches
    **this build's own hard-coded default race** from there
    (`talons_junction`, logged as `"Wipeout HD's own default"`), the same
    "spends what is left of the ticks on the race the front end hands off
    to" behaviour `--ticks`'s own `--help` text names - it never touches
    `Main Menu`'s own row list at all, so `--press`/`--until` here proved
    nothing about `RACE CAMPAIGN` specifically. The same `open_campaign`
    sentinel confirmed this: absent here too.
  - Both attempts' own sentinel/log evidence is in
    `data/scratch/lane-hd-sel/live-walk/` and `data/scratch/lane-hd-sel/cascade-full.log`,
    not committed. **Left for whoever next has a working interactive Xvfb
    setup** - the headless `--menu-page` capture above and the RPCS3
    measurements are what this pass leans on instead, and neither exercises
    `crate::main::session::campaign::open_campaign`/`CampaignStage`'s own
    live wiring end to end. Everything downstream of `open_campaign`
    (`handle_campaign`'s new `Screen::Selection` arms,
    `CampaignStage::open_grid_selection`/`open_selection`) is proven only by
    `crates/ui-screens/src/campaign/selection/tests.rs`'s own model-level tests and
    by reading the code, not by a running session.
- `crates/hd/tests/campaign_selection_ground_truth.rs` (new, `#[ignore]`d,
  needs `data/images/`, checked as raw text rather than through
  `oag_ui::screen::Screens` to avoid a dev-dependency cycle - that crate
  already depends on `oag-hd`): pins `DATA02`'s own copy has neither new
  screen, `DATA06`'s own copy carries all four in document order,
  `campaignList`'s two entries read `FE_RC_FURY` before `FE_RC_HD`, and
  `Grid Selection`/`Grid Selection Fury`'s own `flyerlist` grid names match
  the 0/8 and 8/16 split `oag_hd::campaign::HD_GRID_RANGE`/`FURY_GRID_RANGE`
  and `crate::campaign_stage` assume.
- `crates/ui-screens/src/campaign/selection/tests.rs` (new): the model's own
  stepping/confirm/pointer behaviour against a synthetic fixture, the same
  shape `crates/ui-screens/src/campaign/tests.rs` already gives `GridSelection`.

## The tip ticker, the Confirm/Back legend, `Cell Help` and a podium, 2026-09-21

**All four close this pass, live-driven under Xvfb `:91` with `xdotool` on
`pulse-psp-usa.chd`, not only `--menu-page`.** The mechanism the ticker and
the footer legend both needed turned out to be the same gap: `Skin.xml`'s
`<NavigationController>`/`<TextInfo>`, on `FE Screen`, both entirely
unrecognised by `oag_ui::screen::Screens::collect_widgets` (`_ => {}` drops
the tag and every child) and unreachable through the per-screen widget model
even if the tag were parsed - a `NavigationController` picks *per screen*
which prompts to show, and the ticker's own `TextInfo type="bar"` is two
alternating text buffers sharing one clip viewport, neither of which any
named `Screen` carries as a fact. `oag_ui_screens::campaign::footer` (new module)
reads both directly off the raw parsed tree instead - see its own module doc.

- **The ticker.** `TickerLayout::read` finds `<Viewport
  name="TextInfoIsAlwaysLast" OffsetX="85" OffsetY="235">` (`width="370"
  height="32"`) and the `type="bar"` `TextInfo` inside it; `ticker_draw` is a
  stateless continuous scroll over whatever tips the caller supplies, at
  [`crate::anim::MARQUEE_SPEED`] (60px/s) - **chosen, not measured**: no
  PPSSPP capture in this pass isolated two frames a known tick count apart
  with the ticker moving, so the speed reuses this project's own existing
  scrolling-text rate as the nearest precedent rather than a number invented
  from nothing. **Content is never invented**: `NewsItemBarText1`/`2` carry
  no `idstring` and no literal `string` on disc - populated at runtime by a
  mechanism this build does not have - so `oag_game::records::ticker_tips`
  (moved there 2026-09-27 so `MenuStage`'s own ordinary-menu ticker and
  `capture::menu_page`'s `--menu-page` still could read the identical
  rotation `CampaignStage::ticker_tips` used to keep to itself) only
  rotates the `TKR_NO*` family (no `%d`/`%s`/`%.2f` template), five of the
  seven gated on whether `oag_game::records::Store` carries any row for that
  mode (a real, save-backed "have I raced this yet"), and `TKR_NOTOURN`/
  `TKR_NOHH` shown unconditionally since neither mode ever launches in this
  engine at all. The sibling `type="tag"` `TextInfo` (`NewsItemTagText`,
  fixed at `(30, 235)`) draws nothing, for the identical reason: no idstring,
  no literal string, nothing honest to show. **Confirmed live**: a fresh
  profile's ticker read `"YOU'VE NOT TAKEN PART IN ANY TOURNAMENTS SO
  FAR..."` on `Grid Selection` and `"...YOU HAVEN'T TRIED OUT ANY H[EAD TO
  HEAD RACES]"` on `Cell Selection`, both mid-scroll, matching the disc's own
  `TKR_NOTOURN`/`TKR_NOHH` text exactly.
- **This section corrects this page's own `## Open` entry above**: the
  tip actually captured live on PPSSPP, `"Why don't you try out the Speed
  Lap events?"`, is `TKR_NOSL` - read directly off `PI000/entries.xml`
  (`Data\Plugins\PI000\entries.xml`, the EU disc's English table) - not
  `TKR_NOTOURN` as this page previously named it. `TKR_NOTOURN`'s own text
  is `"You've not taken part in any Tournaments so far - why not give them a
  try in Racebox?"`, a different string; the earlier reading appears to have
  matched the two ids by theme rather than by content. `TKR_NOZONE`/
  `TKR_SONGS`/`TKR_DIST` were correct.
- **`Confirm`/`Back`.** `NavigationLegend::read` finds
  `<NavigationController name="NavigationController">`'s four `Text`
  children (`ControlTextConfirmButton`/`Confirm`/`BackButton`/`Back`) and
  resolves their idstrings - `FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON` are `"ε"`/
  `"γ"`, small-font glyphs for the cross/circle buttons, the same
  private-glyph idiom `Cell Selection`'s own `HELP`/`CHANGE DIFFICULTY` row
  already draws with a literal `β`/`δ` baked into the XML instead of an
  idstring. Drawn unconditionally on `Cell Selection` - the one screen this
  build shows them on, since that is the one screen the 2026-09-14 PPSSPP
  pass measured wanting them; the original's own per-screen gate on which
  prompts a `NavigationController` shows was not traced. **Confirmed live**:
  `ⓧ CONFIRM  Ⓞ BACK` now draws beside `△ HELP  □ CHANGE DIFFICULTY`,
  matching `cell-selection-grid0-default-cell.png` widget for widget.
- **`Cell Help`'s overlay now draws**, as a **static** panel - `Main Help`/
  `Speed Class Help`/`Event Help` (`MSC_HELP_RC_CS`/`MSC_LOAD_VENOM`/
  `MSC_EVENT_SR`, already-resolved `screen.texts` on the `Cell Help` screen
  `Layout::read` reaches the same way it reaches `Cell Selection`'s own) at
  their own authored positions, no `Viewport`/`Animation` scroll timeline
  scripted - the honest step this page's own `Open` entry already named.
  `Speed Class Help`/`Event Help` are the disc's own literal authored text
  regardless of the cell actually selected: no per-cell substitution for
  either idstring is traced.
- **Line5 (`Cell_SavedRecord`) now draws** for Time Trial (`best_total_ticks`)
  and Speed Lap (`best_lap_ticks`), read off the general per-track/mode/class
  `oag_game::records::Store` - a campaign race already folds into that same
  table, keyed by the cell's own track/mode/class, so it is the closest
  honest reading of "this cell's own saved best" without a new per-cell
  store. **Confirmed live**: the Time Trial attempt below (`2:10.48`, no
  medal) reappeared on `Line5` when the same cell's own panel was reopened.
  **`Line8` stays blank on purpose**: it authors the identical
  `OffsetX="260" OffsetY="180"` `Target0` sits at, so drawing it whenever
  `targets_visible` would overlap that row outright, and nothing traces what
  the original shows there instead. `Race`/`Zone`/`Elimination` also answer
  `None` on `Line5`: `Race`'s own record is a position, not a time, and
  `Zone`/`Elimination` have no raw zone/kill count anywhere in
  `oag_game::records::Record` to read at all - a real gap, not one this pass
  closed.
- **The podium walk closes**, resolving this page's own `Open` entry below
  about `Line6`/`Line7` never being seen reading a real medal: with a
  worktree-local `[ai] difficulty = "novice"` (never committed - opponents
  only; `--autopilot-skill` governs the flown craft independently and does
  not need it), `cargo run -p oag-game -- --autopilot --autopilot-skill ace`
  navigated `RACE CAMPAIGN` -> `GRID 1` -> `grid0_3_1` (`Single Race`,
  `03_Track`/Moa Therma White, `Locked="false"`) -> `Team Selection` -> the
  autopilot finished **1st of 8** at `2:16.03` (`results-*` not captured
  separately, see `PODIUM-line6-line7-gold-medal.png`). Back on `Cell
  Selection`, `grid0_3_1`'s own panel now reads `Points 3/3` / `Best Gold`,
  the tier's own hex tints gold on `Grid Selection` (`Gold medals 01/08`,
  `Total points 003/024`), and both hex-adjacent neighbours
  (`grid0_2_1`/`grid0_4_1` in this build's own two-column zigzag) lost their
  lock glyphs - the six-neighbour unlock rule, confirmed live for the first
  time this pass rather than only against the glyph predicate's own unit
  tests. **A different cell was attempted first and did not podium**:
  `grid0_3_2` (`Time Trial`, gold `1:55.00`) finished `2:10.48` at
  `--autopilot-skill ace` with `[ai] difficulty` irrelevant (no opponents) -
  close but over even the bronze target (`2:03.00`), left as a genuine miss
  rather than retried past this pass's own time budget; its own attempt is
  what now reads back through the new `Line5` above.

None of this was captured against a live PPSSPP frame this pass - the
ticker's own scroll speed, the `NavigationController`'s per-screen gate and
`Cell Help`'s own animation timeline are all still open at PPSSPP, only
closed against this build's own picture and behaviour.

### Two defects, found by comparing this pass's own capture against `cellselect-grid0_3_2.png` directly, fixed same day

The section above closed against this build's own live behaviour, not the
reference frame - side by side, `PODIUM-line6-line7-gold-medal.png` showed
two things a player sees at once that neither `oag-ui`'s own tests nor a
solo look at the new draw caught:

1. **The ticker bled past its own viewport.** `TickerLayout::read`'s
   `[85, 235, 370, 32]` was read correctly but never enforced - `ticker_draw`
   drew unclipped, so its text ran from screen `x=0` straight across the
   pilot-tag tab on `Cell Selection`'s own footer. Fixed by routing the
   ticker's own `Draw::Text` through `Renderer::render_with`'s existing
   `clip: Option<(usize, f32, f32)>` - the same mechanism
   `crate::marquee`'s row-value scroll already clips itself with, keyed on
   the draw's own index in the *flattened* list rather than on `y`.
   `ticker_draw` now returns at most one `Draw` (`Option<Draw>`, not
   `Vec<Draw>`) rather than up to two - the earlier double-copy trick for a
   seamless wrap only works unclipped, so the honest trade is a brief,
   real gap at each wrap instead of a seam this build cannot clip away
   (see `oag_ui_screens::campaign::footer::ticker_draw`'s own doc). Confirmed live,
   `data/scratch/lane-pulse/shots/crop-ticker-left.png`: the text now cuts
   cleanly at the tab's own edge.
2. **`Confirm` ran into both button glyphs beside it.** `ControlTextConfirm`
   authors no `font=` in `Skin.xml`, and this crate's fallback (`"Default"`,
   13px) draws through the same single loaded menu-face atlas every other
   campaign-screen label does - wider per glyph than whatever compact face
   authored `Confirm`'s own 47-native-pixel gap to the back button. No
   available role fit it from either end (`crop-legend2-zoom.png`/
   `crop-legend3-zoom.png`), so `NavigationLegend` now right-aligns
   `Confirm` to end just short of the back glyph and shrinks its own scale
   only if that still does not clear the confirm glyph on its other side -
   both **chosen, not measured**, since nothing on disc says where the
   word should end or how far it may shrink. Confirmed live,
   `data/scratch/lane-pulse/shots/FIXED-cellselect-ticker-clipped-legend-fit.png`.

**Not fixed, and now written down rather than silently left**: neither
defect's *root cause* is reachable from this crate. This build loads
exactly one font atlas for the whole menu/campaign path
(`shell.title_font`/`shell.menu_font`, the boot log's own "menu font
`Pulse_20.fnt` (role `menu`): line height 22") and `"default"`/`"small"`
are multipliers on that one atlas, not switches to a genuinely different,
compact, mixed-case face - `Draw::FacedText`'s own `role` is not even
checked against anything (`crates/game/src/render.rs`'s own comment: "the
role is what chose which atlas `set_face_atlas` loaded... nothing here
re-checks it against `role`"), and the one alternate atlas that ever loads
is the `Title` role, not a body face. Two consequences, both left open:

- ~~Every label on this screen still renders upper-case~~ **Fixed
  2026-09-21**, once the atlas this note itself said was missing landed
  (`crates/game/src/boot/fonts.rs`'s `face_atlas_slot`, a separate lane's
  own work): `oag_ui_screens::campaign::draw::text_draw` now picks
  `Draw::FacedText` over the plain `Draw::Text` this note describes
  whenever `text.font` names `"default"` - `super::footer::face_role`,
  the identical check `NavigationLegend::draw` already made for
  `Confirm`/`Back`, now shared rather than duplicated. `Speed
  class`/`Line1`..`8`/`Title`/`Track Line` all author `font="default"`, so
  every one of them picks the real face up automatically, no per-label
  change needed. Confirmed live,
  `data/scratch/lane-hd-sel/shots/pulse-cellselect-after-facerouting.png`
  (`pulse-psp-eu.chd`, French: `Catégorie`/`Tours`/`Armes`/`Points`/
  `Meilleur` all read mixed-case where they read upper-case before).
- ~~`AI difficulty (Medium)` reads `CHANGE DIFFICULTY`~~ - **fixed,
  `pulse-cellsel` lane, 2026-09-28.** `CellSelection_Update`
  (`0x088d6430`, decompiled and named this pass) rebuilds `DifficultyButton`'s
  text every frame with `sprintf("%s (%s)", resolve("RB_AI_DIF"),
  resolve(rung))` - `oag_ui_screens::campaign::draw::difficulty_button_line`
  reproduces it exactly, matching `data/reference/psp-campaign-screens/
  cell-selection-difficulty-hard.png`'s own `"AI difficulty (Hard)"` digit
  for digit (rung word for rung word). See
  `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
  `DifficultyRC` persisted rung and Cell Selection's own square button"
  section.

### The same unclipped-ticker defect recurred in a new capture path, 2026-09-27

**`pulse-menu-ticker` lane, wiring the ordinary menus' own footer ticker
(`MenuStage`, not `CampaignStage`).** `crate::main::menu_stage::footer` copied
this page's own 2026-09-21 fix correctly for the *live* session
(`footer::resolve_clip` mirrors `CampaignStage::ticker_clip_bounds`'s call
site exactly), but `crate::capture::menu_page` - the `--menu-page main` still
this lane also added, so a player-facing render could be judged by eye rather
than only by test - never got it: it called `Renderer::render` with a bare
`None` clip, a line inherited unchanged from before either page's ticker
existed ("a capture is one static frame with no `MenuStage` clock behind it,
so there is nothing here for a value marquee to be mid-scroll of"). That
comment was true until this lane gave the still a second scrolling thing to
draw and did not give it a second clip. Confirmed by screenshot before the
fix (`data/scratch/drive-2026-09-27/ticker/psp-eu-main-before-clip-fix.png`,
`ps2-eu-main-before-clip-fix.png`): the German `TKR_NOTOURN` text ran off the
right edge of the frame on PSP and past its own bar on PS2. `Cell Selection`'s
own still (`crate::capture::campaign_page`) never hit this because its
`static_footer_overlay` draws the ticker with `tips = &[]` - an honest empty
rotation, not a clip - so the bug had no text to be visible with there.

Fixed the same way this page's own 2026-09-21 fix did: `menu_page` now
returns `(Vec<Draw>, TickerClip)` (`TickerClip = Option<(usize, f32, f32)>`),
finding the ticker draw's index by equality after the page's own layers
flatten - the identical `position`-after-`flatten` idiom `MenuStage::render`
uses for the Race Campaign's own grid/cell screens - rather than a hand-
tracked index a later `.extend()` could invalidate. `capture::run` threads it
through to `renderer.render`'s own `clip` parameter. Confirmed live,
`data/scratch/drive-2026-09-27/ticker/psp-eu-main.png`/`ps2-eu-main.png`: the
text now cuts cleanly at each platform's own viewport edge (PSP
`[85, 235, 370, 32]`; PS2 `[113, 392, 493, 53]`, its own front-end root's own
grid, not a scaled copy of the PSP's). Also confirmed scrolling (not just
positioned correctly) via a temporary, reverted-before-commit env-var probe
on `ticker_draw`'s `elapsed` argument, since `--menu-page` runs no clock of
its own: `psp-eu-main-t0.0.png`/`-t1.0.png`/`-t2.5.png` show the same German
sentence sliding left at the documented 60px/s. The first pass through this
probe only exercised the right edge (`elapsed = 0` starts the text flush
against the viewport's own left edge); re-shot at `elapsed = 1.0`/`2.5` after
the fix confirms the left edge clips exactly as cleanly - `push_text`'s own
`clip: Option<(f32, f32)>` trims a glyph's `rect`/`uv` on whichever side it
straddles, not only the right, and `ps2-eu-main-t2.5.png` confirms the same
on PS2's own wider viewport.

**Lesson for whoever adds a third scrolling-text draw to a still capture**:
`Renderer::render`'s `clip` is one slot, shared by whichever of the value
marquee, a `MenuStage` ticker or a `CampaignStage` ticker wants it on a given
frame - a fresh capture call site that draws unclipped text starts from
`None` by default, and nothing except a side-by-side screenshot against the
real viewport catches the omission, since `oag_ui_screens::campaign::footer`'s own
unit tests only check the *draw*, never how a caller clips it.

## Where the cursor lives: one shared slot, measured 2026-10-02

`pulse-cursor-live` lane. PPSSPP v1.20.4 (SDL, software renderer, muted), Xvfb, `pulse-psp-usa.chd`, two
boots. The cursor is read as raw state, not only from a picture: a breakpoint at `CellSelection_Update`
(`0x088d6430`) reads the screen object (`a0`, `0x08d73170` on every read, so the same object throughout), its
selected-cell pointer at `+0xdc`, and that cell's name (the pointer at `cell+0x74`). Screenshots under
`data/scratch/pulse-cursor-live/` (gitignored).

**Falsifier written before the first capture.** The decompile
([`CellSelection_OnEnter`](../ghidra/functions/psp-pulse-usa/race-campaign.md#where-cell-selection-keeps-its-cursor-across-a-back-out-2026-10-02-pulse-loyaltybar))
predicts one `(x, y)` slot shared by every grid. If instead the cursor were per-grid (what this build did), or reset on
entry, a second grid would land on its own remembered cell or its first-unlocked default. The obvious walk
("`grid0_3_2`, then enter `grid1`") cannot tell those apart if the new grid's `(3, 2)` is locked, so the cell
definitions were read first: `grid0` and `grid1` both author `_3_1` and `_3_2` with `Locked="false"` (and
`grid0`'s and `grid1`'s first-unlocked default is `_3_1` in both), so `(3, 2)` into `grid1` separates "shared slot"
(`grid1_3_2`) from "default" (`grid1_3_1`), and the reverse walk separates "shared" from "per-grid".

**Precondition (a poke, not the game's own path).** A fresh profile has only `grid0` unlocked, and a confirm on a
locked tier does nothing. `grid1`'s `PI_Grid` was found by searching for the pointer to its name string (`grid1`
at `0x08d09100`, held at `+0x74` of the object at `0x08f609e0`; `+0xa0` = `Locked` = 1, `+0xa4` = `RequiredPoints`
= 16) and its `Locked` byte written to 0 from `Main Menu`, before entering `Grid Selection`. After that `grid1` opens
normally. Addresses were identical on both boots.

| # | Boot | Walk | Read | Shared slot predicts | Per-grid / default predicts |
| --- | --- | --- | --- | --- | --- |
| A | 1 | `grid0`: default `3_1`, `down` to `3_2`; back to `Grid Selection`; enter `grid1` | **`grid1_3_2`** | `grid1_3_2` | `grid1_3_1` (default) |
| B1 | 1 | in `grid1` `up` to `3_1`; back; enter `grid0` | **`grid0_3_1`** | `grid0_3_1` | `grid0_3_2` (per-grid memory) |
| R2 | 2 | fresh boot: `grid0` `3_1` -> `3_2`; **leave to `Main Menu`**; `RACE CAMPAIGN`; enter `grid1` | **`grid1_3_2`** | `grid1_3_2` | `grid1_3_1` |
| R2-3 | 2 | in `grid1` `up` to `3_1`; back; enter `grid0` | **`grid0_3_1`** | `grid0_3_1` | `grid0_3_2` |

**Result: the shared-slot reading holds, the per-grid memory is falsified.** A cursor left on `(3, 2)` in `grid0` is on
`grid1_3_2`, and one left on `(3, 1)` in `grid1` is on `grid0_3_1` rather than the `grid0_3_2` that grid was last
left on. A and R2 also fall to `grid1_3_1` under the old per-grid code, so they falsify it on their own.

**Leaving the campaign.** In boot 1 the cursor was left on `grid0_3_2`, backed out through `Grid Selection` to `Main
Menu`, and `RACE CAMPAIGN` re-entered: `Cell Selection` opened on `grid0_3_2`, same screen object. Boot 2's R2 row does
the same across a leave and lands on the carried slot. The screen object outlives the campaign, so the slot does too;
`CampaignStage` was rebuilt on every `RACE CAMPAIGN` and forgot it.

**What the slot is checked against: the new grid's cell must show no lock glyph.** The decompile's filter is
`FUN_088a37cc(selector, 4, x, y, 4) == 0`, a flag test on the lock layer (layer `4`) of the tile at the slot, so the
question is the glyph, not the `Locked` byte (the default scan reads the byte; the two differ on any cell a medal
unlocked). Rows B2 to E1 were taken on a fresh profile, where byte and glyph agree on every cell and so cannot tell
them apart; G1/G2 were taken after the two races had put a gold on `grid0_3_2` (the saved medal persisted across a
reboot), which hides the glyph on its six hex neighbours without touching their `Locked` byte:

| # | Walk | Read | Reading |
| --- | --- | --- | --- |
| B2 | cursor left on locked `grid0_2_2`; enter `grid1` (has a locked `grid1_2_2`) | `grid1_3_1` (default) | glyph at the slot: rejected |
| B3 | cursor left on locked `grid1_2_2`; enter `grid0` (has a locked `grid0_2_2`) | `grid0_3_1` (default) | same |
| C1 | cursor left on locked `grid0_2_2`; leave and re-enter **the same grid** | `grid0_3_1` (default) | not carried even within one grid |
| D2 | `grid0_2_2`'s `Locked` byte written 0 (`cell + 0xb9`, was 1) while on `Grid Selection`; cursor left on it; re-enter `grid0` | `grid0_2_2` | glyph gone, carried (byte and glyph agree) |
| E1 | cursor left on `grid0_2_2` (byte now 0); enter `grid1` where `(2, 2)` is locked | `grid1_3_1` (default) | the **new** grid's tile decides, not the old one's |
| **G1** | gold on `grid0_3_2`; cursor left on `grid0_2_2` (byte `+0xb9` read **1**, glyph hidden by the gold neighbour); leave and re-enter `grid0` | `grid0_2_2` | **the glyph decides: the raw byte rule would have reset it** |
| G2 | same boot; cursor left on `grid0_2_1` (byte absent, glyph still drawn, no medal beside it); leave and re-enter | `grid0_3_1` (default) | control: a visible glyph resets it |

G1/G2 are one boot each, one walk each (frames `71`-`74` under `data/scratch/pulse-cursor-live/`). The first
implementation keyed on `Locked == false` and would have failed G1; it now asks the screen's own lock predicate
(`CellSelection::selected_is_locked`: byte set, no medal of its own, no medalled hex neighbour).

**Not measured:** a slot the new grid has no cell at (every slot tried existed), a glyph still fading out at the
moment of re-entry (the lock layer's fade flag is a second bit this walk did not separate), anything on HD/Fury, and a
grid reached by the game's own unlock path rather than the `Locked` poke.

**Implemented** (`crates/game/src/main/campaign_stage.rs`, `CellCursor`, four unit tests): the slot is the cell's
`(x, y)` from its name (`Cell::grid_coords`); `restore` selects the cell at the slot and puts the cursor back on the
default if `selected_is_locked()` says that cell still shows a glyph; `Session::campaign_cursor` carries it across
`RACE CAMPAIGN` openings (written when the campaign closes on any path, including launching a race). HD/Fury run the
same code through the shared stage: **chosen, not measured** there. Confidence **90** for the shared slot and its
persistence across leaving the campaign (decompile plus a runtime read on two boots, one binary), **85** for the
glyph filter (a runtime read, G1 and its control seen once each, the decompile's layer-4 flag test agreeing).

## Open

- ~~`Cell Selection`'s initial cursor was "first cell in document order"~~ -
  **wrong, found and fixed, `pulse-campaign` lane, 2026-09-28.** A live
  PPSSPP capture (fresh profile) opens `grid0`'s own `Cell Selection` on
  `grid0_3_1`, never `grid0_2_1` (`grid_00.xml`'s own first-listed cell,
  what `CellSelection::new`'s old `index: 0` picked). `CellSelection_OnEnter`
  (`0x088d59c4`), decompiled in full this pass, scans document order for the
  first cell whose own `Locked` byte parses as the literal `false` - an
  absent attribute or an explicit `true` are both skipped, which lands on
  `grid0_3_1` (fourth listed, first explicit `false`) exactly. See
  `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s new
  "`CellSelection_OnEnter`'s own default-cursor scan" section for the full
  decompile, and `crates/ui-screens/src/campaign.rs`'s `CellSelection::new`. **Still
  open**: the cursor is separately confirmed to *persist* across a back-out
  to `Grid Selection` and a re-entry in the original, live-tested twice this
  pass - this build's own `CampaignStage::open_cell_selection`
  (`crates/game/src/main/campaign_stage.rs`) rebuilds a fresh
  `CellSelection` on every call instead, so a re-entry here always lands
  back on the document-order default rather than wherever the player left
  it. ~~Not fixed this pass.~~ **Fixed, 2026-09-28 (`pulse-campaign-flow`
  lane)**: `CampaignStage` now keeps a per-grid `CellCursors` memory (**superseded
  2026-10-02**: the cursor is one slot shared by every grid and kept across leaving the
  campaign, see "Where the cursor lives" above),
  written by `back_to_grid_selection` and restored by
  `open_cell_selection` (test
  `campaign_stage::tests::a_re_entered_grid_restores_the_cursor_it_was_left_on`),
  and verified live under Xvfb: moved to `grid0_3_2` (Time Trial, Metropia
  White), backed out to `Grid 1`, re-entered on `grid0_3_2`. ~~**Chosen, not
  measured**: that the memory is keyed per grid (a grid never entered keeps
  its own first-visit default) and that it is forgotten once the campaign
  screens close~~ - **both falsified 2026-10-02 (`pulse-cursor-live`)**: one slot is shared
  by every grid and survives leaving the campaign, see "Where the cursor lives" above.
  Still chosen, not measured: HD/Fury share
  `CampaignStage`, so their `Cell Selection` now persists the cursor too,
  though only Pulse PSP was observed doing it. **Decompile, 2026-10-02
  (`pulse-loyaltybar`, 70; measured and implemented the same day, above)**: the original keeps the cursor as the `Selector` widget's own
  `(x, y)` slot, shared by every grid, so the per-grid keying above probably disagrees on a
  second grid (it would keep `(3, 2)` if the new grid has a cell there). Not changed: see
  [`race-campaign.md`'s section](../ghidra/functions/psp-pulse-usa/race-campaign.md#where-cell-selection-keeps-its-cursor-across-a-back-out-2026-10-02-pulse-loyaltybar)
  for the law and the live check that would settle it.
- ~~The scrolling tip ticker and the button-legend footer row are still not
  drawn.~~ **Both draw, 2026-09-21** - see "The tip ticker, the Confirm/Back
  legend, `Cell Help` and a podium" above. The scroll speed itself is still
  unmeasured against a live PPSSPP frame.
- **`--menu-page grid-select`'s own capture re-checked against
  `grid-selection-page1-grid0-unlocked.png`, 2026-09-25: the earlier "no new
  gap" pass under-checked this.** A side-by-side crop comparison (not just a
  text-content read) found five real render differences, closed here off
  the disc's own data unless noted:
  - **`Medals`/`Required` formatted backwards.** `GridSelection_Update`
    (`0x088dec24`) binds `Medals` through `FUN_08972550(..., s__d__d_08a83544, ...)`
    - the literal at `0x08a83544` reads `"%d/%d"`, unpadded - and `Required`
      through the literal at `0x08a83568`, which reads `"%03d"`, not the `"%d"`
      `race-campaign.md`'s own table previously read it as (corrected there
      too, this change). This build had it backwards: `Medals` was
      zero-padded (`"04/08"`) and `Required` was not (`"12"`); the disc shows
      `"0/8"` and `"012"`. Fixed, `oag_ui_screens::campaign::draw::grid_draw_list`.
  - **The selected hex/cell drew plain white, not tinted.** A shared,
    non-PI001 widget routine (`FUN_088a5700`, reached off the `Selector`/
    `SelectorGlow` widget-name strings at `0x08a7f130`/`0x08a7f178`) pulses
    the selected tile's own colour on a continuous ~1s cycle between
    `0xff33a6b9` and white - `CellMode_Definition.xml` itself only authors
    `Selector` at a fixed, multiply-neutral `i="0xffffffff"`, so this build's
    plain white was the XML's own default with no runtime tint applied at
    all. Fixed as a static draw of the cyan endpoint (`SELECTOR_TINT`,
    `oag_ui_screens::campaign::draw`) - the real widget animates and this build's
    draw-list builder has no clock to animate it with, so this always shows
    the phase `grid-selection-page1-grid0-unlocked.png`/
    `cell-selection-grid0-default-cell.png` happen to have caught, not the
    full pulse. Applied to both `grid_draw_list` and `cell_draw_list`, since
    `race-campaign.md` already documents this widget family as shared
    between the two screens, not `GridSelection`-only.
  - **`Grid Selection`'s locked/unlocked tier hexes drew plain white, not
    dimmed.** `GridSelection_PopulateTiles` (`0x088de6f4`) tints every tier's
    own base hex - locked or not, unconditionally - to RGB `0x34acc2` at
    roughly half alpha; only the lock glyph itself (white, full alpha, the
    same as this build's own untinted default) distinguishes locked from
    unlocked. Fixed (`TIER_OUTLINE_TINT`), `grid_draw_list` only - **not**
    `cell_draw_list`: `CellSelection_PopulateGrid` never calls
    `GridController_SetTileColor` on the base-hex layer at all, so `Cell
    Selection`'s own hex outlines stay at the XML's own default there, a
    real asymmetry between the two screens rather than an oversight. `Cell
    Selection`'s own `Outline_x_y` widgets *do* author a colour
    (`i="FEGlobals->CM_HEX_Outline"`) - **already resolved, not an open
    question**: see the `pulse-campaign` lane's 2026-09-28 correction below,
    this bullet's own "not one of the two confirmed names" claim was stale
    the day it was written.
  - **The up/down page arrows never greyed out.** `FUN_088de9bc`, called at
    the end of every `GridSelection_Update`, tints `up arrow` to
    `0xff505050` when already on the first page and `down arrow` to the same
    grey on the last page, full white otherwise - `CellMode_Definition.xml`
    authors both at a fixed white, so again this was the untinted default.
    Fixed (`ARROW_DISABLED_TINT`), `grid_draw_list`.
  - **The detail panel's row labels/values read smaller, relative to the
    panel, than `grid-selection-page1-grid0-unlocked.png` shows.** Pixel
    measurement (cap height of `0/8`'s glyphs against `GRID 1`'s own, both
    against the same 960x544 frame): the reference's `default`-role text
    measures roughly 16px against the `Menu`-role title's 22px, a ~0.73
    ratio - closer to this build's own `small` role (17/22, `docs/ui/menus-original.md`)
    than to `default`'s own documented `13/22`. Neither `Medals`/`Points`/
    `Required`/their titles author an explicit `scale=` in
    `CellMode_Definition.xml` (checked directly, `just wad cat`), so if this
    is real it points at `FaceScales::default()`'s own `13/22` ratio being
    wrong rather than a per-widget fix - a shared constant several other
    screens also read, and not something this pass's one glyph-height
    measurement is rigorous enough to change safely. **Not fixed - left
    open, named here rather than guessed at.** The measurement method itself
    is crude (a single global brightness threshold, no font-metric baseline)
    and should be treated as a lead, not a confirmed number.

    **Confirmed real, `pulse-campaign` lane, 2026-09-28 - a direct
    same-widget comparison, not another single-screenshot cap-height ratio.**
    A first check against the `.fnt` files' own `+0x10` line-height field
    (`docs/formats/fnt.md`'s table, and a new scratch probe reading the raw
    glyph records, `crates/tools/examples/campaign_font_probe.rs`:
    `pulse_text.fnt`'s own `'S'` glyph is 12px tall at line-height 13,
    `Pulse_20.fnt`'s is 19px at line-height 22) looked like it closed this as
    "two different bitmap fonts, cap-height ratios need not match line-height
    ratios" - but that reasoning doesn't survive comparing the *same* widget
    on the *same* capture instead of two different widgets on two different
    captures. `data/scratch/pulse-campaign/captures/ours-cell-select-fresh.png`
    (this build, fresh profile, `--menu-page cell-select`) and
    `02-cell-selection-default-window.png` (a fresh live PPSSPP capture this
    pass took, `pulse-psp-usa.chd`, both 960x544) crop identically at
    `(538,165)-(700,190)` for `"Speed class"`'s own row: the reference reads
    **18px** tall (cyan-threshold scan, first to last non-black row) against
    this build's **11px**, a 64% difference visible by eye in the crop, not
    just in the numbers - not a brightness-threshold artifact, since both
    crops are read with the identical script against the identical widget.
    `crates/ui-screens/src/campaign/draw.rs`'s `text_draw` draws every `font="..."`
    role off the **Menu** face's own glyphs, scaled down by
    `layout.face_scale(&text.font)` (`13.0/22.0` for `"default"`) rather than
    loading a separate small atlas - a technique that only reproduces the
    original if that ratio is the one the original itself effectively draws
    at, and this comparison says it is not, for this text specifically.
    **Fixed, same pass, once the actual draw path was read rather than
    guessed at - `FaceScales::default()` itself was never the bug.**
    `TitleScale` is confirmed `1.0` on the PSP (`Skin.xml`'s own `<Variable
    global="TitleScale">`, ruling out a title-side scale confound), which
    left `crates/ui-screens/src/campaign/draw.rs`'s own `text_draw` as the only
    other place the ratio could be applied wrong. It was: `text_draw`
    already routes a `font="default"` widget through
    `oag_ui_screens::campaign::footer::face_role`, which since the `face_atlas_slot`
    fix (2026-09-21) resolves to `Draw::FacedText { role: "Default", .. }` -
    a real second atlas (`pulse_text.fnt`, loaded at its own native size),
    not the `Menu` atlas faked smaller. `crates/game/src/render/text.rs`'s
    `push_text` confirms the atlas is read at face value: `(cell.width as
    f32 * scale, cell.height as f32 * scale)` off whichever atlas `slot`
    names, so `scale = 1.0` on `GlyphSlot::Face` already draws the `Default`
    atlas's own 13px-line-height glyphs at their native size. `text_draw`
    was still multiplying that `scale` by `layout.face_scale("default")`
    (`13/22`) on top - shrinking already-native-sized glyphs by another
    `13/22`, roughly 60% too small, which is exactly the ~64% gap this
    pass's crop measurement found. **The identical bug, already found and
    fixed one widget over**: `oag_ui_screens::campaign::footer`'s own `face_scale`
    (`crates/ui-screens/src/campaign/footer.rs:97`) carries the correct form -
    `"default" => 1.0` with a doc comment naming this exact shape - for the
    footer's `Confirm`/`Back`/`Help` prompts, landed when `face_atlas_slot`
    did; `Layout::face_scale` (`crates/ui-screens/src/campaign.rs`, feeding every
    `Line1`..`8`/`Title`/`Track Line`/`Speed class` label on both screens)
    was simply never given the matching fix. Now is: `"default" => 1.0`,
    mirroring `footer::face_scale` exactly. `"small"` is untouched - no real
    `Small`-role atlas is ever loaded, so that text still has to fake its
    size out of the `Menu` atlas's own glyphs, the ratio it always needed.
    Verified against the same aligned crop: this build's `"Speed class"` row
    now reads within a pixel of the PPSSPP reference's 18px at the same
    960x544 scale, both screens' full panels visually match side by side,
    and `cargo nextest run -p oag-ui campaign`/`-p oag-game campaign` both
    stay green - nothing asserted the old, wrong size. Confidence 92.
  - **The title bar text (`RACE CAMPAIGN`) was flagged as possibly
    oversized** - checked against the open question (tracked in this
    project's own frontend handover) of whether Pulse's chrome title should
    flip from the row face to its own taller `Title` role. A scale-normalized crop (both
    frames resampled to the same 960x544) shows the two titles close to the
    same size by eye, and a rough cap-height measurement reads *larger* in
    the reference (36px) than in this build (32px) - the opposite direction
    from "ours is too big." Given the noise in that measurement (the title's
    own `pulse="true"` glow inflates a brightness-threshold height
    differently between the two images) and that it does not support a
    same-direction fix, this capture is read as **not settling** the
    row-face-vs-`Title`-role question either way; the `Next Steps` in that
    handover file stand unchanged.

  The reference frame's scrolling tip and `AAA` badge still draw as an empty
  bar in ours - **not a new finding**,
  `oag_ui_screens::campaign::draw::grid_draw_list`'s own doc already names this as
  `--menu-page`'s own known gap (the capture has no `CampaignStage` behind it
  to drive the ticker's clock or its tip list), not a live-game defect. The
  two captures otherwise disagree only on medal/points state (this
  worktree's own `records.toml` carries real progress, `04/08` gold rather
  than the reference's fresh-profile `0/8`) - a data difference, not a
  rendering one.
  - **`Cell Selection` against `cell-selection-grid0-default-cell.png`,
    checked the same way, time allowing:** the same selected-hex cyan glow
    (now fixed, see above) and the same dim locked-cell outlines/white
    padlocks as `Grid Selection`'s own (the `Outline_` tint the previous
    sentence's own parenthetical named as `Cell Selection`'s one open item
    turned out already fixed - see the `pulse-campaign` lane's 2026-09-28
    correction above). Text formatting, row layout and the
    detail panel's own scale were not separately re-checked against this
    frame this pass - the two screens share `text_draw`/`FaceScales`, so the
    panel-scale bug confirmed above (the `pulse-campaign` lane's 2026-09-28
    direct comparison) applies here too, unverified against this specific
    frame but the same code path.
- **A previous pass's live-walk cell (`grid0_2_1`) is now locked under this
  pass's own rule** - it authors no `Locked` attribute, which defaults to
  `true`, and has no medal or medalled neighbour on a fresh profile. The
  "launch path... driven live" bullet below recorded a walk through that
  exact cell; repeating it today would refuse `Confirm` rather than reach
  `Team Selection`. `grid0_3_1`/`grid0_3_2` (both author `Locked="false"`) are
  the cells to re-walk with, or any cell after the profile has earned it or
  a hex-adjacent medal.
- ~~PPSSPP was not captured against this pass.~~ **Captured, `pulse-campaign`
  lane, 2026-09-28** - the first live PPSSPP capture of this leg.
  `scripts/psp-frontend-capture.py` now walks it (commit `7f2b7855`), and
  this pass drove it by hand under Xvfb `:93`/port 45001 for the
  aligned-crop comparisons the fixes above cite. **The walk is one hop
  shorter than assumed: `Main Menu` (`FE_RACE_CAM`) confirms straight into
  `Grid Selection`, with no `TournamentLoad` frame ever observed** - a 50ms
  poll right on the confirm press, on a genuinely fresh profile (all four
  first-boot dialogs answered first), never caught one. `MainMenu_Definition.xml`
  explains why without contradicting `TournamentLoad`'s own authored
  `Dialog`/`Redirect` blocks (`Icon="Warning" TextID="MSC_MSG_AUTOSAVE3"
  NumOptions="0"`, gated `Entry item="DialogMenu" equals="MSC_SQ_MSG7" goto="Grid
  Selection"`): a zero-option dialog has nothing for a player to answer, so
  it can resolve to its own redirect in fewer ticks than this project's
  polling cadence catches - a `Redirect` passing through in zero rendered
  frames, not evidence the screen or its dialog are unauthored or skipped.
  Say "no frame observed on either a fresh or a returning profile", not
  "never appears" - this pass could not tell those apart. The `Grid`/`Grid1`
  crossfade-vs-settled-state question and the empty-hex-fill-on-a-fresh-save
  question are **still open**; this pass's own profile stayed at zero
  medals throughout (see the cells re-walked below), so neither was
  reachable from it either.
- ~~`Cell Help`'s own overlay is read but not drawn.~~ **Draws as a static
  panel, 2026-09-21** - see "The tip ticker..." above. Its `Viewport`/
  `Animation` scroll timeline (`LimitVerticalScroll="10"`) is still not
  scripted, on purpose.
- ~~The Selector cursor's exact centering is unmeasured.~~ **Fixed
  2026-09-14.** `hex_filled.mip`/`hex_outline.mip` decode to a 32x32 4bpp
  indexed image (`Data.wad` on `pulse-psp-eu.chd`, measured directly off the
  blob's own header and palette alpha), so the selected hex is always 32x32
  against `Selector`'s own authored 42x43
  (`pulse_assets.mip`, `l="42" m="43"`) - `CellMode_Definition.xml` authors
  no offset between the two on either screen, only `Selector`'s own default
  `x="30" y="95"`, which `hex_rect` already overrode outright. Top-left
  aligning a 42x43 sprite to a 32x32 hex's own corner (what this build did
  before) draws the cursor visibly down-and-right of the hex rather than
  around it - confirmed live, `xdotool` hovering a tier on
  `pulse-psp-eu.chd` at 2026-09-14's window scale, screenshot on file with
  the `campaign-pointer` lane. `oag_ui_screens::campaign::centred_selector_draw`
  centres the two rects on each other instead - **chosen, not measured**:
  both sprite sizes are the disc's own, but nothing on disc says centring is
  the right rule for the gap between them, only that top-left alignment
  reads wrong. `the_selector_is_centred_on_the_selected_hex_not_top_left_aligned`/
  `cell_selections_selector_is_also_centred_on_the_selected_hex`
  (`crates/ui-screens/src/campaign/tests.rs`) pin the two screens' own resolved
  rects.
- ~~The launch path is wired but not interactively played in this pass.~~
  **Driven live, 2026-09-14**, mouse-only, under Xvfb with `xdotool`
  (`docs/architecture/menus.md`'s recipe): `RACE CAMPAIGN` -> hover/click a
  tier -> hover/click a `Race` cell (`grid0_2_1`, `16_Track`) -> `Team
  Selection` -> click the ship -> `--autopilot` to the results table -> back
  on `Cell Selection`, `Line6`/`Line7` still read `"0/3"`/`NICHT VERFÜGBAR`
  (German for "none") because both runs (default skill, then
  `--autopilot-skill ace`) finished 4th of eight - this build's default
  `settings.toml` already races opponents at `ace`, the ceiling the flag
  also names, so there was no headroom left to podium without editing a
  config outside this worktree, which this pass declined to do. ~~Still
  open: a live capture of `Line6`/`Line7` actually reading a nonzero
  medal.~~ **Closed, 2026-09-21** - see "The tip ticker..." above: a
  worktree-local `[ai] difficulty = "novice"` (never committed) plus
  `--autopilot-skill ace` finished `grid0_3_1` 1st of 8, and `Cell
  Selection` read `Points 3/3` / `Best Gold` back.

### Wipeout HD/Fury, 2026-09-21: measured on RPCS3

**The real disc has a `Campaign Selection` screen ahead of `Grid Selection`
that this build does not model at all, and its default reaches `Fury`
(`grid8`..`grid15`), not the base campaign `grid0`..`grid7` lives in.**
`scripts/rpcs3-drive.py`'s own `RACE_WALK` already named the shape
(`"Campaign Selection", "Grid Selection Fury", "Cell Selection", ...`) but
this is the first pass to boot it and notice what it implies. Confirmed
twice, on separate boots (own `--config` copy under
`data/scratch/lane-hd/rpcs3-scratch-config.yml`, Xvfb :77, `just
rpcs3-preflight` OK beforehand):

1. Pressing `cross` with no d-pad input at every screen (every other step's
   own default-highlighted row, matching `RACE_WALK`'s own doc comment)
   lands on `Fury`'s `grid8` (`"blitzed"`, `Eliminator`, `The Amphiseum`),
   confirmed by the `EVENT 01/08` counter (`grid_08` is the first of
   `DATA00`'s eight Fury-only grids) and the screen title reading
   `CAMPAIGN` over a red skin, not the grey one this build's own capture
   uses.
2. Adding one `down` press at `Campaign Selection` before confirming landed
   on the exact same `grid8` cell again - `down` is not the toggle between
   the two campaigns, or there is no toggle reachable this way at all.

~~This build's own `oag_hd`/`oag_ui_screens::campaign` has no `Campaign Selection`
state whatsoever~~ - **Modelled and driven, 2026-09-21**, see
"Wipeout HD/Fury: `Campaign Selection`, 2026-09-21" above:
`oag_ui_screens::campaign::selection` reads the screen off `DATA06`'s own copy of
`CellMode_Definition.xml` (the precedence-resolved `DATA02` copy this
build otherwise reads has no such screen at all), `right` on the pad is
measured, three separate RPCS3 boots, as the toggle to the base `Wipeout
HD` campaign, and `crate::campaign_stage::CampaignStage` now opens on it
for HD, slicing the sixteen grids into `grid0`..`grid7`/`grid8`..`grid15`
per the confirmed entry. Screenshots:
`data/scratch/lane-hd/rpcs3-grid0-3-2/01-down.png` (`grid8`, `NOVICE`
rung), `02-triangle.png` (`SKILLED` rung, same cell) - this session's own
worktree, not committed (game content). Confidence 85 on "the screen exists
and defaults to Fury" (two independent boots, consistent); confidence 85 on
`right` reaching the base campaign and confidence ~50 on `up`/`l1`/`r1`
being no-ops rather than a redundant second toggle - see the new section's
own "The toggle" subsection for the three-boot breakdown.

**A second, unplanned finding directly relevant to priority 4's own
question** (which target triple `Elimination`/`NitroBattle` reads):
`Fury`'s `Eliminator` cell's difficulty rungs are named **`NOVICE`**/
**`SKILLED`** on screen (`TARGET 200 (NOVICE)` -> `TARGET 200 (SKILLED)`
after a difficulty press, same numeric target, different rung name), not
`Gold`/`Silver`/`Bronze` - first-party UI evidence, not a decompile, that
`Elimination`-family cells read a differently-named rung triple. This
corroborates, without proving the internal representation, the
`NitroElimNovice`/`Skilled`/`Elite` hypothesis
`crates/hd/src/campaign.rs`'s medal-law question opened with. The footer's
own legend icon reads as PlayStation `Square`
(`DIFFICULTY (NOVICE)`/`DIFFICULTY (SKILLED)`, confirmed by cropping and
zooming the frame), but empirically only a synthetic `triangle` press
changed the rung across a `square`/`triangle`/`l1`/`r1` sequence -
`square`/`l1`/`r1` were all no-ops. **Chosen not to chase which physical
button this actually is** (may be this rig's own `oag` RPCS3 input profile
disagreeing with the icon, or a dropped `square` press - see
`RACE_WALK`'s own doc comment on dropped presses at ~9 fps); the rung
**naming** is the trustworthy part of this finding, not the button.
Confidence 90 on the reading itself - two consecutive, distinct frames on
the real disc, the difference legible in both the `TARGET` line and the
footer legend, not a guess. Confidence unmeasured (0) on how far it
generalises: only one cell, one mode (`Eliminator`), was checked, so
whether every `Elimination`/`NitroBattle` cell reads this same
`Novice`/`Skilled`/`Elite` triple - as opposed to, say, a per-cell mix - is
not settled by this alone.

**2026-09-21, this hypothesis moved from RPCS3 corroboration to a direct
Ghidra reading.** `EBOOT-ps3-hdfury-eu.elf`'s own `PI_Cell` attribute table
(`0x008ae898`) lists `NitroElimElite`/`Skilled`/`Novice` as three of `PI_Cell`'s
own fields, immediately after `EasyGold`..`EasyBronze` - not a coincidental
string reuse. A separate save-migration string (`0x00779a98`) reads `"...to
have HARD(ELITE) for best skill level"`, independently measuring
`Novice`/`Skilled`/`Elite` as this title's own words for `Easy`/`Medium`/
`Hard`, matching this capture's own `TARGET 200 (NOVICE)` -> `(SKILLED)`
reading rather than merely being consistent with it. **Still not settled**:
which function actually compares a race result against this triple, and
whether it is one pass/fail number per rung or a three-tier medal ladder like
`Gold`/`Silver`/`Bronze` - see
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md` for the full evidence
and its own "what is not determined" section.

- **Which archive copy of `grid_00.xml`..`grid_07.xml` the real screen
  shows by default is still not RPCS3-verified** - both attempts this pass
  landed on `Fury` instead of the base campaign; see above. This pass found
  offline (not RPCS3-confirmed) that the precedence-resolved flat copy's
  own numbers equal the per-difficulty copies' `hard` rung, not `medium`,
  corroborated on a *second* cell this pass (`grid0_3_2`: `DATA02` flat
  `3700`/`3800`/`4000` equals `DATA04`/`DATA06`'s own `hard` rung exactly)
  - see "Which archive copy... a real disagreement found" above. Whoever
  next finds a path to the base campaign from `Campaign Selection` can
  settle this in the same boot: `grid0_3_2` (`SpeedLap`, authors
  `Locked="false"` on the disc) discriminates the three rungs by 150-350
  units. `grid0_2_2`, this thread's earlier suggested cell, authors no
  `Locked` attribute at all - **this build's own rule** defaults that to
  locked on a fresh profile (itself chosen, not measured, per the unlock-
  reason bullet below), so it was not used here, but whether the real game
  gates it the same way is not itself established; neither cell has been
  reached on RPCS3 yet. `scripts/rpcs3-drive.py browse`, `just
  rpcs3-preflight`, `docs/formats/hd-frontend.md`'s "How this was
  measured".
- ~~The stray hex outline on `Cell Selection`~~ **Fixed 2026-09-21** - it
  was `Selector` centred on the wrong rect, not a seventh cell; see "The
  stray hex outline was `Selector`, not a seventh cell" above. Left open by
  that fix: whether `Medal_{x}_{y}`'s own oversized-atlas widget draws
  correctly sized once a real medal exists to show it (untestable on a
  fresh profile).
- ~~**The 3-D flyer model behind `Grid Selection`** is not drawn at all~~
  **Drawn 2026-09-30** for the base campaign, through the one mesh-in-menu
  seam `oag_game::preview` now shares - see "The flyer behind `Grid Selection`".
  Fury's eight cards and `Campaign Selection`'s pair followed the same day.
  Open from it: the card's own body and reflection, the glow and the
  tier-change swing.
- ~~The launch path is wired... but not driven live against an HD source~~
  **Driven live, 2026-09-21**, mouse-only under Xvfb :93 with `xdotool`
  (`cargo run -p oag-game -- data/images/hdfury-ps3-eu-dec.iso
  --autopilot`): `Main Menu` -> `RACE CAMPAIGN` -> `Grid Selection` (its own
  confirm rect, `hd_grid_targets`) -> `Cell Selection` -> click the already-
  selected hex (the pointer fix's own first live exercise, see "The stray
  hex outline..." above) -> `LOADING... VINETA K` (no ship picker, matching
  HD's own `race_box: None`) -> race genuinely started (window title `-
  race`, `TickClock` ticking, correct HUD/opponents/track). **Not driven to
  results** - this sandbox's render path is two orders of magnitude slower
  during a race than on the campaign screens (`race scene built in 107.6s`,
  then `741ms`-`8056ms` per frame vs. `10-25ms` outside a race), consistent
  with a software Vulkan fallback rather than a code bug; a full race would
  need on the order of two hours of wall clock at the observed tick rate,
  not spent this pass. `session::campaign::handle_campaign`/
  `launch_campaign_cell` received no HD-specific edit, confirmed once more
  by this walk working unmodified.
  **Driven to results, 2026-09-28**: with the HD render profile cut to
  `render_scale = 50` and MSAA, motion blur and shadows off, the same
  llvmpipe adapter ran the race at 60 ticks a second, and the walk reached
  `EndRace Results`, `EndRace Menu` and back to `Cell Selection` - see
  "Wipeout HD/Fury: walked live, end to end, 2026-09-28" below.
  **2026-09-28, `grid8_3_1` specifically checked on both sides, without a
  new RPCS3 race boot.** The two-orders-of-magnitude slowdown above is
  **this project's own** race render path, not RPCS3's - a fresh RPCS3 boot
  to a running race costs the same few minutes any other capture in this
  document does. This pass skipped a new one anyway because a pre-existing
  capture already had it: `data/reference/hd-capture/talons-matched/`
  (2026-09-13, the default Fury-campaign walk, same `grid8_3_1` this pass's
  own `browse` boot also landed on) carries both
  `screen-Cell-Selection.png` (`blitzed`, `EVENT 01/08`, `SINGLE RACE`,
  `TALON'S JUNCTION`, `VENOM`, `WEAPONS ON`, `LAPS 3`) and three in-race
  frames (`00.png`-`03.png`) whose own `NN.json` carries
  `"track": "Data\\Environments\\Talons_Junction\\track.rcsmodel"` - read
  from `TTY.log`'s own `Loading track model` line inside the *started*
  race, not the pre-launch screen - and whose HUD reads `LAP 1/3` and `POS
  x/8` (matching `grid8_3_1`'s own authored `AICount="7"` plus the player).
  So two of the four fields - **track and laps** - are measured from a
  launched race on the RPCS3 side, not only inferred from the selection
  screen's own text; **mode and class still come only from the selection
  screen** on both sides (the in-race HUD does not label either directly).
  On this project's own side, rather than drive a second race build (the
  slow path measured above), this pass read `launch_campaign_cell`
  (`crates/game/src/main/session/campaign.rs:287`, read-only - a different
  lane's own file), whose mapping for this cell is field-for-field:
  `race_mode_for_cell(cell.mode)` sends the authored string `"Race"`
  straight to `oag_race::Mode::SingleRace` (unit-tested,
  `crates/game/src/campaign.rs`'s own `the_seven_implemented_modes_map_onto_their_oag_race_mode`),
  `race_options.class = cell.class.clone()` keeps `"Venom"` verbatim, and
  `laps_override = cell.laps` carries `3` through unmodified for
  `SingleRace`. `race_options.track` resolves `"17_Track"` through
  `shell.track` to the same circuit `docs/formats/hd-frontend.md`'s own
  `17_Track -> Talons_Junction` measurement already names, matching
  `talons-matched`'s own `TTY.log`-sourced track string. Every one of those
  four values (`Race`/`Venom`/`3`/`Talon's Junction`) also reads identically
  at the *selection* screen on both sides - this pass's own
  `ours-cellselect-default.png` next to the RPCS3 `01.png`/`04.png` frames
  above, all four fields matching pixel-for-text. What this does **not**
  re-confirm is that `finish_launch`/`open_ship_picker` carry those same
  `race_options` through into a *started* race unmodified for this specific
  cell on this project's own side - only that the values reaching
  `race_options` are right; the 2026-09-21 walk already showed a *different*
  HD cell's launch reaching a
  genuinely ticking race with correct HUD/opponents/track, and nothing in
  the code path between the two cells differs.
- **The `Required`/`Required Previous`/`NextPoints` unlock-reason
  predicates are chosen, not measured**, on both which text shows and
  what number a raw `%d` template should carry - see the two sections
  above. An RPCS3 capture of a genuinely locked tier/cell would settle
  both at once.
- **The upper-case defect above is partially closed, 2026-09-21, from the
  render side rather than this crate's own.** `Confirm`/`Back`
  (`oag_ui_screens::campaign::footer`) and the per-row subtitle
  (`oag_title::HelpText`) now draw mixed case, through a second,
  `Default`-role atlas the menu stage loads beside its unchanged `menu`-role
  primary - see `docs/ui/menus-original.md`'s "Two faces, not one swapped
  for the other" section for the full mechanism and the measured fact it
  rests on (Pulse's `menu`/`Small` faces have no lowercase glyph art at
  all; only `Default` does). **Still upper-case**: every `n="default"`
  widget `oag_ui_screens::campaign::draw::text_draw` itself draws - `Speed class`,
  `Laps`, `Weapons`, `Points`, `Best` and their values, and by the same
  mechanism `Change Difficulty` two bullets up. One edit closes it:
  `text_draw`'s single `Draw::Text` literal becoming a role-aware
  constructor (`crate::frontend::Draw::in_role`, `Some("Default")` for a
  `"default"`-labelled `text.font`, matching what `footer.rs` and
  `menu::rows::draw_text_rows` already do) - a change to this crate's own
  `campaign/draw.rs`, outside the lane that found and fixed the mechanism.
  `NavigationLegend`'s own `FE_CONFIRM` shrink-to-fit (`Prompt::left_bound`/
  `align_right_to`, above) is now measured against the atlas it actually
  draws through rather than the wrong one - **on both paths**: the live
  session (`crate::main::menu_stage::MenuStage`'s own `default_atlas`/
  `default_measure`) and `--menu-page`/`capture.rs`, which had its *own*,
  separate `measure` closure still pointed at `menu_font` and was the
  reason the first live capture of this fix (`cellselect-usa-after2.png`)
  showed `Confirm` at roughly a third of `Back`'s size, sitting above the
  baseline - caught in review, not by this lane's own verification, and
  fixed in `crate::capture::run`'s own `campaign_page` call. Measured
  correctly now, `Confirm` **still shrinks, barely**: `pulse_text.fnt`'s
  own native width for `"Confirm"` is 41 native px against a 35px gap
  (`FE_BACK_BUTTON`'s `x` minus `FE_CONFIRM_BUTTON`'s `x` minus one
  glyph-width estimate minus two `GAP` constants, all four chosen) - a
  ~15% reduction, not the ~65% the wrong atlas produced. The face was
  the wrong thing to suspect for the overlap this fixed *before* this
  atlas fix landed; now that the face and the measurement are both right,
  the remaining, much smaller shrink is `GAP`/the glyph-width estimate's
  own numbers, still chosen rather than measured. Confirmed live,
  `cellselect-after3-footer-crop.png` next to the reference frame:
  `Confirm` and `Back` now match in size and sit on the same baseline.
- ~~Wipeout HD/Fury's own `Cell Selection` draws no `Confirm`/`Back` legend
  at all - a different lane's own thread.~~ **Done, 2026-09-25**:
  `oag_game::campaign::load_hd`/`load_omega` now call the same
  `read_footer` Pulse's own `load` does, pointed at
  `oag_hd`/`oag_omega::frontend::names::FRONTEND_ROOT` instead of Pulse's -
  the identical `NavigationController` shape lives on HD/Omega's own shared
  `Skin.xml` too (`Top FE Screen -> FE Screen ->
  BodgeScreenContainingNavigationController`, that literal name is the
  disc's own). Wired into `oag_ui_screens::campaign::hd::hd_cell_draw_list`'s new
  `footer_overlay` parameter, drawn on `Cell Selection` only - the same
  screen Pulse's own draws it on, and for the same reason: neither title's
  `CellMode_Definition.xml` authors a `NavigationButtons` gate on that
  screen, so there is nothing to read a narrower rule off. `Grid Selection`
  still does not draw it, matching Pulse's own scope. Confirmed live via
  `--menu-page cell-select` against `hdfury-ps3-eu-dec.iso`: `CONFIRM`/
  `BACK` both draw at the screen's own authored position
  (`data/scratch/drive-2026-09-25/shots/hd-cellselect2.png`, not committed).
  HD's own ticker stays unbuilt - confirmed by direct read that its shared
  `Skin.xml` authors no `TextInfoIsAlwaysLast` viewport at all, so
  `TickerLayout::read` correctly answers `None` rather than there being
  anything left to wire.

## Wipeout HD/Fury: `--menu-page` stills match a real screen state, and the footer draws on all three, 2026-09-25

**Side-by-side pass**: `--menu-page campaign-select`/`grid-select`/`cell-select`
against `hdfury-ps3-eu-dec.iso` at 1280x720, next to the RPCS3 frames already
on disk from the 2026-09-14/09-21 passes above
(`data/scratch/lane-hd-sel/rpcs3-campaign-selection/`,
`data/scratch/lane-hd/rpcs3-grid0-3-2/`) - no new RPCS3 boot needed, the
existing captures cover all three screens once the settled (not mid-animation)
frames are used (`01-right-tap.png`/`01-l1-tap.png` for `Campaign Selection`,
not the corrupted `00-default.png` grabbed mid-transition).

**A capture-only bug, not a live-session one: `grid-select`/`cell-select`
showed `Event 01/16`, a state no real screen ever reaches.**
`crate::capture::campaign_page::campaign_page`'s HD arms read
`campaign.grids` whole - all sixteen, base `Wipeout HD`'s `grid0`..`grid7`
plus `Fury`'s `grid8`..`grid15` concatenated - because a still has no
`Campaign Selection` step to narrow it the way
`CampaignStage::open_grid_selection` always does before a live session ever
draws either screen. RPCS3's own frame reads `EVENT 01/08` (see "measured on
RPCS3" above); ours read `01/16`. Fixed in
`crates/game/src/capture/campaign_page.rs`: both arms now slice to one
campaign's own eight grids before building `GridSelection`/`CellSelection`,
defaulting to `Fury` when `campaign.grid_layout_fury` is `Some` (using that
layout, not the base campaign's `campaign.grid_layout`, for `Grid
Selection`'s own screen) and falling back to the base campaign otherwise.
`Fury` is picked because it is both the measured default
(`CampaignSelection::new`'s own `index: 0`, "The toggle" above) and the
campaign every RPCS3 reference frame on disk actually shows - a `--menu-page`
still is now directly comparable to those frames rather than to a
combined-16 state nothing on the real disc, or this build's own live
session, ever draws. **Capture-only**: the live `CampaignStage` path already
narrowed correctly before this change; a player was never shown `01/16`.
`crates/game/src/main/campaign_stage.rs`'s own `grid_layout()` already picks
`grid_layout_fury` the same way when `active_campaign == Some(Campaign::Fury)`
- the still-path fix mirrors that existing rule rather than inventing a new
one.

**The footer legend draws on `Campaign Selection` and `Grid Selection` too,
not `Cell Selection` alone.** The entry above ("Done, 2026-09-25") reads
"`Grid Selection` still does not draw it, matching Pulse's own scope" -
correct for Pulse (nothing measured that screen wanting it there), but this
pass's own RPCS3 frames say otherwise for HD specifically:
`rpcs3-campaign-selection/01-right-tap.png`/`01-l1-tap.png` both show
`NAVIGATION  Ⓧ CONFIRM  Ⓞ BACK` under `Campaign Selection`, and
`rpcs3-grid0-3-2/00-default.png` shows the same row plus a third prompt
(`CHANGE DIFFICULTY`) under `Grid Selection Fury` - the identical row `Cell
Selection`'s own frame carries. `hd_grid_draw_list`/`selection::draw_list`
(`crates/ui-screens/src/campaign/hd.rs`/`crates/ui-screens/src/campaign/selection.rs`) both
gained the same `footer_overlay: &[Draw]` parameter `hd_cell_draw_list`
already had, appended to `layers.chrome` right after the title the same way;
`crates/game/src/main/menu_stage.rs`'s `Selection`/`Grid` arms and
`crates/game/src/capture/campaign_page.rs`'s two HD arms all now pass
`campaign.nav_legend_draw(...)`/the already-computed `footer_overlay`
through, the same call `Cell`'s own arm already made. No ticker on either
screen, same reason `Cell Selection` has none (HD's shared `Skin.xml`
authors no `TextInfoIsAlwaysLast` viewport at all).

**Still not drawn, both sourced and left open rather than guessed at:**

- ~~The `Confirm`/`Back`/`Change Difficulty` glyphs themselves~~ - **fixed,
  2026-09-25**, and the `Change Difficulty` bullet's own premise was wrong -
  see "Wipeout HD/Fury: the footer's button glyphs, and the `GOLD MEDALS`
  denominator" below for both corrections.
- ~~`Grid Selection`'s own third prompt, `CHANGE DIFFICULTY`~~ - **not a real
  gap, 2026-09-25**: the RPCS3 frame this bullet was read off is `Cell
  Selection`, not `Grid Selection` - see the same section below. `Grid
  Selection` authors no `DifficultyButton` widget anywhere and draws
  `Confirm`/`Back` only, which is the disc's own answer.
- ~~**The 3-D flyer model behind `Grid Selection`/`Campaign Selection`**~~ -
  `Grid Selection`'s sixteen cards are **drawn, 2026-09-30**, see "The flyer
  behind `Grid Selection`" and "The two cards".

**Recaptured after the fixes above**, all HD, `hdfury-ps3-eu-dec.iso`,
1280x720, `data/scratch/drive-2026-09-25/hd-campaign/` (not committed, game
content): `ours-campaign-select-v4.png`, `ours-grid-select-v4.png`,
`ours-cell-select-v4.png`. `cell-select-v4`'s own default cell now reads
`grid8`'s first entry (`19_Track`, `NitroBattle` mode) rather than `grid0`'s
- `class="NitroBattle"` on the `Speed Class` row is not a misread: `grid_08.xml`
itself authors `class="NitroBattle"` for this cell
(`data/scratch/drive-2026-09-25/hd-xml/DATA00/data/plugins/grids/grid_08.xml`),
the disc's own value, read as-is rather than second-guessed.

Confidence 85: the `Event 01/16` reading and the RPCS3 footer evidence are
both directly observed (the captures named above, and the three RPCS3 frames
cited), not inferred; the `Fury`-default choice for a still with no
navigation state is **chosen, not measured** on the same terms
`CampaignSelection::new`'s own default already is.

## Wipeout HD/Fury: the footer's button glyphs, and the `GOLD MEDALS` denominator, 2026-09-25

**The `Confirm`/`Back`/`Difficulty` icon glyphs draw now, through a genuine
third GPU-side atlas.** `ControlTextConfirmButton`/`BackButton`
(`NavigationController`, the shared front-end root) and `Cell Selection`'s
own `DifficultyButtonIcon` all author `font="buttons"` -
`ps_buttons.fnt`/`PS_BUTTONS.fnt`. This build used to load no atlas for that
role at all, on the reasoning that a wrong glyph is worse than none per
`CLAUDE.md`'s "never invent" rule; that reasoning still holds, and what
closes the gap is loading the *real* face instead of a substitute for it -
`oag_ui::language::roles::BUTTONS`, resolved off every HD language plugin's
own `<Font><Values name="Buttons" ... Src="Data\FE\Fonts\PS_BUTTONS.fnt">`
slot (`docs/formats/hd-frontend.md`'s "`menu_font` is `None`, and that is a
measurement" section already had this - all 32 plugins on the disc declare
it, this pass is the first to load it). Loaded by
`oag_game::boot::fonts::load_buttons_font` into a *third* texture slot
(`oag_game::render::Renderer::set_buttons_atlas`, `MODE_BUTTONS_ATLAS` in
`ui.wgsl`) rather than reusing the one `face_atlas_slot` already spends on
`Title`/`Default`: a screen's own title (`Title` role) and its footer's
button glyphs (`Buttons` role) are on screen in the same frame, so one slot
cannot serve both.

**Verified against the disc before wiring it up, not after.**
`cargo run -p oag-tools --example hd_buttons_font_probe -- <hd iso>
[out.png]` reads `PS_BUTTONS.fnt` through the same `Archives::read_font`
production uses and dumps the codepoints this build actually asks it for:
`FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON`/`DifficultyButtonIcon` (`ε`/`γ`/`δ`) all
carry real glyph boxes, and the atlas dump
(`data/scratch/drive-2026-09-25/hd-footer-glyphs/atlas-confirm-back-difficulty.png`)
shows exactly a circled cross, circle and square - matching RPCS3's own
`Ⓧ CONFIRM`/`Ⓞ BACK`/`⬜ DIFFICULTY (...)` shape for shape. The first attempt
at this probe read the *wrong* archive entry (`/data/fe/fonts/ps_buttons.gtf`,
picked by an unfiltered `.first()` over a substring match) and decoded a
plausible-looking-but-wrong 524,416-byte blob - a reminder that "it decoded
to something" is not evidence it decoded the right thing; see the probe's
own module doc for the correction.

**HD's own `CONFIRM` needed one more fix once its icon stopped being
excluded: it must not shrink-to-fit the way Pulse's does.** Pulse's own
`Confirm`/`Back` glyphs sit close enough together at `menu`-role scale that
the word overlaps the back glyph, so `NavigationLegend::read` right-aligns
and shrinks `FE_CONFIRM` to fit between the two (`docs/ui/campaign-screens.md`'s
own earlier "not reachable" note on that). Once `FE_CONFIRM_BUTTON` stops
being excluded from `prompts`, that same block would fire for HD too -
moving `CONFIRM` off its authored `x="534"` to a shrunk, right-aligned
position RPCS3 never shows. Gated on the word's own authored `scale`, a
disc-authored fact rather than a title check this crate would otherwise
invent: HD's own `ControlTextConfirm` authors `scale="0.8"`
(`<Values idstring="FE_CONFIRM" x="534" ... scale="0.8">`), Pulse's authors
none (`1.0` by default) - the shrink block now only applies when
`scale > 0.99`.

**Correction to the "measured on RPCS3" section above: `00-default.png` is
`Cell Selection`, not `Grid Selection Fury`.** The 2026-09-14/09-21 passes
cited `data/scratch/lane-hd/rpcs3-grid0-3-2/00-default.png` as a corrupted
capture of `Grid Selection Fury` showing a third footer prompt,
`CHANGE DIFFICULTY`. Re-reading the same directory this pass: every *clean*
capture in it (`01-down.png`, `02-square.png`, `02-triangle.png`) is
unmistakably `Cell Selection` - the hex grid, the `blitzed` title, the
`EVENT TYPE`/`TRACK`/`SPEED CLASS` detail panel - and `00-default.png` itself
is the same layout, just corrupted by a comb-artifact capture, not a
different screen. `Grid Selection`/`Grid Selection Fury` author no
`DifficultyButton` widget anywhere in `CellMode_Definition.xml` at all (both
`DATA02`'s and `DATA06`'s copies checked directly), so the footer row this
build now draws there - `Confirm`/`Back` only - is the disc's own answer,
not a remaining gap.

**The footer's own third prompt reads `DIFFICULTY (<rung>)`, not the disc's
authored `"Change Difficulty"` string - a real, measured mismatch, left open
rather than guessed at.** Zoomed crops of the clean captures
(`02-square.png`'s own `difficulty-icon-zoom.png`, re-derived this pass as
`data/scratch/drive-2026-09-25/hd-footer-glyphs/difficulty-icon-zoom.png`)
read `⬜ DIFFICULTY (NOVICE)` and, after a `Triangle` press,
`⬜ DIFFICULTY (SKILLED)` - composed at runtime from the current difficulty
rung, not the literal `string="Change Difficulty"`
`DifficultyButton` authors in `CellMode_Definition.xml`. The rung words
themselves are disc strings (`Easy`→`"NOVICE"`, `Medium`→`"SKILLED"`,
`Hard`→`"ELITE"`, `entries.xml`). **Not implemented this pass**: this build's
own `CellSelection::difficulty` also defaults to rung `1` (`SKILLED`) where
RPCS3's own fresh-profile default reads `NOVICE` (rung `0`) in the same
captures - two open mismatches on the same widget, named together rather
than fixing the text and leaving the default silently wrong, or vice versa.
`Cell Selection` still draws the disc's own authored `"Change Difficulty"`
string in the meantime, which is honest disc content, just not what RPCS3
shows at runtime.

**Both open mismatches fixed, `hd-difficulty` lane, 2026-09-28 (later the
same day).** `oag_ui_screens::campaign::hd::hd_difficulty_button_line` now computes
`RB_AI_DIF`/`RB_DIF (<rung>)` the same way `crate::campaign::draw::difficulty_button_line`
already does for Pulse, mode-gated per `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
`CellSelection_UpdateDifficultyButton_q` finding; `CellSelection::difficulty`
now defaults to `Difficulty::Easy` on HD (`with_default_difficulty`, wired
from both the live session and the `--menu-page cell-select` still), Pulse's
own `Medium` default untouched. **The default is now measured on a
genuinely fresh profile**, not the non-fresh one every earlier pass on this
page had: `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/` verified empty
before boot, `scripts/rpcs3-drive.py capture --nav-shots`' settled arrival
frame (no `DifficultyButton` press, no comb-artifact risk) reads `AI
DIFFICULTY (NOVICE)` on `grid8_3_1` - confidence 90, corroborating rather
than superseding the `Race`-mode `RB_AI_DIF` reading above. See
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s "The TOC-xref trap"
section for the Ghidra side, including why the `RB_AI_DIF`/`RB_DIF` call
site itself was not found this pass.

**Pulse's own sibling mechanism is fixed, `pulse-cellsel` lane, 2026-09-28 -
a related but not identical fix, not reusable here as-is.** Pulse's
`CellSelection_Update` (`0x088d6430`) builds an analogous runtime template
for the identical `DifficultyButton` widget, but the wording differs on
every axis this HD reading names as open: the label reads `"AI difficulty"`
(`RB_AI_DIF`, the bare-lowercase idstring `Single Player`'s own `Difficulty`
row already authors), not `"DIFFICULTY"`; the rung words are Pulse's own
bare `"Easy"`/`"Medium"`/`"Hard"` idstrings, not HD's `entries.xml`
`Novice`/`Skilled`/`Elite`; and Pulse's own fresh-profile default is
measured at rung `1` (`Medium`) via `Profile_SetDifficultyRC(profile, 1)`, not
`0`. See
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The `DifficultyRC`
persisted rung" section - a real decompile now exists for the *shape* of
this mechanism (a getter/setter pair over a hashed profile record, a
`CellSelection_CommitSelection`-time persist), which the next HD pass on
this widget's own bare `"DIFFICULTY"` idstring and default-rung mismatch can
use as a structural reference without assuming either title's exact wording
or default carries over.

**Correction, 2026-09-28, two parts.**

First: this reading holds for `grid8_3_2` (`Eliminator`, The Amphiseum)
specifically, not every cell. The cell this section's own captures used
authors an `Elimination`-family target triple (`TARGET 200 (<rung>)`); a
`Race`-mode cell reads differently. `grid8_3_1` (Fury's default cell, Single
Race, Talon's Junction) reads **`AI DIFFICULTY (<rung>)`** - the extra `AI`
confirmed across two independent boots, this project's own 2026-09-28 pass
and a pre-existing 2026-09-13 capture
(`data/reference/hd-capture/talons-matched/screen-Cell-Selection.png`) - not
a clipped crop of this section's own frames (re-checked at a wider crop
directly). See `docs/ui/campaign-screens.md`'s "Which block is which
difficulty" section (2026-09-28, above this page) for the full three-boot
account.

Second, and this corrects the paragraph above rather than just extending
it: **both `"DIFFICULTY"` and `"AI DIFFICULTY"` are authored string-table
entries, not runtime-only text.** `psarc_grep`-ing `DATA04.PSARC`'s
`/data/plugins/languages/american/entries.xml` directly turns up
`<entry id="RB_DIF" string="DIFFICULTY">` and `<entry id="RB_AI_DIF"
string="AI DIFFICULTY">` as two distinct idstrings - so the earlier claim
that the bare word was "not authored as a string-table entry anywhere this
build has read" was wrong; only the `"(<rung>)"` suffix composition is
runtime text, not the words themselves. `DifficultyButton`'s own widget in
`CellMode_Definition.xml` still authors one unconditional literal
(`string="Change Difficulty"`, checked directly on `DATA06`'s copy, no
`<Entry>` redirect near it) - so the choice between `RB_DIF`/`RB_AI_DIF` is
made by the executable at draw time, keyed on something this pass did not
trace, not by an XML-authored per-mode string swap the way `hd_target_title`
already is for the `TARGET` header.

The same `entries.xml` also carries `UPDATE_ANNOUNCEMENT`'s own text, which
plausibly explains the split rather than just describing it: `"Update 1.20
introduces new Novice, Skilled and Elite difficulty options to the campaign
for all Time Trial, Speed Lap & Zone events... Rather than having difficulty
options for just Single Race and Tournament events in campaign, it is now
possible... to select Novice, Skilled or Elite difficulty options for all
events"` - i.e. `AI DIFFICULTY` (opponent skill, `MAN_1_CAM_4`: `"press the
Square button to adjust the difficulty level of your opponents"`) is the
original, `Single Race`/`Tournament`-only mechanic, and bare `DIFFICULTY`
is the later addition covering every other mode's own target-threshold
rung. `Eliminator`/`NitroBattle` reading bare `DIFFICULTY` fits this reading
(not itself named in that announcement string, but sharing the
target-threshold shape with `TimeTrial`/`SpeedLap`/`Zone`) rather than
contradicting it - not confirmed against a `TimeTrial`/`SpeedLap`/`Zone`
capture this pass, which would be the direct check.

**The `GOLD MEDALS` denominator is closed - `87` for `Wipeout HD`, `80` for
`Fury`, both exactly a campaign's own total cell count.** RPCS3 reads
`"0 / 87"` beside the `HD` flyer and `"0 / 80"` beside `Fury`'s
(`data/scratch/lane-hd-sel/rpcs3-campaign-selection/01-right-tap.png`/
`01-left-tap.png`) - two different numbers, one per campaign, which is what
an earlier pass's own note ("this build's own parse of `DATA00`'s eight Fury
grids totals 80 cells, a gap this pass does not explain") missed: it read
only the `HD`-side capture (`87`) and never the `Fury`-side one (`80`)
sitting beside it in the same directory, so a *correct* count on the *wrong*
side read as unexplained. Every campaign cell carries exactly one `<Gold>`
target, so a campaign's total possible gold medals is exactly its own cell
count - `grid.cells.len()` summed across the campaign's own eight grids
(`oag_game::main::campaign_stage::CampaignStage::campaign_gold_medal_totals`),
no invented denominator and no title-carried table. `selection::draw_list`
now draws `{earned} / {total}` instead of a bare numerator.

**Closing it surfaced a real, separate bug: `grid_04.xml`'s own `<Values>`
tag is missing a `>` on the disc, in all three of its copies, and this
build's own XML reader used to drop the whole grid because of it.** Before
this pass, `Wipeout HD`'s own total read `77`, not `87` - ten cells short,
exactly `grid4`'s own count. `grid_04.xml` authors
`<Values RequiredPoints="22" ... RotY="-0.5"</Values>` (no `>` before
`BillboardName`'s value ends), and `oag_tables::fexml::parse`'s tag scanner,
looking only for an unquoted `>`, ran straight past the missing one and
stopped at the real `</Values>`'s own - merging `Values`'s attributes,
`</Values`, and no `>` in between into one never-closed opening tag, so every
one of `grid4`'s own ten `<PI_Cell>` elements became a child of the dangling
`Values` node instead of `PI_Grid`. An earlier pass found this exact defect,
correctly, and **chose not to fix it** - reasoning that recovering the
intended cell boundaries without knowing whether the original tolerates the
same break would be guessing, not reading what is there. RPCS3's own `"0 / 87"`
answers that open question directly: the original's own parser does
tolerate it, since `87` is only reachable by counting all ten of `grid4`'s
cells. So `oag_tables::fexml::tag_end` now recovers the same way: an
unquoted `<` also ends a tag, one character short of it - see that
function's own doc for the mechanism, `crates/tables/src/fexml/tests.rs`'s
`an_unquoted_lt_ends_a_tag_one_character_short_of_it`/
`a_stray_double_lt_does_not_panic` for the synthetic pins, and
`crates/hd/tests/campaign_grids_ground_truth.rs`'s
`the_precedence_resolved_campaign_is_sixteen_grids_mixed_schema`/
`every_grid_file_on_every_archive_parses` for the real-disc ones (`grid4` =
10 cells, `DATA02`/`04`/`06` each total 87). A blast-radius check
(`rg --no-ignore -n '="[^"]*"</[A-Za-z]' data/scratch/drive-2026-09-25/hd-xml`)
found the identical shape recurring in `stats_definition.xml`'s and
`endrace_definition.xml`'s own `<Values ... RotY="-0.5"</Values>`
trophy/rank-model blocks (all three HD archives that carry either file) -
not a one-off typo, the same authoring tool's own repeated mistake - so the
fix is general rather than special-cased to `grid4`.

**Not fixed this pass, named rather than silently left stale:**
`oag_ui_screens::endrace::hd`'s own `text_draw` (a different function from
`oag_ui_screens::campaign::draw::text_draw`) has no `face_role` check at all, so
`EndRace Results`/`Rewards`/`Menu`'s own `ControlTextConfirmButton`
(`font="buttons"`) still draws nothing even though a `Buttons`-role atlas
now loads - see `docs/formats/hd-endrace-screens.md`'s own table and
`oag_ui_screens::endrace::hd::hd_results_draw_list`'s doc for the gap. Whether Omega
also declares a `Buttons` slot is unverified this pass -
`data/images/omega-ps4-eu.pkg` is a raw PS4 package, not directly openable
the way the decrypted PS3 ISO is (`no ISO 9660 primary volume descriptor
found`; the day-one patch's own `data09.psarc` would need extracting first),
and that was not attempted here.

Confidence 90 on the glyph verification (a disc-read atlas dump compared
directly against the codepoints this build's own footer resolves, plus the
RPCS3 shape match); confidence 90 on the `00-default.png` correction (every
clean frame in its own directory reads unambiguously as `Cell Selection`,
and both `Campaign Selection` archives' own `CellMode_Definition.xml` were
read directly to confirm `Grid Selection`/`Grid Selection Fury` author no
`DifficultyButton`); confidence 95 on the `GOLD MEDALS` denominator and the
`grid4` fix (both sides' own RPCS3 captures read directly, the raw disc
bytes read directly through the live archive reader rather than a cached
dump, and the corrected counts pinned by a ground-truth test against the
real disc).

## Wipeout HD/Fury: walked live, end to end, 2026-09-28

**The whole campaign path now runs in a live session on HD, not only as
`--menu-page` stills.** Xvfb `:93` (1280x720), `hdfury-ps3-eu-dec.iso`,
release build, `--autopilot --no-audio`, `WAYLAND_DISPLAY` unset, an
isolated `XDG_CONFIG_HOME` (`window_size = "1200x680"`, the HD render
profile cut to `render_scale = 50` with MSAA, motion blur and shadows off;
the adapter is llvmpipe, `renderer: vulkan: llvmpipe` in the log, and at
these settings the race ran at 60 ticks a second). Input was the
keyboard through `xdotool keydown`/`keyup` with a 120 ms hold after
`xdotool windowfocus --sync` - the earlier pass's delivery gap did not
reproduce. What a player sees, screen by screen (two or more frames each):

1. **`Main Menu`**: the `<HorizMenu>` strip, `RACE CAMPAIGN` highlighted.
   Enter opens the campaign.
2. **`Campaign Selection`**: `CAMPAIGN SELECT`, `CAMPAIGN MODES`, the two
   boxes `FURY CAMPAIGN` (`0 / 80`) and `HD CAMPAIGN` (`GOLD MEDALS`,
   `0 / 87`), the white selection frame on Fury first. `Right` moves the
   frame to HD, `Left` back.
3. **`Grid Selection`** (HD): `Event 01/08`, `POINTS ACHIEVED 00/06`,
   `TOTAL POINTS AVAILABLE 000/018`, the point-cloud flyer. On Fury:
   `00/07`, `000/021`, a different flyer. **Superseded 2026-09-30**: those
   are this build's own fractions, which RPCS3 does not show - it reads bare
   numbers (`0`, `18`), and the flyer is a flat card now drawn for the base
   campaign; see "The flyer behind `Grid Selection`".
4. **`Cell Selection`** (HD `grid0`): `Single Race`, `VINETA K`, Venom,
   weapons on, 3 laps, `0/3` points, best `NONE`, the three target medals,
   `10 MORE POINTS NEEDED TO UNLOCK:`, footer `AI DIFFICULTY (NOVICE)`. On
   Fury: Talon's Junction, `0/21`, `12 MORE POINTS`.
5. **Enter launches straight into the race - no `Team Selection`.** The boot
   logs `selection screens: track unread, ship unread` on HD, so
   `Session::open_ship_picker` returns `false` and `launch_campaign_cell`
   takes its no-picker fallback. The race flies `settings.race.team`
   (`Assegai` here), which the RACE page's TEAM row sets. **Closed
   2026-09-29**: the original does show one, and this build now does too -
   see "Wipeout HD/Fury: `Team Selection`, 2026-09-29" below.
6. **The race**: 8 craft on Vineta K, the HD HUD, 60 ticks a second. The
   autopilot finished 5th.
7. **`EndRace Results`**: `RESULTS`, `5TH PLACE`, the standings grid
   (`1.51.65` .. `1.54.95`, the player's row highlighted red, `-` for the
   two craft still racing), the `LOYALTY` header and its empty bracket,
   `CONFIRM`. It matches the existing `--menu-page endrace-results`
   capture, including the near-invisible white `POS`/`TIME` headers on the
   light panel. The grid's 8th row sits over the panel's dark footer bar.
8. **Enter goes to `EndRace Menu`** - `Rewards` is skipped, as HD's own
   `EndRaceMenuRedirect` authors (`docs/ui/endrace-screens.md`). `MENU`,
   `RACE AGAIN`, `RETURN TO GRID`, `VIEW RESULTS AGAIN`, `CONFIRM`. **The
   rows are white on the light, see-through panel over a bright race
   scene, and no row shows a visible cursor**, so a player cannot tell
   which one Enter will pick.
9. **Enter (the default row) is `RETURN TO GRID`**: back on `Cell Selection`
   for the same event, the backdrop still animating. `records.toml` gained
   the HD race record and `[[campaign]] cell = "grid0_3_1"
   last_difficulty = "easy"`. No `[[loyalty]]` row, which is correct: HD's
   loyalty law is not recovered and the Rewards screen that banks it is
   never entered. (Superseded 2026-10-02: the law is recovered and a race
   launched with a team banks on `Results` - `endrace-screens.md`'s HD section.)
10. **Back, Back** returns through `Grid Selection` to `Campaign
    Selection`, with the frame still on HD.
11. **Cursor persistence on Fury**: `Down` to the Eliminator cell (The
    Amphiseum, `TARGET 200 (NOVICE)`, footer `Difficulty (NOVICE)`), Back,
    Enter - the cursor came back on the Eliminator cell. That is our
    `CellCursors` (now the shared `CellCursor`, see "Where the cursor lives"), still **chosen, not measured** on HD.

Screenshots are under `data/scratch/drive-2026-09-28/clw/shots/h*.png`
(gitignored, game content). Left open by this walk: the `EndRace Menu`
cursor's visibility on HD, the 8th Results row overlapping the footer bar,
and whether HD's original has a ship screen on the campaign path.

## Wipeout HD/Fury: `Team Selection`, 2026-09-29

**The original shows a ship screen between `Cell Selection` and the race,
and this build now reads it off the disc and walks it.** Confidence **95**
that the screen is on the campaign path, from four independent lines:

1. **`TTY.log`'s own screen sequence.** `docs/reverse-engineering/rpcs3-debugger.md`'s
   cold-boot table (measured 2026-08-19, every step the default row) reads
   `Cell Selection` -> `Team Selection` -> `Launch Game`.
2. **Captures of the screen itself on that path.**
   `scripts/rpcs3-drive.py capture --nav-shots` photographs each screen
   `TTY.log` names; the Fury-campaign default walk left a
   `screen-Team-Selection.png` on two cold boots
   (`data/reference/hd-capture/talons-matched/`, see
   `docs/reverse-engineering/rpcs3-capture.md`) and on the fresh-profile
   boot (`data/scratch/hd-difficulty/rpcs3-fresh-default/`).
3. **The authored redirect.** Both copies of `CellMode_Definition.xml`
   (`DATA02`, `DATA06`) give `Cell Selection` a `Cell Mode Redirect Team`
   whose `Default` is `goto="Team Selection"`; the `Launch Game` redirect
   beside it is disabled (spelled `<aRedirect>`) under the comment "removed
   this since we're not having forced or suggested ship anymore, so always go
   to the team selection screen after grid select".
4. **Which file is live.** `DATA00`'s `skin.xml` - the served front-end
   root (`docs/formats/hd-frontend.md`) - includes
   `SrcRel="Team_Selection_Definition.xml"`, which only `DATA06` carries,
   and does not include the older `Selection_Definition.xml` that
   `DATA02`/`DATA03`/`DATA05` carry (whose own `Team Selection` is therefore
   dead data on this pressing).

### What the file authors

`Team_Selection_Definition.xml` nests three single-screen children
(`Team Selection`, `Team Selection Player 1`/`Player 2`) inside `Team
Selection Top Level`, which holds every widget. The single-player child holds
only the `RC_SHIPSEL` title ("SHIP SELECT") and two redirects: `TeamRedirect`
(`Main Menu->Mode == FE_ONLINE` goes to `GameLobby`, else `Team Launch
Transition`, which goes to `Launch Game` after 0.7 s) and `TeamRedirectBack`
with no `goto`. The parent's widgets, with their `Item` offsets folded:

| Widget | Where (1920x1080) | Drawn here |
| --- | --- | --- |
| `MiniText` `RC_CHOOSE_TEAM` / `RC_NAV_TEAM` / `RC_SHIP_MODEL` / `OPT_STATS` | (160,140) / (160,336) / (705,140) / (705,833) | yes, bullet and text |
| `Bracket` x3 (team, hex grid, ship model with `middle`) | (160,170) 506x156 / (160,366) 506x600 / (705,170) 1052x650 | no - rects only, for layout and pointer |
| `LogoOutline` images and fills | around (160,170) | yes |
| `Logo` (no `src`) | (160,185) 512x128 | yes, the team's own `Data\Ships\<team>\FE\Logo.gtf` |
| `HexSelection` 5 cols x 7 rows | offset (272,395) | yes, see "The `NAVIGATE TEAM` honeycomb" |
| `Model name="ShipModel"` | `OriginX=1220 OriginY=412 z=-24 RotX=0.4 RotY=-0.5` | no (read, not drawn) |
| `Block` `Slide_0..3` (`RC_SPEED`/`RC_THRUST`/`RC_HANDLING`/`RC_SHIELD`) with `Nobble_N`, `Label_N` | from (705,858) | yes, the stat bars |
| `Block` `Slide_4` `ER_LOY` with nobbles | (1250,770) | box and label only |
| `Padlock`, `Unlockcondition`, `LiveryString`, `netLobby*` | - | `LiveryString` only, see below |

`RC_NAV_TEAM` ("NAVIGATE TEAM") is in `DATA06`'s English `entries.xml` and
in none of `DATA02`..`DATA05`'s, so the reader overlays the file's own ids
from `DATA06`'s copy where the served table lacks them
(`oag_game::campaign::hd_data06_strings`), the same shape `Campaign
Selection`'s overlay already takes.

### The ratings are per model, confidence 90

HD authors `<FE speed thrust handling shield>` on each `PI_TeamModel`
(`DATA00`'s `Data\Plugins\Frontend\Definition.xml`), in half points, not
on the team the way Pulse does. The screen prints each as tenths, three
digits: the settled `racebox` frame reads Feisar `070`/`080`/`100`/`080`,
which is its `normal` model's `7/8/10/8`, and the fresh-profile campaign
frame reads `080`/`085`/`080` for speed, thrust and shield, which is its
`concept1`'s `8/8.5/8` and not `normal`'s. So the ratings follow the
selected model, and the fresh-profile campaign default is `concept1` -
consistent with `rpcs3-capture.md`'s own finding that a Fury-campaign race
flies the `concept1` livery. Two frames, one team: 90, not higher. **What
this does not show** is that the original always races what its screen
selects: `rpcs3-capture.md` records a `racebox` walk whose screen showed the
classic hull while the race flew `concept1`. The fresh-profile default is
settled by "Walked on RPCS3 with the input pressed, 2026-09-29" below.

The seven `HexSelection` rows match `PI_TeamModel` file order (`chrome_c1`,
`nitro`, `concept1`, `normal`, `SKIN1`, `SKIN2`, `chrome`): on the
fresh-profile frame the only two rows with a ship icon rather than a padlock
are the third and fourth, the two models with no `<Unlock>`. Confidence
**70** (one frame, and the icon art is the widget's own). This build keeps
its own three-variant axis (`""`, `_c1`, `_n1`, `oag_hd::race::TEAM_VARIANTS`)
rather than widening to seven; each variant's stats come off the model racing
out of that directory, preferring one with no `<Unlock>` - `Feisar_c1` is
both `chrome_c1` and `concept1` - which is **chosen, not measured**
(`oag_raceplay::catalogue::Team::model_for_directory`).

**The stat bar, measured off the settled frame**: each block is split where
its nobble sits (`0.69`, `0.79`, `1.0` of the block for `070`, `080`, `100`);
left of it the inside is `102/255`, right of it `65/255` over the black page.
Those are `HD_Grey`'s `150` and the block's own `AlwaysSolidColor`'s `100`
each through `Block_Render`'s translucent two-pass fill (`x 0.676`, see
`crate::menu::block`). How the widget composes the two is unread; this build
fills the two halves side by side and draws the border once over both
(`oag_ui::menu::block::draw_split_fill`, chosen).

### Walked on RPCS3 with the input pressed, 2026-09-29

`scripts/rpcs3-drive.py`'s session with a scratch walk that taps `right`,
`down`, `up` and `left` and photographs after each press (isolated
`XDG_CONFIG_HOME`, `Xvfb :94`, muted; shots and the script in
`data/scratch/hd-frontend-polish/`). Two boots on a **fresh profile**
(`savedata` empty, `EpilepsyWarning` and `FirstPlay` answered), one on the
old save of 2026-09-21.

- **Axes, confidence 90**: `Right`/`Left` step the team (Feisar -> Qirex ->
  Piranha), `Up`/`Down` step the model row of the `NAVIGATE TEAM` hex
  column (the highlight moves down one hex; the stat digits change in
  step). Racebox and the Fury campaign alike. The former "chosen, not
  measured" reading was right.
- **The model row survives a team step, confidence 90**: on the fresh
  campaign boot `Right` from Feisar's `concept1` landed on Qirex's
  `concept1` (`085`/`080`, the red overhang bars), and `Down` from there on
  Qirex's `normal` (`080`/`070`, `8/7/8/9`), both matching `DATA00`'s
  `PI_TeamModel` `<FE>` rows. This build restarted the livery on every team
  step; it now keeps it (`Picker::land_on`).
- **A fresh profile opens on `concept1`, on both routes, confidence 90**:
  the third `HexSelection` row highlighted and Feisar reading `080`/`085`/
  `*`/`080` on a fresh boot of the Fury campaign and again on a fresh boot of
  Racebox (three fresh-profile frames with the earlier `rpcs3-fresh-default`
  one). The disc authors no default: `Team_Selection_Definition.xml` and
  `CellMode_Definition.xml` (`DATA06`) carry no model, livery or default
  attribute, the `Cell Mode Redirect Game` that once forced a ship is the
  disabled `<aRedirect>`, and the first unlocked `PI_TeamModel` in file order
  is `concept1` (`chrome_c1` is `<Unlock locked="true">`, `nitro` needs
  `grid8`), which fits but is not a proven rule.
- **A profile that already holds a save opened elsewhere**: the same disc,
  the old 2026-09-21 save, campaign and Racebox both opened on `normal`
  (`070`/`080`) - and `rpcs3-capture.md`'s 2026-09-13 note of a "fresh"
  Racebox opening on the classic hull was very likely that. So the original
  remembers something per profile; **what, where and when it is written is
  unread** (a plain `Back` and re-entry within one boot reopened on
  `concept1` again, which says nothing since the first press had landed on a
  locked row).
- **This build**: `oag_game::settings::Race::opening_variant` opens Ship
  Select on `_c1` (`Fury Concept`) on HD while `race.variant` is empty and
  `race.variant_chosen` is false; the flag is set by the first livery
  change or any Confirm and persists in the config, so a deliberate classic
  hull sticks. "Never picked" as the definition of a fresh profile is
  **chosen, not measured**. Not done: the original's fresh profile also
  opens on **Feisar**, the first team; this build opens on
  `settings.race.team` (`assegai` by default).

### Chosen, not measured

- **The livery order**: this build cycles three variants (`HD`, `Fury
  Concept`, `Fury Nitro`), not the original's seven rows, and wraps; see
  the RPCS3 section below for what was measured about the axes.
- **`LiveryString`** shows the selected variant's label (`HD`, `Fury
  Concept`, `Fury Nitro`) when the team offers more than one. The widget is
  authored empty and filled by code this build has not read; it stands in
  for the hex row the original highlights, which is not drawn.
- **Back** returns to `Cell Selection` on the same cell and difficulty rung
  (`Session::reopen_cell_selection`) - `TeamRedirectBack` authors no
  `goto`, which reads as "back to the screen it came from". Pulse's own
  ship-screen Back still reopens `Grid Selection`, unchanged.
- **Pointer**: the `CHOOSE TEAM` frame's left/right halves step the team,
  the `NAVIGATE TEAM` frame's top/bottom halves the livery, the `SHIP MODEL`
  frame confirms, the secondary button backs out
  (`oag_ui_screens::picker::pointer`'s `hd_targets`).
- **The RACE page's START** opens `Track Creation` first and this screen
  after it (not in Zone), see "Wipeout HD/Fury: `Track Creation`, 2026-09-29"
  below.
- **The heading scale** (`0.45` of the menu face, a 14-unit bullet, text 20
  units right of it) is read off the frame by eye.

### Not drawn, and why

- **The 3-D ship.** The `ShipModel` pose is read. HD's per-team `screen.xml`
  names `<team>\ship_FE.vex`, which no HD archive carries; the race hull
  (`ship.vex` + `ship.rcsmodel`) is not reachable from the preview path, so
  the `SHIP MODEL` frame is empty. The biggest thing still missing.
- **The hex grid** (`HexSelection`): drawn since 2026-10-08, see below.
- **The bracket corner marks, the padlock, the unlock condition text and
  the loyalty value** - no unlock or loyalty state is kept.

### The `NAVIGATE TEAM` honeycomb, 2026-10-08 (hd-ship-select)

`HexSelection` authors only a shape and colours (`columns="5" rows="7"`,
`HexCol 0x64808080`, `HexColFade 0x32808080`, `SelectedColumnCol 0x64ff0000`,
`SelectedLockCol 0xff808080`, `SelectedLockColFade 0xff242424`); the art is
the front end's own. Drawn by `oag_ui_screens::picker::hd::hex`:

| Piece | Source | Confidence |
| --- | --- | --- |
| cell | `Data\FE\Images\Hexagon_HD.gtf` (72x62 art in 128x64) at 1.22 | 70 |
| cursor ring | `Hexagon_HD_OUTLINE.gtf`, tint read off the frame (`185/35/55`); the original's ring is about twice as thick: **chosen, not measured** | 40 |
| padlock | `Padlock.gtf` (the image this screen authors at 512x512), visible art 91x129 texels drawn at 0.44; colour `SelectedLockCol` on the selected column, faded toward `SelectedLockColFade` by 0.2 and 0.65 one and two columns out (frame reads 128, 110, 68) | 70 |
| model icon | `<modellocation>\FE\thumb<LiveryNumber>.gtf` - `Feisar_c1\FE\thumb4.gtf` for `concept1`, `Feisar\FE\thumb0.gtf` for `normal`; census: every model of every team carries one | 85 (the frame's two icons match thumb4 and thumb0) |
| selected column | `SelectedColumnCol` drawn twice on the bare cell (the two-pass fill `Block_Render` uses): `0x64` twice over black is 160, the frame reads (151, 6, 20) | 70 |
| neighbour columns | the same cells in `HexCol`, open rows ghosting their thumbnail in black at 0.35: **chosen, not measured** (the frame shows faint silhouettes) | - |

Geometry, one settled RPCS3 frame (Feisar selected, fresh profile of the
maintainer's save): first column centre = the authored origin `x=272`, column
pitch 72.2, row pitch 82.8, odd columns half a row lower, hexagon 88x76. **70**:
one frame, in authored units through the page scale of "The layout scale"
below, which rests on one clean anchor (the stat blocks' 545-unit spacing).

**The columns are teams, the rows models**: `Right` from `concept1` lands on
the next team's `concept1` (2026-09-29 walk), so the centre column is the
selected team and its neighbours the teams either side (wrapping: chosen).
The cursor ring sits on the row of the chosen livery. Our picker offers three
liveries (`""`, `_c1`, `_n1`), each on the row its model occupies
(`Team::hex_cells`, `model_for_directory`).

**The lock rule is inherited, labelled so**: a row is open when it is a model
this build lets a player race, padlocked otherwise - Pulse's law, unmeasured
on HD. HD keeps no unlock state (`gates_variants` is false), so the original's
fresh-profile frame opens exactly `concept1` and `normal` while this build
also opens `nitro` (its `_n1` variant races). A visible difference, kept so
the cursor never rests on a locked cell.

**The classic hulls' thumbnails are `DATA06`'s**: `DATA02` also ships
`thumb0..3`, a 256x64 top-down plan, and this build's mount order reaches it
first. The frame shows `DATA06`'s 126x64 three-quarter view, so the sheet
reads the **last** archive's copy of an `FE\thumb` name
(`hex::is_thumb`, `read_front_end_first`). Which rule the original uses is
unread; the later archive winning is the reading both frames agree with.

Pointer: each open hexagon is a target (`hex_contains`, so a click near an
edge picks the right cell); a click selects that team and livery and confirms
when it is already the chosen cell; a padlocked cell is nothing. This
replaces the old top/bottom half of the `NAVIGATE TEAM` bracket. Pad:
unchanged (left/right team, up/down livery). **Chosen**: the pad visits only
the offered rows; whether the original's cursor can rest on a locked row is
unmeasured. Tests: `picker::hd::tests::*` (the miniature) and
`hd_reads_its_own_team_selection` (the disc: the five colours, the origin,
Feisar's open rows and thumbnails, every model's thumbnail on the sheet).

### The layout scale: the original is not drawn 1:1, 2026-10-08

Open question 12 of `hd-frontend.md` ("is the 1920x1080 space presented 1:1")
has an answer on this frame: **no**. One anchor that does not depend on this
build's drawing gives a uniform scale of **0.904** (picture units per
authored unit; confidence 60, one frame): the stat blocks' authored spacing
of 545 units reads 513 px, and their 60-unit row pitch reads 57 px, within a
pixel. The title rule spans the same `306..1811` px on all five pages walked,
so the transform is page-wide; the offset comes from the hex origin and the
block's left edge, which disagree by about 4 units. The offset is about `(+150, -8)` units. Our front end draws 1:1, so
every HD page is laid out about 1.1 times larger than the original's frame,
which is what the Ship Select sheet shows. **Not applied, and why:** HD's
Game Options carry `Screen Size` / `Safe Area Setting`, and the RPCS3 profile
the frame came from is the maintainer's save, so the scale may be that
setting rather than the default. A fresh-profile boot with the setting at its
default is the measurement that settles it; until then nothing here is
inset. Not the offset's origin either: a pure safe-area scale about the
centre would not move the centre, and this one does.

### Walked live, 2026-09-29

Xvfb `:94`, debug build, `--autopilot --no-audio`, isolated
`XDG_CONFIG_HOME`, keyboard through `xdotool`: `Main Menu` -> `Campaign
Selection` -> `Grid Selection` -> `Cell Selection` -> Enter -> `SHIP SELECT`
(Feisar logo, `070 080 100 080`, footer `CONFIRM`/`BACK`). `Right` -> Qirex;
`Down` -> `Fury Concept` with Qirex `concept1`'s `085 080 085 090`. `Escape`
-> `Cell Selection`, same cell. `C` to `SKILLED`, Enter, `Escape` -> still
`TARGET (SKILLED)`. Mouse: a click on the team frame's right half -> the next
team, on the hex frame's top half -> the previous livery, on the ship frame
-> the race loads. Enter on `Fury Concept` flies `Data\Ships\Feisar_c1`.
`RACEBOX` -> START -> `SHIP SELECT` -> `Escape` -> the RACE page. Shots in
`data/scratch/hd-team-select/live/`.

That walk found a real bug, fixed in the same change: the ship picker chose
its livery axis by whether the row's first id was empty, which HD's variant
table's classic-hull suffix also is, so `_c1`/`_n1` were written to
`race.skin` and the race flew the classic hull. It now keys on whether the
team declares skins.

## Wipeout HD/Fury: `Track Creation`, 2026-09-29

**The original shows a circuit screen on the RACEBOX path, and never on the
campaign path; this build reads it off the disc and walks it there.**

### Where it is shown, confidence 90

1. **Not on the campaign path.** `Cell Selection` redirects straight to
   `Team Selection`, and `TTY.log`'s cold-boot table
   (`docs/reverse-engineering/rpcs3-debugger.md`) reads `Cell Selection` ->
   `Team Selection` -> `Launch Game` with no circuit screen between. Confidence
   95, the same evidence the "Team Selection" section above rests on.
2. **On the RACEBOX path.** `racebox_definition.xml`'s `Single Player` screen
   redirects everything but `Tournament` to `goto="Track Creation"` (all five
   archives carrying the file agree; `hd-frontend.md`, confidence 94), and
   `scripts/rpcs3-drive.py browse --nav "Main Menu=right" --nav "Track
   Creation=right"` photographed the screen 27 times
   (`data/reference/hd-capture/track-carousel/`, `racebox/screen-Track-Creation.png`).
3. **Its own redirects.** `TrackRedirect`: `Main Menu->Mode == FE_ONLINE` goes to
   `CreateMPGame`; `Single Player->Mode == ai race` or `Ghost Viewer` goes to
   `Launch Game` (neither mode is enabled in `Single Player`'s mode list); every
   other case goes to `Team Selection`. `TrackRedirectBack` authors
   `forward="none"` and no `goto`: Back returns to the RACE page.
4. **Which file is live, and a correction.** `DATA00`'s served `skin.xml`
   includes `SrcRel="Track_Selection_Definition.xml"`, which only `DATA06`
   carries. **`racebox_definition.xml` does not author the screen** - it only
   names `Track Creation` as a `goto` target. The Ghidra page
   `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md` says the
   screen is in that file; it is not, and none of the five copies contains a
   `Track Creation` screen.

`Tournament C`, in the same file, is a separate screen (`TrackNumber0..11`,
`SelectedTrack0..11`, add/remove/randomise buttons) and is **not read**.

### `right` wraps at twelve, confidence 80; the rows are directions, confidence 90

**Measured**: the 27 frames of the `right`-only walk show Vineta K at press 0,
12 and 24, and every frame from 13 to 24 repeats the frame twelve earlier - the
highlight on the same top hex of the `TrackHexSelection` grid
(`columns="9" rows="2"`) and no `ReverseIcon`/`REVERSE` text in the circuit model
frame, where this build draws it for a reversed circuit. Names alone would not
tell a 24-entry list from a 12-wide wrap (the original never spells a direction
in a name), so the score rests on the cursor and the missing glyph, not on the
repeat.

**Measured 2026-09-29, confidence 90**: the grid's two rows are the two
directions. An RPCS3 walk pressing `right`, `down`, `right`, `down`, `up`,
`left` on `Track Creation` (`data/scratch/hd-frontend-polish/rpcs3-racebox-oldsave/`):
`Right` Anulpha Pass -> Moa Therma; `Down` on Anulpha Pass drew `REVERSE`
and the circular arrow icon with the lower hex highlighted, `Right` from
there stepped Moa Therma reversed (the same order), a second `Down` stayed
put, `Up` went back to forward with the icon gone, `Left` stepped back to
Anulpha Pass. The `CIRCUIT DIRECTION` hexes and the direction icon do the
work; the name never gains `REVERSE`. The earlier "24 entries, forward
then reverse" reading (`rpcs3-capture.md`) is therefore disproved, not just
no longer the one this build follows. The forward order is measured
(Vineta K, Anulpha Pass, Moa Therma, Chenghou Project, Metropia, Sebenco Climb,
Ubermall, Sol 2, Talon's Junction, The Amphiseum, Modesto Heights, Tech De Ra);
`Zone`'s four environments are not among them.

### What the file authors, and what is drawn

| Widget | Where (1920x1080) | Drawn here |
| --- | --- | --- |
| Title `RB_TRACK_SEL` | the skin's title position | yes |
| `MiniText` x3 (`CHOOSE CIRCUIT`, `CIRCUIT MODEL`, `CIRCUIT DIRECTION`) and `RACE INFORMATION` | (160,140) / (857,140) / (160,422) / (160,690) | yes |
| `EmblemOutline` images and fills | around (160,170) | yes |
| `Emblem` (no `src`) | (270,178) 192x192 | yes: `<environment>\FE\TrackSelectEmblem_Fury.gtf`, one per circuit on `DATA00` |
| `TrackName` (`idstring` a placeholder) | (500,370) centred | yes, the circuit's name |
| `Info1`/`Info2` `CIRCUIT LENGTH`/`RACE DISTANCE` | (880,680)/(880,705) | yes: `4.4KM` / `13.2KM` on Vineta K, length x the race's lap count |
| RECORDS heading, header row, 3x4 cells | from (160,744) | yes: `PERSONAL`/`FRIENDS`/`GLOBAL` and `---` |
| `ReverseIcon1..3` + `FE_REVERSE` text | (860,175) | yes, on a reverse circuit only (**chosen**: no frame has the cursor on the second row) |
| `Infinity` | (1080,705) | yes, where the race has no lap count |
| `Bracket` x3 | (160,170) 680x242 / (160,452) 680x230 / (856,170) 900x565 | rects only, for layout and pointer |
| `FlyByMovie` (`preview.bik`, Bink) | (660,360) 260x170 | no |
| `Model name="TrackModel"` | `OriginX=1308 OriginY=440 z=-180` | no (read, not drawn) |
| `TrackHexSelection` | (210,500), 9x2 | no |
| `Squares` page dots, `Padlock`, `Unlockcondition`, `furyship1..3` | - | no |

The numbers above use the circuit's own spline for the length
(`oag_raceplay::circuit_length`, measured on a worker); the lap count is the
race's own setting (`oag_raceplay::catalogue::race_laps`), so the frame's `13.2KM`
is three laps and the number here follows the speed class. The PERSONAL, FRIENDS
and GLOBAL ids (`FE_PERSONAL`/`FE_FRIENDS`/`FE_GLOBAL`) are the string table's
own entries with that text, looked up through `DATA06`'s `entries.xml` where the
served table lacks them; the widgets author `string="16"`, a width, and the
original fills them from code. That id choice is **chosen, not measured**.

### Chosen, not measured

- **Axes**: measured, see "`right` wraps at twelve" above -
  left/right steps along a row, up/down switches direction.
  `oag_ui_screens::picker::Picker::with_rows`. **The order of the reverse row** is
  the forward order, also measured (one walk).
- **A reverse circuit is labelled with its forward twin's name**: the frame
  never spells a direction in the name.
- **Pointer**: the `CHOOSE CIRCUIT` frame's left/right halves step the circuit,
  the hex frame's top/bottom halves switch the direction row, the circuit model
  frame confirms, the secondary button backs out
  (`oag_ui_screens::picker::hd::track::targets`).
- **Confirm** opens `Team Selection` (the redirect's default), except in Zone,
  which forces its own hull as on Pulse. **Back** reopens the RACE page; Back from
  `Team Selection` returns here on the same circuit and direction.
- **The three RACE page rows** (`TEAM`, `VARIANT`, `TRACK`) are dropped: both
  screens exist now, and HD's `Single Player` page carries none of them.

### Not drawn, and why

- **The circuit wireframe** (`TrackModel`): HD ships no `FE\forward.vex`; the
  folder holds only `fe_grad.gtf`, the emblem and `preview.bik`. The pose is read.
- **The fly-by** (`FlyByMovie`): `preview.bik` per environment is Bink; the
  screen has no video widget yet.
- **The hex grid** (`TrackHexSelection`): its art and cell pitch are the widget
  class's own and unread; the frames show per-circuit emblems in dim hexes, the
  selected one red with a white outline.
- **The page squares** (`Squares`, top right of the circuit model frame): empty
  container filled by code.
- **The RECORDS cells**: this build keeps no per-circuit record in the shape
  (name, team, time) the table wants, so every cell reads `---`.

### Walked live, 2026-09-29

Xvfb `:94`, debug build, `--autopilot --no-audio`, isolated `XDG_CONFIG_HOME`,
`xdotool`: `Main Menu` -> `RACEBOX` -> START -> `TRACK SELECT` on Vineta K;
`Right` -> Anulpha Pass (`5.1KM`); `Down` -> Vineta K reversed with the REVERSE
glyph; `Right` twice -> Moa Therma reversed; Enter -> `SHIP SELECT`; Escape ->
`TRACK SELECT` still on Moa Therma reversed; Enter, Enter -> the race loads
`Data\Environments\03_Track\track_reversed.vex`. Mouse: a click on the
frame's right half stepped, the left half stepped back, a right click
returned to the RACE page. Shots in `data/scratch/hd-track-select/live/`.

## Wipeout HD/Fury and Omega: a hover lands on the hexagon drawn, 2026-09-30

`omega-frontend-fixes`. A maintainer playing Omega could not start a race from a
hexagon. Walked from the language picker under Xvfb: the keyboard path was fine
(`Return`, `Return` launches `grid0_3_1`; `Down`, `Return` launches `grid0_3_2`), and
the **mouse** failed. The hex sprites (`Hexagon_HD.mip`, `_OUTLINE`, `_THICK_OUT`) are
128x64 power-of-two textures whose opaque pixels are a 72x62 hexagon in the top-left
corner (alpha box x 1..72, y 1..62 on Omega's; `Hexlock_HD.mip`'s glyph is x 20..54).
[`oag_ui_screens::campaign::pointer`] tested the placed texture rectangle as if it were the
hexagon, so each hit region was a hexagon 128 wide and 111 tall centred 28 units right of
the art. Neighbours overlapped and the first target in cell order won: hovering
`grid0_3_2` (Speed Lap, unlocked) selected `grid0_2_2` (Time Trial, `Locked` unset), and a
confirm on a padlocked cell does nothing (the measured Pulse behaviour, kept).

The target is now the box of the sprite's opaque pixels (`Sheet::opaque_extent`: alpha at
half strength or more, **chosen, not measured**), built by `oag_game::campaign::hit`, which
the live session and the test both call.
`crates/game/tests/campaign_pointer_ground_truth.rs` probes the centre and six points at
0.7 of the circumradius of every occupied hex of every grid on Omega and on HD/Fury and
requires each to land on its own cell; it fails on the old targets on both. The same probes
run on `pulse-psp-eu` (480x272 grid) and pass with the old targets and the new ones, so Pulse
was not broken and is not changed in practice.

Two smaller mouse dead ends on the same screens, fixed together:

- **Omega's `Grid Selection` has no flyer card**, so the click target that used to fall
  back to `Flyer Pad Lock`'s authored rect (`x=934 y=304 512x512`, the padlock that sits on
  a card) was an invisible square. With no card drawn a click anywhere in the page's content
  band (`hd::CONFIRM_BAND`, between the header and footer rules) confirms the one tier on
  screen; the arrows still win where they overlap. Chosen, not measured.
- **A cell the engine cannot race says so.** `NitroBattle` and `Detonator` (25 of Omega's
  167 cells) refused the launch with only a log line. `Cell Selection` now shows
  `OAG_CAMPAIGN_MODE_UNSUPPORTED` (or `OAG_CAMPAIGN_TRACK_UNAVAILABLE` for a circuit the
  disc lacks) a row above the footer, in the legend's own word face
  (`NavigationLegend::notice`), until the next input. The wording and the row are ours. A
  locked cell still shows nothing beyond its padlock.

## Wipeout HD PSN: the campaign with no chooser, 2026-10-07

Wipeout HD's PSN download (`NPEA00057` v3.00, no Fury) carries no `DATA06`, so it
has no `Campaign Selection` and no `Grid Selection Fury`: RACE CAMPAIGN opens
`Grid Selection` over its eight grids directly. Measured on RPCS3 (main menu,
`EVENT 01/08`, `CHOOSE RACE`), and the same screen, grids and cell layout as the
disc's HD branch: the package's `data02` screen file and grids are the disc's
`DATA02` ones byte for byte, and its `data04` grids are `DATA06`'s HD grids bar the
`Campaign="HD"` attribute. The wiring is `oag_title::Campaign::screen_archive`
(`Some(DATA06)` on the disc, `None` on PSN) and `load_hd` skipping the selection
pair when it is absent; no new reader. Evidence, frames and the art differences
from RPCS3's `Cell Selection` are in `docs/formats/hd-psn.md`, "The campaign".
Lineage check (2048 / Omega): **checked, differs** - Omega's front end is HD's
`PI001` plugin carried forward and reads its screens through its
own `load_omega` branch off `data09`, which `screen_archive` does not touch; 2048 has no `CellMode_Definition.xml`.

## Wipeout HD/Fury: what still differs from the original, ranked - measured 2026-10-08

Matched pairs of every page a player passes on the way to a race: the
original on RPCS3 (`BCES-00664`, 2000x1200 virtual display, frame cropped
`2000x1125+0+37`, a 24-frame burst across each transition and settled frames
after) against this build, walked live under software Vulkan (Xvfb,
`--no-audio`, a fresh profile; about 5 fps, so bursts of ours are slow motion
and **no geometry is read off the live crops**) and as `--menu-page` stills
at 1280x720. Sheets (original above, ours below) are under
`data/scratch/hd-fe-look/sheets/page-*.png`, raw frames under `ref/` and
`ours-live/` there. Sightings: Main Menu, Ship Select and Racebox setup on two
boots (this lane and `hd-capture/*`), Campaign Selection and Grid on this
boot plus the `hd-flyer` lane's; Cell Selection once. **Confounds, not
gaps:** the RPCS3 profile carries the maintainer's save (default team
Feisar against our Assegai, first circuit Vineta K against our Anulpha Pass,
unlock state), and the walk reached no Tournament cell (not reached; the grids
hold 24 `Tournament` cells in the archive read, none walked).

1. **Ship Select has no craft, no team hex column.** The original draws the
   team's hull static in the `SHIP MODEL` frame (no turntable: five seconds
   apart the pose is identical, so Pulse's 12 s `orbit_for` does not apply),
   a column of seven team hexes with padlocks and hull icons, and corner
   brackets. Ours draws the empty frame and the livery name as text.
   Sheets: `page-ship-select.png`, `page-ship-select-still.png`. The pose is
   authored (`ShipModel`, `OriginX=1220 OriginY=412 z=-24 RotX=0.4 RotY=-0.5`).
   Omega: checked, applies (draws an empty frame too), not wired.
   **Fixed 2026-10-08 (hd-fe-look): the craft draws** - the team's race hull
   (`Data\Ships\<team>\ship.vex` + `.rcsmodel`, livery as the race paints it)
   at a fixed pose, `FrontEnd::ship_preview_hull` Title data (`Some("ship.vex")`
   on HD only). Two things found: HD's hull has one stray vertex hundreds of
   units out (bound 257 against a hull some 13 long), so the viewer's
   bounding-sphere framing drew a speck - `preview::frame_hull` frames from the
   1st-99th percentile box instead; and the pose (`hull_orbit`: yaw 5.9, pitch
   0.5, zoom 0.6) is **chosen, not measured**, fitted by eye to the original's
   Feisar frame (different team, so shape differs). The authored `ShipModel`
   `RotX`/`RotY` are not composed yet. Sheet: `page-ship-select-after.png`.
   Still missing on this screen: the brackets, the loyalty value, and the
   original's default team (save-state confound).
   **Fixed 2026-10-08 (hd-ship-select): the team hex column draws** - see
   "The `NAVIGATE TEAM` honeycomb" below. Omega: checked, applies (same
   `Team_Selection_Definition.xml` in `data09.psarc`, which `crates/omega`
   skips), not wired; its thumbnails and `Padlock.gtf` were not censused.
2. **Cell Selection lacks the card.** The original lays the grid's red flyer
   (the event logo "blitzed", the striped rail, a black hex field of about 30
   cells with the event's hexes lit), an event-type icon beside each of
   Event Type / Track / Speed Class / Weapons, the next grid's logo under
   "12 MORE POINTS NEEDED TO UNLOCK", and bold upper-case values
   (`SINGLE RACE`, `VENOM`). Ours draws six hexes, no card, no icons, values
   in mixed case. Needs the flyer's face-on pose (the open "swing" in the
   `hd-flyer` thread). Cell-to-cell d-pad moves land on the same cells on both
   sides (`sheets/cell-nav.png`). Omega: same screen, same gap.
   **Fixed 2026-10-08 (hd-cell-select): the card, the hex field, the four icons,
   the brackets, the next-grid logo and the capitalised values draw** - see
   "`Cell Selection`'s card, field, icons and brackets" below.
3. **Frame chrome on the campaign pages.** The original carries the
   `SCREEN TITLE` caption, a bold title with the arrow, a rule above the page
   and a `NAVIGATION` line (d-pad glyphs) in the footer on every page. Ours
   has no top rule on Campaign Select and Grid, no caption, no NAVIGATION
   entry, and a title at another size and offset. Ours shows keyboard glyphs
   (auto style on a keyboard-driven run, not a gap).
   **Fixed 2026-10-08 (hd-cell-select)**: the rules and the title's bullet were
   being drawn and then lost (a shared quad buffer, below), the title is now the
   original's 150 grey, the `SCREEN TITLE` caption and the `NAVIGATION` entry
   draw. Not fixed: the title's size and offset against the 0.904 layout scale.
4. **No page transitions.** On entering Campaign Select the original scrambles
   the title, slides the two flyers in and flies the particle scene through
   (`trans-campsel-original.png`); ours cuts. `--menu-anim-phase 0.3` draws a
   byte-identical image to the settled one (compared, 0 differing pixels), so
   no tween exists on these pages.
5. **Main Menu is our own tree.** Original: `MAIN MENU` with CAMPAIGN,
   RACEBOX, ONLINE, OPTIONS, RECORDS tabs plus a `GAME GUIDE INFO` feed;
   ours: `OPENANTIGRAV` with RACE CAMPAIGN, RACEBOX, REMIX, RECORDS, OPTIONS,
   QUIT (`page-main-menu-still.png`). Chosen by the project's menu tree, not
   a bug in HD's wiring.
6. **Racebox setup page.** Original: `RACEBOX`, rows RACE TYPE / SPEED CLASS /
   WEAPONS / AI DIFFICULTY / NUMBER OF PLAYERS / SPLIT SCREEN / TARGET / ZONE
   TARGET and a menu-icon panel; ours: `RACE` with MODE / SPEED CLASS / AI
   DIFFICULTY / START / BACK (`page-racebox-setup.png`).
7. **Track Select.** Layout and records table match; missing: the hex grid
   of circuit emblems, the shaded grey circuit model (it is a shaded model,
   not a wireframe - the status cell's word is wrong), and the moving fly-by
   picture (it moves: 17 % of the window's pixels differ across 1.5 s). Ours
   also draws the hex window's stills as nothing (`no slideshow chain`).
8. **Loading, EndRace.** Not re-paired: ours on software Vulkan sat on Ship
   Select for 32 s without reaching a load. See the `hd-loading-screen` and
   `lane-hd-endrace` captures.

Not a gap (checked): what looked like an `ELIMINATOR` against `NitroBattle`
mismatch on The Amphiseum was two different cells (`grid8_3_2` Elimination,
`grid8_2_1` NitroBattle) reached by dropped presses, not a label bug.

### `Cell Selection`'s card, field, icons and brackets (2026-10-08, hd-cell-select)

Measured against `data/scratch/hd-fe-look/ref/campaign/088-cell-settled1.png`
(RPCS3, the first cell of `09_blitzed`, a fresh zero-medal profile) and drawn
with `--menu-page cell-select` on the same grid; sheets under
`data/scratch/hd-cell-select/` (`before.png`, `a5.png`, `b3.png`).

**The red card is the grid's flyer seen from behind.** `Cell Selection` authors
no `<Flyer>` of its own (the top-level `FlyerModel` is shared), and
`CampaignStage::flyer_shows` returned nothing for the screen - that is why no
card drew. The picture is `Data\FE\Flyers\<grid>\flyer_Back.vex` (`bgplane`,
`stripes`, the wordmark layer; textures `flyer_back_colour.gtf`, `scroll_col.gtf`,
`<grid>_elements.gtf`), through the same path as the front card
(`oag_game::flyer`, `Side::Back`). Its camera aspect is 1.5389 like the front's
(`+0x20` = `0x4a2c`, the Fury window).

- **Rectangle, measured; pose derived.** The frame puts the card on authored
  columns 165 to 1756 and rows 139 to 937 (centred, 1.994 wide for its height; the
  page's rule `GridTopBar` inside it spans 190 to 1690). Reading it through the
  frame's own mapping (`px = 157 + 0.94 * x`, from `GridTopBar`'s two ends) rather
  than guessing the layout scale. A card `CARD_HEIGHT` tall at `FOV_Y` fills 798 of
  1080 rows at 82.5 units, face-on: `CELL_POSE`, no fitted number. The width needs
  `BACK_STRETCH` 1.2965 (= 1.994 / 1.5389), **chosen, not measured**: the stretch
  scales the card and its picture together. `BACK_WINDOW` 0.321 is the Fury
  window, chosen.
- **The outline is the front's, mirrored.** `00_flyer.vex`'s `cardShape` (chamfer
  top-left, notches) cut the back at its chamfer top-right with the notches on the
  other edge, as the frame shows; no reflection under it.
- **Cards are drawn at twice their texel colour: `CARD_GAIN` 2.0, measured,
  mechanism unread.** The back panel's texel is 63 and the frame shows 130; the
  stripe texel 197 shows 255; `Grid Selection`'s front card's dominant red is 255
  where the undoubled texel drew 198 (so this also brightens Grid's card, which the
  flyer thread had called "duller reds"). Done through the vertex colour with
  `vertex_colour_is_light` off, no shader change. **Not on `Campaign Selection`'s
  two cards** (`CAMPAIGN_GAIN` 1.0): undoubled they matched the frame; doubled the HD
  card clamps to a white rectangle. Which materials carry the factor is unread
  (`basicnonalpha`'s fragment block multiplies by a patched `float1`, slot `0x2c`;
  the value was not read).
- Not drawn: the wordmark's glow and bloom (the thread's open item), the
  `blitzed` sliver above the wordmark's ascenders (a layer clipped by the window).

**The hex field is 32 black hexes.** `bBg_x_y` (black `Hexagon_HD.mip`, colour
`0xff000000`) and `Bg_x_y` (`Hexagon_HD_OUTLINE.mip`, `CM_HEX_Bg`) draw on every
slot; `Outline_`, `Lock_` and `Medal_` stay on the grid's own cells. The pointer
targets stay occupied-only.

**The four emblems.** `Event`/`Track`/`Speed Class`/`Weapons Emblem` are `<Image>`s
with a 96 by 96 rect and no `src`; `oag_ui::screen::Screen::slots` keeps such an
image (`Slot`, data only). The disc ships a white `*_bw.gtf` set named as the
screen's own values: `singlerace`, `timetrial`, `speedlap`, `zone`, `tournament`,
`head2head`, `eliminator`, `detonator`; `venom`, `flash`, `rapier`, `phantom`;
`weaponson`, `weaponsoff` (`Data\FE\Images\<stem>_bw.gtf`), and each circuit's
`FE\TrackSelectEmblem_bw.gtf`. Four of four icons on the frame (single race, the
circuit ring, one-star `VC`, the `X`) are the file their name says, tinted
`150,150,150`. **Mapping by file name, not a table the disc authors, confidence
70**; a mode with no such file (`NitroBattle`, `Custom Grid`, `AI Race`) draws no
picture. `oag_ui_screens::campaign::hd::cell_emblems`; the circuit id to folder
comes from the catalogue (`oag_game::campaign::load` takes `tracks`).

**The brackets.** `<Bracket corner="true">` is now kept as `Screen::brackets`
(data only; `picker::hd` still reads its own). Each rectangle is four marks of
`Data\FE\Images\corner2.gtf` (a 16 by 16 rounded top-left corner,
`docs/formats/gtf.md`), mirrored to the other three corners, flush with the edges,
in the bracket's `Colour`: two rectangles that touch (x 750 to 850 and 850 to 1250)
make the frame's "Y" joints. **Measured on one frame, confidence 65.**
`GridTopBar`, the rule across the card, is a colour-less `Slot` that the frame
draws solid `150,150,150` (confidence 70), and `flyerbarcode.gtf`, authored on the
screen and until now on no sheet, is added to the campaign's sheet and draws.

**The unlock box.** `flyerlogo` carries the next grid's `Logo.gtf` and `NextPoints`
the points still needed (`RequiredPoints` less earned), exactly as `Grid Selection`
draws them, and neither draws once the figure is met or on the last grid.

**Text.** The screen authors every label and value `TitleColor` (`0xFF646464`); the
frame draws headings at `150,150,150` and values at 255 (`cell_text`, **measured on
one frame, confidence 60**, by widget name), and `SINGLE RACE`/`VENOM`/the counter in
capitals where the string table holds `Single Race`/`Venom`. `CAMPAIGN RECORD` reads
`NONE` with no saved record (the frame does).

**Frame chrome (item 3).**

- **A real bug, in the split render path.** `flyer::render_list` calls
  `Renderer::render_with` twice into one encoder; `render_with` fills the quad
  buffer with `queue.write_buffer`, which lands before either pass, so both passes
  drew the *second* call's quads and the first's rules, bullet and `FuryBackdrop`
  neighbours vanished. That is why `Grid Selection` and `Campaign Selection` had no
  top rule or title bullet (item 3's "no top rule"). `Renderer::renew_quad_buffer`
  between the calls fixes it for every card page.
- `oag_hd::frontend::MENU_SKIN.title` was `TitleColor` (`100,100,100`); the frame's
  titles on `Main Menu`, `Grid Selection` and `Cell Selection` are `150,150,150`,
  `HD_Grey` as the title widgets author it (the two diverge in the skin the front end
  loads). Omega's `MENU_SKIN` shares HD's front end and was not changed: checked,
  applies, not wired (no PS4 capture).
- The root authors `<MiniText idstring="FE_SCREEN_TITLE">` at (160, 48): the string
  itself, `SCREEN TITLE`, a placeholder the shipped game draws on every page.
  `NavigationLegend` carries it and draws it through `picker::hd::draw_labels`.
- `ControlTextNavigationButton` (the four d-pad glyphs) and `ControlTextNavigation`
  (`FE_NAVIGATION`, x 260) are siblings of the footer's controller, outside the
  prompts the legend read; they are now a `PromptKind::Navigation` in the legend.

**Omega** (census 2026-10-08, `data/extracted/ps4`): ships its own flyers
(`000_flyer`, `0000_flyer`, `.gnf`/PS4 `.rcsmodel`), a `cellmode_definition.xml` in
`data07`/`data09` and per-circuit `TrackSelectEmblem_BW.gnf`; no `corner2` and no
`*_bw` mode icons under those names. Checked, applies in part, not wired (`load_omega`
draws no flyer and no emblem); the quad-buffer fix and the legend's caption apply to
its campaign pages as far as they draw.

**Still open** (item 4, page transitions): the screen authors per-widget
`transition`/`delay` keys (`bBg_0_0` 0.4 s, the emblems 0.05 to 0.275 s, the
right column 0.35 to 0.625 s, `EnableTransition` 0.3/0.5), not curves, and the
page-in is a vertical wipe (`transitiontype="vwipe"`) on the emblems; nothing here
plays them. A tween measured off `trans-campsel-original.png` remains the next step.
