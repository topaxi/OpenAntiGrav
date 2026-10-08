//! The Race Campaign's two screens: **Grid Selection** and **Cell
//! Selection**.
//!
//! Wipeout Pulse authors both in `Data\Plugins\PI001\GUI\CellMode_Definition.xml`,
//! as `<Screen type="GridSelection" name="Grid Selection">` and
//! `<Screen type="CellSelection" name="Cell Selection">`, and fills them at
//! runtime from the campaign's own 236 `PI_Cell` records
//! (`oag_tables::race_campaign`). See `docs/formats/race-setup.md`'s "The
//! Race Campaign" section, `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`
//! for the decompiled law, and `docs/ui/campaign-screens.md` for what this
//! build measures and draws.
//!
//! **The same rule `oag_ui_screens::picker` follows: the tree is ours, the
//! presentation is the disc's.** Every position, colour and texture sub-rect
//! below comes off `CellMode_Definition.xml` through [`Layout::read`], the
//! same way [`crate::picker::Layout::read`] reads `Track Creation`/`Team
//! Selection`. Nothing here invents a layout number.
//!
//! # A player-progress source is optional
//!
//! `Grid_PointsEarned`/`Grid_CountMedalsAtLeast`/`Cell_SavedMedal` read a
//! saved profile record - [`GridSummary::from_grid`]/[`CellSelection::new`]
//! take none, and draw the **fresh-profile** state: zero medals, zero points
//! earned, no saved record - the same reading `docs/formats/race-setup.md`
//! gives `Team Selection`'s `Loyalty` bar (drawn as absent, never as a
//! zero-filled bar that would read as a real zero). A caller that *does*
//! have a save - `oag_game`'s own `records::Store`, which this crate cannot
//! depend on without pulling the composition root's persistence into a
//! presentation crate - reaches the real numbers through
//! [`GridSummary::from_grid_with_medals`]/[`CellSelection::with_medals`]
//! instead, each taking a plain `Fn(&str) -> Option<Medal>` keyed on a
//! cell's own `name` rather than the store type itself.
//!
//! # The launch path is the composition root's, not this crate's
//!
//! Confirming a cell is this crate's own [`Event::Confirmed`] and no more -
//! [`CellSelection::update`] does not know what a confirm *means*, the same
//! way [`crate::picker::Picker`] does not launch a race either. Which
//! campaign modes can actually launch, resolving the cell's own track
//! against the source, and evaluating the medal a finished race earned all
//! live in `oag_game`/`crate::main::session::campaign` - see
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "how a campaign
//! event launches" and `docs/architecture/persistence.md`.
//!
//! # The hex grid: an outline always, a medal-colour swatch only where earned
//!
//! Both screens author two widgets at every hex slot - `Outline_x_y` (the
//! base hex, always drawn) and `Medal_x_y` (a colour swatch, drawn only over
//! a cell/tier that has actually earned a medal/points) - not one filled hex
//! everywhere, which is what this build drew before 2026-09-14. See
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
//! `CellSelection_PopulateGrid`/`GridSelection_PopulateTiles` pseudocode and
//! `docs/ui/campaign-screens.md`. The swatch's own tint is **chosen, not
//! measured** - `medal_argb`/`medal_tint` in [`draw`] say why.
//!
//! # `Lock_x_y` / `Lock_n_0`: drawn under a measured three-term rule
//!
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Unlock rules,
//! cell and tier" traces the glyph's own visibility in full, on both
//! screens: locked by default, cleared by the cell's/tier's own medal or
//! points, and (cell only) cleared by a hex-adjacent cell's medal too. See
//! [`CellSelection::cell_shows_lock`]/[`GridSelection::tier_shows_lock`].
//! **Whether the same byte also refuses `Confirm` is not settled by any
//! decompiled PI001 function**, but a live capture measured that it does -
//! this build reuses the identical predicate to gate a confirm at the
//! composition root (`oag_game::main::session::campaign::handle_campaign`),
//! which is `chosen, not measured` on the exact mechanism, not on the
//! glyph's own timing.
//!
//! # `Line{n} Title`: five resolved, three still blank
//!
//! `Line1`/`Line2`/`Line3`/`Line6`/`Line7 Title` resolve to real idstrings
//! (`RC_SC`/`RC_LAPS`/`RB_WEAP`/`ER_POINTS`/`IG_HUD_BEST`) - see [`draw`]'s
//! own doc on [`draw::cell_draw_list`]. `Line4`/`Line5`/`Line8 Title` stay
//! blank: `Line4` never appears in `CellSelection_PopulateDetail`'s own
//! table at all, and `Line5`/`Line8` are `Cell_SavedRecord` - the saved best
//! this build has no record for - sharing one offset (`Item OffsetX="260"
//! OffsetY="180"`, the same swap idiom `docs/formats/race-setup.md`
//! documents for `Single Player`'s `Zone`/`DifficultyNaText`).

use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::{Cell, Difficulty, Grid, Medal};

use oag_ui::language::StringTable;
use oag_ui::screen::{Screen, Screens};

pub mod draw;
pub mod flyer;
pub mod footer;
pub mod hd;
mod hex;
pub mod pointer;
pub mod selection;

pub(crate) use hex::{hex_image, hex_rect};

pub use draw::{cell_draw_list, cell_help_draw, grid_draw_list};

