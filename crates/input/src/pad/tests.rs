use super::*;

/// One tick resolved under the default mapping: the triggers are the two
/// airbrakes, linear.
fn airbrakes(reading: Reading) -> PadState {
    resolve(reading, TriggerConfig::default())
}

/// The same reading under the mapping this file had before the airbrakes were
/// analog: R2 is thrust, L2 is a brake on both sides.
fn thrust_brake(reading: Reading) -> PadState {
    resolve(
        reading,
        TriggerConfig {
            mode: TriggerMode::ThrustBrake,
            ..TriggerConfig::default()
        },
    )
}

/// A trigger pull, conditioned the way the default config conditions it. What
/// the axis assertions below compare against, so a change to the deadzone or
/// the saturation point does not have to be re-derived by hand in ten places.
fn pull(raw: f32) -> f32 {
    condition(raw, 1.0)
}

#[test]
fn the_south_button_is_cross_whatever_the_pad_prints_on_it() {
    assert_eq!(map_button(gilrs::Button::South), Some(Button::Cross));
    assert_eq!(map_button(gilrs::Button::East), Some(Button::Circle));
}

#[test]
fn the_shoulders_are_the_airbrakes() {
    assert_eq!(map_button(gilrs::Button::LeftTrigger), Some(Button::L));
    assert_eq!(map_button(gilrs::Button::RightTrigger), Some(Button::R));

    let state = airbrakes(Reading {
        buttons: Button::L.bit(),
        ..Reading::default()
    });
    assert_eq!(state.airbrake_left, 1.0);
    assert_eq!(state.airbrake_right, 0.0);
}

#[test]
fn every_bound_button_is_in_the_polled_list() {
    for button in BOUND_BUTTONS {
        assert!(map_button(button).is_some(), "{button:?} is polled unbound");
    }
}

#[test]
fn an_unbound_button_changes_nothing() {
    assert_eq!(map_button(gilrs::Button::Mode), None);
    assert_eq!(map_button(gilrs::Button::LeftThumb), None);
    // The analog triggers are deliberately not buttons: they are read as
    // axes and turned into airbrakes, or thrust and brake, by `resolve`.
    assert_eq!(map_button(gilrs::Button::RightTrigger2), None);
    assert_eq!(map_button(gilrs::Button::LeftTrigger2), None);
}

#[test]
fn drift_inside_the_deadzone_does_not_steer() {
    let state = airbrakes(Reading {
        stick_x: STICK_DEADZONE * 0.9,
        stick_y: -STICK_DEADZONE * 0.9,
        ..Reading::default()
    });
    assert_eq!(state.stick_x, 0.0);
    assert_eq!(state.stick_y, 0.0);
}

#[test]
fn a_stick_at_full_travel_still_reaches_one() {
    let state = airbrakes(Reading {
        stick_x: 1.0,
        stick_y: -1.0,
        ..Reading::default()
    });
    assert_eq!(state.stick_x, 1.0);
    assert_eq!(state.stick_y, -1.0);
}

#[test]
fn the_deadzone_is_rescaled_rather_than_stepped() {
    let just_outside = airbrakes(Reading {
        stick_x: STICK_DEADZONE + 0.001,
        ..Reading::default()
    });
    assert!(
        just_outside.stick_x.abs() < 0.01,
        "a step at the edge of the deadzone: {}",
        just_outside.stick_x
    );
}

#[test]
fn a_pad_that_could_not_be_opened_reads_as_nothing_held() {
    let mut pad = Pad::none();
    assert!(!pad.is_available());
    assert_eq!(pad.poll(), PadState::default());
    assert!(pad.names().is_empty());
}

#[test]
fn each_trigger_drives_its_own_airbrake() {
    let left = airbrakes(Reading {
        brake: 0.4,
        ..Reading::default()
    });
    assert_eq!(left.airbrake_left, pull(0.4));
    assert_eq!(left.airbrake_right, 0.0, "L2 is one side, not both");

    let right = airbrakes(Reading {
        throttle: 0.4,
        ..Reading::default()
    });
    assert_eq!(right.airbrake_right, pull(0.4));
    assert_eq!(right.airbrake_left, 0.0);
    assert_eq!(
        right.held & Button::Cross.bit(),
        0,
        "R2 is an airbrake here, not a second thrust button"
    );
}

/// The rule the whole mapping rests on: a pulled trigger produces the same
/// button mask its shoulder would, *and* an analog value where the shoulder
/// gives 1.0. Without the mask half, `oag_gameplay::ship_controls` never sees
/// the pressed edge the veteran sideshift's double-tap is read off, so a
/// player on the triggers could not sideshift at all.
#[test]
fn a_pulled_trigger_sets_its_shoulders_bit_and_keeps_its_analog_value() {
    let state = airbrakes(Reading {
        brake: 0.6,
        ..Reading::default()
    });
    assert_ne!(state.held & Button::L.bit(), 0, "no edge to double-tap");
    assert_eq!(state.held & Button::R.bit(), 0, "the other side is idle");
    assert!(
        state.airbrake_left < 1.0,
        "the bit quantised the pull away: {}",
        state.airbrake_left
    );
    assert_eq!(state.airbrake_left, pull(0.6));
}

