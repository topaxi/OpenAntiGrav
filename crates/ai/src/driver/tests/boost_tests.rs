//! [`Driver::affords_boost`]: the gate an opponent's Turbo is fired on.

use super::*;
use crate::SpeedPlan;

/// `peak` for `ticks` from the start of `line`, on `plan`, with this
/// project's tuning and no hull ceiling.
fn affords(line: &Line, plan: Option<&SpeedPlan>, peak: f32, ticks: f32) -> bool {
    let tuning = Tuning::default();
    Driver::default().affords_boost(
        &Context {
            line,
            tuning: &tuning,
            pilot: &Pilot::BALANCED,
            field: &Field::EMPTY,
            yaw_ceiling: None,
            plan,
        },
        peak,
        ticks,
    )
}

#[test]
fn a_straight_with_no_braking_ahead_affords_a_boost() {
    let line = straight();
    let plan = SpeedPlan::unlimited(&line);
    assert!(affords(&line, Some(&plan), 300.0, 60.0));
    assert!(affords(&line, None, 300.0, 60.0));
}

/// The plan's braking zone is the refusal the corner model alone missed: a
/// straight line with a target of 80 halfway down it.
#[test]
fn a_braking_zone_inside_the_boosted_stretch_refuses_it() {
    let line = straight();
    let plan = SpeedPlan::unlimited(&line).with_target(30, 80.0);
    // 300 for 60 ticks covers 300 units, sample 30 is 300 units in.
    assert!(!affords(&line, Some(&plan), 300.0, 61.0));
    // Short of it, the same plan affords the boost.
    assert!(affords(&line, Some(&plan), 300.0, 30.0));
}

/// The corner model still refuses where a plan reads unbounded: a plan's
/// target is infinite wherever its search never had to slow.
#[test]
fn a_corner_refuses_a_boost_even_on_an_unbounded_plan() {
    let line = right_hand_corner(60.0);
    let plan = SpeedPlan::unlimited(&line);
    assert!(!affords(&line, Some(&plan), 300.0, 30.0));
}
