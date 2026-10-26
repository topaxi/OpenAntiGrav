//! The race the selection screens are leading to: one description, whichever entry point began it.
//!
//! The RACE page keeps its picks in the settings (`race.mode`, `race.class`) and builds the
//! launch options from them at START. A campaign cell builds the launch options itself and leaves
//! the settings alone (`Session::launch_campaign_cell`), so a screen that read the settings
//! showed the RACE page's last mode on a cell's race: a campaign Zone tile previewed the team's
//! ordinary hull and offered its variants. Every selection screen reads [`PendingRace::resolve`]
//! instead, so a new entry point gets the right mode and class by being the launch source.

use crate::settings;

/// The mode and speed class of the race that is about to launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRace {
    /// The rules the race runs under.
    pub mode: oag_race::Mode,
    /// The speed class, spelled as the launch options spell it.
    pub class: String,
}

impl PendingRace {
    /// The race `launch` describes when a campaign cell built it (`launch` is `Some`), else the one
    /// the RACE page's `settings` describe. An unrecognised stored mode token leaves the default
    /// mode, as the launch does.
    #[must_use]
    pub fn resolve(settings: &settings::Race, launch: Option<&oag_raceplay::Options>) -> Self {
        match launch {
            Some(options) => Self {
                mode: options.mode,
                class: options.class.clone(),
            },
            None => Self {
                mode: oag_race::Mode::from_name(&settings.mode).unwrap_or_default(),
                class: settings.class.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests;
