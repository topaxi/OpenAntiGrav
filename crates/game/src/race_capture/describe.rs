//! What a headless capture reads off a finished race for the scoreboard.

use super::*;

/// What the finished race records, for the scoreboard's personal-best lines.
///
/// The same conversion `RaceStage::observation` makes for the real session, duplicated rather
/// than shared: a capture has no `RaceStage` to call it on, and `crate::records` is deliberately
/// free of a dependency on `Race` itself - see that module's own doc. No campaign cell is
/// selected for a headless capture either - see `RaceStage::observation`'s own doc.
#[must_use]
pub fn observation(race: &Race) -> crate::records::Observation {
    let standing = &race.sim.world.ships[0].standing;
    crate::records::Observation {
        finished: race.finished(),
        place: Some(race.player_place()),
        laps_completed: crate::records::laps_completed(
            standing.lap,
            race.finished(),
            race.sim.world.laps_target(),
        ),
        tick: oag_race::race_clock_ticks(standing.finish_tick.unwrap_or(race.sim.world.tick)),
        best_lap_ticks: standing.best_lap_ticks,
        campaign_medal: None,
        campaign_difficulty: None,
    }
}
