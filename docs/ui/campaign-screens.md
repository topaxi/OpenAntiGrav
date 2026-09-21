# The Race Campaign's two screens: `Grid Selection` and `Cell Selection`

**Status: both screens draw, off the disc's own `CellMode_Definition.xml` and
the real 236-cell campaign, and confirming a cell launches it.** The player
can page through all sixteen grid tiers, drill into a tier's own hex grid,
and see a selected cell's detail panel - track, class, laps, weapons,
points, medal, and (where the disc shows them) the three medal targets.
Confirming a cell in one of the five modes this engine implements
(`Race`/`Time Trial`/`Speed Lap`/`Zone`/`Elimination`) opens `Team
Selection` and launches the cell's own race; a cell in one of the four this
engine does not (`Tournament`/`Head2Head`/`Custom Grid`/`AI Race`) logs why
and stays on `Cell Selection`. See "Confirming a cell launches" below and
`docs/architecture/persistence.md` for the medal it earns and where it is
kept. **Both screens answer a mouse and a finger too, since 2026-09-14** -
hovering a tier tile or a hex selects it, a second click on the selection
confirms it, the paging arrows are clickable, and the secondary button (or
a click/tap while `Cell Help` is open) backs out - the same vocabulary
`docs/architecture/menus.md`'s "A mouse and a finger" already gives every
other front-end screen. See
[`oag_ui::campaign::pointer`](../../crates/ui/src/campaign/pointer.rs) for
the targets and the two-tap idiom, and its own module doc for why a hex is
hit-tested as a hexagon rather than its bounding box - the staggered grid's
neighbours overlap at their corners otherwise. Implemented in
[`oag_ui::campaign`](../../crates/ui/src/campaign.rs) (model, layout, draw),
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
(`MaxX="4" MaxY="1"`) is the current page's four tiers, and up/down (the
`up arrow`/`down arrow` at `(117, 84)`/`(117, 167)`) pages through all
sixteen one at a time - the same wrapping-list idiom
[`oag_ui::picker::Picker`](../../crates/ui/src/picker.rs) already uses for
`Track Creation`'s circuit list. `oag_ui::campaign::GridSelection` implements
it that way: one flat `index` over every grid, `page()` and `slot()` derived
from it, `counter()` reproducing the formula exactly.

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
`docs/architecture/persistence.md`). `oag_ui::campaign::GridSummary::from_grid_with_medals`/
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
shipped ones never reach. `oag_ui::campaign::draw::grid_title` looks the
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

`oag_ui::campaign::GridSelection::tier_shows_lock`/`CellSelection::cell_shows_lock`
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

`oag_ui::campaign::CellSelection` reads the selected `(x, y)` position
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
the raw `NN_Track` id. `oag_ui::campaign::draw::cell_title` mirrors
`crate::menu::mode_label`'s own `MSC_EVENT_*`-head-before-the-colon reading
for the five modes this engine implements, plus `MSC_EVENT_TOURN`/`_HTH`
for `Tournament`/`Head2Head` (present on disc, resolved the identical way,
but unmeasured against a live frame - no capture reaches either mode);
`Custom Grid`/`AI Race`/an HD `Other` mode still fall back to the raw
spelling, since neither authors an `MSC_EVENT_*` entry at all.
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
| `Line7` | `IG_HUD_GOLD`/`SILVER`/`BRONZE` for a saved medal, `MSC_NONE` otherwise | only when `Target0..2` are not (see below) |
| `Line7 Title` | `IG_HUD_BEST` (`"Best"`) | same as `Line7` |
| `Target0..2` | `cell.gold`/`silver`/`bronze`, formatted as `M:SS.CC` for Time Trial/Speed Lap, a plain number otherwise | Time Trial / Zone / Elimination / Speed Lap only |
| `Target0..2 Title` | `IG_HUD_TARGET`, resolved | same as `Target0..2` |
| `Target0..2 Image` | the gold/silver/bronze swatch (`0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`) | same as `Target0..2` |

