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

/// A ship with a shield large enough for several rolls.
fn armable() -> (ShipState, Dimensions) {
    let dimensions = Dimensions {
        shield: 100.0,
        ..Dimensions::default()
    };
    let state = ShipState {
        shield: 100.0,
        ..ShipState::default()
    };
    (state, dimensions)
}

/// One tick of [`advance_gesture`] at a fixed 60 Hz, **airborne**, with the
/// axis where the caller put it and the d-pad where the caller put it.
fn gesture_tick(
    state: &mut ShipState,
    dimensions: &Dimensions,
    steer_x: f32,
    dpad: Option<TapDirection>,
) -> bool {
    airborne_tick(state, dimensions, steer_x, dpad, false)
}

/// The same, with [`ShipControls::computer_driven`] under the caller's control.
fn airborne_tick(
    state: &mut ShipState,
    dimensions: &Dimensions,
    steer_x: f32,
    dpad: Option<TapDirection>,
    computer_driven: bool,
) -> bool {
    let input = ShipControls {
        steer_x,
        roll_tap_left: dpad == Some(TapDirection::Left),
        roll_tap_right: dpad == Some(TapDirection::Right),
        computer_driven,
        ..ShipControls::default()
    };
    advance_gesture(state, &input, dimensions, 8.0, false, 1.0 / 60.0)
}

/// Three airborne taps from one driver kind, and whether they armed.
fn three_taps(computer_driven: bool, shield: f32) -> (bool, ShipState) {
    let (mut state, dimensions) = armable();
    state.shield = shield;
    let mut armed = false;
    for dir in [TapDirection::Right, TapDirection::Left, TapDirection::Right] {
        armed = airborne_tick(&mut state, &dimensions, 0.0, Some(dir), computer_driven);
    }
    (armed, state)
}

/// The invented AI-only floor: an AI craft under
/// [`AI_ROLL_SHIELD_FLOOR`] of its pool does not arm, and a human on the same
/// shield does.
///
/// **This pins a choice, not a finding** - see the constant's own doc comment.
/// The asymmetry is the whole point of the test: if a change ever makes the
/// floor apply to a pad as well, the second half fails.
#[test]
fn the_ai_floor_refuses_an_ai_and_not_a_pad() {
    // The pool is 100.0 in `armable`, so the floor is 20.0 and the recovered
    // `cost < shield` gate would still allow this at 8.0.
    let under = AI_ROLL_SHIELD_FLOOR * 100.0 - 1.0;

    let (ai, state) = three_taps(true, under);
    assert!(!ai, "an AI armed a roll below the floor");
    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.shield, under, "and it was not charged");

    let (pad, state) = three_taps(false, under);
    assert!(pad, "the floor reached the human path");
    assert_eq!(state.roll_target, 1.0);
    assert!(state.shield < under, "and the pad paid the recovered cost");
}

/// And above the floor an AI arms exactly as a pad does, so the test above is
/// measuring the floor rather than a driver kind that can never roll.
#[test]
fn an_ai_above_the_floor_arms_like_a_pad() {
    let over = AI_ROLL_SHIELD_FLOOR * 100.0 + 1.0;
    let (ai, ai_state) = three_taps(true, over);
    let (pad, pad_state) = three_taps(false, over);
    assert!(ai && pad);
    assert_eq!(ai_state.roll_target, pad_state.roll_target);
    assert_eq!(ai_state.shield, pad_state.shield);
}

#[test]
fn three_d_pad_taps_arm_a_roll() {
    let (mut state, dimensions) = armable();
    for (i, dir) in [TapDirection::Left, TapDirection::Right, TapDirection::Left]
        .into_iter()
        .enumerate()
    {
        let armed = gesture_tick(&mut state, &dimensions, 0.0, Some(dir));
        assert_eq!(armed, i == 2, "only the third tap completes the pattern");
    }
    assert_eq!(state.roll_target, -1.0, "left-right-left rolls to -1");
    assert_eq!(state.shield, 92.0, "8% of a 100 shield pool");
}

#[test]
fn three_axis_crossings_arm_a_roll_with_no_button_at_all() {
    let (mut state, dimensions) = armable();
    // Back to centre between each push, so each one is a fresh crossing.
    let mut armed = false;
    for push in [1.0, -1.0, 1.0] {
        armed = gesture_tick(&mut state, &dimensions, push, None);
        gesture_tick(&mut state, &dimensions, 0.0, None);
    }
    assert!(armed, "right-left-right arms on the third crossing");
    assert_eq!(state.roll_target, 1.0);
}

