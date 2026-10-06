//! What a driver takes from a speed plan: how much of it, and how much of its
//! own lateral character it may spend on top. See [`crate::plan`] and
//! `docs/gameplay/ai.md`, "The speed plan".

use super::{Context, Personality, Tuning};
use crate::SpeedPlan;

/// The timestep the plan's lookahead is converted at: ADR-0007's fixed 60 Hz (a
/// driver is never told its `dt`).
pub(super) const PLAN_DT: f32 = 1.0 / 60.0;

/// How much of the speed plan a driver of this level and character takes.
///
/// **The corner model's grip-belief ladder, converted the same way**: a corner
/// speed goes as the square root of `lateral_accel`, so the plan is scaled by
/// `sqrt(grip_believed * commitment)`, 0.55, 0.69, 0.84 and 1.00 from Novice to
/// Ace for a balanced pilot (`docs/gameplay/ai.md`'s difficulty table).
/// **Capped at one**: above it a pilot would ask for more than the craft was
/// seen to manage. Chosen, not measured.
#[must_use]
pub(super) fn plan_margin(tuning: &Tuning, personality: &Personality) -> f32 {
    let belief = tuning.lateral_accel / Tuning::default().lateral_accel;
    (belief * personality.commitment).clamp(0.0, 1.0).sqrt()
}

/// The gap between an Ace's and a Novice's share of the plan, `1 - sqrt(0.30)`:
/// the denominator that gives a Novice its whole lateral character back.
const NOVICE_GAP: f32 = 0.452_277_25;

/// How much of its lateral character (line bias, wander, inside line) a driver
/// spends on a plan.
///
/// **A handicap, scaled with the plan margin**: one with no plan, and on a plan
/// `(1 - margin)` over [`NOVICE_GAP`], so a balanced Ace holds the plan's line
/// exactly, an Elite spends a third, a Skilled two thirds, a Novice all. The
/// plan is the speed the craft carries *on* the line; a pilot holding its own
/// part of the corridor at that speed is a pilot into the wall. On the 96 lone
/// rows full character was clean on 15, gated on the plan's slack ahead (three
/// settings) on 15-23, and with none on 45; the field went 41 destroyed to 24.
/// Chosen, not measured, beyond those three points.
pub(super) fn plan_slack(ctx: &Context<'_>, personality: &Personality) -> f32 {
    if !ctx.plan.is_some_and(|plan| plan.len() == ctx.line.len()) {
        return 1.0;
    }
    let level = ((1.0 - plan_margin(ctx.tuning, personality)) / NOVICE_GAP).clamp(0.0, 1.0);
    level.max(traffic(ctx))
}

/// How close the nearest rival this driver has noticed is: one when touching,
/// zero at [`TRAFFIC_RANGE`] or with nobody about.
///
/// **In traffic a pilot is a pilot again**: a field of Aces holding the plan's
/// one line at its one pace runs nose to tail and sticks
/// (`craft_sticking_ground_truth`: 2,101 overlapped pair-ticks, 1,420
/// sustained, past the old pathology's 1,062), so character returns as rivals
/// close in, and a lone craft still drives the plan's line. Chosen, not
/// measured. [`TRAFFIC_RANGE`]: the gap inside which that happens, **chosen, not
/// measured**.
const TRAFFIC_RANGE: f32 = 40.0;

fn traffic(ctx: &Context<'_>) -> f32 {
    [ctx.field.ahead, ctx.field.behind, ctx.field.alongside]
        .into_iter()
        .flatten()
        .map(|rival| ((TRAFFIC_RANGE - rival.gap.abs()) / TRAFFIC_RANGE).clamp(0.0, 1.0))
        .fold(0.0, f32::max)
}

/// How far ahead of a takeoff run-up a lower level stops holding its share of
/// the plan's pace: room for a craft held to 0.88 of it to reach full speed,
/// two to three seconds near the top end. **Chosen, not measured.**
pub(super) const RUN_UP_REACH: f32 = 450.0;

/// The least share of the plan's pace a level holds on the way into a jump. The
/// field's takeoffs at `05_Track` forward's first jump (VENOM, three seeds)
/// fell short at 119-120 units/s and cleared from 120.4, against the plan's
/// 128; this is Skilled's own share, 123. **Chosen from that measurement, not
/// measured off the original.**
const RUN_UP_SHARE: f32 = 0.96;

/// The speed a driver of this level and character holds at `index` on `plan`:
/// the plan's lowest target over the airbrake ramp ahead, times [`plan_margin`],
/// and for lower levels no more than `Tuning::pace_share` of the plan's pace.
pub(super) fn target(
    plan: &SpeedPlan,
    index: usize,
    speed: f32,
    tuning: &Tuning,
    personality: &Personality,
    run_up: bool,
) -> f32 {
    let corners = plan.target_ahead(
        index,
        speed,
        crate::plan::LEAD_TICKS * personality.patience,
        PLAN_DT,
    ) * plan_margin(tuning, personality);
    // The level's share of the plan's own pace, which reaches the straights; at
    // one it is no cap, so the top two levels drive the plan.
    // **Not much below the plan on the way into a jump**: a share of the pace
    // before a gap arrives short of the far lip (a Novice at VENOM on `05_Track`
    // forward, held to 0.88 of 128 units/s, hit the lip at sample 207 every
    // lap). On a run-up the share is at least [`RUN_UP_SHARE`], not lifted to
    // one: a Skilled craft landing at the plan's full pace at PHANTOM could not
    // brake to its corner margin and left the road at samples 330-410. Chosen,
    // not measured.
    let share = if run_up {
        tuning.pace_share.max(RUN_UP_SHARE)
    } else {
        tuning.pace_share
    };
    if share < 1.0 {
        corners.min(plan.pace(index) * share)
    } else {
        corners
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Difficulty;

    #[test]
    fn the_plan_margin_is_the_difficulty_ladder_in_corner_speed() {
        // `ai.md`'s difficulty table: 0.55, 0.69, 0.84, 1.00 of the measured
        // corner speed, Novice to Ace, balanced pilot.
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
