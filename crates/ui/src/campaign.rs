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
//! **The same rule `oag_ui::picker` follows: the tree is ours, the
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
use oag_tables::race_campaign::{Cell, Grid, Medal};

use crate::frontend::Placed;
use crate::language::StringTable;
use crate::screen::{Screen, Screens};

pub mod draw;
pub mod hd;
pub mod pointer;

pub use draw::{cell_draw_list, grid_draw_list};

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
}

impl GridSummary {
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
        }
    }
}

/// `Grid Selection`: sixteen tiers, paged four at a time.
#[derive(Debug, Clone)]
pub struct GridSelection {
    grids: Vec<GridSummary>,
    index: usize,
}

impl GridSelection {
    #[must_use]
    pub fn new(grids: Vec<GridSummary>) -> Self {
        Self { grids, index: 0 }
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

    /// Up/down wrap over every grid, one at a time - the same `Picker::update`
    /// idiom `oag_ui::picker` already uses for a wrapping list. Left/right are
    /// inert: nothing in `CellMode_Definition.xml` authors a left/right arrow
    /// on this screen, unlike `Track Creation`'s livery row.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if self.grids.is_empty() {
            return out;
        }
        if input.take(Button::Down) {
            out.extend(self.step(1));
        }
        if input.take(Button::Up) {
            out.extend(self.step(-1));
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
    index: usize,
    help_open: bool,
    /// **HD only.** Which of [`Cell::targets_for_difficulty`]'s three rungs
    /// (`0` easy .. `2` hard) HD's own `DifficultyButton` widget currently
    /// shows - `docs/ui/campaign-screens.md`'s HD section: the target row is
    /// "three wide, not nine", one triple on screen at a time. Starts at `1`
    /// (medium), the rung [`Cell::gold`]/[`silver`]/[`bronze`] themselves
    /// already mean on a cell with no [`Cell::difficulty_targets`] at all -
    /// so a Pulse cell, which never authors one, draws identically whatever
    /// this holds. Never read by [`draw::grid_draw_list`]/[`cell_draw_list`],
    /// only by [`crate::campaign::hd`]'s own draw.
    difficulty: u8,
}

impl CellSelection {
    /// `cells` is one [`Grid`]'s own list, in document order. `index` starts
    /// on the first cell the grid names - not necessarily hex position
    /// `(0, 0)`, since not every grid fills that slot. The fresh-profile
    /// reading - every cell's own medal absent - see [`Self::with_medals`]
    /// for the real one.
    #[must_use]
    pub fn new(cells: Vec<Cell>) -> Self {
        Self::with_medals(cells, &|_| None)
    }

    /// The real reading: `medal_of` answers a cell's own best saved medal by
    /// its `name` - see [`GridSummary::from_grid_with_medals`]'s own doc for
    /// why this takes a closure rather than the store type itself.
    #[must_use]
    pub fn with_medals(cells: Vec<Cell>, medal_of: &dyn Fn(&str) -> Option<Medal>) -> Self {
        let medals = cells.iter().map(|cell| medal_of(&cell.name)).collect();
        Self {
            cells,
            medals,
            index: 0,
            help_open: false,
            difficulty: 1,
        }
    }

    /// **HD only.** The rung [`Self::difficulty`] currently holds, `0` easy
    /// through `2` hard - see that field's own doc.
    #[must_use]
    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    /// **HD only.** Steps [`Self::difficulty`] to the next rung, wrapping
    /// `easy -> medium -> hard -> easy`. Always available, even on a cell
    /// with no [`Cell::difficulty_targets`] - `DifficultyButton` is authored
    /// unconditionally on HD's own screen (see
    /// `docs/ui/campaign-screens.md`'s HD section), and stepping it there is
    /// inert rather than refused, since [`Cell::targets_for_difficulty`]
    /// already falls back to the one triple such a cell has regardless of
    /// which rung is asked for.
    pub fn cycle_difficulty(&mut self) {
        self.difficulty = (self.difficulty + 1) % 3;
    }

    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Cell> {
        self.cells.get(self.index)
    }

    /// The selected cell's own best saved medal, `None` on a fresh profile
    /// or a cell never raced.
    #[must_use]
    pub fn selected_medal(&self) -> Option<Medal> {
        self.medals.get(self.index).copied().flatten()
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

    fn face_scale(&self, font: &str) -> f32 {
        match font.to_ascii_lowercase().as_str() {
            "default" => self.faces.default,
            "small" => self.faces.small,
            _ => 1.0,
        }
    }
}

/// The resolved screen rect of `Medal_{x}_{y}` (or `Outline_{x}_{y}`,
/// wherever a slot draws only the empty state) - position and size, the
/// same rect [`pointer::hit`] tests against. Neither screen authors an
/// explicit width or height for a hex: its size is whichever of
/// `hex_filled.mip`/`hex_outline.mip` the sprite sheet places.
pub(super) fn hex_rect(
    screen: &Screen,
    x: usize,
    y: usize,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Option<[f32; 4]> {
    for prefix in ["Medal_", "Outline_"] {
        let name = format!("{prefix}{x}_{y}");
        if let Some(image) = screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some(name.as_str()))
            && let Some(placed) = sprites(&image.src)
        {
            let width = image.width.unwrap_or(placed.width as f32);
            let height = image.height.unwrap_or(placed.height as f32);
            return Some([image.x, image.y, width, height]);
        }
    }
    None
}