#[cfg(test)]
mod tests;

/// The grid `CellMode_Definition.xml` is authored in on the PSP - see
/// `crate::picker::PSP_GRID`, which this mirrors for the same PS2-scaling
/// reason.
const PSP_GRID: [f32; 2] = [480.0, 272.0];

/// How many grid tiers `Grid Selection`'s own `GridController` shows at
/// once - `MaxX="4" MaxY="1"`, confirmed by `GridSelection_Update`'s own
/// `honey` formula (`index*4+1, index*4+4, max*4+4` against 16 grids: `max`
/// only closes at 3 if it counts *pages* of four, not grids or tiers
/// directly). See `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
const GRIDS_PER_PAGE: usize = 4;

/// One selectable thing an [`Event`] can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The selection moved - a new tier on `Grid Selection`, a new cell on
    /// `Cell Selection`.
    Moved,
    /// The player confirmed. **Not wired past this crate** - see the module
    /// doc's stub section.
    Confirmed,
    /// `Cell Selection` only: `Cell Help` opened or closed (`triangle`).
    Help,
    /// The player backed out.
    Back,
    /// **HD only, ours - no Pulse cell ever authors
    /// [`Cell::difficulty_targets`], so this never fires on that title.**
    /// `CellSelection::difficulty` stepped to a different rung
    /// (`square`, or a click on HD's own `DifficultyButton` widget) - see
    /// [`CellSelection::cycle_difficulty`].
    DifficultyChanged,
}

impl Event {
    /// The navigation sound for it, or none: `Cell Help` opening has no cue
    /// recovered. See `crate::picker::Event::nav`.
    #[must_use]
    pub fn nav(self) -> Option<oag_ui::menu::nav::Nav> {
        use oag_ui::menu::nav::Nav;
        match self {
            Self::Moved => Some(Nav::UpDown),
            Self::Confirmed => Some(Nav::Accept),
            Self::Back => Some(Nav::Decline),
            Self::DifficultyChanged => Some(Nav::LeftRight),
            Self::Help => None,
        }
    }
}

/// A grid tier, reduced to what `GridSelection_Update` binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSummary {
    /// The grid's own name, e.g. `"grid0"` - what `GridSelection_Update`
    /// binds `Title` to directly, with no further formatting. Reads as
    /// literally `"grid0"` on screen, which looks like an internal id
    /// because it is one; no friendlier label is authored anywhere this
    /// build has read. See `docs/ui/campaign-screens.md`.
    pub name: String,
    /// `Grid_CellCount`: `grid.cells.len()`.
    pub cell_count: u32,
    /// `Grid_PointsPossible`: `3 * cell_count` - [`Grid::max_points`].
    pub max_points: u32,
    /// `grid->RequiredPoints`. `0` renders as `FE_NA` (`grid15`'s own value).
    pub required_points: u32,
    /// `Grid_CountMedalsAtLeast(grid, 0)`: cells on this grid whose best
    /// saved medal is gold. `0` on a fresh profile, or whenever
    /// [`Self::from_grid`] built this without a progress source - see the
    /// module doc.
    pub gold_medals: u32,
    /// `Grid_PointsEarned`: the sum of `Medal::points()` over every cell's
    /// own best saved medal on this grid. `0` on the same terms as
    /// [`Self::gold_medals`].
    pub points_earned: u32,
    /// `PI_Grid`'s own `Locked` byte (`+0xa0`) - the tier's *static default*,
    /// not the final answer: `GridSelection_PopulateTiles` also clears the
    /// glyph once this grid has scored any points of its own, or once the
    /// previous tile's own points meet its own `RequiredPoints` - see
    /// [`GridSelection::tier_shows_lock`], `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
    /// "Unlock rules, cell and tier".
    pub locked: bool,
    /// `FlyerName` (`"01_uplift"`): which `Data/FE/Flyers/<name>/` this grid's
    /// card, back and logo are read from. HD/Fury only - see
    /// [`flyer`]. `None` on every Pulse grid.
    pub flyer_name: Option<String>,
}

impl GridSummary {
    /// A tier with nothing in it, for a screen asked to draw before any grid
    /// has loaded - a stand-in for the absence, not a reading.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            cell_count: 0,
            max_points: 0,
            required_points: 0,
            gold_medals: 0,
            points_earned: 0,
            locked: false,
            flyer_name: None,
        }
    }

    /// The fresh-profile reading: every earned figure zero, because there is
    /// no progress to report. See the module doc.
    #[must_use]
    pub fn from_grid(grid: &Grid) -> Self {
        Self::from_grid_with_medals(grid, &|_| None)
    }

    /// The real reading: `medal_of` answers a cell's own best saved medal by
    /// its `name`, from whatever store the caller holds - see the module
    /// doc for why this crate takes a closure rather than the store type
    /// itself.
    #[must_use]
    pub fn from_grid_with_medals(grid: &Grid, medal_of: &dyn Fn(&str) -> Option<Medal>) -> Self {
        let mut gold_medals = 0;
        let mut points_earned = 0;
        for cell in &grid.cells {
            if let Some(medal) = medal_of(&cell.name) {
                points_earned += medal.points();
                if medal == Medal::Gold {
                    gold_medals += 1;
                }
            }
        }
        Self {
            name: grid.name.clone(),
            cell_count: u32::try_from(grid.cells.len()).unwrap_or(u32::MAX),
            max_points: grid.max_points(),
            required_points: grid.required_points,
            gold_medals,
            points_earned,
            locked: grid.locked,
            flyer_name: grid.flyer_name.clone(),
        }
    }
}

