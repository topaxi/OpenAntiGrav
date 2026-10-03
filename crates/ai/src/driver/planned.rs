//! What a driver takes from a speed plan: how much of it, and how much of its
//! own lateral character it may spend on top. See [`crate::plan`] for the plan
//! itself and `docs/gameplay/ai.md`, "The speed plan", for the measurements.

use super::{Context, Personality, Tuning};

/// The timestep the plan's lookahead is converted at. ADR-0007's fixed 60 Hz:
/// a driver is never told its `dt`, and the simulation never runs at another.
pub(super) const PLAN_DT: f32 = 1.0 / 60.0;

/// How much of the speed plan a driver of this level and character takes.
///
/// **The same ladder the corner model's grip belief was**, converted the same
/// way: `Difficulty::tune` scales `lateral_accel` by the level's grip belief,
/// and a corner speed goes as its square root, so the plan is scaled by
/// `sqrt(grip_believed * commitment)` - 0.55, 0.69, 0.84 and 1.00 of the plan
/// from Novice to Ace for a balanced pilot, exactly the corner-speed fractions
/// `docs/gameplay/ai.md`'s difficulty table already promises. **Capped at
/// one**: a pilot drawn above a commitment of one would ask for more than the
/// craft was seen to manage, and the plan is that limit. Chosen, not measured.
#[must_use]
pub(super) fn plan_margin(tuning: &Tuning, personality: &Personality) -> f32 {
    let belief = tuning.lateral_accel / Tuning::default().lateral_accel;
    (belief * personality.commitment).clamp(0.0, 1.0).sqrt()
}

/// The gap between an Ace's and a Novice's share of the plan, `1 - sqrt(0.30)`:
/// the denominator that gives a Novice its whole lateral character back.
const NOVICE_GAP: f32 = 0.452_277_25;

/// How much of its lateral character - the line bias, the wander, the inside
/// line - a driver spends on a plan.
///
/// **A handicap, scaled with the plan margin**: one when there is no plan (the
/// corner model brakes with its own margin), and on a plan `(1 - margin)` over
/// [`NOVICE_GAP`], so a balanced Ace holds the plan's line exactly, an Elite
/// spends a third of its character, a Skilled two thirds and a Novice all of
/// it. The plan is the speed the craft carries *on* the line; a pilot holding
/// its own part of the corridor at that speed is a pilot into the wall.
/// Measured on the 96 lone rows: the plan with full character was clean on
/// 15, with it gated on the plan's slack ahead (three settings) on 15-23, and
/// with none on 45; the field went 41 destroyed to 24. Chosen, not measured,
/// beyond those three points.
pub(super) fn plan_slack(ctx: &Context<'_>, personality: &Personality) -> f32 {
    if !ctx.plan.is_some_and(|plan| plan.len() == ctx.line.len()) {
        return 1.0;
    }
    ((1.0 - plan_margin(ctx.tuning, personality)) / NOVICE_GAP).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Difficulty;

    #[test]
    fn the_plan_margin_is_the_difficulty_ladder_in_corner_speed() {
        // `ai.md`'s difficulty table: 0.55, 0.69, 0.84, 1.00 of the measured
        // corner speed from Novice to Ace, for a balanced pilot.
        let expected = [0.55f32, 0.69, 0.84, 1.00];
        for ((_, level), want) in Difficulty::ALL.iter().zip(expected) {
            let tuning = level.tune(&Tuning::default());
            let margin = plan_margin(&tuning, &Personality::NEUTRAL);
            assert!((margin - want).abs() < 0.01, "{level:?}: {margin}");
        }
    }

    #[test]
    fn no_pilot_is_handed_more_than_the_plan() {
        let personality = Personality {
            commitment: 1.05,
            ..Personality::NEUTRAL
        };
        assert_eq!(plan_margin(&Tuning::default(), &personality), 1.0);
    }
}
