//! The thrust the original flies the player's craft at once it has crossed
//! the line for the last time.
//!
//! **Recovered, confidence 90** (`docs/ghidra/functions/psp-pulse-usa/race-finish.md`,
//! measured 2026-10-04 on PPSSPP): from the frame after the finish the
//! player's own AI driver runs `AI_ComputeOpponentThrust` indexed by the
//! player's finishing place, and that law holds the thrust at
//! [`oag_tables::ai_race_stats::AiRaceStats::finished_player_thrust`] -
//! `56.7` % of full after a first place, `56.0` % after a fourth, Venom, Easy,
//! Talon's Junction. Ours flew the finished craft at its own driver's racing
//! pace instead, about 125-135 u/s against the original's 92-110.
//!
//! What is ported is a **cap**: the finished player's thrust is the
//! smaller of what `oag-ai`'s driver asks for and the law's figure. The
//! original's AI has no corner lift of its own to keep, ours does, and the
//! cap keeps it - the `min` is **chosen, not measured**. The law's other
//! regime, when the player has fallen about 195 units behind its target and
//! the clamp lets go, needs the opponents' `spread` wander this project does
//! not model, and is not reproduced. Opponents are untouched: they race on
//! this project's own driver (the maintainer's standing rule), and this is
//! the finished player's own autopilot, which matches the original.

use oag_tables::ai_race_stats::{self, AiRaceStats};
use oag_tables::handling::SpeedClass;
use oag_tables::race_campaign::Difficulty;
use oag_tables::track_stats::{self, ModeTerm, TrackStats};

use super::Race;

/// The class's thrust table and the race's skill scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinishedThrust {
    /// The race's class's `AIRaceStats_<class>.xml` rows.
    pub stats: AiRaceStats,
    /// `AI_ResolveSkillScale`'s value for this race.
    pub skill: f32,
}

impl FinishedThrust {
    /// The law's thrust after finishing in `place`, as a fraction of full.
    #[must_use]
    pub fn fraction(&self, place: u8) -> f32 {
        self.stats.finished_player_thrust(place, self.skill) / 100.0
    }
}

/// This project's four tiers on the original's three rungs: `Novice` is
/// Easy, `Skilled` Medium, `Elite` Hard (the equivalence
/// `oag_ai::Difficulty::tune_at_scale` documents), and `Ace`, which the
/// original has no rung for, Hard.
fn rung(difficulty: oag_ai::Difficulty) -> Difficulty {
    match difficulty {
        oag_ai::Difficulty::Novice => Difficulty::Easy,
        oag_ai::Difficulty::Skilled => Difficulty::Medium,
        oag_ai::Difficulty::Elite | oag_ai::Difficulty::Ace => Difficulty::Hard,
    }
}

/// `AI_ResolveSkillScale`'s mode term: `g_game_mode` 3 is the Single Race
/// (read live), 9 Head to Head (decompile). Every other mode adds nothing,
/// Tournament (`g_game_mode` 4, `docs/ghidra/functions/psp-pulse-usa/state-machine.md`)
/// included, which is what the decompile reads.
fn mode_term(mode: oag_race::Mode, weapons_on: bool, full_grid: bool) -> ModeTerm {
    match mode {
        oag_race::Mode::SingleRace => ModeTerm::Race {
            weapons: weapons_on,
            full_grid,
        },
        oag_race::Mode::Head2Head => ModeTerm::HeadToHead,
        _ => ModeTerm::None,
    }
}

/// Everything [`read`] needs to know about the race.
pub(super) struct RaceTerms<'a> {
    /// The speed class as the disc spells it.
    pub class: &'a str,
    /// The opponents' tier.
    pub difficulty: oag_ai::Difficulty,
    /// The mode.
    pub mode: oag_race::Mode,
    /// Whether weapons are on.
    pub weapons_on: bool,
    /// Whether all eight craft race.
    pub full_grid: bool,
    /// The track's own `stats.xml`, when read.
    pub track_stats: Option<&'a TrackStats>,
}

/// The class's `AIRaceStats` off `archives`, and the race's skill scale.
///
/// `None`, said so in `report`, when the class is not one of the four or
/// the file is absent or does not parse - Pulse ships one per class (PSP and
/// PS2 alike), Pure one file of another shape. HD/Fury, 2048 and Omega ship
/// Pulse's own four, value for value
/// (`docs/ghidra/functions/ps3-hdfury-eu/ai-stats.md`), so they read theirs
/// here too; the cap is **Pulse's law, unmeasured on HD**: HD's
/// `AI_ComputeOpponentThrust` (`0x000fdf00`) gives a finished craft a spread
/// of `75` rather than `50` and a wander offset of `8 - place`, which the
/// lower-stop formula does not read and which no HD capture has tested. With no
/// `stats.xml` read (anything but Pulse on a PSP disc: PS2 Pulse keeps its
/// track table where this project does not read it) the skill scale is
/// `2.0`, the value the original substitutes only when its own table is
/// absent - for PS2 a **chosen, not measured** stand-in, which caps the
/// finished craft at about 64 % at every tier.
pub(super) fn read(
    archives: &mut oag_assets::Archives,
    terms: &RaceTerms,
    report: &mut Vec<String>,
) -> Option<FinishedThrust> {
    let Some(class) = SpeedClass::from_name(terms.class) else {
        report.push(format!(
            "finished thrust: class {:?} has no AIRaceStats file; the finished craft keeps \
             its driver's own thrust",
            terms.class
        ));
        return None;
    };
    let entry = ai_race_stats::entry_name(class);
    let stats = match archives.read_name(&entry) {
        Ok(blob) => match ai_race_stats::from_blob(&blob, class) {
            Ok(stats) => stats,
            Err(e) => {
                report.push(format!("finished thrust: {entry} did not parse ({e})"));
                return None;
            }
        },
        Err(e) => {
            report.push(format!(
                "finished thrust: {entry} did not read ({e}); the finished craft keeps its \
                 driver's own thrust"
            ));
            return None;
        }
    };
    let skill = track_stats::ambient_skill_scale(
        terms.track_stats,
        class,
        rung(terms.difficulty),
        mode_term(terms.mode, terms.weapons_on, terms.full_grid),
    );
    let thrust = FinishedThrust { stats, skill };
    report.push(format!(
        "finished thrust: {entry}, skill {skill:.2}, first place {:.1} %, last {:.1} %",
        100.0 * thrust.fraction(1),
        100.0 * thrust.fraction(8)
    ));
    Some(thrust)
}

impl Race {
    /// Replaces the skill scale the finished player's thrust is read at:
    /// a campaign cell's own position on the track's curve, which
    /// `AI_ResolveSkillScale` uses instead of the ambient value. A race with
    /// no table loaded ignores it.
    pub fn set_finished_thrust_skill(&mut self, skill: f32) {
        if let Some(thrust) = self.sim.finished_thrust.as_mut() {
            thrust.skill = skill;
        }
    }

    /// The law's thrust cap for `slot`, a fraction of full, once that craft
    /// has finished; `None` before the line or with no table loaded.
    #[must_use]
    pub fn finished_thrust_cap(&self, slot: usize) -> Option<f32> {
        let thrust = self.sim.finished_thrust?;
        if !self.sim.world.ships[slot].standing.finished() {
            return None;
        }
        Some(thrust.fraction(self.places()[slot]))
    }
}

#[cfg(test)]
mod tests;