/// `Grid Selection`: sixteen tiers, paged four at a time.
#[derive(Debug, Clone)]
pub struct GridSelection {
    grids: Vec<GridSummary>,
    index: usize,
    /// How many tiers a page turn (`Up`/`Down`) moves by - [`GRIDS_PER_PAGE`]
    /// for Pulse's own four-hex page, `1` for HD/Fury's one-tier-at-a-time
    /// flyer pager (`with_per_page`). [`Self::step`] (`Left`/`Right`, the
    /// paging arrows) always moves by one tier regardless - see
    /// [`Self::update`]'s own doc for the measurement this splits on.
    per_page: usize,
}

impl GridSelection {
    #[must_use]
    pub fn new(grids: Vec<GridSummary>) -> Self {
        Self {
            grids,
            index: 0,
            per_page: GRIDS_PER_PAGE,
        }
    }

    /// Overrides [`Self::per_page`] - HD/Fury's own `Grid Selection` calls
    /// this with `1`, since its own `Grid Selection` pages one flyer at a
    /// time rather than Pulse's four-hex page. See
    /// `docs/ui/campaign-screens.md`'s "`Grid Selection` is not a hex grid
    /// of tiers - it is a flyer pager".
    #[must_use]
    pub fn with_per_page(mut self, per_page: usize) -> Self {
        self.per_page = per_page.max(1);
        self
    }

    #[must_use]
    pub fn grids(&self) -> &[GridSummary] {
        &self.grids
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Moves the selection directly to `index`, clamped to the last grid -
    /// how `Cell Selection`'s own `Back` lands `Grid Selection` on the tier
    /// it was opened from, rather than resetting to the first.
    pub fn set_index(&mut self, index: usize) {
        self.index = index.min(self.grids.len().saturating_sub(1));
    }

    #[must_use]
    pub fn selected(&self) -> Option<&GridSummary> {
        self.grids.get(self.index)
    }

    /// Which page of [`GRIDS_PER_PAGE`] is on screen.
    #[must_use]
    pub fn page(&self) -> usize {
        self.index / GRIDS_PER_PAGE
    }

    /// The selected tier's own hex slot within its page, `0..GRIDS_PER_PAGE`.
    #[must_use]
    pub fn slot(&self) -> usize {
        self.index % GRIDS_PER_PAGE
    }

    /// The `honey` counter's own formula: `"{first}-{last} / {total}"`, where
    /// `first`/`last` are the current page's 1-based bounds. See
    /// `GridSelection_Update`'s `"%d-%d / %d"` binding.
    #[must_use]
    pub fn counter(&self) -> String {
        let page = self.page();
        let first = page * GRIDS_PER_PAGE + 1;
        let last = ((page + 1) * GRIDS_PER_PAGE).min(self.grids.len().max(1));
        format!("{first}-{last} / {}", self.grids.len())
    }

    /// Whether the selected tier still shows its `Lock_n_0` glyph -
    /// `GridSelection_PopulateTiles`'s own three-term rule: the tier's own
    /// `Locked` byte is set, it has scored no points of its own, and the
    /// immediately preceding tile's own points have not met its own
    /// `RequiredPoints` either. See
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Unlock
    /// rules, cell and tier".
    ///
    /// **Also what gates `Confirm`** - `docs/ui/campaign-screens.md`
    /// measured live that confirming a locked tier does nothing, but no
    /// PI001 function traced ever refuses the transition on this byte, so
    /// reusing the glyph's own predicate for the refusal is **chosen, not
    /// measured**: the real gate lives in an un-decompiled function
    /// upstream of `GridSelection_CommitSelection`. See
    /// `crate::main::session::campaign::handle_campaign` in `oag_game`,
    /// which is where this is actually applied.
    #[must_use]
    pub fn selected_is_locked(&self) -> bool {
        self.tier_shows_lock(self.index)
    }

    /// [`Self::selected_is_locked`], for any tier by flat index - what
    /// [`draw::grid_draw_list`] calls per tile rather than only the
    /// selected one.
    #[must_use]
    pub(crate) fn tier_shows_lock(&self, index: usize) -> bool {
        let Some(grid) = self.grids.get(index) else {
            return false;
        };
        if !grid.locked || grid.points_earned > 0 {
            return false;
        }
        let Some(previous) = index.checked_sub(1).and_then(|i| self.grids.get(i)) else {
            return true;
        };
        previous.points_earned < previous.required_points
    }

    /// **Superseded, 2026-09-25.** Up/down and left/right were previously
    /// read backwards: this used to wrap the flat index by one tier per
    /// `Up`/`Down` press and leave `Left`/`Right` inert, on the reasoning
    /// that `CellMode_Definition.xml` authors no left/right arrow image on
    /// this screen. That reasoning does not follow - the screen's own
    /// `GridController name="Grid" MaxX="4" MaxY="1"` is a **row** of four
    /// hexes, and a maintainer playing this build reported left/right dead
    /// on `Grid Selection`.
    ///
    /// Measured live against PPSSPP (`pulse-psp-usa.chd`, 2026-09-25, Xvfb,
    /// the existing zero-medal profile - see `docs/ui/campaign-screens.md`'s
    /// "Measured against PPSSPP, 2026-09-25" for the full screenshot walk):
    ///
    /// - `Down` from `grid0` landed on `grid4`'s own `"GRID 5"` - a full
    ///   page (`index += 4`), not one tile.
    /// - `Right` from `grid0` landed on `grid1`'s own `"GRID 2"` - one tile
    ///   within the page (`index += 1`).
    /// - `Right` pressed three more times from there landed on `grid3`'s
    ///   own `"GRID 4"`, the page's own last slot; a fifth `Right` **stayed
    ///   on `"GRID 4"`** rather than crossing into `grid4`'s `"GRID 5"` -
    ///   `Left`/`Right` clamp at the current page's own ends, they do not
    ///   wrap into the next/previous page.
    /// - `Left` from `grid0` (the deck's own first slot) also stayed put.
    /// - `Down` three more times from `grid0` reached `grid12`'s own
    ///   `"PHANTOM GRID 1"` (the last page, `"13-16 / 16"`); one more
    ///   `Down` **stayed there** rather than wrapping to `"GRID 1"` -
    ///   `Up`/`Down` clamp at the deck's own ends too. Symmetrically, `Up`
    ///   from `grid0` also stayed on `"GRID 1"`.
    /// - `Right` once from `grid0` then `Down` once landed on `grid5`'s own
    ///   `"GRID 6"`, not `grid4`'s `"GRID 5"` - a page turn keeps the same
    ///   slot within the new page, it does not reset to the page's own
    ///   first slot.
    ///
    /// So: no direction ever wraps on this screen. [`Self::page_step`]/
    /// [`Self::tile_step`] (in `campaign::pointer`, alongside the pointer
    /// code that shares them with the paging-arrow clicks) carry the
    /// clamped arithmetic; this method only decides which button maps to
    /// which. Confidence 90 - six separate presses, each read
    /// digit-for-digit off the honey counter and the title, in one session.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if self.grids.is_empty() {
            return out;
        }
        if input.take(Button::Down) {
            out.extend(self.page_step(1));
        }
        if input.take(Button::Up) {
            out.extend(self.page_step(-1));
        }
        if input.take(Button::Right) {
            out.extend(self.tile_step(1));
        }
        if input.take(Button::Left) {
            out.extend(self.tile_step(-1));
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }
}

