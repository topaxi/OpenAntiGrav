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
        roll_armed: true,
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
        roll_armed: true,
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
        roll_armed: true,
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

/// Closes the gap the unit tests above leave: they drive [`ShipState`] fields directly and none
/// exercises the three *consumers* of [`ShipState::roll_payout_timer`]. Wired through a completed
/// gesture end to end (`arm`, `advance_phase` to completion, `release` on a simulated landing),
/// then checks the two consumers this module can see without `crate::engine`. **Does not touch
/// the input layer**: nothing claims a real race reaches this path ([`arm`]'s caller in
/// `crate::forces::evaluate` has no tap-event producer yet).
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
    airborne_tick(state, dimensions, steer_x, dpad, 0.0)
}

/// The same, with [`ShipControls::roll_shield_floor`] under the caller's
/// control.
fn airborne_tick(
    state: &mut ShipState,
    dimensions: &Dimensions,
    steer_x: f32,
    dpad: Option<TapDirection>,
    roll_shield_floor: f32,
) -> bool {
    let input = ShipControls {
        steer_x,
        roll_tap_left: dpad == Some(TapDirection::Left),
        roll_tap_right: dpad == Some(TapDirection::Right),
        roll_shield_floor,
        ..ShipControls::default()
    };
    advance_gesture(state, &input, dimensions, 8.0, false, 1.0 / 60.0)
}

/// Three airborne taps against one shield floor, and whether they armed.
fn three_taps(roll_shield_floor: f32, shield: f32) -> (bool, ShipState) {
    let (mut state, dimensions) = armable();
    state.shield = shield;
    let mut armed = false;
    for dir in [TapDirection::Right, TapDirection::Left, TapDirection::Right] {
        armed = airborne_tick(&mut state, &dimensions, 0.0, Some(dir), roll_shield_floor);
    }
    (armed, state)
}

/// The floor an AI carries and a pad does not: the same gesture on the same shield arms for a pad
/// and refuses for a driver keeping a fifth back. **Pins a choice, not a finding**
/// ([`within_budget`]): the floor is a controls field, so if a change fills it in on the human
/// path the second half fails.
#[test]
fn the_invented_floor_refuses_where_a_pad_arms() {
    // The pool is 100.0 in `armable`, the cost 8.0, so a fifth kept back needs
    // 28.0 to spend from. The recovered `cost < shield` gate alone would allow
    // this at anything over 8.0.
    let under = 27.0;

    let (floored, state) = three_taps(0.20, under);
    assert!(!floored, "a roll armed below the budget");
    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.shield, under, "and it was not charged");

    let (pad, state) = three_taps(0.0, under);
    assert!(pad, "the floor reached a caller that did not ask for one");
    assert_eq!(state.roll_target, 1.0);
    assert!(state.shield < under, "and the pad paid the recovered cost");
}

/// **The floor is hard: it compares the pool the roll would leave.** The other reading lets a
/// craft exactly on its floor spend anyway and land under it, an opponent rolling itself down to
/// nothing.
#[test]
fn the_floor_is_measured_after_the_cost_not_before_it() {
    // Exactly on the floor before the cost, and a whole cost under it after.
    let (on_the_floor, state) = three_taps(0.20, 20.0);
    assert!(
        !on_the_floor,
        "spending it would have left 12 against a 20 floor"
    );
    assert_eq!(state.shield, 20.0);

    // And a shield the cost clears the floor from arms, so this is measuring
    // the arithmetic rather than a floor that never lets anything through.
    let (clears, state) = three_taps(0.20, 28.0);
    assert!(clears);
    assert_eq!(
        state.shield, 20.0,
        "left exactly on the floor, not under it"
    );
}

/// Above the budget the floored craft arms exactly as an unfloored one does, so
/// the tests above measure the floor rather than a caller that can never roll.
#[test]
fn above_the_budget_a_floored_craft_arms_like_an_unfloored_one() {
    let over = 40.0;
    let (floored, floored_state) = three_taps(0.20, over);
    let (pad, pad_state) = three_taps(0.0, over);
    assert!(floored && pad);
    assert_eq!(floored_state.roll_target, pad_state.roll_target);
    assert_eq!(floored_state.shield, pad_state.shield);
}

/// The direct request arms the same roll the gesture does, without a tap: **the deviation's own
/// entry point**. No tap history is touched, `oag_ai::Driver` is the only setter, and it still
/// pays the recovered cost.
#[test]
fn a_direct_request_arms_a_roll_with_no_taps_at_all() {
    let (mut state, dimensions) = armable();
    let input = ShipControls {
        roll_request: Some(TapDirection::Right),
        ..ShipControls::default()
    };
    let armed = advance_gesture(&mut state, &input, &dimensions, 8.0, false, 1.0 / 60.0);
    assert!(armed);
    assert_eq!(state.roll_target, 1.0);
    assert_eq!(state.shield, 92.0, "8% of a 100 shield pool, as a tap pays");
    assert_eq!(state.roll_taps, [0, 0, 0], "and no tap history was written");
}

