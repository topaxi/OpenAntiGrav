//! How a race was launched beyond its [`oag_replay::header::Setup`]: the settings that change
//! the player's physics or the opponents, as the header's `options` table spells them.
//!
//! **One mechanism for writing and rebuilding**: the composition root builds a [`Launch`], calls
//! [`Race::apply_launch`] on the started race and writes [`Launch::to_table`] into the header;
//! [`super::rebuild`] reads the table back with [`Launch::from_table`] and applies it the same
//! way. Anything the launch sets on a race belongs here, so a recording and its rebuild cannot
//! drift apart.

use std::collections::BTreeMap;

use oag_gameplay::ControlScheme;
use oag_physics::pilot_assist::Level;

use crate::Race;

/// The launch settings a replay records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Launch {
    /// Which scheme maps the pilot's buttons.
    pub scheme: ControlScheme,
    /// How good the opponents are: the one setting here that reaches [`crate::Options`] and so
    /// the load, not the started race.
    pub difficulty: oag_ai::Difficulty,
    /// The operator's autopilot flies the player's craft.
    pub autopilot: bool,
    /// The Pilot Assist level every seat starts with.
    pub pilot_assist: Level,
}

/// Why a header's `options` table did not read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("replay option {key} = {value:?} is not one this build knows")]
pub struct UnknownOption {
    /// The key.
    pub key: &'static str,
    /// The text it held.
    pub value: String,
}

impl Launch {
    /// The `options` table of a header: only what differs from the defaults is written, and an
    /// absent key reads as the default, as every file written before the table was read is.
    #[must_use]
    pub fn to_table(&self) -> BTreeMap<String, String> {
        let mut table = BTreeMap::new();
        table.insert("scheme".to_string(), self.scheme.name().to_string());
        table.insert("difficulty".to_string(), self.difficulty.name().to_string());
        if self.autopilot {
            table.insert("autopilot".to_string(), "true".to_string());
        }
        if self.pilot_assist != Level::Off {
            table.insert(
                "pilot_assist".to_string(),
                self.pilot_assist.name().to_string(),
            );
        }
        table
    }

    /// Reads a header's `options` table.
    ///
    /// # Errors
    ///
    /// [`UnknownOption`] for a value this build does not know: a rebuild with a guess in its
    /// place would disagree later, and a refusal names the key.
    pub fn from_table(table: &BTreeMap<String, String>) -> Result<Self, UnknownOption> {
        let get = |key: &'static str| table.get(key).map(String::as_str);
        let unknown = |key: &'static str, value: &str| UnknownOption {
            key,
            value: value.to_string(),
        };
        let scheme = match get("scheme") {
            None => ControlScheme::default(),
            Some(value) => {
                ControlScheme::from_name(value).ok_or_else(|| unknown("scheme", value))?
            }
        };
        let difficulty = match get("difficulty") {
            None => oag_ai::Difficulty::default(),
            Some(value) => {
                oag_ai::Difficulty::from_name(value).ok_or_else(|| unknown("difficulty", value))?
            }
        };
        // `true`, which the switch this level replaced wrote, means Extreme.
        let pilot_assist = match get("pilot_assist") {
            None => Level::Off,
            Some("true") => Level::Extreme,
            Some(value) => Level::from_name(value).ok_or_else(|| unknown("pilot_assist", value))?,
        };
        let autopilot = match get("autopilot") {
            None | Some("false") => false,
            Some("true") => true,
            Some(value) => return Err(unknown("autopilot", value)),
        };
        Ok(Self {
            scheme,
            difficulty,
            autopilot,
            pilot_assist,
        })
    }
}

impl Race {
    /// Applies the launch settings to a race before its first tick. What the composition root
    /// and [`super::rebuild`] both call.
    pub fn apply_launch(&mut self, launch: &Launch) {
        self.set_control_scheme(launch.scheme);
        self.set_autopilot(launch.autopilot);
        self.set_pilot_assist(launch.pilot_assist);
    }
}
