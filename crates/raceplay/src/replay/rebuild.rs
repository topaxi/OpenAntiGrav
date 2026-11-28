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
use crate::extensions::Prepared;
use crate::{Loaded, Options, Race, SimSetup, ViewSetup};

impl Options {
    /// The options a header's race was loaded with, over `source`.
    ///
    /// # Errors
    ///
    /// A header with no `[setup]` (written before the table existed, or by a writer with no
    /// request to record), or a mode token this build does not know.
    pub fn for_replay(header: &Header, source: &str) -> Result<Self> {
        let mut options = Self {
            source: source.to_string(),
            ..Self::default()
        };
        options.apply_replay(header)?;
        Ok(options)
    }

    /// Sets what `header` records of the launch request on these options, leaving the rest (the
    /// source, mods, texture packs) as they are: a caller that built its options from a command
    /// line takes the recorded race's circuit, team, class, mode and seed this way.
    ///
    /// # Errors
    ///
    /// As [`Self::for_replay`].
    pub fn apply_replay(&mut self, header: &Header) -> Result<()> {
        let Some(setup) = &header.setup else {
            bail!("the replay records no launch request ([setup]), so its race cannot be rebuilt");
        };
        let launch = Launch::from_table(&header.options)?;
        self.mode = oag_race::Mode::from_recorded_name(&setup.mode)
            .with_context(|| format!("the replay names the mode {:?}", setup.mode))?;
        self.track = setup.track.clone();
        self.team = setup.team.clone();
        self.hull_variant = setup.hull_variant.clone();
        self.skin = setup.skin.clone();
        self.opponent_teams = setup.opponent_teams.clone();
        self.opponents = setup.opponents;
        self.no_driver_for_class = setup.no_driver_for_class;
        self.people = setup.people;
        self.views = setup.views;
        self.laps_override = setup.laps_override;
        self.weapons_override = setup.weapons_override;
        self.eliminator_kill_target = setup.eliminator_kill_target;
        self.eliminator_score_target = setup.eliminator_score_target;
        self.add_on_campaign = setup.add_on_campaign.clone();
        self.zone_model = setup.zone_model;
        self.class = setup.class.clone();
        self.difficulty = launch.difficulty;
        self.skill_level = launch.skill_level;
        self.seed = Some(header.seed);
        self.recorded_mods = Some(header.mods.clone());
        self.recorded_active_mods = Some(header.active_mods.clone());
        // An online room's race mounted a network race's layers, so its rebuild does too.
        if super::Room::from_table(&header.options)?.is_some() {
            self.network = true;
        }
        Ok(())
    }

    /// [`Self::replay_setup`] with `circuit`, the entry the load settled on, as the track when
    /// none was named, so a rebuild from the header does not depend on the default being the
    /// same one. A Zone race's entry is the zone variant of the circuit named and a foreign
    /// track has no archive entry: both keep what was named.
    #[must_use]
    pub fn replay_setup_on(&self, circuit: &str) -> oag_replay::header::Setup {
        let mut setup = self.replay_setup();
        if setup.track.is_none()
            && self.mode != oag_race::Mode::Zone
            && !circuit.starts_with("foreign:")
        {
            setup.track = Some(circuit.to_string());
        }
        setup
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

/// The race `header` was recorded from, loaded from `source`.
///
/// # Errors
///
/// [`Options::for_replay`]'s or the load's.
pub fn load_recorded(header: &Header, source: &str) -> Result<Loaded> {
    let options = Options::for_replay(header, source)?;
    crate::load(&options).context("loading the replay's race")
}

/// The race `header` was recorded from, started from `setup`, `view` and `modules` with the
/// recorded launch settings
/// and its recorded seats, before its first tick.
///
/// The seats are the header's: a split screen makes its slots [`Controller::Local`] after
/// [`Race::start`], and `World::controllers` is hashed, so a race that left them to the start
/// disagrees at tick zero.
///
/// # Errors
///
/// A launch setting this build does not know.
pub fn start_recorded(
    setup: SimSetup,
    view: ViewSetup,
    modules: Prepared,
    header: &Header,
) -> Result<Race> {
    let mut race = Race::start(setup, view, modules);
    apply_recorded(&mut race, header)?;
    Ok(race)
}

/// What a recorded race had written to it after [`Race::start`]: `header`'s launch settings and
/// its seats. [`start_recorded`] does it to a race it starts; a viewer that was handed a race
/// loaded by the ordinary path does it to that one.
///
/// # Errors
///
/// A launch setting this build does not know.
pub fn apply_recorded(race: &mut Race, header: &Header) -> Result<()> {
    crate::extensions::check_recorded(&header.mods, race.core.sim.mods_used()).map_err(|why| {
        anyhow::anyhow!("the replay's simulation modules are not this race's: {why}")
    })?;
    let launch = Launch::from_table(&header.options)?;
    race.core.sim.apply_launch(&launch);
    for &slot in &header.slots {
        if let Some(controller) = race.core.sim.world.controllers.get_mut(usize::from(slot)) {
            *controller = Controller::Local;
        }
    }
    // After the seats: an online room's people are remote, not local.
    if let Some(room) = super::Room::from_table(&header.options)? {
        race.core.sim.apply_room(&room);
    }
    // After the seats, as the session applies the AI choice: which driver flies reads who is
    // human.
    if let Some(ai) = launch.original_ai {
        race.core.sim.use_original_ai(ai.skill, ai.level);
    }
    Ok(())
}

/// The race `header` was recorded from, loaded from `source` and started as
/// [`start_recorded`] starts it.
///
/// # Errors
///
/// [`load_recorded`]'s and [`start_recorded`]'s.
pub fn rebuild(header: &Header, source: &str) -> Result<Race> {
    let loaded = load_recorded(header, source)?;
    start_recorded(loaded.setup, loaded.view, loaded.modules, header)
}
