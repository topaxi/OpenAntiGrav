# The campaign grid draws and does not launch

2026-09-14. `Grid Selection` and `Cell Selection` - the Race Campaign's own
two screens - now draw, off `Data\Plugins\PI001\GUI\CellMode_Definition.xml`
and the real 236-cell campaign
([`docs/formats/race-campaign.md`](../../docs/formats/race-campaign.md)).
This is the frontend-drawing half; the launch half is the other lane's, per
the gameplay handover's own instruction not to wire cell-selection without
first tracing the launch path.

**Update, same day: the launch landed too, in the `campaign-launch-wiring`
lane.** The title is kept only so this thread's index line still matches -
confirming a cell in one of the five modes this engine runs now opens `Team
Selection` and launches the cell's own race, evaluates the medal earned and
persists it keyed on the cell's own `name`, and feeds it back into
`Grid Selection`'s `Medals`/`Points` rows and `Cell Selection`'s `Line6`/
`Line7`. See `docs/ui/campaign-screens.md`'s "Confirming a cell launches"
and `docs/architecture/persistence.md`'s "where a career system attaches"
for the full detail. **Not closed by that pass**: everything below this
paragraph that is not struck through is still open, and it did not attempt
a live PPSSPP capture or an interactive play-test of the new launch either
- see `docs/ui/campaign-screens.md`'s own `Open` entry on why.

**Update, 2026-09-14, `campaign-picture` lane: the picture itself, against
the PPSSPP frames the `campaign-ppsspp-results` lane measured the same
day.** Most of six items landed: the hex grid draws `Outline_x_y` always
and `Medal_x_y` only where a medal/points are earned (was: `hex_filled.mip`
everywhere); `Lock_x_y`/`Lock_n_0` draw under
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s measured
three-term rule on both screens, and a locked tile now refuses `Confirm`
too (chosen, not measured, on the exact refusal mechanism - see
`docs/ui/campaign-screens.md`'s "Grid tiers and cells lock and unlock");
`Grid Selection`'s `Title` resolves through a real per-grid idstring
(`"GRID 1"`, and `"PHANTOM GRID 1"` on `grid12`..`grid15` - not derivable
from a formula), `Cell Selection`'s `Title`/`Track Line` resolve through the
disc's own mode/circuit names, and the panel's five row labels (`Speed
class`/`Laps`/`Weapons`/`Points`/`Best`) resolve to real idstrings instead
of staying blank. **Not landed**: the tip ticker and the `Confirm`/`Back`
half of the button-legend footer - see this thread's `Open` section, which
replaces the two locking bullets below (struck through) with the measured
rule now implemented.

**Update, 2026-09-21, `pulse-campaign-ticker` lane: the tip ticker, the
`Confirm`/`Back` footer legend, `Cell Help`'s static overlay and `Line5`
(`Cell_SavedRecord`, Time Trial/Speed Lap) all now draw - and the podium walk
closed live**, `[ai] difficulty = "novice"` (worktree-local, never committed)
plus `--autopilot-skill ace` finishing `grid0_3_1` 1st of 8 and `Cell
Selection` reading `Points 3/3` / `Best Gold` back. New module:
`oag_ui::campaign::footer`, reading `Skin.xml`'s `<NavigationController>`/
`<TextInfo>` directly off the raw parsed tree - neither tag is one
`oag_ui::screen::Screens::collect_widgets` recognises, and even parsed,
neither fits the per-screen widget model (a `NavigationController` picks
prompts *per screen*; the ticker is two alternating text buffers sharing one
clip viewport). Full detail in `docs/ui/campaign-screens.md`'s own
2026-09-21 section. **Still open below**: the `Grid`/`Grid1` duplicate
mystery, a live PPSSPP capture of the ticker/footer/`Cell Help` (this pass
verified only against this build's own live behaviour, not the original's),
the ticker's own scroll speed (chosen, not measured), and `Line8`/`Zone`/
`Elimination`'s own saved records (no raw count anywhere in
`oag_game::records::Record` to read).

Milestone: **M7 - Shell and polish**.

Read first: [`docs/ui/campaign-screens.md`](../../docs/ui/campaign-screens.md)
(what draws, what is measured vs chosen, every number this pass read off the
disc), `oag_ui::campaign` (`crates/ui/src/campaign.rs`, model + layout +
draw), `oag_game::campaign` (`crates/game/src/campaign.rs`, the shared
read), `crates/game/src/main/campaign_stage.rs` and
`crates/game/src/main/session/campaign.rs` (the live flow).

## Open