/// `Cell Selection`: the 32-position staggered hex grid, filled to whichever
/// of the 8-16 cells the selected [`Grid`] actually carries.
#[derive(Debug, Clone)]
pub struct CellSelection {
    cells: Vec<Cell>,
    /// Parallel to [`Self::cells`] - `medals[i]` is `cells[i]`'s own best
    /// saved medal. All `None` when built through [`Self::new`]. See the
    /// module doc's "a player-progress source is optional".
    medals: Vec<Option<Medal>>,
    /// Parallel to [`Self::cells`] - `records[i]` is `cells[i]`'s own saved
    /// best, in centiseconds (`Cell_SavedRecord`, drawn as `Line5`). All
    /// `None` when built through [`Self::new`]/[`Self::with_medals`] - see
    /// [`Self::with_medals_and_records`]. **Time Trial/Speed Lap only**
    /// today: the general per-track/mode/class record store
    /// (`oag_game::records::Store`) keeps a lap/total tick count for every
    /// mode, but Zone's zone count and Elimination's kill count have no
    /// field there at all, so a caller building this has nothing honest to
    /// pass for either - see `crate::main::campaign_stage::CampaignStage`'s
    /// own construction of the closure.
    records: Vec<Option<i64>>,
    /// Parallel to [`Self::cells`] - `difficulties[i]` is `cells[i]`'s own
    /// medal's earned [`Difficulty`], only ever set via
    /// [`Self::with_difficulty`] - `None` for any cell whose saved medal
    /// predates `oag_game::records::CampaignRecord::best_difficulty`
    /// existing at all (every pre-2026-09-28 row, on any title). See
    /// [`Self::difficulty_at`]/[`Self::selected_difficulty`]'s own docs for
    /// how a caller reads this.
    difficulties: Vec<Option<Difficulty>>,
    index: usize,
    help_open: bool,
    /// The rung the `DifficultyButton` widget's square-button prompt
    /// currently shows (built by [`draw::cell_draw_list`]), and what
    /// `crate::main::session::campaign` (`oag_game`) records alongside a
    /// confirmed cell's own medal on every title - see
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
    /// `DifficultyRC` persisted rung" section. On HD it also selects which
    /// of [`Cell::targets_for_difficulty`]'s three rungs the target row
    /// shows (`docs/ui/campaign-screens.md`'s HD section: "three wide, not
    /// nine", one triple on screen at a time); on Pulse, whose cells never
    /// author [`Cell::difficulty_targets`], the target row draws identically
    /// whatever this holds. Starts at [`Difficulty::Medium`] - Pulse's own
    /// fresh-profile default (`Profile_SetDifficultyRC(profile,
    /// 1)`, the same section above) - unless overridden by
    /// [`Self::with_default_difficulty`]: **HD/Fury's own fresh-profile
    /// default is `Difficulty::Easy`, not `Medium`**, measured directly on a
    /// genuinely fresh RPCS3 profile (`~/.config/rpcs3/dev_hdd0/home/00000001/savedata/`
    /// empty before boot, not merely unread) - `grid8_3_1` (`Cell Selection`'s
    /// own default cell) reads `AI DIFFICULTY (NOVICE)` on arrival, no
    /// `DifficultyButton` press. Confidence 90: one settled `--nav-shots`
    /// frame (no comb-artifact risk, no `Square` press to mis-pair), one
    /// title, one campaign (`Fury`) - `docs/reverse-engineering/rpcs3-capture.md`'s
    /// "Cell Selection: `DifficultyButton` toggle" section carries the
    /// capture. `Profile_GetDifficultyRC`'s own fallback global
    /// (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`) was not
    /// traced to its initializer, so this is the RPCS3 reading, not an
    /// independent Ghidra one. Read by [`crate::campaign::hd`]'s own draw and
    /// by [`draw::cell_draw_list`]'s `DifficultyButton` text, never by
    /// [`draw::grid_draw_list`].
    difficulty: Difficulty,
}

