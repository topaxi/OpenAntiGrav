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
kept. Implemented in
[`oag_ui::campaign`](../../crates/ui/src/campaign.rs) (model, layout, draw),
[`oag_game::campaign`](../../crates/game/src/campaign.rs) (the shared read
off an open source), `crates/game/src/main/campaign_stage.rs` (what the
session holds open) and `crates/game/src/main/session/campaign.rs` (the
flow: `RACE CAMPAIGN` opens it, confirm and back move between the two
screens, and a confirmed cell launches). Captured headlessly with
`--menu-page grid-select` / `--menu-page cell-select`, on `pulse-psp-eu` -
the drawing alone; the launch itself needs a live window, not captured
against the running original in this pass (see [Open](#open)).

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
| `Line6` | the selected cell's own saved points over `Medal::Gold.points()`, `"0/3"` with no saved medal | always |
| `Line7` | `IG_HUD_GOLD`/`SILVER`/`BRONZE` for a saved medal, `MSC_NONE` otherwise | always |
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
- **The launch path is wired but not interactively played in this pass.**
  All of it is proven by the unit/integration suite
  (`crates/game/src/campaign/tests.rs`, `crates/game/src/records/tests.rs`,
  `crates/game/src/race/tests/mode_override.rs`,
  `crates/ui/src/campaign/tests.rs`) and by the full `just`/`just test-data`
  gate, but nobody has driven `Cell Selection` from a real keyboard and
  watched a campaign race finish and a medal appear on the results table
  and back on the hex grid: this pass ran in an environment with no way to
  send synthetic keyboard input to a native Wayland window and no safe way
  to screenshot one window in isolation from a shared desktop, so a live
  play-test was not attempted rather than faked. A maintainer with a normal
  desktop session can: `cargo run -p oag-game -- data/images/pulse-psp-eu.chd`,
  navigate `RACE CAMPAIGN` -> a tier -> a `Race`/`Time Trial` cell -> confirm
  -> `Team Selection` -> confirm, let (or `--autopilot`) the race finish,
  and check the results table's `BEST MEDAL` line and the cell's own
  `Line6`/`Line7` back on `Cell Selection`.