/// The grounded gate is the outermost of the four and the direct request does
/// not get past it either. **If this fails, the deviation has been let through
/// a recovered gate rather than placed on top of one.**
#[test]
fn a_direct_request_on_the_ground_arms_nothing() {
    let (mut state, dimensions) = armable();
    let input = ShipControls {
        roll_request: Some(TapDirection::Right),
        ..ShipControls::default()
    };
    let armed = advance_gesture(&mut state, &input, &dimensions, 8.0, true, 1.0 / 60.0);
    assert!(!armed);
    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.shield, 100.0, "and nothing was charged");
}

/// And the recovered `cost < shield` refuses it too.
#[test]
fn a_direct_request_on_an_empty_shield_arms_nothing() {
    let (mut state, dimensions) = armable();
    state.shield = 4.0;
    let input = ShipControls {
        roll_request: Some(TapDirection::Left),
        ..ShipControls::default()
    };
    let armed = advance_gesture(&mut state, &input, &dimensions, 8.0, false, 1.0 / 60.0);
    assert!(!armed);
    assert_eq!(state.shield, 4.0);
}

/// A second roll requested after a completed one starts from level. **Regression**: `roll_phase`
/// stays at `+-1.0` after a completed roll, so a request that armed without levelling would reach
/// its target the tick it was made and collect the landing payout for nothing.
#[test]
fn a_requested_roll_after_a_completed_one_starts_from_level() {
    let (mut state, dimensions) = armable();
    state.roll_phase = 1.0;
    state.roll_target = 1.0;
    let input = ShipControls {
        roll_request: Some(TapDirection::Right),
        ..ShipControls::default()
    };
    assert!(advance_gesture(
        &mut state,
        &input,
        &dimensions,
        8.0,
        false,
        1.0 / 60.0
    ));
    assert_eq!(state.roll_phase, 0.0, "the new roll starts from level");
    assert_eq!(state.roll_target, 1.0);
}

