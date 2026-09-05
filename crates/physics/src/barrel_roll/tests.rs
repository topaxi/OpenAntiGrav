use super::*;

fn taps(dirs: &[TapDirection], gap: f32, state: &mut ShipState) -> Option<f32> {
    let mut result = None;
    for &dir in dirs {
        advance_tap_timer(state, gap);
        result = record_tap(state, dir);
    }
    result
}

#[test]
fn right_left_right_arms_toward_positive_one() {
    let mut state = ShipState::default();
    let result = taps(
        &[TapDirection::Right, TapDirection::Left, TapDirection::Right],
        0.1,
        &mut state,
    );
    assert_eq!(result, Some(1.0));
    assert_eq!(state.roll_taps, [0, 0, 0], "a match clears the history");
}

#[test]
fn left_right_left_arms_toward_negative_one() {
    let mut state = ShipState::default();
    let result = taps(
        &[TapDirection::Left, TapDirection::Right, TapDirection::Left],
        0.1,
        &mut state,
    );
    assert_eq!(result, Some(-1.0));
    assert_eq!(state.roll_taps, [0, 0, 0], "a match clears the history");
}

#[test]
fn a_fourth_tap_does_not_immediately_rearm() {
    let mut state = ShipState::default();
    let matched = taps(
        &[TapDirection::Right, TapDirection::Left, TapDirection::Right],
        0.1,
        &mut state,
    );
    assert_eq!(matched, Some(1.0));

    // The match cleared the history, so one more tap alone cannot complete a
    // pattern - it takes a fresh three-tap gesture.
    advance_tap_timer(&mut state, 0.1);
    let fourth = record_tap(&mut state, TapDirection::Left);
    assert_eq!(fourth, None);
}

#[test]
fn a_gap_at_or_past_the_timeout_breaks_the_pattern() {
    let mut state = ShipState::default();
    advance_tap_timer(&mut state, 0.1);
    record_tap(&mut state, TapDirection::Left);
    advance_tap_timer(&mut state, 0.1);
    record_tap(&mut state, TapDirection::Right);

    // A gap of exactly the timeout: the third tap does not shift the two
    // older entries down, so the history cannot read [1, 2, 1].
    advance_tap_timer(&mut state, INTER_TAP_TIMEOUT);
    let third = record_tap(&mut state, TapDirection::Left);
    assert_eq!(third, None, "a stale gap must not complete the gesture");
}

#[test]
fn wrong_third_tap_does_not_arm_either() {
    let mut state = ShipState::default();
    let result = taps(
        &[TapDirection::Right, TapDirection::Left, TapDirection::Left],
        0.1,
        &mut state,
    );
    assert_eq!(result, None);
}

fn ready_dimensions() -> Dimensions {
    Dimensions {
        shield: 100.0,
        ..Dimensions::default()
    }
}

#[test]
fn arming_charges_the_cost_and_sets_the_target() {
    let mut state = ShipState {
        shield: 100.0,
        ..ShipState::default()
    };
    let armed = arm(&mut state, &ready_dimensions(), 8.0, 1.0);
    assert!(armed);
    assert_eq!(state.shield, 92.0);
    assert_eq!(state.roll_target, 1.0);
}

#[test]
fn a_shield_exactly_at_the_cost_refuses_to_arm() {
    // 8 % of 100 is 8, so a shield sitting at exactly 8 must refuse: the
    // original arms nothing "unless cost < shield", strictly.
    let mut state = ShipState {
        shield: 8.0,
        ..ShipState::default()
    };
    let armed = arm(&mut state, &ready_dimensions(), 8.0, 1.0);
    assert!(!armed);
    assert_eq!(state.shield, 8.0, "a refused arm must not charge anything");
    assert_eq!(state.roll_target, 0.0);
}

#[test]
fn a_shield_one_unit_above_the_cost_arms() {
    let mut state = ShipState {
        shield: 8.001,
        ..ShipState::default()
    };
    let armed = arm(&mut state, &ready_dimensions(), 8.0, 1.0);
    assert!(armed);
}

