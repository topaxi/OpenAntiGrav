# The Race Campaign's two screens: `Grid Selection` and `Cell Selection`

**Status: both screens draw, off the disc's own `CellMode_Definition.xml` and
the real 236-cell campaign. The player can page through all sixteen grid
tiers, drill into a tier's own hex grid, and see a selected cell's detail
panel - track, class, laps, weapons, points, medal, and (where the disc
shows them) the three medal targets. Confirming a cell is a stub: it does
not launch a race.** Implemented in
[`oag_ui::campaign`](../../crates/ui/src/campaign.rs) (model, layout, draw),
[`oag_game::campaign`](../../crates/game/src/campaign.rs) (the shared read
off an open source), `crates/game/src/main/campaign_stage.rs` (what the
session holds open) and `crates/game/src/main/session/campaign.rs` (the
flow: `RACE CAMPAIGN` opens it, confirm and back move between the two
screens). Captured headlessly with `--menu-page grid-select` /
`--menu-page cell-select`, on `pulse-psp-eu`.

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

### The two things this build cannot show yet

**Every medal/points figure is the fresh-profile one**, because this
project keeps no per-cell campaign save at all -
`crates/game/src/records.rs`'s own doc: `Observation::campaign_medal` is
`None` at the only call site that builds one today. `GridSelection_Update`
binds `Medals` to `Grid_CountMedalsAtLeast(grid, 0) / Grid_CellCount(grid)`
and `Points` to `Grid_PointsEarned / Grid_PointsPossible` - both read a
saved record this build does not keep, so both draw as if nothing has ever
been raced: `"00/{cell_count:02}"` and `"000/{max_points:03}"`. `Required`
*is* real - `grid.required_points`, or `FE_NA` on `grid15`'s own `0` - since
it is authored on the grid itself, not derived from a save.

**`Title` is the grid's own raw name**, e.g. `"grid0"`, not a friendlier
"GRID 1". `GridSelection_Update` binds it to `grid->name` (`+0x74`)
directly with no further formatting, and nothing on disc carries a nicer
label for a tier - the XML's own `string="GRID 1"` is the same runtime
placeholder pattern `Medals`/`Points`/`Required` are. Drawn as measured
rather than invented a prettier scheme.

## Grid tiers are not locked

`Unlock_GridPointsMet` gates the *next* grid on the *previous* grid's
`RequiredPoints` being met - and that comparison needs
`Grid_PointsEarned`, which needs the same per-cell save this build does not
have. There is nothing to sum, so every tier draws open rather than reading
a lock this build cannot evaluate honestly. **Chosen, not measured**:
leaving every tier reachable is this project's own call, not a reading of
the disc (the disc's own `Cell Selection`/`Grid Selection` would show locks
on an unplayed profile too - grid1..15 all authoring `Locked="true"`, see
below).

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

| Widget | Source | Shown for |
| --- | --- | --- |
| `Title` | `cell.mode.as_str()` - the raw enum spelling (`"Head2Head"`, not "Head to Head") | always |
| `Track Line` | `cell.track`, or `"{n} Races"` off `tournament_tracks.len()` | always |
| `Line1` | `cell.class` (raw string - `"Zone"` on a Zone cell, not a speed class) | all but Zone |
| `Line2` | `cell.laps`, or `RC_INF` when absent/zero/Zone | always |
| `Line3` | `FE_ON`/`FE_OFF` off `cell.weapons` | Race / Head2Head / Tournament only |
| `Line6` | `"0/3"` - earned points over `Medal::Gold.points()`, always `0` earned (no save) | always |
| `Line7` | `MSC_NONE`, resolved ("NONE"/"NÉANT"/...) - no medal is ever saved | always |
| `Target0..2` | `cell.gold`/`silver`/`bronze`, formatted as `M:SS.CC` for Time Trial/Speed Lap, a plain number otherwise | Time Trial / Zone / Elimination / Speed Lap only |
| `Target0..2 Title` | `IG_HUD_TARGET`, resolved | same as `Target0..2` |
| `Target0..2 Image` | the gold/silver/bronze swatch (`0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`) | same as `Target0..2` |

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

### Four widgets this build deliberately does not draw

- **`Lock_x_y` and `Lock_n_0`, on both screens.** `Locked` on a `PI_Cell`/
  `PI_Grid` has no traced consumer - `race-campaign.md`'s "what is not
  determined" scores this 50 and says explicitly not to implement a lock
  from it. Every cell and every tier draws open.
- **`Line{n} Title`, every `n`, on `Cell Selection`.**
  `CellSelection_PopulateDetail`'s own table names `Line1`..`Line8` as
  *values* but never their `Title` companions - unlike `Target0..2 Title`
  (`IG_HUD_TARGET`, a real idstring) or `Medals Title`/`Points Title`/
  `Required Title` (`RC_GM`/`RC_TP`/`RC_PN`) on `Grid Selection`. Their own
  authored strings are template junk (`"l1 title"`, `"--7"`), not an
  idstring, so there is nothing to resolve and nothing safe to invent -
  skipped uniformly.
- **`Line4`.** Never appears in `PopulateDetail`'s own table at all.
- **`Line5`/`Line8`.** Both `Cell_SavedRecord` - the saved best this build
  has no record for - sharing one offset (`Item OffsetX="260" OffsetY="180"`,
  the same swap idiom `race-setup.md` documents for `Single Player`'s
  `Zone`/`DifficultyNaText`).

## The stub boundary

Per `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s own "what is
not determined" section - the launch path (which globals `Cell Selection`'s
own confirm redirect writes before falling through to `Team Selection`/
`Launch Game`) is not traced - confirming a grid tier opens `Cell Selection`
on that tier's own cells (pure screen navigation), but confirming a cell
**does not** open `Team Selection` or launch a race.
`session::campaign::handle_campaign` logs and stays on `Cell Selection`.

`Cell Help` (`triangle`) is modelled as a boolean toggle
(`CellSelection::help_open`) that suspends movement while open, matching
the disc's own `Watch` element redirecting every directional press back to
`Cell Help` while it is the present screen - but the overlay itself is not
drawn yet; see [Open](#open).

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

## Open

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
- **`Cell Help`'s own overlay is read (`Main Help`/`Speed Class Help`/
  `Event Help`, all resolvable idstrings) but not drawn.** Its `Viewport`/
  `Animation` timeline (`LimitVerticalScroll="10"`, `Key Time="0" Y="0"` ->
  `Time="10" Y="-100"`) is the same kind of reveal
  `oag_ui::screen::Screens::collect_widgets` already discards the timeline
  of and keeps the widget for elsewhere in this crate - a static overlay
  would be honest, an animated one is not built.
- **The Selector cursor's exact centering is unmeasured.** It is drawn at
  the selected hex's own `Medal_{x}_{y}` position, top-left aligned rather
  than centred against the hex's own (smaller) extent - the XML gives no
  number for the offset between a `42x43` selector and a hex sized however
  `hex_filled.mip` actually is.
- **The launch path.** See `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
  "what is not determined" section and `docs/architecture/persistence.md`'s
  "what is not built yet".
