//! The points law a Tournament leg scores by, and the running standings it
//! feeds - see `docs/ghidra/functions/psp-pulse-usa/tournament.md` for the
//! full decompilation this reimplements and
//! `docs/gameplay/race-modes.md#tournament` for the summary.
//!
//! **Deliberately generic over the grid size (`N`) rather than importing
//! `oag_gameplay::MAX_SHIPS`.** This crate does not depend on `oag-gameplay`
//! (see the crate's own module doc for why), so the caller - `oag_game`,
//! which does - supplies the constant itself.

/// `g_tournament_points_by_position` (`0x08ab0ba4`), read for positions
/// 1..8: **8, 6, 5, 4, 3, 2, 1, 0.** Confidence 88 - a direct table read.
/// Indexed `[position - 1]`; see [`points_for_place`] for the 1-based
/// wrapper every caller should use instead of indexing this directly.
pub const POINTS_BY_POSITION: [u32; 8] = [8, 6, 5, 4, 3, 2, 1, 0];

/// A leg's own points for one entrant: [`POINTS_BY_POSITION`] by the place it
/// holds when the leg ends, `0` for a craft that is out of the race.
///
/// **The place is the live one, not the finishing one.** A leg ends on the
/// person's own finish (`Race_BuildEndRaceResult` runs on the tick after it,
/// `docs/gameplay/after-the-finish.md`), and the rest of the field is still
/// racing then. The original scores every craft off `craft+0xbd8`, the rank
/// `Race_UpdatePositions` writes every tick (racing craft ranked by progress,
/// finishers by time - `docs/ghidra/functions/psp-pulse-usa/race-progress.md`,
/// confidence 90), so an opponent that has not crossed yet scores the place it
/// is running in. Gating on "has finished" scored every opponent `0` whenever
/// the person won.
///
/// `in_race` is `false` for the original's two zero conditions
/// (`Tournament_PointsForCraft`, `0x08826ef4`): the destroyed flag
/// (`craft+0x860 & 0x1000`) and `Ship_State` 7. The caller decides what that
/// means for its board; this engine ends a leg without a finish only when the
/// person's craft is wrecked, and its opponents respawn rather than stay out.
///
/// `place` outside `1..=8` (should not happen on an eight-slot grid) scores
/// `0` rather than panicking.
#[must_use]
pub fn points_for_place(place: u8, in_race: bool) -> u32 {
    if !in_race {
        return 0;
    }
    match place {
        1..=8 => POINTS_BY_POSITION[usize::from(place - 1)],
        _ => 0,
    }
}

/// The running per-slot points total across every leg raced so far, and the
/// standings rank it sorts into.
///
/// **Legs are stored, not accumulated in place**, so that recording the same
/// leg twice - `Session::frame`'s finish transition can re-enter a tick, the
/// same reason `RaceStage::result_saved` exists - overwrites rather than
/// double-counts. See [`Self::record_leg`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standings<const N: usize> {
    leg_points: Vec<[u32; N]>,
}

impl<const N: usize> Default for Standings<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Standings<N> {
    /// No legs raced yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            leg_points: Vec::new(),
        }
    }

    /// Records leg `leg`'s own points (0-based), overwriting whatever this
    /// leg previously held rather than adding to a running total - see the
    /// struct's own doc for why that matters.
    ///
    /// `places`/`in_race` are indexed by grid slot, in the current leg's
    /// own slot order - the same order the original's bubble sort walks
    /// before tie-breaking on it, per [`Self::ranks`].
    pub fn record_leg(&mut self, leg: usize, places: &[u8; N], in_race: &[bool; N]) {
        let points = std::array::from_fn(|slot| points_for_place(places[slot], in_race[slot]));
        match leg.cmp(&self.leg_points.len()) {
            std::cmp::Ordering::Less => self.leg_points[leg] = points,
            std::cmp::Ordering::Equal => self.leg_points.push(points),
            // A leg recorded out of order, skipping one - never observed,
            // chosen rather than a panic: the skipped leg(s) score zero
            // rather than this call failing outright.
            std::cmp::Ordering::Greater => {
                self.leg_points.resize(leg, [0; N]);
                self.leg_points.push(points);
            }
        }
    }

    /// Points scored so far, per slot, summed across every recorded leg.
    #[must_use]
    pub fn totals(&self) -> [u32; N] {
        let mut totals = [0u32; N];
        for leg in &self.leg_points {
            for (total, points) in totals.iter_mut().zip(leg.iter()) {
                *total = total.saturating_add(*points);
            }
        }
        totals
    }

    /// The standings rank, 1-based, per slot - **descending by total, ties
    /// broken by slot order**.
    ///
    /// `Race_BuildEndRaceResult`'s own sort is a bubble sort with a strict
    /// `<` swap condition, so two equal totals are never swapped and
    /// whichever led going in stays ahead - confidence 85. [`Vec::sort_by`]
    /// is documented stable, which reproduces exactly that: a tie keeps its
    /// original (slot) order rather than being resolved by anything else.
    #[must_use]
    pub fn ranks(&self) -> [u8; N] {
        let totals = self.totals();
        let mut order: [usize; N] = std::array::from_fn(|slot| slot);
        order.sort_by(|&a, &b| totals[b].cmp(&totals[a]));
        let mut ranks = [0u8; N];
        for (rank, &slot) in order.iter().enumerate() {
            ranks[slot] = u8::try_from(rank + 1).unwrap_or(u8::MAX);
        }
        ranks
    }
}

#[cfg(test)]
mod tests;
