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

/// One craft of a finished Eliminator, as the end-of-race builder sorts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finisher {
    /// The score the race is won on (whole points on a score race, kills on a kill race).
    pub points: u32,
    /// Kills, the first tie-break.
    pub kills: u32,
    /// Whether a person flies it, the second.
    pub person: bool,
}

/// Each craft's 1-based place on the end-of-race board: `result[craft]`, with `field` in grid
/// order.
///
/// The end-of-race builder's bubble sort (`0x0005ac00`, `docs/ghidra/functions/ps3-hdfury-eu/
/// cell-medal-award.md`, "End of race", confidence 72): a higher score ranks first; on equal
/// score **fewer kills** rank first; on equal score and kills a person ranks above an opponent.
/// It swaps only on a strict difference, so craft still equal keep grid order (the lower slot
/// first). This is not [`ranked`], the HUD's rows (the later slot on top, kills ignored): the
/// place a campaign cell's medal is read from is this one.
#[must_use]
pub fn final_places(field: &[Finisher]) -> Vec<u8> {
    let above = |a: &Finisher, b: &Finisher| {
        a.points > b.points
            || (a.points == b.points
                && (a.kills < b.kills || (a.kills == b.kills && a.person && !b.person)))
    };
    field
        .iter()
        .enumerate()
        .map(|(index, mine)| {
            let ahead = field
                .iter()
                .enumerate()
                .filter(|&(other, theirs)| {
                    other != index
                        && (above(theirs, mine) || (!above(mine, theirs) && other < index))
                })
                .count();
            u8::try_from(ahead + 1).unwrap_or(u8::MAX)
        })
        .collect()
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

    fn finisher(points: u32, kills: u32, person: bool) -> Finisher {
        Finisher {
            points,
            kills,
            person,
        }
    }

    #[test]
    fn a_higher_score_places_first() {
        let field = [
            finisher(90, 0, false),
            finisher(177, 3, false),
            finisher(177, 3, true),
        ];
        assert_eq!(final_places(&field), [3, 2, 1]);
    }

    /// Equal score: the craft with fewer kills is above, so a player who tied an opponent on
    /// points with more kills is placed second, not first.
    #[test]
    fn on_equal_score_fewer_kills_place_first() {
        let field = [finisher(177, 1, false), finisher(177, 4, true)];
        assert_eq!(final_places(&field), [1, 2]);
        let field = [finisher(177, 4, false), finisher(177, 1, true)];
        assert_eq!(final_places(&field), [2, 1]);
    }

    /// Equal score and kills: the person is above an opponent even from a later slot.
    #[test]
    fn on_equal_score_and_kills_a_person_is_above() {
        let field = [
            finisher(50, 2, false),
            finisher(50, 2, false),
            finisher(50, 2, true),
        ];
        assert_eq!(final_places(&field), [2, 3, 1]);
    }

    /// Two opponents equal in everything keep grid order, and the places are always a
    /// permutation: no two craft share one, unlike the board's shown number.
    #[test]
    fn places_are_distinct_and_full_ties_keep_grid_order() {
        let field = [
            finisher(0, 0, false),
            finisher(0, 0, false),
            finisher(0, 0, false),
        ];
        assert_eq!(final_places(&field), [1, 2, 3]);
        assert!(final_places(&[]).is_empty());
    }
}