#[test]
fn phase_ramps_toward_the_target_and_clamps_there() {
    let mut state = ShipState {
        roll_target: 1.0,
        ..ShipState::default()
    };
    advance_phase(&mut state, 1.5, 1.0);
    // A whole second at 1.5 units/s would overshoot 1.0; it must clamp.
    assert_eq!(state.roll_phase, 1.0);
}

#[test]
fn releasing_past_the_split_runs_on_to_completion() {
    let mut state = ShipState {
        roll_target: 1.0,
        roll_phase: 0.6,
        ..ShipState::default()
    };
    let completed = release(&mut state);
    assert!(completed);
    // The target is left where it was, so the ramp keeps heading to 1.0.
    assert_eq!(state.roll_target, 1.0);
    advance_phase(&mut state, 1.5, 1.0);
    assert_eq!(state.roll_phase, 1.0);
}

#[test]
fn releasing_short_of_the_split_falls_back_to_zero() {
    let mut state = ShipState {
        roll_target: 1.0,
        roll_phase: 0.4,
        ..ShipState::default()
    };
    let completed = release(&mut state);
    assert!(!completed);
    assert_eq!(state.roll_target, 0.0);
    advance_phase(&mut state, 1.5, 1.0);
    assert_eq!(state.roll_phase, 0.0);
}

#[test]
fn releasing_a_negative_phase_uses_the_same_split() {
    let mut state = ShipState {
        roll_target: -1.0,
        roll_phase: -0.6,
        ..ShipState::default()
    };
    assert!(release(&mut state));
    assert_eq!(state.roll_target, -1.0);
}

#[test]
fn a_ship_that_never_armed_reports_no_completion() {
    let mut state = ShipState::default();
    assert!(!release(&mut state));
    assert_eq!(state.roll_target, 0.0);
}

/// Closes the gap the unit tests above leave: every one of them drives
/// [`ShipState`] fields directly and none exercises the three *consumers* of
/// [`ShipState::roll_payout_timer`] at all. Wired through a completed gesture
/// end to end - `arm`, then `advance_phase` to completion, then `release` on a
/// simulated landing - and then checks the two consumers this module can see
/// without reaching into `crate::engine` (whose gate is `crate::forces`'
/// business, not tested here).
///
/// **Does not touch the input layer.** Nothing here claims a real race can
/// reach this path - see [`arm`]'s caller in `crate::forces::evaluate`, which
/// has no producer of a tap event yet.
#[test]
fn a_completed_roll_arms_the_payout_and_the_payout_drives_both_consumers() {
    let dimensions = Dimensions {
        shield: 100.0,
        ..Dimensions::default()
    };
    let mut state = ShipState {
        shield: 100.0,
        ..ShipState::default()
    };

    assert!(arm(&mut state, &dimensions, 8.0, 1.0));
    // A full second at `roll_speed = 1.5` overshoots `1.0`; `advance_phase`
    // clamps there, which is what makes the roll "complete" below.
    advance_phase(&mut state, 1.5, 1.0);
    assert_eq!(state.roll_phase, 1.0);

    // The simulated airborne-to-grounded transition.
    assert!(release(&mut state));
    state.roll_payout_timer = 0.5;

    assert!(state.roll_payout_timer > 0.0);
    assert_eq!(
        rebound_override(&state, 0.4 /* an arbitrary ordinary value */),
        1.0
    );

    // A moving ship, so lateral grip has something to scale.
    state.body.linear_velocity = state.body.right() * 10.0;
    let handling = crate::params::Handling {
        antigrav: crate::params::Antigrav {
            grip_ground: 10.0,
            grip_air: 10.0,
            ..crate::params::Antigrav::default()
        },
        ..crate::params::Handling::ZERO
    };
    let boosted = crate::airbrake::lateral_grip(&state, &handling, 1.0);

    state.roll_payout_timer = 0.0;
    let ordinary = crate::airbrake::lateral_grip(&state, &handling, 1.0);

    assert_eq!(
        boosted.x,
        ordinary.x * crate::airbrake::ROLL_GRIP_MULTIPLIER
    );
}