/// The d-pad and the axis are one signal, not two.
///
/// This project's input layer drives the analog axis from the d-pad as well
/// (`oag_input::pad::larger`), so a real d-pad press arrives as a button edge
/// *and* an axis crossing on the same tick. Recording both would shift the
/// history twice and leave a doubled direction in it that can never match an
/// alternation - which would make the d-pad leg silently unreachable.
#[test]
fn a_press_that_is_also_a_crossing_records_one_tap() {
    let (mut state, dimensions) = armable();
    gesture_tick(&mut state, &dimensions, -1.0, Some(TapDirection::Left));
    assert_eq!(state.roll_taps, [0, 0, 1], "one entry, not two");
}

/// A thumb rolled straight from one side to the other has no neutral tick.
#[test]
fn a_direct_side_to_side_flip_is_a_tap() {
    let (mut state, dimensions) = armable();
    gesture_tick(&mut state, &dimensions, -1.0, None);
    gesture_tick(&mut state, &dimensions, 1.0, None);
    assert_eq!(state.roll_taps, [0, 1, 2]);
}

/// Holding the axis over is one tap, the way holding the d-pad is.
#[test]
fn a_held_axis_does_not_repeat() {
    let (mut state, dimensions) = armable();
    for _ in 0..10 {
        gesture_tick(&mut state, &dimensions, 1.0, None);
    }
    assert_eq!(state.roll_taps, [0, 0, 2], "one entry for ten held ticks");
}

/// Ordinary cornering is under the threshold and records nothing.
///
/// [`crate::probe`]'s slalom holds the axis at `+-0.8` for exactly this reason,
/// which is what keeps the committed determinism hashes free of armed rolls.
#[test]
fn steering_short_of_the_threshold_is_not_a_tap() {
    let (mut state, dimensions) = armable();
    for push in [0.8, -0.8, 0.8, -0.8] {
        gesture_tick(&mut state, &dimensions, push, None);
        gesture_tick(&mut state, &dimensions, 0.0, None);
    }
    assert_eq!(state.roll_taps, [0, 0, 0]);
    assert_eq!(state.shield, 100.0, "and so nothing was ever charged");
}

/// The gesture advances the inter-tap timer exactly once a tick.
///
/// Calling [`advance_tap_timer`] beside [`advance_gesture`] would double it and
/// halve [`INTER_TAP_TIMEOUT`] without failing anything else.
#[test]
fn the_gesture_advances_the_tap_timer_once_per_tick() {
    let (mut state, dimensions) = armable();
    state.roll_tap_timer = 0.0;
    gesture_tick(&mut state, &dimensions, 0.0, None);
    assert_eq!(state.roll_tap_timer, 1.0 / 60.0);
}

/// An empty shield completes the gesture and arms nothing.
#[test]
fn a_flat_shield_refuses_the_arm_but_still_clears_the_history() {
    let (mut state, dimensions) = armable();
    state.shield = 1.0;
    for dir in [TapDirection::Right, TapDirection::Left, TapDirection::Right] {
        assert!(!gesture_tick(&mut state, &dimensions, 0.0, Some(dir)));
    }
    assert_eq!(state.roll_target, 0.0, "nothing armed");
    assert_eq!(state.shield, 1.0, "and nothing was charged");
    assert_eq!(state.roll_taps, [0, 0, 0], "the gesture still completed");
}

/// One tick of [`advance_gesture`] with the contact bit set.
fn grounded_tick(
    state: &mut ShipState,
    dimensions: &Dimensions,
    dpad: Option<TapDirection>,
) -> bool {
    let input = ShipControls {
        roll_tap_left: dpad == Some(TapDirection::Left),
        roll_tap_right: dpad == Some(TapDirection::Right),
        ..ShipControls::default()
    };
    advance_gesture(state, &input, dimensions, 8.0, true, 1.0 / 60.0)
}

/// The gate that makes an AI craft affordable: a craft on the ground cannot
/// arm, and pays nothing for trying.
///
/// The original zeroes the whole tap history every grounded tick rather than
/// refusing at the arm - see [`advance_gesture`].
#[test]
fn a_grounded_craft_cannot_arm_and_is_not_charged() {
    let (mut state, dimensions) = armable();
    for dir in [TapDirection::Right, TapDirection::Left, TapDirection::Right] {
        assert!(!grounded_tick(&mut state, &dimensions, Some(dir)));
    }
    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.shield, 100.0, "a grounded craft paid for a roll");
    assert_eq!(
        state.roll_taps,
        [0, 0, 0],
        "the history survived the ground"
    );
}

