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
- **`Cell Help`'s own text is resolved (`Main Help`/`Speed Class Help`/
  `Event Help`) but the overlay is not drawn.** Its `Viewport`/`Animation`
  scroll timeline (`LimitVerticalScroll="10"`) is the kind of reveal
  `oag_ui::screen::Screens::collect_widgets` already discards elsewhere in
  this crate; a static (non-scrolling) overlay would be the honest next
  step, not a scripted one.
- **Grid-tier locking draws every tier open.** `Unlock_GridPointsMet` needs
  `Grid_PointsEarned`, which needs a per-cell save this build does not keep
  (see below) - so there is nothing to compare a `RequiredPoints` against.
  Chosen, not measured; documented in `campaign-screens.md`.
- **`Locked`/`Lock_x_y` is deliberately unimplemented**, at the RE pass's own
  confidence 50 (`race-campaign.md`'s "what is not determined": no traced
  consumer for the byte on either a `PI_Cell` or a `PI_Grid`). No lock glyph
  draws anywhere. Do not implement one from the attribute without first
  tracing its consumer.
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
  `CellSelection::with_medals`. **`Line5`/`Line8` are unchanged** - both are
  `Cell_SavedRecord`, a saved best time/zone-count/kill-count this pass did
  not add a store for, only the medal.
- **A PPSSPP capture of the walk above**, to move every confidence score in
  `docs/ui/campaign-screens.md` from "read off the XML and the decompile"
  to "measured against a live frame", the same way `selection-screens.md`'s
  own numbers already are. Nobody has held PPSSPP for this thread yet.
- ~~An interactive play-test of the new launch.~~ **Driven live, 2026-09-14**,
  in the `campaign-pointer` lane, mouse-only under Xvfb: `Cell Selection`
  confirming, `Team Selection`, an `--autopilot` race finishing and the
  results table all confirmed by screenshot. Not fully closed: both runs
  finished 4th of eight (this build's default `[ai] difficulty` is already
  `ace`, the ceiling `--autopilot-skill` also offers), so `Line6`/`Line7`
  were only seen reading the *no medal* state live, never a podium one -
  see `docs/ui/campaign-screens.md`'s own `Open` entry for the one step
  left (a lower `[ai] difficulty` or a threshold-medal mode cell) and why
  this pass didn't take it.
- **Cell Help's static overlay.**
