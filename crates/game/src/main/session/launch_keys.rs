//! `launch_race` with its inputs named by setting key: one launch for any page whose
//! track and craft may come from two titles, a mod's page above all
//! (`docs/architecture/adr/0082-*.md`, decision 5; `docs/tools/mods.md`).
//!
//! The row names where each input is read from (`race.mode`, `mod.<id>.<key>`); the rules for combining them stay here: CUSTOM host, the
//! craft falling back to the track's title, the class with no driver.

use std::path::PathBuf;

use log::{error, info, warn};

use oag_game::launcher::Candidate;
use oag_game::remix;
use oag_raceplay::{Options, catalogue};
use oag_ui::{menu, strings};

use crate::hints;
use crate::settings::{self, Settings};

use super::Session;
use super::menus::combine_variant;
use super::remix_menu::{craft_catalogue, resolve_craft_backing};

/// Every input of a launch, read out of the settings. An empty string is an input the
/// row did not name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    pub track_title: String,
    pub track: String,
    pub craft_title: String,
    pub team: String,
    pub variant: String,
    pub mode: String,
    pub class: String,
    pub difficulty: String,
}

/// What a CUSTOM track file needs on top of its row, resolved by the session before the
/// options are built.
#[derive(Debug, Clone)]
pub(crate) struct Custom {
    /// The disc of the title the file names as its host, if it names one.
    pub host: Option<Candidate>,
    /// The file to race.
    pub file: PathBuf,
}

/// The stored text of the setting `key`, or `None` for a key nothing stores.
fn setting_text(settings: &Settings, key: &str) -> Option<String> {
    if let Some((id, name)) = settings::mod_settings::split(key) {
        return Some(
            settings
                .mod_settings
                .get(id)
                .and_then(|keys| keys.get(name))
                .cloned()
                .unwrap_or_default(),
        );
    }
    Some(match key {
        "race.mode" => settings.race.mode.clone(),
        "race.class" => settings.race.class.clone(),
        "race.team" => settings.race.team.clone(),
        "race.track" => settings.race.track.clone(),
        "race.variant" => settings.race.variant.clone(),
        "ai.difficulty" => settings.ai.difficulty.clone(),
        _ => return None,
    })
}

/// Reads every key `keys` names out of `settings`.
///
/// # Errors
///
/// The first key no setting stores.
pub(crate) fn read_inputs(settings: &Settings, keys: &menu::LaunchInputs) -> Result<Inputs, String> {
    let read = |key: &Option<String>| match key {
        None => Ok(String::new()),
        Some(key) => setting_text(settings, key)
            .ok_or_else(|| format!("{key:?} is not a setting a launch can read")),
    };
    Ok(Inputs {
        track_title: read(&keys.track_title)?,
        track: read(&keys.track)?,
        craft_title: read(&keys.craft_title)?,
        team: read(&keys.team)?,
        variant: read(&keys.variant)?,
        mode: read(&keys.mode)?,
        class: read(&keys.class)?,
        difficulty: read(&keys.difficulty)?,
    })
}