impl CellSelection {
    /// `cells` is one [`Grid`]'s own list, in document order. `index` starts
    /// on the first cell the grid's own `Locked` byte parses as literal
    /// `false` - **not** the first cell the grid names, and **not**
    /// `Cell::locked`'s own "absent defaults locked" reading either, which
    /// is a display rule for the lock glyph, not this. `CellSelection_OnEnter`
    /// (`0x088d59c4`, decompiled in full 2026-09-28), on a fresh entry with
    /// no cell already selected: `for cell in cells (document order): if
    /// cell->flags & 8 == 0 && cell->Locked (+0xb9) == 0: select it, break` -
    /// a plain linear scan for the first cell whose own byte is literally
    /// `false`, `None`/absent and `true` both skipped alike. Measured live
    /// against PPSSPP, `pulse-psp-usa.chd`, fresh profile: `grid0`'s own
    /// default cursor is `grid0_3_1` (`Locked="false"`, fourth in document
    /// order), never `grid0_2_1` (no `Locked` attribute, first in document
    /// order) - `docs/ui/campaign-screens.md`'s 2026-09-14/2026-09-28
    /// sections. `cell->flags & 8` was not chased this pass (no consumer
    /// this project has read names it); every authored cell this pass
    /// checked reads `0` there, so it has not yet excluded anything real.
    /// Falls back to the first cell in document order when no cell's own
    /// byte is literal `false` (every grid this project has read authors at
    /// least one, but the decompile itself leaves the selection at whatever
    /// it already was - `0`/none - in that case, which this reimplementation
    /// cannot leave a screen showing) - **chosen, not measured**, for a case
    /// not yet observed on a real grid. The fresh-profile reading - every
    /// cell's own medal absent - see [`Self::with_medals`] for the real one.
    #[must_use]
    pub fn new(cells: Vec<Cell>) -> Self {
        Self::with_medals(cells, &|_| None)
    }

    /// The real reading: `medal_of` answers a cell's own best saved medal by
    /// its `name` - see [`GridSummary::from_grid_with_medals`]'s own doc for
    /// why this takes a closure rather than the store type itself.
    #[must_use]
    pub fn with_medals(cells: Vec<Cell>, medal_of: &dyn Fn(&str) -> Option<Medal>) -> Self {
        Self::with_medals_and_records(cells, medal_of, &|_| None)
    }

    /// [`Self::with_medals`], plus `record_of` - a cell's own saved best, in
    /// centiseconds, by `name`. See [`Self::records`]'s own doc for why this
    /// is `None` for most cells even on a real save.
    #[must_use]
    pub fn with_medals_and_records(
        cells: Vec<Cell>,
        medal_of: &dyn Fn(&str) -> Option<Medal>,
        record_of: &dyn Fn(&str) -> Option<i64>,
    ) -> Self {
        let medals = cells.iter().map(|cell| medal_of(&cell.name)).collect();
        let records = cells.iter().map(|cell| record_of(&cell.name)).collect();
        let index = cells
            .iter()
            .position(|cell| cell.locked == Some(false))
            .unwrap_or(0);
        let difficulties = vec![None; cells.len()];
        Self {
            cells,
            medals,
            records,
            index,
            difficulties,
            help_open: false,
            difficulty: Difficulty::Medium,
        }
    }

    /// Attaches each cell's own earned medal difficulty alongside
    /// [`Self::medals`] - see [`Self::difficulty_at`]/[`Self::selected_difficulty`].
    /// A separate builder rather than a fourth closure on
    /// [`Self::with_medals_and_records`]: [`Self::difficulty`] itself (a
    /// different, browsed-not-earned difficulty) is a plain field rather
    /// than threaded through the constructor either, for the same reason.
    #[must_use]
    pub fn with_difficulty(mut self, difficulty_of: &dyn Fn(&str) -> Option<Difficulty>) -> Self {
        self.difficulties = self
            .cells
            .iter()
            .map(|cell| difficulty_of(&cell.name))
            .collect();
        self
    }