/// The gap between the two thresholds is deliberate: feathering the brake
/// through a corner must not read as a sideshift tap.
#[test]
fn a_feathered_trigger_brakes_without_counting_as_a_press() {
    let raw = (TRIGGER_DEADZONE + TRIGGER_THRESHOLD) / 2.0;
    let state = airbrakes(Reading {
        brake: raw,
        ..Reading::default()
    });
    assert!(state.airbrake_left > 0.0, "a feather is still braking");
    assert_eq!(state.held, 0, "and it is not a press");
}

#[test]
fn a_resting_trigger_applies_no_airbrake_at_all() {
    let state = airbrakes(Reading {
        brake: TRIGGER_DEADZONE * 0.9,
        throttle: TRIGGER_DEADZONE * 0.9,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_left, 0.0);
    assert_eq!(state.airbrake_right, 0.0);
    assert_eq!(state.held, 0);
}

/// A pad whose trigger tops out short of 1.0 must still reach a full airbrake,
/// or the ship can never brake as hard as a keyboard can.
#[test]
fn a_trigger_short_of_full_travel_still_reaches_one() {
    let state = airbrakes(Reading {
        brake: TRIGGER_SATURATION,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_left, 1.0);
}

/// The regression guard for the quantisation trap `resolve` documents: the
/// digital contribution is L1 and R1, never the bit a trigger synthesised.
#[test]
fn a_held_shoulder_wins_over_a_lighter_trigger() {
    let state = airbrakes(Reading {
        buttons: Button::R.bit(),
        throttle: 0.4,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_right, 1.0, "the shoulder is the harder ask");

    let state = airbrakes(Reading {
        buttons: Button::L.bit(),
        throttle: 0.4,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_right, pull(0.4), "L1 is not R2's business");
}

#[test]
fn the_response_curve_bends_the_pull_without_moving_its_ends() {
    let half = (1.0 + TRIGGER_DEADZONE) / 2.0;
    let linear = condition(half, 1.0);
    let sensitive = condition(half, 0.5);
    let fine = condition(half, 2.0);

    assert!(
        sensitive > linear && linear > fine,
        "a curve that does nothing: {sensitive} {linear} {fine}"
    );
    for curve in [0.5, 1.0, 2.0] {
        assert_eq!(condition(TRIGGER_DEADZONE, curve), 0.0, "zero moved");
        assert_eq!(condition(1.0, curve), 1.0, "full travel moved");
    }
}

/// The exponent arrives from a settings file as a bare `f32`, and both a wild
/// value and a NaN reach the airbrake axis if nothing stops them here.
#[test]
fn the_curve_is_clamped_and_nan_becomes_linear() {
    let config = TriggerConfig::default();
    assert_eq!(config.with_curve(0.0).curve(), *CURVE_RANGE.start());
    assert_eq!(config.with_curve(1e9).curve(), *CURVE_RANGE.end());
    assert_eq!(config.with_curve(f32::NAN).curve(), 1.0);
    assert_eq!(config.with_curve(-4.0).curve(), *CURVE_RANGE.start());
}

#[test]
fn a_trigger_mode_round_trips_through_its_token() {
    for mode in TriggerMode::ALL {
        assert_eq!(TriggerMode::from_name(mode.name()), Some(mode));
        assert_eq!(mode.name().parse::<TriggerMode>(), Ok(mode));
    }
    assert_eq!(TriggerMode::from_name("nonsense"), None);
    assert_eq!(TriggerMode::default(), TriggerMode::Airbrakes);
}

#[test]
fn r2_past_the_threshold_is_thrust_under_the_old_mapping() {
    let idle = thrust_brake(Reading {
        throttle: TRIGGER_THRESHOLD,
        ..Reading::default()
    });
    assert_eq!(idle.held & Button::Cross.bit(), 0, "resting trigger");

    let pulled = thrust_brake(Reading {
        throttle: 1.0,
        ..Reading::default()
    });
    assert_ne!(pulled.held & Button::Cross.bit(), 0);
}

/// L2 is "brake", which the original's action set does not have. Both
/// airbrakes is the closest thing to one, and it stays analog: setting the
/// L and R bits instead would quantise it to 0 or 1.
#[test]
fn l2_pulls_both_airbrakes_under_the_old_mapping() {
    let state = thrust_brake(Reading {
        brake: 0.4,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_left, pull(0.4));
    assert_eq!(state.airbrake_right, pull(0.4));
    assert_eq!(state.held, 0, "no button bit, or the analog value is lost");
}

#[test]
fn a_held_shoulder_wins_over_a_lighter_brake_under_the_old_mapping() {
    let state = thrust_brake(Reading {
        buttons: Button::R.bit(),
        brake: 0.3,
        ..Reading::default()
    });
    assert_eq!(state.airbrake_left, pull(0.3));
    assert_eq!(state.airbrake_right, 1.0);
}