/// Sets the two sources, circuit, team, mode, class and difficulty `inputs` name on
/// `race_options`. Returns `false` when there is nothing to launch; what was set so far
/// stays, so the caller keeps the options for the next try. A track title left empty
/// is the title `race_options` already reads from.
pub(crate) fn apply_two_source(
    titles: &[Candidate],
    race_options: &mut Options,
    inputs: &Inputs,
    custom: Option<&Custom>,
) -> bool {
    // A launch names its own track: a disc circuit, or below a CUSTOM file.
    race_options.eliminator_kill_target = None;
    race_options.eliminator_score_target = None;
    race_options.laps_override = None;
    race_options.weapons_override = None;
    if let Some(mode) = oag_race::Mode::from_name(&inputs.mode) {
        race_options.mode = mode;
    }
    // Carried as a **name**, see the same assignment in `menus.rs`. The union the class
    // came from is built from the ladders of the titles this machine can open, so
    // `VECTOR` is in it exactly when Pure's data is present.
    let class = inputs.class.trim();
    if !class.is_empty() {
        race_options.class = class.to_string();
    }
    // A class no controller covers races without opponents; the AI DIFFICULTY row says
    // so (`Session::ai_difficulties`).
    race_options.no_driver_for_class = !oag_ai::coverage::project_driver_covers(&race_options.class);
    if let Some(difficulty) = oag_ai::Difficulty::from_name(&inputs.difficulty) {
        race_options.difficulty = difficulty;
    }
    // The TEAM row hides every team that does not author the class; with none left there
    // is nothing to launch, and racing a default team that cannot load the class is the
    // failure the row exists to prevent.
    if craft_catalogue(titles, &inputs.craft_title, &inputs.class)
        .is_some_and(|catalogue| catalogue.teams.is_empty())
    {
        warn!(
            "no team of {} authors the speed class {:?}; pick another class or craft title",
            inputs.craft_title, race_options.class
        );
        return false;
    }

    let track_title = inputs.track_title.as_str();
    // A CUSTOM file's track side loads from the disc of the title it names as its host
    // (ADR-0076, decision 13), the craft from CRAFT TITLE's; a file that names none (a
    // Workshop track) is flown on CRAFT TITLE's.
    let craft_backing = resolve_craft_backing(titles, &inputs.craft_title);
    let track_candidate = if let Some(custom) = custom {
        custom.host.clone().or_else(|| craft_backing.cloned())
    } else if track_title.is_empty() {
        titles
            .iter()
            .find(|candidate| candidate.source == race_options.source)
            .cloned()
    } else {
        titles
            .iter()
            .find(|candidate| candidate.title() == track_title)
            .cloned()
    };
    let Some(track_candidate) = track_candidate else {
        warn!("no track title chosen yet, so there is nothing to launch");
        return false;
    };
    if let Some(custom) = custom {
        oag_game::custom_tracks::apply(race_options, &custom.file);
    }
    race_options.source = track_candidate.source.clone();

    // An unpicked CRAFT TITLE means "the same as the track's", on the same terms
    // `race::Options::craft_source: None` does - not a separate source opened
    // redundantly. A picked one this machine can no longer find (its disc left the search
    // path since it was saved) falls back the same way, reported rather than silent.
    let craft_source = if inputs.craft_title.is_empty() || (custom.is_some() && custom.is_some_and(|c| c.host.is_none())) {
        None
    } else {
        // `resolve_craft_backing` is what resolves "Wipeout HD" to 2048's own package
        // when this machine has no real HD source.
        match craft_backing {
            Some(candidate) => Some(candidate.source.clone()),
            None => {
                warn!(
                    "{} is not a title this machine can currently open; racing craft from \
                     {track_title} instead",
                    inputs.craft_title
                );
                None
            }
        }
    };
    race_options.craft_source.clone_from(&craft_source);

    // A CUSTOM track is a file, not a circuit of the source's, so no name is looked up.
    if custom.is_none() {
        match remix::catalogue(&track_candidate.source) {
            Ok(catalogue) => {
                let chosen = catalogue
                    .track(&inputs.track)
                    .or_else(|| {
                        warn!(
                            "{track_title} does not offer {:?}; racing the first circuit it \
                             does offer, which is what the menu shows",
                            inputs.track
                        );
                        catalogue.tracks.first().map(|(track, _)| track)
                    })
                    .map(catalogue::Track::entry_name);
                match chosen {
                    Some(entry) => race_options.track = Some(entry),
                    None => warn!(
                        "{track_title} offers no circuit at all; racing whatever was already \
                         selected"
                    ),
                }
            }
            Err(e) => warn!("{}: {e:#}", track_candidate.source),
        }
    }

    // The team's own title, for VARIANT to combine against - whichever title actually
    // supplied the craft: the picked one, or the track's when craft follows it unpicked.
    let variant_title = if craft_source.is_some() {
        craft_backing.and_then(Candidate::playable)
    } else {
        match &track_candidate.state {
            oag_game::launcher::State::Playable(title) => Some(*title),
            oag_game::launcher::State::Unavailable(_) | oag_game::launcher::State::NeedsKey => None,
        }
    };
    let craft_catalogue_source = craft_source.unwrap_or(track_candidate.source);
    match remix::catalogue(&craft_catalogue_source) {
        Ok(catalogue) => match catalogue.team(&inputs.team) {
            Some(team) => {
                let team = team.to_string();
                let (combined, hull_variant) = match variant_title {
                    Some(title) => {
                        let (combined, hull_variant, warning) =
                            combine_variant(title, &team, &inputs.variant);
                        if let Some(warning) = warning {
                            warn!("{warning}");
                        }
                        (combined, hull_variant)
                    }
                    // No title to check `team_variants`/`hull_variants` against: racing the
                    // bare id, which fails if `team` needs a second directory. Warned rather
                    // than silent, so a dead-end team picker names itself instead of
                    // surfacing as `race::load`'s own "no entry at ..." error.
                    None => {
                        warn!(
                            "{team}: could not resolve its own title, so no VARIANT was \
                             applied; this fails if {team} needs one"
                        );
                        (team, None)
                    }
                };
                race_options.team = Some(combined);
                race_options.hull_variant = hull_variant.map(str::to_string);
            }
            None => warn!(
                "this craft title does not offer team {:?}, racing as its own default instead",
                inputs.team
            ),
        },
        Err(e) => warn!("{craft_catalogue_source}: {e:#}"),
    }
    true
}

