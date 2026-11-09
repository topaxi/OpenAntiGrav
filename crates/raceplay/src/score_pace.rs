//! How a score Eliminator's opponents spend their weapons, so a race reaches its target at about
//! the original's length (`docs/gameplay/race-modes.md`, "The pace of a score Eliminator").
//!
//! **Chosen, not measured.** The original's Novice race on Vineta K took ten laps to reach 200
//! (`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`); the terms below hold ours near it.
//! Opponent behaviour is a design axis: these are tuned for a race, not read off the original.
//!
//! What it changes, in a race whose Eliminator is won on points and nowhere else:
//!
//! - **Trigger.** Every tier fires its Mines, Bombs and Cannon at [`TRIGGER_SHARE`] of its pilot's
//!   own appetite. Novice's appetite is otherwise zero (`oag_ai::Difficulty::aggression`), and a
//!   Novice that picked one up held it for the rest of the race, so its slot never took another pad.
//! - **Hold.** An opponent keeps a fresh pickup [`ScorePace::hold_ticks`] before it may spend it
//!   at all (fire, drop, absorb, Turbo, Autopilot), on top of the measured `0.8` s gate
//!   (`oag_ai::weapon_ai::HOLD_TICKS`). A held weapon blocks the next pad, so this paces supply; ours
//!   otherwise spent most pickups within about two seconds. Only the Hard rung waits: its authored
//!   `absorbScale` (`0.6`) leaves it absorbing little.
//! - **Absorb.** The measured decision's absorb half spends the weapon on the Eliminator's Shield,
//!   the player's own path (`Race::eliminator_absorb`), instead of keeping it.
//! - **No pad seeking and no pack.** `PadSeeking::EmptySlot` and `Race::eliminator_pack_scale` are
//!   ours, tuned for Pulse's five-kill race; on a 200-point race they doubled the supply and the
//!   pack's density.

use oag_tables::race_campaign::Difficulty;

/// The share of a pilot's own trigger span a score Eliminator's opponent fires its dropped and held
/// weapons at, on every tier. **Chosen, not measured.**
pub const TRIGGER_SHARE: f32 = 0.2;

/// Seconds the Hard rung's opponent keeps a fresh pickup before it may spend it. **Chosen, not
/// measured.**
pub const HARD_HOLD_SECONDS: u32 = 20;

/// The terms a score Eliminator's opponents spend weapons on; `None` on every other race.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScorePace {
    /// Ticks a fresh pickup is kept before it may be spent.
    pub hold_ticks: u32,
}

impl ScorePace {
    /// The terms at `difficulty`, by the original's rung.
    #[must_use]
    pub fn at(difficulty: oag_ai::Difficulty) -> Self {
        let hold_seconds = match crate::finished_thrust::rung(difficulty) {
            Difficulty::Easy | Difficulty::Medium => 0,
            Difficulty::Hard => HARD_HOLD_SECONDS,
        };
        Self {
            hold_ticks: hold_seconds * 60,
        }
    }

    /// The race's terms: present only where the Eliminator is won on points.
    #[must_use]
    pub fn of(
        score_law: Option<oag_race::ScoreLaw>,
        difficulty: oag_ai::Difficulty,
    ) -> Option<Self> {
        score_law.map(|_| Self::at(difficulty))
    }

    /// An opponent flying `own` at `difficulty`: the level's own temper, then `pace`'s trigger in a
    /// score race. Every site that hands an opponent its pilot goes through this.
    #[must_use]
    pub fn temper_opponent(
        pace: Option<Self>,
        difficulty: oag_ai::Difficulty,
        own: &oag_ai::Pilot,
    ) -> oag_ai::Pilot {
        let tempered = difficulty.temper(own);
        pace.map_or(tempered, |pace| pace.temper(own, tempered))
    }

    /// `tempered` with its trigger set to [`TRIGGER_SHARE`] of `own`'s, the pilot before any
    /// difficulty scaled it.
    #[must_use]
    pub fn temper(self, own: &oag_ai::Pilot, tempered: oag_ai::Pilot) -> oag_ai::Pilot {
        oag_ai::Pilot {
            trigger: oag_ai::Span::new(
                own.trigger.low * TRIGGER_SHARE,
                own.trigger.high * TRIGGER_SHARE,
            ),
            ..tempered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_score_race_has_terms() {
        assert_eq!(ScorePace::of(None, oag_ai::Difficulty::Novice), None);
        let law = oag_race::ScoreLaw {
            damage: 0.1,
            kill: 20.0,
            lap: 5.0,
            death: 5.0,
            opponent_scale: 1.15,
        };
        assert!(ScorePace::of(Some(law), oag_ai::Difficulty::Novice).is_some());
    }

    #[test]
    fn only_the_hard_rung_waits() {
        assert_eq!(ScorePace::at(oag_ai::Difficulty::Novice).hold_ticks, 0);
        assert_eq!(ScorePace::at(oag_ai::Difficulty::Skilled).hold_ticks, 0);
        assert_eq!(ScorePace::at(oag_ai::Difficulty::Elite).hold_ticks, 1200);
        assert_eq!(ScorePace::at(oag_ai::Difficulty::Ace).hold_ticks, 1200);
    }

    #[test]
    fn a_novice_opponent_has_a_trigger_in_a_score_race() {
        let own = oag_ai::Pilot::BALANCED;
        let tempered = oag_ai::Difficulty::Novice.temper(&own);
        assert_eq!(
            tempered.trigger.high, 0.0,
            "Novice's own temper fires nothing"
        );
        let paced = ScorePace::at(oag_ai::Difficulty::Novice).temper(&own, tempered);
        assert!(paced.trigger.high > 0.0);
        assert_eq!(paced.ram, tempered.ram, "only the trigger moves");
    }
}
