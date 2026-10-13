//! Which AI flies a race's opponents: the recovered original driver
//! (`oag_ai::original`) or this project's own (`oag_ai::Driver`).
//!
//! **The original by default, in every race that has opponents** (maintainer,
//! 2026-10-13), until this project's own driver is solid. It is a run flag,
//! `--ai original|ours`, and never a player setting or a menu row: the AI is
//! chosen by the build and the game mode, which keeps the 2026-10-11 rule
//! recorded in `oag_raceplay`'s `original_ai.rs`. `--ai ours` is a developer's
//! switch for testing our driver and overrides every race, the campaign's
//! included.
//!
//! [`decide`] is the pure half, so the rules test without a disc; [`apply`]
//! hands its answer to [`Race::use_original_ai`], which keeps its own honest
//! fallback (no tables for the title and class, or no closed lap) and logs it.

use oag_raceplay::Race;
use oag_title::original_driver::EventAi;

/// `--ai`: whose driver flies the opponents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum AiChoice {
    /// The recovered original driver, with this project's driver as the
    /// fallback where its tables did not read. The default.
    #[default]
    Original,
    /// This project's own driver everywhere, for testing it.
    Ours,
}

/// What the race being started is, as far as the AI choice cares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    /// A campaign cell: its resolved `AI_ResolveSkillScale` (`None` for a cell
    /// that authors none, which reads the race's ambient value) and its rung.
    Campaign {
        /// The cell's resolved skill scale.
        skill: Option<f32>,
        /// The cell's integer rung, `0` Easy to `2` Hard.
        level: usize,
    },
    /// A 2048 campaign event and its authored AI, `None` for one with no `ai`
    /// block (Elimination), which keeps this project's driver.
    Event2048(Option<EventAi>),
    /// Anything else with opponents: race box, remix, a direct `--race`. The
    /// race's `[ai] difficulty` stands in for the rung.
    Free(oag_ai::Difficulty),
}

/// Whose driver flies, and the reason when it is ours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    /// The original driver, at this skill scale (`None` reads the ambient
    /// `AI_ResolveSkillScale` the load resolved) and rung. `event` carries a
    /// 2048 event's two AI skills.
    Original {
        /// See [`Source::Campaign::skill`].
        skill: Option<f32>,
        /// The pack shuffle's level, `0..=2`.
        level: usize,
        /// A 2048 event's pad-tracking and weapon-avoidance skills.
        event: Option<(f32, f32)>,
    },
    /// This project's driver, and why.
    Ours(Reason),
}

/// Why a race keeps this project's driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// `--ai ours`.
    Flag,
    /// The race fields no opponents.
    NoOpponents,
    /// A 2048 event that authors no `ai` block (Elimination).
    EventWithoutAi,
}

impl Reason {
    /// The words the race-start log line uses.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Flag => "--ai ours",
            Self::NoOpponents => "the race has no opponents",
            Self::EventWithoutAi => "the 2048 event authors no AI block",
        }
    }
}

/// The pack-shuffle level (`0` Easy, `1` Medium, `2` Hard) a race with no
/// campaign rung flies at, from the `[ai] difficulty` the race box carries.
///
/// **Chosen, not measured.** The original's level exists only as the
/// campaign's three rungs; this project's four named difficulties have no
/// original counterpart, so Novice is the easy rung, Ace the hard one, and
/// the default (Elite) and Skilled sit on Medium so an untouched race box
/// flies the rung a campaign cell with none authored does.
#[must_use]
pub const fn free_level(difficulty: oag_ai::Difficulty) -> usize {
    match difficulty {
        oag_ai::Difficulty::Novice => 0,
        oag_ai::Difficulty::Skilled | oag_ai::Difficulty::Elite => 1,
        oag_ai::Difficulty::Ace => 2,
    }
}

/// The rules: `opponents` is whether the grid holds any.
#[must_use]
pub fn decide(choice: AiChoice, opponents: bool, source: Source) -> Decision {
    if choice == AiChoice::Ours {
        return Decision::Ours(Reason::Flag);
    }
    if !opponents {
        return Decision::Ours(Reason::NoOpponents);
    }
    match source {
        Source::Campaign { skill, level } => Decision::Original {
            skill,
            level,
            event: None,
        },
        Source::Event2048(Some(ai)) => Decision::Original {
            skill: Some(ai.skill_scale),
            level: ai.level,
            event: Some((ai.pad_tracking, ai.weapon_avoidance)),
        },
        Source::Event2048(None) => Decision::Ours(Reason::EventWithoutAi),
        Source::Free(difficulty) => Decision::Original {
            skill: None,
            level: free_level(difficulty),
            event: None,
        },
    }
}

/// Decides and acts on `race`, logging once which AI flies and why. Returns
/// whether the original driver flies (`false` for the reasons in [`Reason`]
/// and for [`Race::use_original_ai`]'s own fallbacks, which it logs itself).
pub fn apply(race: &mut Race, choice: AiChoice, source: Source) -> bool {
    let opponents = race.sim.world.ship_count > 1;
    match decide(choice, opponents, source) {
        Decision::Ours(reason) => {
            log::info!("AI: this project's driver flies ({})", reason.text());
            false
        }
        Decision::Original {
            skill,
            level,
            event,
        } => {
            let flies = race.use_original_ai(skill, level);
            if flies && let Some((pad_tracking, weapon_avoidance)) = event {
                race.set_original_event_ai_skills(pad_tracking, weapon_avoidance);
            }
            flies
        }
    }
}

#[cfg(test)]
mod tests;