impl Session {
    /// Launches the race `keys` name, from the settings they read.
    pub(crate) fn launch_keyed(&mut self, keys: &menu::LaunchInputs) {
        // Defensive, the same reason `Session::launch_from_settings` clears all of these:
        // a cell abandoned mid-`Team Selection` must never leak its own eliminator target
        // or lap override into an unrelated launch.
        self.campaign_cell = None;
        self.campaign_custom = None;
        self.campaign_difficulty = None;
        self.campaign_ai_skill_scale = None;
        let inputs = match read_inputs(&self.settings, keys) {
            Ok(inputs) => inputs,
            Err(why) => {
                warn!("cannot launch this race: {why}");
                return;
            }
        };
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        self.inherit_cli_foreign_track(&mut race_options);
        let custom = if self.custom_track_selected(&inputs.track_title) {
            let resolved = self
                .custom_track_host(&inputs.track)
                .map(|host| host.cloned())
                .and_then(|host| {
                    self.custom_tracks
                        .raceable(&inputs.track)
                        .map(|file| Custom {
                            host,
                            file: file.to_path_buf(),
                        })
                });
            match resolved {
                Ok(custom) => Some(custom),
                Err(why) => {
                    warn!("cannot race this custom track: {why}");
                    self.race_options = Some(race_options);
                    return;
                }
            }
        } else {
            None
        };
        if !apply_two_source(&self.titles, &mut race_options, &inputs, custom.as_ref()) {
            self.race_options = Some(race_options);
            return;
        }
        info!(
            "launching {}",
            race_options
                .foreign_track
                .as_ref()
                .map(|path| path.display().to_string())
                .or_else(|| race_options.track.clone())
                .unwrap_or_else(|| "this source's own default circuit".to_string())
        );
        self.race_options = Some(race_options);
        match self.launch_race() {
            Ok(()) => {
                let strings = strings::project_table(self.settings.language.as_deref());
                println!("\n{}{}", hints::race_keys(&strings), hints::esc_to_menu(&strings));
            }
            Err(e) => error!("cannot start this race: {e:#}"),
        }
    }
}

#[cfg(test)]
#[path = "launch_keys/tests.rs"]
mod tests;
