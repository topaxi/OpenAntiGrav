//! A score Eliminator's awards: what a craft earns for damage, a kill and a lap, and what it loses
//! when it is destroyed. `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`.

use crate::Standing;

/// The awards of one race, a plain value resolved once at load. `None` on the race means its
/// Eliminator is won on kills.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreLaw {
    /// Points per point of damage dealt.
    pub damage: f32,
    /// Points for a kill.
    pub kill: f32,
    /// Points for a lap.
    pub lap: f32,
    /// Points a destroyed craft loses. Not scaled.
    pub death: f32,
    /// What an opponent's awards are multiplied by; a person's are not.
    pub opponent_scale: f32,
}

impl ScoreLaw {
    /// The multiplier of a craft's own awards.
    #[must_use]
    pub fn scale(&self, person: bool) -> f32 {
        if person { 1.0 } else { self.opponent_scale }
    }

    /// Credits `amount` of damage to the attacker.
    pub fn credit_damage(&self, attacker: &mut Standing, person: bool, amount: f32) {
        attacker.score += self.damage * amount * self.scale(person);
    }

    /// Credits a kill's points. The fatal blow's damage points are its own.
    pub fn credit_kill(&self, attacker: &mut Standing, person: bool) {
        attacker.score += self.kill * self.scale(person);
    }

    /// Credits a completed lap.
    pub fn credit_lap(&self, craft: &mut Standing, person: bool) {
        craft.score += self.lap * self.scale(person);
    }

    /// Takes a destroyed craft's points, never below zero.
    pub fn charge_death(&self, craft: &mut Standing) {
        craft.score -= self.death.min(craft.score);
    }
}

impl Standing {
    /// The score as the race reads it: the rows print this and the target compares it, both
    /// truncated (`fctidz` in `Hud_UpdateEliminatorRows` and `SPElimination_UpdateTarget`).
    #[must_use]
    pub fn whole_score(&self) -> u32 {
        // Truncates, and a score is never negative.
        self.score as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAW: ScoreLaw = ScoreLaw {
        damage: 0.1,
        kill: 20.0,
        lap: 5.0,
        death: 5.0,
        opponent_scale: 1.15,
    };

    #[test]
    fn a_person_earns_each_award_flat() {
        let mut craft = Standing::default();
        LAW.credit_damage(&mut craft, true, 30.0);
        assert!((craft.score - 3.0).abs() < 1e-5);
        LAW.credit_kill(&mut craft, true);
        assert!((craft.score - 23.0).abs() < 1e-5);
        LAW.credit_lap(&mut craft, true);
        assert!((craft.score - 28.0).abs() < 1e-5);
    }

    #[test]
    fn an_opponent_earns_each_award_scaled() {
        let mut craft = Standing::default();
        LAW.credit_damage(&mut craft, false, 80.0);
        assert!((craft.score - 9.2).abs() < 1e-5);
        LAW.credit_kill(&mut craft, false);
        LAW.credit_lap(&mut craft, false);
        assert!((craft.score - 37.95).abs() < 1e-4);
    }

    #[test]
    fn a_death_costs_five_flat_and_stops_at_zero() {
        let mut craft = Standing {
            score: 41.4,
            ..Standing::default()
        };
        LAW.charge_death(&mut craft);
        assert!((craft.score - 36.4).abs() < 1e-5);
        craft.score = 3.0;
        LAW.charge_death(&mut craft);
        assert_eq!(craft.score.to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn the_whole_score_truncates() {
        let craft = Standing {
            score: 199.99,
            ..Standing::default()
        };
        assert_eq!(craft.whole_score(), 199);
    }
}