- **PPSSPP was not captured against this pass.** Everything drawn is read
  off `CellMode_Definition.xml`'s own XML and `race-campaign.md`'s
  decompiled bindings, never cross-checked against a live screenshot -
  unlike `docs/ui/selection-screens.md`'s numbers, which are. The walk:
  `Main Menu` (`FE_RACE_CAM`) -> `TournamentLoad` (an autosave dialog behind
  `MSC_SQ_MSG7`, reachable once on a fresh profile) -> `Grid Selection` ->
  `Cell Selection` -> `Cell Help`. `scripts/psp-frontend-capture.py` walks
  as far as `Team Selection` today; this leg needs adding, with the
  first-boot and autosave dialogs answered. See
  `docs/reverse-engineering/ppsspp-debugger.md` and
  `scripts/psp-drive.py preflight`.
- **The `Grid`/`Grid1` duplicate `GridController` is read but not resolved.**
  `Grid Selection` authors two identical controllers at one position - `Grid`
  (`focus="true"`, a staggered `delay` reveal on its own hexes) and `Grid1`
  (`startenabled="false"`, otherwise byte-identical minus `delay`). This
  build skips whichever carries `startenabled="false"`, which is safe
  (`docs/ui/campaign-screens.md` says why - a name collision in
  `screen.images` otherwise) but does not settle *why* two exist: an
  entrance-animation buffer that gets swapped to the settled state, or a
  paging crossfade between four-tile pages. A PPSSPP capture across a page
  turn would likely settle it.
- ~~`Cell Help`'s own text is resolved but the overlay is not drawn.~~
  **Done, 2026-09-21** - draws as a static panel, `oag_ui::campaign::draw::cell_help_draw`.
  Its `Viewport`/`Animation` scroll timeline is still not scripted, on
  purpose - see `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~Grid-tier locking draws every tier open.~~ **Done, 2026-09-14**, in the
  `campaign-picture` lane, once a per-cell save existed to compare
  `Grid_PointsEarned` against and `race-campaign.md`'s own later pass traced
  the actual glyph rule. See the update paragraph above.
- ~~`Locked`/`Lock_x_y` is deliberately unimplemented.~~ **Done, 2026-09-14**
  - `race-campaign.md`'s own confidence rose to 82-85 once its "Unlock
    rules, cell and tier" section traced the consumer in full; the
    `campaign-picture` lane implemented the draw side the same day. See the
    update paragraph above.
- **PPSSPP measured 2026-09-14 - `docs/ui/campaign-screens.md`'s "Measured
  against PPSSPP" section has the full readings and frame paths
  (`data/reference/psp-campaign-screens/`, gitignored).** Three corrections
  to this file's own prose above, all confirmed live rather than inferred:
  `Grid Selection`'s `Title` reads `"GRID 1"`/`"GRID 5"`/`"GRID 9"`, not the
  raw `grid->name` this file and `campaign-screens.md` both assumed - the
  "runtime placeholder" reading of the XML's own `string="GRID 1"` was
  backwards. `Cell Selection`'s `Title` is a localised mode name
  (`"SINGLE RACE"` for a `Race` cell, matching the Racebox's own `RACE TYPE`
  wording), not `cell->mode`'s bare enum spelling. The `Track Line` is the
  circuit's own display name (`"Moa Therma White"`, `"Talon's Junction
  White"`), not the raw `03_Track`/`16_Track` string. Also new: a scrolling
  tip ticker along the bottom of both screens (not documented anywhere
  before this pass, not drawn here at all), and behavioural evidence
  (confidence 70, not breakpoint-verified) that **grid-tier locking gates
  `Confirm` in the original** - pressing it on a `Locked` grid does nothing -
  which is stronger than "cosmetic lock glyph" and worth weighing against
  the "chosen, not measured" note two bullets up. `race-campaign.md`'s own
  `Locked`/`Lock_x_y` reading (the *cell*-level glyph, a different byte) was
  independently settled by a different pass the same day; this file's bullet
  above still describes this build's own choice not to implement it, which
  stands unless whoever picks this thread up next decides otherwise.

## Next Steps

- ~~Wire the launch, once the other lane's trace lands.~~ **Done,
  2026-09-14**, in the `campaign-launch-wiring` lane -
  `Session::launch_campaign_cell` (`crates/game/src/main/session/campaign.rs`)
  is the caller; see `docs/ui/campaign-screens.md`'s "Confirming a cell
  launches" and `docs/architecture/persistence.md`. The track/mode spelling
  mismatch this bullet worried about turned out not to be one: a cell's own
  `track=` id already shares `catalogue::Track::id`'s spelling, so
  `Shell::track(mode, id)` resolves it directly.