**`Line7`/`Target0..2` are mutually exclusive, not additive, 2026-09-14** -
see "A cell's detail panel: `Best`/`Target` looks mutually exclusive" below,
whose finding this now implements: `Line7`/`Line7 Title` draw only when
`targets_visible` is false, sharing the row `Target0..2` would otherwise
sit in.

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
Fixed; `crates/ui/src/campaign/tests.rs`'s
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
!= 0`). `oag_ui::campaign::draw::{grid_draw_list, cell_draw_list}` now
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
  `oag_ui::campaign` this pass did not need to touch) but without the
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
mechanism: `oag_ui::campaign`'s `hex_position` (`crates/ui/src/campaign.rs`)
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
from the XML directly and are re-derivable from `crates/ui/src/campaign/
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
  directly observed.
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
[`oag_ui::campaign::Layout::read_authored`](../../crates/ui/src/campaign.rs)
reaches both by the same two names unmodified - the discriminating question
this thread opened with ("does `FlyerSelection` combine what Pulse splits
into two?") turned out to be no: read whole, the file is the same two-screen shape, just
authored differently inside each screen. Implemented in
[`oag_ui::campaign::hd`](../../crates/ui/src/campaign/hd.rs), wired through
`oag_game::campaign::load` (title-dispatched to a Wipeout HD/Fury branch the
same way the rest of this build picks a title's tables),
`crate::main::campaign_stage::CampaignStage::is_hd`/`circuit_names`/
`cell_grid_summary`, and a draw/pointer dispatch in `crate::main::menu_stage`/
`crate::main::session::pointer`. **The confirm/launch path is untouched** -
`Screen::Grid`/`Screen::Cell` and `oag_ui::campaign::Event` are shared
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
Selection`'s own lock rule. `oag_ui::campaign::GridSelection`'s model is
reused as-is - one flat `index`, `step`, `selected_is_locked` - since a
one-tier-at-a-time pager is exactly that shape; only the *draw* differs
(`hd_grid_draw_list`, not `draw::grid_draw_list`).

Measured off the widget names and the panel's own numbers:

| Thing | HD widget | Reads |
| --- | --- | --- |
| page counter | `EventNum`/`GridNum` | `"Event {:02}/{:02}"`, 1-based - the literal template both widgets author |
| medal fraction | `Medals Title` (`RC_POINTSACH`) / `Points` | `Grid_CountMedalsAtLeast`/`Grid_CellCount`, `"00/06"` shape - same numbers Pulse's own `Medals` field carries |
| points fraction | `Points Title` (`RC_TOTPOINTSAV`) / `TotPoints` | `points_earned`/`max_points`, `"000/018"` |
| unlock reason | `Required`/`Required Previous` | two texts for two distinct lock reasons - see below |
| flyer lock | `Flyer Pad Lock` | gated on `selected_is_locked()` |

**`Required`/`Required Previous` pick between two texts for two distinct
reasons a tier is locked - chosen, not measured**, on the same terms as the
glyph rule they read: `Required` (its own idstring `RC_POINTSTOUNL`) shows
when the tier's own lock is not explained by the previous tile falling
short; `Required Previous` (a literal string) shows when it is. No capture
or decompile settles which predicate each is actually bound to; this reuses
`GridSelection::selected_is_locked`'s own two terms to choose between them
rather than showing both or neither.

**`RC_POINTSTOUNL`'s own string is a raw `%d` template** (`"NOCH %d PUNKTE
ZUM FREISCHALTEN VON:"` in German - "still %d points needed to unlock
from:") that nothing in this build's string-table reader formats. `hd.rs`
substitutes the one concrete figure this screen already carries
(`required_points`) - **chosen, not measured**, since which number the
template actually wants (this grid's own requirement, a different one, the
gap to the previous tile) is not settled by reading the file alone, and
whether the widget is gated on the grid genuinely being short that many
points is unmeasured too - this build shows it whenever the lock reason
picks that text at all.

**Not drawn, and said so in the loader/draw code**: the 3-D flyer model
itself (`Data\FE\Flyers\00_flyer.vex`, `<Flyer name="FlyerModel">`) - this
crate draws a flat 2-D list, not a mesh scene, and wiring a `.vex` flyer
through `oag_render` behind this screen is out of this pass's scope; and the
per-grid `flyerlogo` (`Data\FE\Flyers\01_uplift\Logo.gtf`), whose path is
the **same literal string on every grid** in the file with no per-grid
attribute to pick a different one, so drawing it would show grid 9's own
logo under grid 1's name.

### `Cell Selection` is the same 32-slot staggered hex grid Pulse's is

`GridController name="Grid" OffsetX="170" OffsetY="230"`, `MaxX="7"
MaxY="5"` - the identical bounding shape and widget-name scheme
(`{Bg,Outline,Lock,Medal}_{x}_{y}`) Pulse's own `Cell Selection` authors,
just with a fourth layer (`Bg_x_y`, `Hexagon_HD_OUTLINE.mip`/`.gtf` - see
below) under Pulse's three. `oag_ui::campaign::CellSelection` and
[`hex_rect`](../../crates/ui/src/campaign.rs) (which already searches for a
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
| `Track` | [`hd_track_line`](../../crates/ui/src/campaign/hd.rs) | through `oag_ui::language::CircuitNames`, not `strings.get_or_id` directly - see below |
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
the raw id - **simplified from `oag_game::catalogue::label`**: that function
also appends `FE_REVERSE` where a reversed circuit shares its forward
twin's own name, which needs the full circuit catalogue this module has no
access to at draw time, so a reversed HD circuit whose name collides with
its forward twin is not disambiguated here. The live session
(`session::campaign::open_campaign`) passes the real
`Shell::circuit_names`; the headless `--menu-page cell-select` capture
passes `CircuitNames::default()` (a no-op fold, same gap `picker_page`'s own
RACE-page capture already has) - see `oag_game::campaign::menu_page`'s own
doc for why building one there was out of scope this pass.

**A three-rung difficulty toggle, `DifficultyButton` - new state, backed by
an existing function.** `oag_tables::race_campaign::Cell::targets_for_difficulty`
already existed (added alongside the schema itself); nothing there changed.
What is new: `CellSelection::difficulty`/`cycle_difficulty`, a `0..=2`
field defaulting to `1` (medium - the rung `gold`/`silver`/`bronze` already
mean on a cell with no `difficulty_targets`, so it is inert on every Pulse
cell) and a wrapping step, bound to `Square` on the pad (free on this
screen in this build) and to a click on `DifficultyButton`'s own text via
[`difficulty_button_rect`](../../crates/ui/src/campaign/hd.rs) - **an
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
folds case and backslashes for the read, but `oag_game::sprite::Sheet::get`
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

**Root cause: `hex_rect` (`crates/ui/src/campaign.rs`) tried `Medal_{x}_{y}`
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
(`crates/ui/src/campaign/tests.rs`), a synthetic fixture asserting
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
prefix swap is a no-op for them (`crates/ui/src/campaign/tests.rs`'s own
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
named `Screen` carries as a fact. `oag_ui::campaign::footer` (new module)
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
  mechanism this build does not have - so `CampaignStage::ticker_tips` only
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
   (see `oag_ui::campaign::footer::ticker_draw`'s own doc). Confirmed live,
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

- **Every label on this screen still renders upper-case** (`SPEED CLASS`,
  `MOA THERMA WHITE`, `CONFIRM`) where the reference shows mixed case
  (`Speed class`, `Talon's Junction White`, `Confirm`) - pre-existing,
  present before this lane and on every `Line1`..`8`/`Title`/`Track Line`
  label already, not only the two widgets this pass added. Fixing it needs
  a second, correctly-sized atlas loaded for the menu stage, not a role
  string chosen differently in `oag_ui::campaign`.
- **`AI difficulty (Medium)` reads `CHANGE DIFFICULTY`** on this build - a
  pre-existing label (`DifficultyButton`'s own authored `string="Change
  Difficulty"` in `CellMode_Definition.xml`, drawn through
  `oag_ui::campaign::draw`'s generic `_ => text.string.clone()` fallback
  that predates this lane) rather than the original's own template showing
  the current rung. Left as-is on the team lead's own instruction; the
  string and its mechanism are named here for whoever picks up the atlas
  work above, since fixing the case issue would make this one legible
  without also fixing the missing rung substitution.

## Open

- ~~The scrolling tip ticker and the button-legend footer row are still not
  drawn.~~ **Both draw, 2026-09-21** - see "The tip ticker, the Confirm/Back
  legend, `Cell Help` and a podium" above. The scroll speed itself is still
  unmeasured against a live PPSSPP frame.
- **A previous pass's live-walk cell (`grid0_2_1`) is now locked under this
  pass's own rule** - it authors no `Locked` attribute, which defaults to
  `true`, and has no medal or medalled neighbour on a fresh profile. The
  "launch path... driven live" bullet below recorded a walk through that
  exact cell; repeating it today would refuse `Confirm` rather than reach
  `Team Selection`. `grid0_3_1`/`grid0_3_2` (both author `Locked="false"`) are
  the cells to re-walk with, or any cell after the profile has earned it or
  a hex-adjacent medal.
- **PPSSPP was not captured against this pass.** Every number above is read
  off the disc's own XML and the executable's decompiled binding, not
  cross-checked against a live screenshot the way `selection-screens.md`'s
  numbers are. The walk: `Main Menu` (`FE_RACE_CAM`) -> `TournamentLoad`
  (behind `MSC_MSG_AUTOSAVE3`, gated on `MSC_SQ_MSG7`, on a fresh profile
  only reached once) -> `Grid Selection` -> `Cell Selection` -> `Cell Help`.
  `scripts/psp-frontend-capture.py` walks as far as `Team Selection` today
  and would need this leg added, with the autosave dialog answered.
  `docs/reverse-engineering/ppsspp-debugger.md` and
  `scripts/psp-drive.py preflight` are the entry points. Two things this
  would settle that reading alone cannot: whether the `Grid`/`Grid1`
  duplicate reads as a crossfade or a settled state, and what the empty-hex
  fill looks like against the filled/current one on a genuinely fresh save.
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
  the `campaign-pointer` lane. `oag_ui::campaign::centred_selector_draw`
  centres the two rects on each other instead - **chosen, not measured**:
  both sprite sizes are the disc's own, but nothing on disc says centring is
  the right rule for the gap between them, only that top-left alignment
  reads wrong. `the_selector_is_centred_on_the_selected_hex_not_top_left_aligned`/
  `cell_selections_selector_is_also_centred_on_the_selected_hex`
  (`crates/ui/src/campaign/tests.rs`) pin the two screens' own resolved
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

**This build's own `oag_hd`/`oag_ui::campaign` has no `Campaign Selection`
state whatsoever** - it goes straight to whichever grid
`oag_assets::Archives::read_name` precedence resolves (`DATA02`'s `grid0`),
so there is today no path in this build to `Fury`'s own campaign, and the
original "which archive copy does the real screen show" question is
**still not settled for `grid0` specifically** - both boots this pass spent
landed on `Fury` instead. Screenshots:
`data/scratch/lane-hd/rpcs3-grid0-3-2/01-down.png` (`grid8`, `NOVICE`
rung), `02-triangle.png` (`SKILLED` rung, same cell) - this session's own
worktree, not committed (game content). **Left open, higher-value than the
original question**: whoever picks this up next should find whether
`Campaign Selection`'s other entry (the base `Wipeout HD` campaign) is
reachable at all in this Fury-branded disc, and if so, wire this build's
own front end to model the screen rather than skipping straight to a grid.
Confidence 85 on "the screen exists and defaults to Fury" (two independent
boots, consistent); confidence 0 (unmeasured) on whether the base campaign
is reachable by any input from there.

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
- **The 3-D flyer model behind `Grid Selection`** is not drawn at all -
  `oag_ui` draws a flat 2-D list, and putting a `.vex` mesh behind a 2-D
  screen needs a render-side mechanism this pass did not build. Whoever
  picks this up should decide whether that mechanism belongs in
  `oag_render` generally or is specific to this one screen.
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
- **The `Required`/`Required Previous`/`NextPoints` unlock-reason
  predicates are chosen, not measured**, on both which text shows and
  what number a raw `%d` template should carry - see the two sections
  above. An RPCS3 capture of a genuinely locked tier/cell would settle
  both at once.
- **The upper-case defect above is partially closed, 2026-09-21, from the
  render side rather than this crate's own.** `Confirm`/`Back`
  (`oag_ui::campaign::footer`) and the per-row subtitle
  (`oag_title::HelpText`) now draw mixed case, through a second,
  `Default`-role atlas the menu stage loads beside its unchanged `menu`-role
  primary - see `docs/ui/menus-original.md`'s "Two faces, not one swapped
  for the other" section for the full mechanism and the measured fact it
  rests on (Pulse's `menu`/`Small` faces have no lowercase glyph art at
  all; only `Default` does). **Still upper-case**: every `n="default"`
  widget `oag_ui::campaign::draw::text_draw` itself draws - `Speed class`,
  `Laps`, `Weapons`, `Points`, `Best` and their values, and by the same
  mechanism `Change Difficulty` two bullets up. One edit closes it:
  `text_draw`'s single `Draw::Text` literal becoming a role-aware
  constructor (`crate::frontend::Draw::in_role`, `Some("Default")` for a
  `"default"`-labelled `text.font`, matching what `footer.rs` and
  `menu::rows::draw_text_rows` already do) - a change to this crate's own
  `campaign/draw.rs`, outside the lane that found and fixed the mechanism.
  `NavigationLegend`'s own `FE_CONFIRM` shrink-to-fit (`Prompt::left_bound`/
  `align_right_to`, above) is now measured against the atlas it actually
  draws through rather than the wrong one, and **still shrinks**: `Confirm`
  at `pulse_text.fnt`'s own native width is 91 native px against a 35px
  gap (`FE_BACK_BUTTON`'s `x` minus `FE_CONFIRM_BUTTON`'s `x` minus one
  glyph-width estimate minus two `GAP` constants, both chosen). The face
  was the wrong thing to suspect for the overlap this fixed *before* this
  atlas fix landed; now that the face is right, the suspect is `GAP`/the
  glyph-width estimate, both still chosen rather than measured.
