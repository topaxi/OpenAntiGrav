//! Rebuilding a recorded race from its header and a source.
//!
//! The header carries the title it was recorded on, the resolved launch request
//! ([`oag_replay::header::Setup`]) and the launch settings (`options`); the **source** - the
//! disc image or extracted package of that title - is the reader's own, so it is an argument.
//! Whether the rebuilt race is the recorded one is the tick-zero hash's to say
//! ([`oag_replay::Verifier::new`]).

use anyhow::{Context, Result, bail};
use oag_gameplay::Controller;
use oag_replay::Header;

use super::Launch;
use crate::{Options, Race, Setup};

impl Options {
    /// The options a header's race was loaded with, over `source`.
    ///
    /// # Errors
    ///
    /// A header with no `[setup]` (written before the table existed, or by a writer with no
    /// request to record), or a mode token this build does not know.
    pub fn for_replay(header: &Header, source: &str) -> Result<Self> {
        let Some(setup) = &header.setup else {
            bail!("the replay records no launch request ([setup]), so its race cannot be rebuilt");
        };
        let launch = Launch::from_table(&header.options)?;
        let mode = oag_race::Mode::from_name(&setup.mode)
            .with_context(|| format!("the replay names the mode {:?}", setup.mode))?;
        Ok(Self {
            source: source.to_string(),
            track: setup.track.clone(),
            team: setup.team.clone(),
            hull_variant: setup.hull_variant.clone(),
            skin: setup.skin.clone(),
            opponent_teams: setup.opponent_teams.clone(),
            opponents: setup.opponents,
            no_driver_for_class: setup.no_driver_for_class,
            people: setup.people,
            views: setup.views,
            laps_override: setup.laps_override,
            weapons_override: setup.weapons_override,
            eliminator_kill_target: setup.eliminator_kill_target,
            eliminator_score_target: setup.eliminator_score_target,
            add_on_campaign: setup.add_on_campaign.clone(),
            zone_model: setup.zone_model,
            class: setup.class.clone(),
            mode,
            difficulty: launch.difficulty,
            seed: Some(header.seed),
            ..Self::default()
        })
    }

    /// The request a race loaded with these options is recorded under.
    #[must_use]
    pub fn replay_setup(&self) -> oag_replay::header::Setup {
        oag_replay::header::Setup {
            mode: self.mode.name().to_string(),
            class: self.class.clone(),
            track: self.track.clone(),
            team: self.team.clone(),
            hull_variant: self.hull_variant.clone(),
            skin: self.skin.clone(),
            opponent_teams: self.opponent_teams.clone(),
            opponents: self.opponents,
            no_driver_for_class: self.no_driver_for_class,
            people: self.people,
            views: self.views,
            laps_override: self.laps_override,
            weapons_override: self.weapons_override,
            eliminator_kill_target: self.eliminator_kill_target,
            eliminator_score_target: self.eliminator_score_target,
            add_on_campaign: self.add_on_campaign.clone(),
            zone_model: self.zone_model,
        }
    }
}

/// The setup of the race `header` was recorded from, loaded from `source`.
///
/// # Errors
///
/// [`Options::for_replay`]'s or the load's.
pub fn load_recorded(header: &Header, source: &str) -> Result<Setup> {
    let options = Options::for_replay(header, source)?;
    let loaded = crate::load(&options).context("loading the replay's race")?;
    Ok(loaded.setup)
}

/// The race `header` was recorded from, started from `setup` with the recorded launch settings
/// and its recorded seats, before its first tick.
///
/// The seats are the header's: a split screen makes its slots [`Controller::Local`] after
/// [`Race::start`], and `World::controllers` is hashed, so a race that left them to the start
/// disagrees at tick zero.
///
/// # Errors
///
/// A launch setting this build does not know.
pub fn start_recorded(setup: Setup, header: &Header) -> Result<Race> {
    let launch = Launch::from_table(&header.options)?;
    let mut race = Race::start(setup);
    race.apply_launch(&launch);
    for &slot in &header.slots {
        if let Some(controller) = race.sim.world.controllers.get_mut(usize::from(slot)) {
            *controller = Controller::Local;
        }
    }
    Ok(race)
}

/// The race `header` was recorded from, loaded from `source` and started as
/// [`start_recorded`] starts it.
///
/// # Errors
///
/// [`load_recorded`]'s and [`start_recorded`]'s.
pub fn rebuild(header: &Header, source: &str) -> Result<Race> {
    start_recorded(load_recorded(header, source)?, header)
}