    /// Overrides [`Self::difficulty`]'s own starting rung - the browsed one
    /// the `DifficultyButton` prompt shows on arrival, not a per-cell earned
    /// one. A separate builder rather than a constructor parameter for the
    /// same reason [`Self::with_difficulty`] is: this field's own doc names
    /// the default ([`Difficulty::Medium`]) as Pulse's, measured; a caller on
    /// HD/Fury needs this to reach that title's own different measured
    /// default ([`Difficulty::Easy`]) without every other caller (every
    /// existing Pulse one, and every test not about this specifically)
    /// having to name a rung it does not care about.
    #[must_use]
    pub fn with_default_difficulty(mut self, difficulty: Difficulty) -> Self {
        self.difficulty = difficulty;
        self
    }

    /// The rung [`Self::difficulty`] currently holds - see that field's own
    /// doc.
    #[must_use]
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// Puts [`Self::difficulty`] back on `difficulty` - a screen reopened on
    /// the rung the player had left it at, see
    /// `oag_game`'s `Session::reopen_cell_selection`.
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
    }

    /// Steps [`Self::difficulty`] to the next rung, wrapping `easy -> medium
    /// -> hard -> easy` ([`Difficulty::next`]) - `CellSelection_Update`'s own
    /// `(+0xf0 + 1) % 3` on a `Square` press, on both titles
    /// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
    /// `DifficultyRC` persisted rung" section). Always available, even on a
    /// cell with no [`Cell::difficulty_targets`] - `DifficultyButton` is
    /// authored unconditionally on both titles' own screens, and stepping it
    /// there is inert rather than refused on HD, since
    /// [`Cell::targets_for_difficulty`] already falls back to the one triple
    /// such a cell has regardless of which rung is asked for.
    pub fn cycle_difficulty(&mut self) {
        self.difficulty = self.difficulty.next();
    }

    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Cell> {
        self.cells.get(self.index)
    }

    /// Moves the selection to the cell named `name`, if this grid carries
    /// one - `EndRace Menu`'s own `RETURN TO GRID`, which lands back on the
    /// exact cell the race launched from rather than resetting to the first.
    /// A no-op, silently, if no cell here carries that name any more (a DLC
    /// pack unmounted between launch and return, say) - the same "stay
    /// where the screen already was" choice
    /// [`GridSelection::set_index`]'s own out-of-range clamp makes.
    pub fn select_by_name(&mut self, name: &str) {
        if let Some(index) = self.cells.iter().position(|cell| cell.name == name) {
            self.index = index;
        }
    }

    /// The selected cell's own best saved medal, `None` on a fresh profile
    /// or a cell never raced.
    #[must_use]
    pub fn selected_medal(&self) -> Option<Medal> {
        self.medals.get(self.index).copied().flatten()
    }

    /// The selected cell's own saved record, in centiseconds - see
    /// [`Self::records`]'s own doc.
    #[must_use]
    pub fn selected_record(&self) -> Option<i64> {
        self.records.get(self.index).copied().flatten()
    }

    /// [`Self::selected_medal`]'s own earned [`Difficulty`] - `Line7`'s
    /// suffix (`CellSelection_PopulateDetail`, `"Gold (Medium)"` rather than
    /// a bare medal word). `None` on a cell with no medal, and on any row
    /// [`Self::with_difficulty`]'s own closure answered `None` for. Not
    /// title-gated: `oag_game::records::Store::record_campaign` now writes
    /// [`CampaignRecord::best_difficulty`] on every title, matching
    /// `Race_RecordResult`'s own unconditional write. See
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
    /// `DifficultyRC` persisted rung" section.
    #[must_use]
    pub fn selected_difficulty(&self) -> Option<Difficulty> {
        self.difficulties.get(self.index).copied().flatten()
    }

    #[must_use]
    pub fn help_open(&self) -> bool {
        self.help_open
    }

    /// A cell's own best saved medal, by hex position rather than list
    /// index - what the six-neighbour lock check and [`draw::cell_draw_list`]'s
    /// own `Medal_x_y` gate both need. `None` for an unoccupied slot as much
    /// as for an occupied one with no medal; the two are not distinguished
    /// here, the same way `Cell_BestMedal` on a slot with no `PI_Cell` at
    /// all is not this function's problem to separate from one that has a
    /// cell but no medal.
    #[must_use]
    pub fn medal_at(&self, x: u32, y: u32) -> Option<Medal> {
        self.cells
            .iter()
            .zip(&self.medals)
            .find(|(cell, _)| cell.grid_coords() == Some((x, y)))
            .and_then(|(_, medal)| *medal)
    }

    /// [`Self::medal_at`]'s own cell's earned [`Difficulty`], by hex
    /// position - `None` on any cell [`Self::with_difficulty`]'s own closure
    /// answered `None` for, whether or not it has a medal at all. Consumed
    /// only by [`crate::campaign::hd`]'s own medal-icon draw today (a caller
    /// drawing a medal icon with no difficulty to key on picks its own
    /// fallback - see `oag_ui_screens::campaign::hd::hd_medal_frame`'s own doc for
    /// what it uses); Pulse's own equivalent is
    /// [`Self::selected_difficulty`], keyed by the current selection rather
    /// than a hex position, since Pulse never draws more than one cell's own
    /// difficulty suffix at a time (`Line7`, not a hex icon).
    #[must_use]
    pub fn difficulty_at(&self, x: u32, y: u32) -> Option<Difficulty> {
        self.cells
            .iter()
            .zip(&self.difficulties)
            .find(|(cell, _)| cell.grid_coords() == Some((x, y)))
            .and_then(|(_, difficulty)| *difficulty)
    }

    /// Whether the selected cell still shows its `Lock_x_y` glyph -
    /// `CellSelection_PopulateGrid`'s own three-term rule. See
    /// [`Self::cell_shows_lock`] and, for why this is also what gates
    /// `Confirm`, [`GridSelection::selected_is_locked`]'s own doc (the
    /// identical reasoning, applied to the other screen).
    #[must_use]
    pub fn selected_is_locked(&self) -> bool {
        self.selected()
            .is_some_and(|cell| self.cell_shows_lock_cell(cell))
    }

    /// [`Self::selected_is_locked`], for any occupied `(x, y)` rather than
    /// only the selected one - what [`draw::cell_draw_list`] calls per hex.
    /// `false` for an unoccupied slot, which has no `Cell` to be locked.
    #[must_use]
    pub(crate) fn cell_shows_lock(&self, x: u32, y: u32) -> bool {
        self.cells
            .iter()
            .find(|cell| cell.grid_coords() == Some((x, y)))
            .is_some_and(|cell| self.cell_shows_lock_cell(cell))
    }

    /// `cell.Locked (+0xb9) != 0 and Cell_BestMedal(cell) == 0xff and no
    /// hex-adjacent cell has a medal either` -
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Unlock
    /// rules, cell and tier". **The absent-`Locked`-attribute default is
    /// `true`**, confidence 72 on that page: `PI_Cell_ParseElement` never
    /// writes a default onto the byte itself, but every one of `grid0`'s six
    /// glyphed cells is exactly the six that author no `Locked` attribute at
    /// all, on a live capture with zero medals anywhere to trigger the
    /// neighbour clause instead.
    fn cell_shows_lock_cell(&self, cell: &Cell) -> bool {
        if !cell.locked.unwrap_or(true) {
            return false;
        }
        let Some((x, y)) = cell.grid_coords() else {
            return true;
        };
        if self.medal_at(x, y).is_some() {
            return false;
        }
        !neighbour_offsets(x)
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = i64::from(x) + dx;
                let ny = i64::from(y) + dy;
                (nx >= 0 && ny >= 0).then(|| self.medal_at(nx as u32, ny as u32))
            })
            .any(|medal| medal.is_some())
    }

    /// Moves to the cell nearest `(dx, dy)` away from the current one, in the
    /// hex grid's own coordinate space (`Cell::grid_coords`, not screen
    /// pixels). **Chosen, not measured**: the original's own directional
    /// adjacency on a staggered hex grid is unread - see
    /// `docs/ui/campaign-screens.md`. This picks whichever other cell's own
    /// `(x, y)` has the most positive dot product with `(dx, dy)` and, among
    /// ties, the smallest perpendicular offset - which reduces to "the
    /// nearest neighbour in roughly that direction" without needing the
    /// staggered geometry spelled out twice.
    fn step(&mut self, dx: f32, dy: f32) -> bool {
        let Some(current) = self.selected().and_then(Cell::grid_coords) else {
            return false;
        };
        let (cx, cy) = (
            f32::from(u16::try_from(current.0).unwrap_or(u16::MAX)),
            f32::from(u16::try_from(current.1).unwrap_or(u16::MAX)),
        );
        let mut best: Option<(usize, f32, f32)> = None;
        for (index, cell) in self.cells.iter().enumerate() {
            if index == self.index {
                continue;
            }
            let Some((x, y)) = cell.grid_coords() else {
                continue;
            };
            let (ex, ey) = (
                f32::from(u16::try_from(x).unwrap_or(u16::MAX)) - cx,
                f32::from(u16::try_from(y).unwrap_or(u16::MAX)) - cy,
            );
            let along = ex * dx + ey * dy;
            if along <= 0.0 {
                continue;
            }
            let perpendicular = (ex * dy - ey * dx).abs();
            let better = match &best {
                None => true,
                Some((_, best_perp, best_along)) => {
                    perpendicular < *best_perp
                        || ((perpendicular - best_perp).abs() < f32::EPSILON && along < *best_along)
                }
            };
            if better {
                best = Some((index, perpendicular, along));
            }
        }
        if let Some((index, _, _)) = best {
            self.index = index;
            true
        } else {
            false
        }
    }

    /// The four directions and the three buttons this screen answers to.
    /// Movement is inert while [`Self::help_open`] - the disc's own `Watch`
    /// element redirects every directional press straight back to `Cell
    /// Help` while it is the present screen, which this reproduces as "do
    /// nothing" rather than modelling a redirect loop.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Triangle) {
            self.help_open = !self.help_open;
            out.push(Event::Help);
            return out;
        }
        if self.help_open {
            if input.take(Button::Cross) || input.take(Button::Circle) {
                self.help_open = false;
                out.push(Event::Help);
            }
            input.take(Button::Up);
            input.take(Button::Down);
            input.take(Button::Left);
            input.take(Button::Right);
            return out;
        }
        if input.take(Button::Down) && self.step(0.0, 1.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Up) && self.step(0.0, -1.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Right) && self.step(1.0, 0.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Left) && self.step(-1.0, 0.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        // `Square` is unbound on this screen in this build - nothing else
        // in `crate::campaign`/`oag_game` consumes it here, so stepping
        // `Self::difficulty` on every press is harmless on Pulse (drawn by
        // [`draw::cell_draw_list`], which never reads this field) and is
        // HD's own `DifficultyButton` toggle. The original's own PPSSPP
        // binding of `Square` to a *different* control (an AI-difficulty
        // setting, not this field - `docs/ui/campaign-screens.md`'s "AI
        // difficulty (square) cycles" finding) is not reproduced by this;
        // this build's AI difficulty is a settings-file value, not a
        // per-screen toggle.
        if input.take(Button::Square) {
            self.cycle_difficulty();
            out.push(Event::DifficultyChanged);
        }
        out
    }
}

