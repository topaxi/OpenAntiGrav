//! A session's ghost: raced from `<config dir>/oag/ghosts/` when a Time
//! Trial or Speed Lap starts, and written back when the run beats it.
//!
//! The two ends of `oag_game::ghosts` a windowed session needs, beside the
//! records they mirror: [`arm`] where `Stage::build_race_stage` resolves the
//! records key, and [`save`] at the same two places `records.toml` is written
//! (the finish transition and leaving a race). See ADR-0055, "which lap, and
//! when it is saved".

use log::{error, info};
use oag_game::{ghosts, records};
use oag_raceplay as race;
use oag_raceplay::Ghost;

/// Loads `key`'s stored ghost into `race` and starts recording, on a mode that
/// races one with one person flying it. A no-op otherwise: a ghost is a single
/// player's Time Trial or Speed Lap (maintainer, 2026-10-14).
pub(crate) fn arm(
    race: &mut race::Race,
    key: &records::Key,
    mode: oag_race::Mode,
    team: &str,
    seed: u64,
    options: std::collections::BTreeMap<String, String>,
) {
    if !races_a_ghost(mode, race.people()) {
        return;
    }
    if let Some(stored) = ghosts::load(key)
        && let Some(lap) = stored.ghost
    {
        info!(
            "racing the stored ghost: a lap of {} ticks, flown as {}",
            lap.lap_ticks, stored.header.team
        );
        race.set_ghost(Ghost {
            lap,
            team: stored.header.team,
        });
    }
    let mut header = ghosts::header(key, team, seed);
    header.options = options;
    race.start_recording(header);
}

/// Writes the run's best lap as `key`'s ghost when it set one since the last
/// call, and it beats the stored file. Logged, never fatal - the
/// `records.toml` contract.
pub(crate) fn save(race: &mut race::Race, key: &records::Key) {
    // A lap from a race a second person was in, split or in another window, is
    // not a ghost anybody should race against.
    if race.people() > 1 {
        return;
    }
    let Some(replay) = race.take_new_best_ghost() else {
        return;
    };
    match ghosts::save(key, &replay) {
        Ok(true) => info!(
            "saved a new ghost: a lap of {} ticks",
            replay.ghost.as_ref().map_or(0, |lap| lap.lap_ticks)
        ),
        Ok(false) => {}
        Err(e) => error!("could not save the ghost: {e:#}"),
    }
}

/// Whether a race of `mode` with `people` flying it races a ghost: a Time
/// Trial or Speed Lap with one person.
fn races_a_ghost(mode: oag_race::Mode, people: usize) -> bool {
    mode.races_a_ghost() && people == 1
}

#[cfg(test)]
mod tests {
    use super::races_a_ghost;
    use oag_race::Mode;

    #[test]
    fn only_a_single_players_time_trial_or_speed_lap_races_a_ghost() {
        assert!(races_a_ghost(Mode::TimeTrial, 1));
        assert!(races_a_ghost(Mode::SpeedLap, 1));
        assert!(!races_a_ghost(Mode::TimeTrial, 2));
        assert!(!races_a_ghost(Mode::SpeedLap, 4));
        assert!(!races_a_ghost(Mode::Head2Head, 1));
    }
}
