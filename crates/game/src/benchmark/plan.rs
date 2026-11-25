//! The fixed race every benchmark flies.
//!
//! One definition for the in-game benchmark; `scripts/bench-render.py`'s `COMMON` table is the
//! same race spelled as flags, and `the_offscreen_bench_flies_the_same_race` holds the two
//! together. A row changed here changes what every stored result means.
//!
//! Only what the simulation reads is pinned. Graphics settings, the HUD and the preset are the
//! player's (or the preset's): they change what the frames cost, never the race.

use oag_race::Mode;
use oag_raceplay::Options;

/// The world generator's seed.
pub const SEED: u64 = 1;

/// The race mode: a Single Race against the full AI field, weapons on.
pub const MODE: Mode = Mode::SingleRace;

/// The speed class: the CLI's default, which every title resolves against its own ladder.
pub const CLASS: &str = "venom";

/// Ticks after which a race that has not ended is stopped and reported `finished: false`.
pub const TICK_CAP: u32 = 40_000;

/// Frames left out of every summary (not out of the file's frame count): the first drawn frames
/// pay pipeline and buffer first use.
pub const WARMUP_FRAMES: usize = 60;

/// Sets every option the simulation reads to the benchmark's race, over whatever the front end
/// had chosen: the title's default circuit and team, a Single Race at the default class and the
/// default AI difficulty, the field and weapons the mode ships with, one person, no seats picked, no craft
/// posed. The player's craft is flown by the autopilot (`Session::autopilot`, not an option).
pub fn pin(options: &mut Options) {
    options.mode = MODE;
    options.seed = Some(SEED);
    options.track = None;
    options.foreign_track = None;
    options.team = None;
    options.hull_variant = None;
    options.skin = None;
    options.livery = None;
    options.class = CLASS.to_string();
    options.no_driver_for_class = !oag_ai::coverage::project_driver_covers(CLASS);
    options.difficulty = oag_ai::Difficulty::default();
    options.skill_level = None;
    options.laps_override = None;
    options.weapons_override = None;
    options.eliminator_kill_target = None;
    options.eliminator_score_target = None;
    options.zone_stage = None;
    options.opponents = false;
    options.opponent_teams = Vec::new();
    options.grid_teams = Vec::new();
    options.seat_picks = Vec::new();
    options.field = Vec::new();
    options.pose = None;
    options.camera = None;
    options.fov_override = None;
    options.people = 0;
    options.demo = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `scripts/bench-render.py` is the offscreen bench's race; it must say what this says.
    #[test]
    fn the_offscreen_bench_flies_the_same_race() {
        let script = include_str!("../../../../scripts/bench-render.py");
        let common = script
            .split("COMMON = [")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("COMMON table");
        assert!(common.contains("\"--mode\", \"single_race\""), "{common}");
        assert!(
            common.contains(&format!("\"--seed\", \"{SEED}\"")),
            "{common}"
        );
        assert!(common.contains("\"--autopilot\""), "{common}");
        assert!(
            !common.contains("--class"),
            "the script takes the CLI's class default"
        );
        assert!(
            script.contains(&format!("TICK_CAP = {TICK_CAP}")),
            "tick cap"
        );
    }
}