/// `g_anCellNeighbourOffsets` (`0x08ab1e68`), the two six-entry `(dx, dy)`
/// tables `CellSelection_PopulateGrid` picks between on `x`'s own parity -
/// read directly off the raw bytes, not inferred - see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Unlock rules,
/// cell and tier". The four orthogonal offsets are identical either way;
/// only the two diagonals flip between up-left/up-right (even columns) and
/// down-left/down-right (odd columns), the offset-column convention this
/// screen's own staggered hex grid needs.
fn neighbour_offsets(x: u32) -> [(i64, i64); 6] {
    if x.is_multiple_of(2) {
        [(-1, 0), (0, -1), (1, 0), (0, 1), (-1, -1), (1, -1)]
    } else {
        [(-1, 0), (0, -1), (1, 0), (0, 1), (-1, 1), (1, 1)]
    }
}

/// The disc's own layout for one campaign screen, with every fixed string
/// already resolved.
#[derive(Debug, Clone)]
pub struct Layout {
    pub screen: Screen,
    /// How much larger this screen's grid is than the PSP's - see
    /// `crate::picker::Layout::scale`.
    pub scale: [f32; 2],
    /// The `default`/`small` faces' own scale against the loaded `menu`
    /// face - see `crate::picker::FaceScales`, which this reuses rather than
    /// duplicating: both screens are authored in the same three faces
    /// `Selection_Definition.xml` is.
    pub faces: crate::picker::FaceScales,
}