- ~~Feed `oag_game::records::Observation::campaign_medal` once a cell is
  selected.~~ **Done, 2026-09-14.** `RaceStage::campaign_medal`
  (`crates/game/src/main/race_stage.rs`) evaluates the per-mode value -
  narrower than this bullet assumed: `Observation::tick` turned out to be
  the wrong quantity for `Speed Lap` (a mode that never finishes, so "the
  tick it was left on" is not "how fast" - checked against
  `grid_00.xml`'s own authored targets, see
  `docs/architecture/persistence.md`), which reads `best_lap_ticks`
  instead; `Zone`'s zone count and `Elimination`'s kill count did turn out
  to be reachable without widening `Observation` at all - `RaceStage`
  already has `self.race` in hand, so the value is computed there and only
  the already-evaluated medal crosses into `Observation`.
  `Grid Selection`'s `Medals`/`Points` and `Cell Selection`'s `Line6`/
  `Line7` all read real numbers now, fed through a `Fn(&str) ->
  Option<Medal>` closure rather than the store type itself - see
  `oag_ui::campaign::GridSummary::from_grid_with_medals`/
  `CellSelection::with_medals`. ~~`Line5`/`Line8` are unchanged.~~ **`Line5`
  done, 2026-09-21** - `CellSelection::with_medals_and_records` reads the
  general per-track/mode/class `oag_game::records::Store` for Time Trial/
  Speed Lap. **`Line8` still blank on purpose** (collides with `Target0`'s
  own row) and **`Zone`/`Elimination` still have no raw count anywhere in
  `oag_game::records::Record` to read** - see
  `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~A PPSSPP capture of the walk above~~, to move every confidence score in
  `docs/ui/campaign-screens.md` from "read off the XML and the decompile"
  to "measured against a live frame". **Done, 2026-09-14**, in the
  `campaign-ppsspp-results` lane - see `docs/ui/campaign-screens.md`'s
  "Measured against PPSSPP" section, which that lane owns; do not edit
  inside it.
- ~~The scrolling tip ticker and the `Confirm`/`Back` half of the footer
  legend still do not draw.~~ **Done, 2026-09-21** - `oag_ui::campaign::footer`.
  The ticker's own scroll speed is chosen, not measured, and the
  `Grid`/`Grid1` duplicate `GridController` question above is still open.
- **A previous live walk's cell (`grid0_2_1`) is locked under the rule this
  pass implemented.** The "interactive play-test" bullet below drove
  through `grid0_2_1` on a fresh profile; that cell authors no `Locked`
  attribute (defaults locked) and has no medal or medalled neighbour, so a
  repeat of that walk today would refuse `Confirm` rather than reach `Team
  Selection`. `grid0_3_1`/`grid0_3_2` (`Locked="false"`) are the cells to
  re-walk with.
- ~~An interactive play-test of the new launch.~~ **Driven live, 2026-09-14**,
  in the `campaign-pointer` lane, mouse-only under Xvfb: `Cell Selection`
  confirming, `Team Selection`, an `--autopilot` race finishing and the
  results table all confirmed by screenshot. ~~Not fully closed: both runs
  finished 4th of eight... `Line6`/`Line7` were only seen reading the *no
  medal* state.~~ **Closed, 2026-09-21** - a worktree-local `[ai] difficulty
  = "novice"` (never committed) plus `--autopilot-skill ace` podiumed
  `grid0_3_1` 1st of 8; `Cell Selection` read `Points 3/3` / `Best Gold`
  back. See `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~Cell Help's static overlay.~~ **Done, 2026-09-21.**

## What is still open after this pass

- **The `Grid`/`Grid1` duplicate `GridController`** - still read but not
  resolved, see above.
- **No live PPSSPP capture of the ticker, the `Confirm`/`Back` legend or
  `Cell Help`'s overlay** - all three were verified against this build's own
  live behaviour under Xvfb, not against the original.
- **The ticker's own scroll speed** is chosen (`crate::anim::MARQUEE_SPEED`
  reused), not measured.
- **`Line8` and `Zone`/`Elimination`'s own saved records** - `Line8` would
  collide with `Target0`'s own row if drawn unconditionally, and neither
  mode has a raw zone/kill count anywhere in `oag_game::records::Record` to
  read at all; both are real gaps, not implemented this pass.
- **This whole screen renders every label upper-case** (`SPEED CLASS` for
  the reference's `Speed class`, and `CONFIRM` for `Confirm`) - not
  something `oag_ui::campaign` can fix on its own. The menu/campaign
  render path loads exactly one font atlas (`shell.title_font`); `"default"`/
  `"small"` are scale multipliers on that same atlas, not switches to a
  genuinely different, compact, mixed-case face, and `Draw::FacedText`'s
  own `role` is not checked against anything today - the one alternate
  atlas that ever loads is the `Title` role, not a body face. Fixing this
  needs a second, correctly-sized atlas wired into the menu stage, a change
  well outside this crate. See `docs/ui/campaign-screens.md`'s own
  "Two defects... fixed same day" section for the full trace.
- **`AI difficulty (Medium)` reads `CHANGE DIFFICULTY`** - a pre-existing
  label (`DifficultyButton`'s own `string="Change Difficulty"`, predating
  this lane) rather than the original's own template showing the current
  rung. Left as-is on the team lead's own instruction pending the atlas fix
  above, which would make the original's own wording legible in the first
  place.
