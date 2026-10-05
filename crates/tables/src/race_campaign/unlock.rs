//! `Unlock_GridPointsMet` (`0x0888ebd8`): whether a named grid has earned at
//! least its own authored `RequiredPoints`. See
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Grid0..Grid14:
//! what the gate actually evaluates".

use super::{Grid, Medal};

impl Grid {
    /// `Grid_PointsEarned` (`0x088c048c`): the sum of `Cell_MedalPoints(cell,
    /// 0xff)` over this grid's own cells - each cell's own best saved medal,
    /// answered by `medal_of` off the cell's `name`, converted to points via
    /// [`Medal::points`] and summed. `0` on a fresh profile, or whenever
    /// `medal_of` answers `None` for every cell here - the same
    /// "player-progress source is optional" reading
    /// `oag_ui_screens::campaign::GridSummary::from_grid` gives the rest of a grid's
    /// own numbers.
    #[must_use]
    pub fn points_earned(&self, medal_of: &dyn Fn(&str) -> Option<Medal>) -> u32 {
        self.cells
            .iter()
            .filter_map(|cell| medal_of(&cell.name))
            .map(Medal::points)
            .sum()
    }
}

/// `Unlock_GridPointsMet` (`0x0888ebd8`), in full: find the grid among
/// `grids` whose own [`Grid::name`] matches `target` case-insensitively -
/// `"Grid0"` in an `<Unlock Grid="Grid0"/>` row against `"grid0"` in
/// `grid_00.xml`, the same match the original applies - and pass when that
/// grid's own [`Grid::points_earned`] reaches its own
/// [`Grid::required_points`]. `target` is normally a [`Grid::unlock_grid`]
/// string, read straight off the disc's own `<Unlock Grid="...">` element -
/// never a hand-typed table, per this project's own "never invent what the
/// assets already author" rule.
///
/// Returns `false` when no grid in `grids` matches `target` - the original's
/// own `Unlock_GridName` yields nothing to look up in that case, and this
/// reimplements the predicate that consumes its result, not the lookup
/// itself failing open.
///
/// **Does not implement `asSelected`** - `Unlock_GridName`'s own reading of
/// the literal string `"asSelected"` as "whichever grid is currently
/// selected on screen," which needs a caller's own UI state this pure
/// function has no way to receive. No `<Unlock Grid="...">` row on any of
/// the sixteen shipped grid files authors that value (every one from
/// `grid1` on names the grid immediately before it), so nothing in this
/// engine has needed to resolve it yet; a caller that does should read
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s own reading of
/// `Unlock_GridName` (`0x0888ef1c`) before adding it. Confidence 88 on the
/// predicate itself, the same score that page gives it.
#[must_use]
pub fn grid_points_met(
    grids: &[Grid],
    target: &str,
    medal_of: &dyn Fn(&str) -> Option<Medal>,
) -> bool {
    grids
        .iter()
        .find(|grid| grid.name.eq_ignore_ascii_case(target))
        .is_some_and(|grid| grid.points_earned(medal_of) >= grid.required_points)
}