impl Layout {
    /// Reads `Grid Selection` or `Cell Selection` off the parsed
    /// `CellMode_Definition.xml`, resolving every `idstring` through
    /// `strings` - the same second pass `crate::picker::Layout::read` makes,
    /// so a fixed label like `Medals Title` (`RC_GM`) or `Target0 Title`
    /// (`IG_HUD_TARGET`) already carries its text by the time a draw
    /// function's generic `text.string.clone()` fallback reaches it.
    /// `None` when the screen is not in `screens` at all.
    ///
    /// [`PSP_GRID`] is the grid the file is authored in - correct for Pulse.
    /// A title whose own `CellMode_Definition.xml` is authored at a
    /// different resolution (Wipeout HD's is 1920x1080, not the PSP's
    /// 480x272) needs [`Self::read_authored`] instead, or every position on
    /// the screen scales by the wrong factor.
    #[must_use]
    pub fn read(
        screens: &Screens,
        name: &str,
        strings: &StringTable,
        faces: crate::picker::FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        Self::read_authored(screens, name, strings, faces, grid, PSP_GRID)
    }

    /// [`Self::read`], with the file's own authored grid stated explicitly
    /// rather than assumed to be the PSP's - see that function's own doc.
    #[must_use]
    pub fn read_authored(
        screens: &Screens,
        name: &str,
        strings: &StringTable,
        faces: crate::picker::FaceScales,
        grid: [f32; 2],
        authored: [f32; 2],
    ) -> Option<Self> {
        let mut screen = screens.by_name(name)?.clone();
        for text in &mut screen.texts {
            if let Some(id) = text.idstring.as_deref()
                && let Some(resolved) = strings.get(id)
            {
                text.string = Some(resolved.to_string());
            }
        }
        let scale = [grid[0] / authored[0], grid[1] / authored[1]];
        Some(Self {
            screen,
            scale,
            faces,
        })
    }

    /// `"default"` draws `1.0`, not `self.faces.default` (a ratio against
    /// the `menu` role, 13/22): `text_draw` routes a `"default"`-labelled
    /// widget through its own `Default`-role atlas at that face's own
    /// native size (`crates/game/src/boot/fonts.rs`'s `face_atlas_slot`),
    /// the same fix `oag_ui_screens::campaign::footer`'s own `face_scale` already
    /// carries for the footer's prompts - see that function's doc.
    /// Applying `faces.default` on top double-scaled it down, since the
    /// atlas is already native-sized: `Cell Selection`'s `"Speed class"`
    /// row measured 11px tall (960x544, cyan-threshold scan) against a
    /// fresh PPSSPP capture's 18px for the same widget - confirmed fixed
    /// by the same measurement, pixel-identical to the reference
    /// (`172..189`, 18px, both) after this change -
    /// `docs/ui/campaign-screens.md`'s 2026-09-28 section has the full
    /// capture comparison. `"small"` still needs the ratio: no real
    /// `Small`-role atlas is ever loaded, so that text still fakes its
    /// size out of the `menu` atlas's own glyphs.
    fn face_scale(&self, font: &str) -> f32 {
        match font.to_ascii_lowercase().as_str() {
            "default" => 1.0,
            "small" => self.faces.small,
            _ => 1.0,
        }
    }
}