/// And the history is cleared rather than merely ignored, so a gesture cannot
/// span a takeoff: two taps on the ground plus one in the air is not a roll.
#[test]
fn a_gesture_cannot_span_a_takeoff() {
    let (mut state, dimensions) = armable();
    grounded_tick(&mut state, &dimensions, Some(TapDirection::Right));
    grounded_tick(&mut state, &dimensions, Some(TapDirection::Left));
    assert!(!gesture_tick(
        &mut state,
        &dimensions,
        0.0,
        Some(TapDirection::Right)
    ));
    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.shield, 100.0);
}

/// The same three taps in the air do arm, so the test above is measuring the
/// gate and not a broken fixture.
#[test]
fn the_same_three_taps_in_the_air_still_arm() {
    let (mut state, dimensions) = armable();
    gesture_tick(&mut state, &dimensions, 0.0, Some(TapDirection::Right));
    gesture_tick(&mut state, &dimensions, 0.0, Some(TapDirection::Left));
    assert!(gesture_tick(
        &mut state,
        &dimensions,
        0.0,
        Some(TapDirection::Right)
    ));
    assert_eq!(state.roll_target, 1.0);
}

/// A completed pattern levels the phase whether or not the shield could pay -
/// the original's `swc1 f22, 0x87c` sits outside the `cost < shield` test.
#[test]
fn a_completed_pattern_levels_the_phase_even_when_the_arm_is_refused() {
    let (mut state, dimensions) = armable();
    state.shield = 0.0;
    state.roll_phase = 0.4;
    gesture_tick(&mut state, &dimensions, 0.0, Some(TapDirection::Right));
    gesture_tick(&mut state, &dimensions, 0.0, Some(TapDirection::Left));
    assert!(!gesture_tick(
        &mut state,
        &dimensions,
        0.0,
        Some(TapDirection::Right)
    ));
    assert_eq!(state.roll_phase, 0.0);
    assert_eq!(state.roll_target, 0.0);
}

/// **Two rolls in opposite directions are two rotations, not one and then
/// two.**
///
/// The bug this pins was reported from play on 2026-09-06 - "the roll rotates
/// twice where the original rotates once" - and it was invisible to every
/// other test here, because the phase is correct at every single tick in
/// isolation. [`release`] leaves [`ShipState::roll_target`] at `+-1.0` on a
/// completed roll and [`advance_phase`] parks the phase there, which is level
/// on screen (`ease(1.0) = 1.0`, and one turn is one turn). The next roll the
/// *other* way then ramps `+1.0 -> -1.0`, a traversal of `2.0` and so two full
/// turns.
///
/// What fixes it is the original's own `swc1 f22, 0x87c` at `0x08846e5c` /
/// `0x08846f54`: a completed alternation levels the phase before the ramp
/// starts. So the assertion is on the *traversal*, which is what a viewer
/// sees, rather than on the endpoint, which was always right.
#[test]
fn a_second_roll_the_other_way_travels_one_turn_and_not_two() {
    let (mut state, dimensions) = armable();

    for dir in [TapDirection::Right, TapDirection::Left, TapDirection::Right] {
        gesture_tick(&mut state, &dimensions, 0.0, Some(dir));
    }
    assert_eq!(state.roll_target, 1.0);
    // Fly it out to the stop, the way a completed roll ends up.
    for _ in 0..120 {
        advance_phase(&mut state, 1.5, 1.0 / 60.0);
    }
    assert_eq!(state.roll_phase, 1.0, "the first roll finished");

    // Land it: past the split, so `release` completes rather than cancels and
    // leaves the target where it is - which is the state the bug needs.
    assert!(release(&mut state));
    assert_eq!(state.roll_target, 1.0);
    assert_eq!(state.roll_phase, 1.0);

    // Now the opposite gesture.
    for dir in [TapDirection::Left, TapDirection::Right, TapDirection::Left] {
        gesture_tick(&mut state, &dimensions, 0.0, Some(dir));
    }
    assert_eq!(state.roll_target, -1.0, "it armed the other way");
    assert!(
        state.roll_phase <= 0.0,
        "the second roll started from level and not from +1.0, which would \
         draw two turns: {}",
        state.roll_phase
    );

    let mut travelled = 0.0f32;
    let mut previous = state.roll_phase;
    for _ in 0..120 {
        advance_phase(&mut state, 1.5, 1.0 / 60.0);
        travelled += (state.roll_phase - previous).abs();
        previous = state.roll_phase;
    }
    assert_eq!(state.roll_phase, -1.0, "the second roll finished");
    assert!(
        travelled <= 1.0,
        "the second roll travelled {travelled} of phase, and one turn is 1.0"
    );
}
