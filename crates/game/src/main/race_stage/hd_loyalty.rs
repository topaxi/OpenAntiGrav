//! Wipeout HD/Fury's loyalty award for a finished race: the law is
//! `oag_hd::loyalty::award` (`docs/ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md`),
//! and this is the part that maps this project's race onto its inputs.

use oag_hd::loyalty::{self, Inputs, Tally, Tier, mode};
use oag_tables::race_campaign::Difficulty;

/// The id `g_GameState+0xe0` holds for one of this project's modes. The named
/// ones (`SPArcade` 3, `SPTournament` 4, `SPTimeTrial` 5/10, `SPElimination`
/// 8) are `mode-manager.md`'s, 88; Zone `6` is read off two unrelated
/// functions, 75. **`Head2Head` is chosen, not measured**: HD ships no such
/// mode, and a duel against one craft is closest to a single race.
#[must_use]
pub(crate) const fn hd_mode_id(mode: oag_race::Mode) -> u32 {
    match mode {
        oag_race::Mode::SingleRace | oag_race::Mode::Head2Head => mode::SINGLE_RACE,
        oag_race::Mode::Tournament => mode::TOURNAMENT,
        oag_race::Mode::TimeTrial => mode::TIME_TRIAL,
        oag_race::Mode::SpeedLap => mode::SPEED_LAP,
        oag_race::Mode::Zone => mode::ZONE,
        oag_race::Mode::Eliminator => mode::ELIMINATION,
    }
}

/// What a race feeds [`hd_award`]: the three tallies this project keeps and
/// the rung the cell was launched on. `perfect_laps` and `perfect_zones` are
/// not among them - nothing here counts either, so they read `0` and the
/// award is short by what a perfect lap or zone would have paid.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HdRace {
    pub(crate) mode: oag_race::Mode,
    pub(crate) laps: u32,
    pub(crate) kills: u32,
    pub(crate) zones: u32,
    /// The campaign rung the cell launched on. `None` (a race with no cell)
    /// is **chosen, not measured**: `Easy`, the rung HD's own campaign model
    /// opens on in this build, because the executable has no "no rung" - its
    /// `g_GameState+0xdc` always holds one.
    pub(crate) difficulty: Option<Difficulty>,
}

/// This race's HD award.
#[must_use]
pub(crate) fn hd_award(race: HdRace) -> u32 {
    let tier = match race.difficulty.unwrap_or(Difficulty::Easy) {
        Difficulty::Easy => Tier::Easy,
        Difficulty::Medium => Tier::Medium,
        Difficulty::Hard => Tier::Hard,
    };
    loyalty::award(Inputs {
        mode: hd_mode_id(race.mode),
        tally: Tally {
            laps: race.laps,
            perfect_laps: 0,
            zones: race.zones,
            perfect_zones: 0,
            kills: race.kills,
        },
        tier,
        ai_present: race.mode.has_opponents(),
    })
}

#[cfg(test)]
#[path = "hd_loyalty/tests.rs"]
mod tests;
