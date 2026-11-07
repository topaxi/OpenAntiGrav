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

/// Loads `key`'s stored ghost into `race` (unless `auto_load` is off, Pulse's `Auto Load
/// Ghost`) and starts recording, on a mode that races one with one person flying it. A no-op
/// otherwise: a ghost is a single player's Time Trial or Speed Lap.
pub(crate) fn arm(
    race: &mut race::Race,
    key: &records::Key,
    mode: oag_race::Mode,
    auto_load: bool,
    team: &str,
    seed: u64,
    options: std::collections::BTreeMap<String, String>,
) {
    arm_in(
        ghosts::dir().as_deref(),
        race,
        key,
        mode,
        auto_load,
        team,
        seed,
        options,
    );
}

/// [`arm`] reading the stored ghost from under `root` (`None`: nowhere, so none).
#[allow(clippy::too_many_arguments)]
fn arm_in(
    root: Option<&std::path::Path>,
    race: &mut race::Race,
    key: &records::Key,
    mode: oag_race::Mode,
    auto_load: bool,
    team: &str,
    seed: u64,
    options: std::collections::BTreeMap<String, String>,
) {
    if !races_a_ghost(mode, race.people()) {
        return;
    }
    let stored = root.and_then(|root| ghosts::auto_load_from(&ghosts::path_in(root, key), key, auto_load));
    if let Some(stored) = stored
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
    race.start_recording(header_for(key, team, seed, options));
}

/// The header a recording of the race filed under `key` carries: the key, `team`, `seed` and
/// the launch settings the race started with.
pub(crate) fn header_for(
    key: &records::Key,
    team: &str,
    seed: u64,
    options: std::collections::BTreeMap<String, String>,
) -> oag_replay::Header {
    let mut header = ghosts::header(key, team, seed);
    header.options = options;
    header
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
    use super::{arm_in, races_a_ghost};
    use oag_race::Mode;

    #[test]
    fn only_a_single_players_time_trial_or_speed_lap_races_a_ghost() {
        assert!(races_a_ghost(Mode::TimeTrial, 1));
        assert!(races_a_ghost(Mode::SpeedLap, 1));
        assert!(!races_a_ghost(Mode::TimeTrial, 2));
        assert!(!races_a_ghost(Mode::SpeedLap, 4));
        assert!(!races_a_ghost(Mode::Head2Head, 1));
    }

    /// A stored ghost is raced when AUTO LOAD GHOST is on and not when it is off: the choice
    /// `arm` is handed reaches the load. Dropping the gate in `arm_in` turns the second half red.
    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn a_stored_ghost_is_raced_only_with_auto_load_on() {
        use oag_core::math::{Quat, Vec3};
        use oag_replay::{GhostLap, Pose, Recorder};

        let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
            return;
        };
        let key = oag_game::records::Key::new(
            "Wipeout Pulse",
            Some(r"Data\Environments\16_Track\track.vex"),
            "time_trial",
            "Venom",
        );
        let root = std::env::temp_dir().join(format!("oag-arm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let lap = GhostLap {
            lap_ticks: 100,
            poses: vec![
                Pose {
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                };
                101
            ],
        };
        let replay = Recorder::new(oag_game::ghosts::header(&key, "feisar", 1), 7).replay(Some(lap));
        let path = oag_game::ghosts::path_in(&root, &key);
        assert!(oag_game::ghosts::save_to(&path, &key, &replay).expect("writes"));

        for (auto_load, raced) in [(true, true), (false, false)] {
            let loaded = oag_raceplay::load(&oag_raceplay::Options {
                source: image.display().to_string(),
                seed: Some(1),
                opponents: false,
                ..oag_raceplay::Options::default()
            })
            .expect("loading the race");
            let mut race = oag_raceplay::Race::start(loaded.setup);
            arm_in(
                Some(&root),
                &mut race,
                &key,
                Mode::TimeTrial,
                auto_load,
                "feisar",
                1,
                Default::default(),
            );
            assert_eq!(race.ghost().is_some(), raced, "auto load {auto_load}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
