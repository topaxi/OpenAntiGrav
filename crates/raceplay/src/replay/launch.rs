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

/// The recovered original driver flying the opponents, and what it was asked to do
/// ([`Race::use_original_ai`]): written to the race after its seats, so it is part of the state
/// a rebuild must reproduce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalAi {
    /// A campaign cell's resolved skill scale; `None` reads the load's ambient one.
    pub skill: Option<f32>,
    /// The pack shuffle's level, `0..=2`.
    pub level: usize,
}

/// The launch settings a replay records.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// The original driver flies the opponents, when it does. `None`: this project's driver.
    pub original_ai: Option<OriginalAi>,
    /// A campaign cell's resolved skill scale, which tunes the field and the finished player's
    /// thrust ([`Race::apply_campaign_skill`]). `None` outside a campaign cell.
    pub campaign_skill: Option<f32>,
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
        table.extend(Self::ai_table(self.original_ai, self.campaign_skill));
        table
    }

    /// The keys of an `options` table that say who flies the opponents and at what skill: what
    /// [`Self::to_table`] writes for them, and what the composition root adds to a header once
    /// the AI choice is made.
    ///
    /// `campaign_skill` is its own key, not `ai_skill`: a campaign cell is tuned whichever driver
    /// flies it, and a 2048 event's driver skill tunes nothing.
    #[must_use]
    pub fn ai_table(
        original_ai: Option<OriginalAi>,
        campaign_skill: Option<f32>,
    ) -> BTreeMap<String, String> {
        let mut table = BTreeMap::new();
        if let Some(ai) = original_ai {
            table.insert("ai".to_string(), "original".to_string());
            table.insert("ai_level".to_string(), ai.level.to_string());
            if let Some(skill) = ai.skill {
                table.insert("ai_skill".to_string(), skill.to_string());
            }
        }
        if let Some(scale) = campaign_skill {
            table.insert("campaign_skill".to_string(), scale.to_string());
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
        let original_ai = match get("ai") {
            None | Some("ours") => None,
            Some("original") => Some(OriginalAi {
                skill: match get("ai_skill") {
                    None => None,
                    Some(value) => Some(value.parse().map_err(|_| unknown("ai_skill", value))?),
                },
                level: match get("ai_level") {
                    None => 1,
                    Some(value) => value.parse().map_err(|_| unknown("ai_level", value))?,
                },
            }),
            Some(value) => return Err(unknown("ai", value)),
        };
        let campaign_skill = match get("campaign_skill") {
            None => None,
            Some(value) => Some(
                value
                    .parse()
                    .map_err(|_| unknown("campaign_skill", value))?,
            ),
        };
        Ok(Self {
            scheme,
            difficulty,
            autopilot,
            pilot_assist,
            original_ai,
            campaign_skill,
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
        if let Some(scale) = launch.campaign_skill {
            self.apply_campaign_skill(scale);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(original_ai: Option<OriginalAi>) -> Launch {
        Launch {
            scheme: ControlScheme::default(),
            difficulty: oag_ai::Difficulty::default(),
            autopilot: false,
            pilot_assist: Level::Normal,
            original_ai,
            campaign_skill: None,
        }
    }

    /// The original driver is part of the state a rebuild must reproduce, so the table carries
    /// what it was asked and reads it back; an absent key is this project's driver.
    #[test]
    fn the_original_drivers_request_round_trips_through_the_table() {
        for ai in [
            None,
            Some(OriginalAi {
                skill: None,
                level: 1,
            }),
            Some(OriginalAi {
                skill: Some(0.37),
                level: 2,
            }),
        ] {
            let launch = launch(ai);
            assert_eq!(Launch::from_table(&launch.to_table()), Ok(launch), "{ai:?}");
        }
        assert!(!launch(None).to_table().contains_key("ai"));
        let mut table = launch(None).to_table();
        table.insert("ai".into(), "telepathy".into());
        assert_eq!(Launch::from_table(&table).unwrap_err().key, "ai");
        table.insert("ai".into(), "original".into());
        table.insert("ai_level".into(), "high".into());
        assert_eq!(Launch::from_table(&table).unwrap_err().key, "ai_level");
    }

    /// The campaign scale is its own key: written when there is one, whoever flies, and read
    /// back; a file without it raced no campaign cell.
    #[test]
    fn the_campaign_skill_round_trips_apart_from_the_original_drivers() {
        let original = Some(OriginalAi {
            skill: Some(3.7),
            level: 2,
        });
        assert!(!launch(original).to_table().contains_key("campaign_skill"));
        assert_eq!(
            Launch::from_table(&launch(original).to_table()).map(|l| l.campaign_skill),
            Ok(None)
        );
        for (original_ai, campaign_skill) in
            [(None, Some(0.5)), (original, Some(3.7)), (None, None)]
        {
            let launch = Launch {
                campaign_skill,
                ..launch(original_ai)
            };
            assert_eq!(
                Launch::from_table(&launch.to_table()),
                Ok(launch),
                "{launch:?}"
            );
        }
        let mut table = launch(original).to_table();
        table.insert("campaign_skill".into(), "lots".into());
        assert_eq!(
            Launch::from_table(&table).unwrap_err().key,
            "campaign_skill"
        );
    }
}