/// A refused request leaves the phase where it was, because levelling is the
/// original's answer to a completed *pattern* and a request is not one.
#[test]
fn a_refused_request_does_not_level_the_phase() {
    let (mut state, dimensions) = armable();
    state.shield = 4.0;
    state.roll_phase = 0.75;
    state.roll_target = 1.0;
    let input = ShipControls {
        roll_request: Some(TapDirection::Right),
        ..ShipControls::default()
    };
    assert!(!advance_gesture(
        &mut state,
        &input,
        &dimensions,
        8.0,
        false,
        1.0 / 60.0
    ));
    assert_eq!(state.roll_phase, 0.75);
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

/// The d-pad and the axis are one signal, not two. The input layer drives the analog axis from
/// the d-pad too (`oag_input::pad::larger`), so a d-pad press is a button edge *and* an axis
/// crossing on one tick; recording both would shift the history twice and leave a doubled
/// direction that can never match an alternation, making the d-pad leg silently unreachable.
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

/// Ordinary cornering is under the threshold and records nothing. [`crate::probe`]'s slalom holds
/// the axis at `+-0.8` for this reason, keeping the committed determinism hashes free of rolls.
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

/// The gesture advances the inter-tap timer exactly once a tick: calling [`advance_tap_timer`]
/// beside [`advance_gesture`] would double it and halve [`INTER_TAP_TIMEOUT`] without failing
/// anything else.
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

/// The gate that makes an AI craft affordable: a craft on the ground cannot arm and pays nothing
/// for trying. The original zeroes the whole tap history every grounded tick rather than refusing
/// at the arm ([`advance_gesture`]).
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

/// **Two rolls in opposite directions are two rotations, not one and then two.**
///
/// Reported from play on 2026-09-06 ("the roll rotates twice where the original rotates once"),
/// and invisible to every other test because the phase is correct at every tick in isolation.
/// [`release`] leaves [`ShipState::roll_target`] at `+-1.0` on a completed roll and
/// [`advance_phase`] parks the phase there, level on screen (`ease(1.0) = 1.0`). The next roll
/// the *other* way then ramps `+1.0 -> -1.0`, a traversal of `2.0`: two full turns. The fix is
/// the original's own `swc1 f22, 0x87c` at `0x08846e5c` / `0x08846f54`, a completed alternation
/// levelling the phase before the ramp, so the assertion is on the *traversal*, not the endpoint.
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

/// The maintainer's report from play: a roll pays out on its landing and again on every later
/// airborne-to-grounded transition, because `release` left `roll_target` and `roll_phase` at
/// `+-1.0` with nothing saying the roll was spent. The original gates the payout on the arm bit,
/// which its landing clears: one roll, one payout.
#[test]
fn a_roll_pays_out_once_however_many_landings_follow() {
    let (mut state, dimensions) = armable();
    assert!(arm(&mut state, &dimensions, 8.0, 1.0));
    advance_phase(&mut state, 1.5, 1.0);

    let mut payouts = 0;
    for _ in 0..5 {
        if release(&mut state) {
            payouts += 1;
        }
        advance_phase(&mut state, 1.5, 1.0);
    }
    assert_eq!(payouts, 1);
    assert_eq!(state.roll_phase, 1.0, "the phase still runs on to the end");
}

#[test]
fn a_second_roll_pays_out_again_after_the_first() {
    let (mut state, dimensions) = armable();
    for _ in 0..2 {
        assert!(arm(&mut state, &dimensions, 8.0, -1.0));
        state.roll_phase = 0.0;
        advance_phase(&mut state, 1.5, 1.0);
        assert!(release(&mut state));
        assert!(!release(&mut state));
    }
}

fn alternation(state: &mut ShipState, dimensions: &Dimensions, first: TapDirection) -> bool {
    let second = match first {
        TapDirection::Left => TapDirection::Right,
        TapDirection::Right => TapDirection::Left,
    };
    let mut armed = false;
    for dir in [first, second, first] {
        armed = gesture_tick(state, dimensions, 0.0, Some(dir));
    }
    armed
}

/// The maintainer's report: a barrel roll could be triggered during a barrel roll. The original
/// branches past both pattern compares while an arm bit is set (`0x08846d14`), so a second
/// alternation mid-roll charges nothing, levels nothing and does not restart the ramp.
#[test]
fn an_alternation_during_a_roll_is_refused_and_does_not_restart_it() {
    let (mut state, dimensions) = armable();
    assert!(alternation(&mut state, &dimensions, TapDirection::Right));
    let shield = state.shield;
    for _ in 0..20 {
        advance_phase(&mut state, 1.5, 1.0 / 60.0);
    }
    let mid = state.roll_phase;
    assert!(mid > 0.0 && mid < 1.0);

    assert!(!alternation(&mut state, &dimensions, TapDirection::Left));
    assert_eq!(state.shield, shield, "no second charge");
    assert_eq!(state.roll_phase, mid, "the phase was not levelled");
    assert_eq!(state.roll_target, 1.0, "the target was not flipped");
}

/// A finished roll keeps its arm bit until the next ground/air transition, so a second roll in
/// the same flight is refused after the first completes too: one roll per flight.
#[test]
fn a_finished_roll_still_refuses_another_until_the_craft_lands() {
    let (mut state, dimensions) = armable();
    assert!(alternation(&mut state, &dimensions, TapDirection::Right));
    for _ in 0..120 {
        advance_phase(&mut state, 1.5, 1.0 / 60.0);
    }
    assert_eq!(state.roll_phase, 1.0);
    let shield = state.shield;

    assert!(!alternation(&mut state, &dimensions, TapDirection::Left));
    assert!(!alternation(&mut state, &dimensions, TapDirection::Right));
    assert_eq!(state.shield, shield);
    assert_eq!(state.roll_phase, 1.0);

    assert!(release(&mut state), "the landing pays out the one roll");
    // The grounded ticks of the landing zero the history (`advance_gesture`, contact).
    state.roll_taps = [0, 0, 0];
    assert!(
        alternation(&mut state, &dimensions, TapDirection::Left),
        "the next flight rolls again"
    );
    assert!(state.shield < shield);
}

/// The direct request (the AI's route) obeys the same bit.
#[test]
fn a_direct_request_during_a_roll_is_refused() {
    let (mut state, dimensions) = armable();
    assert!(arm(&mut state, &dimensions, 8.0, 1.0));
    let shield = state.shield;
    let input = ShipControls {
        roll_request: Some(TapDirection::Left),
        ..ShipControls::default()
    };
    assert!(!advance_gesture(
        &mut state,
        &input,
        &dimensions,
        8.0,
        false,
        1.0 / 60.0
    ));
    assert_eq!(state.shield, shield);
    assert_eq!(state.roll_target, 1.0);
}

/// Taps made during a roll still shift into the history, as in the original, where the shift
/// runs before the gate.
#[test]
fn taps_made_during_a_roll_still_enter_the_history() {
    let (mut state, dimensions) = armable();
    assert!(arm(&mut state, &dimensions, 8.0, 1.0));
    gesture_tick(&mut state, &dimensions, 0.0, Some(TapDirection::Left));
    assert_eq!(state.roll_taps[2], TapDirection::Left as u8);
}
