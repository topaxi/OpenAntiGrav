//! Demos back to back with no window: what `oag-game --demo --dry-run` flies, and what a test
//! counts.
//!
//! The same flow as the windowed session's (`oag_game::demo`), the same race hook
//! ([`oag_raceplay::Race::fly_as_demo`]) and the same opponents' driver ([`crate::ai_choice`]).

use anyhow::{Context, Result};

use oag_gameplay::PlayerInputs;
use oag_raceplay::{self as race, Race};

use super::{Leave, Pick, Run, chain_seed, pick};
use crate::remix::Catalogue;

/// One demo of a chain, as it ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The circuit's id (`PI_Track`).
    pub track: String,
    /// The team the camera craft raced as.
    pub team: String,
    /// Slots a person flies: `0` in a demo.
    pub humans: usize,
    /// Craft on the grid.
    pub craft: usize,
    /// Craft that travelled at least [`MOVED`] metres from the grid.
    pub moved: usize,
    /// Ticks it ran.
    pub ticks: u32,
    /// Why it ended.
    pub leave: Leave,
}

/// How far a craft must have gone from its grid slot to count as having flown (metres).
pub const MOVED: f32 = 50.0;

/// Flies `demos` demos in a row on the circuits and teams `catalogue` offers (the circuits
/// `playable` passes), each `length` ticks or until its race finishes, from `seed`. `base` is the
/// run's options: source, class, difficulty.
///
/// # Errors
///
/// A circuit that will not load.
pub fn run(
    base: &race::Options,
    catalogue: &Catalogue,
    playable: impl Fn(&race::catalogue::Track) -> bool,
    seed: u64,
    demos: u32,
    length: u32,
) -> Result<Vec<Report>> {
    let tracks: Vec<_> = catalogue
        .tracks
        .iter()
        .map(|(track, _)| track)
        .filter(|track| playable(track))
        .collect();
    let teams: Vec<_> = catalogue.teams.iter().map(|team| team.value.as_str()).collect();
    let mut reports = Vec::new();
    let mut previous: Option<Pick> = None;
    let rule = oag_source::title::identify(&base.source)
        .with_context(|| format!("{} is not a title this build knows", base.source))?
        .demo;
    for n in 0..demos {
        let mut options = base.clone();
        let Some(chosen) = pick(&rule, chain_seed(seed, n), tracks.len(), teams.len(), previous)
        else {
            break;
        };
        previous = Some(chosen);
        options.mode = oag_race::Mode::SingleRace;
        options.track = Some(tracks[chosen.track].entry_name());
        options.team = Some(teams[chosen.team].to_string());
        options.seed = Some(chain_seed(seed, n));
        let loaded = race::load(&options)
            .with_context(|| format!("loading demo {} on {}", n + 1, tracks[chosen.track].id))?;
        let (demo, difficulty) = (loaded.title.demo, loaded.setup.difficulty);
        let mut race = Race::start(loaded.setup);
        crate::ai_choice::apply(
            &mut race,
            crate::ai_choice::AiChoice::default(),
            crate::ai_choice::Source::Free(difficulty),
        );
        race.fly_as_demo();
        let craft = usize::from(race.sim.world.ship_count);
        let start: Vec<_> = (0..craft)
            .map(|slot| race.sim.world.ships[slot].physics.body.position)
            .collect();
        let mut run = Run::new(false);
        let leave = loop {
            race.tick(&PlayerInputs::default());
            if let Some(leave) = run.tick(length.min(demo.duration_ticks()), race.finished()) {
                break leave;
            }
        };
        let moved = (0..craft)
            .filter(|&slot| {
                let at = race.sim.world.ships[slot].physics.body.position;
                (at - start[slot]).length() >= MOVED
            })
            .count();
        reports.push(Report {
            track: tracks[chosen.track].id.clone(),
            team: teams[chosen.team].to_string(),
            humans: race.sim.world.human_slots().count(),
            craft,
            moved,
            ticks: run.ticks(),
            leave,
        });
    }
    Ok(reports)
}
