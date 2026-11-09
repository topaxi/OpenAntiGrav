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

/// The row each craft takes on a ranked list, top to bottom: `result[row]` is an index into
/// `points` (kills on a kill race, whole points on a score race).
///
/// `FUN_0881af38`'s and `Hud_UpdateEliminatorRows`' rule, read off their decompiles and checked
/// against a live frame: a craft's row is `count - 1 - (number of crafts with strictly fewer)`,
/// and a taken row moves the craft up one at a time until a free one turns up, crafts taken in
/// array order. Equal craft therefore stack with the later one on top.
#[must_use]
pub fn ranked(points: &[u32]) -> Vec<usize> {
    let count = points.len();
    let mut rows = vec![usize::MAX; count];
    for (craft, &mine) in points.iter().enumerate() {
        let fewer = points.iter().filter(|&&other| other < mine).count();
        let mut row = count - 1 - fewer;
        while rows[row] != usize::MAX {
            row -= 1;
        }
        rows[row] = craft;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live PPSSPP frame (`AG Systems 1`, `Feisar 1`, `Qirex 1`, `AAA 0`,
    /// `Triakis 0`, `Goteki 45 0`, `Piranha 0`, `EG-X 0`, top to bottom) is the
    /// column this rule produces from a grid array in which the equal-kill craft
    /// sit in the order `EG-X`, `Piranha`, `Goteki 45`, `Triakis`, `AAA` and
    /// `Qirex`, `Feisar`, `AG Systems`: earlier in the array is lower.
    #[test]
    fn equal_craft_stack_with_the_later_craft_on_top() {
        // Array order: 0 EG-X, 1 Piranha, 2 Goteki 45, 3 Triakis, 4 AAA,
        // 5 Qirex, 6 Feisar, 7 AG Systems.
        let kills = [0, 0, 0, 0, 0, 1, 1, 1];
        assert_eq!(ranked(&kills), [7, 6, 5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn more_kills_rank_higher_whatever_the_array_order() {
        // Craft 2 leads on three, craft 0 and 3 tie on one, craft 1 has none.
        assert_eq!(ranked(&[1, 0, 3, 1]), [2, 3, 0, 1]);
    }

    #[test]
    fn a_lone_craft_is_row_zero() {
        assert_eq!(ranked(&[4]), [0]);
        assert!(ranked(&[]).is_empty());
    }

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
