//! A session's ghost: raced from `<config dir>/oag/ghosts/` when a Time
//! Trial or Speed Lap starts, and written back when the run beats it.
//!
//! The two ends of `oag_game::ghosts` a windowed session needs, beside the
//! records they mirror: [`arm`] where `Stage::build_race_stage` resolves the
//! records key, and [`save`] at the same two places `records.toml` is written
//! (the finish transition and leaving a race). See ADR-0055, "which lap, and
//! when it is saved".

use log::{error, info};
use oag_game::race::{self, Ghost};
use oag_game::{ghosts, records};

/// Loads `key`'s stored ghost into `race` and starts recording, on a mode that
/// races one. A no-op on every other mode.
pub(crate) fn arm(
    race: &mut race::Race,
    key: &records::Key,
    mode: oag_race::Mode,
    team: &str,
    seed: u64,
    options: std::collections::BTreeMap<String, String>,
) {
    if !ghosts::races_a_ghost(mode) {
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
