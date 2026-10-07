use super::*;
use crate::input::Input;

fn held(button: Button) -> Input {
    let mut input = Input::new();
    input.begin_frame(button.bit());
    input
}

#[test]
fn cross_is_thrust() {
    let snapshot = InputSnapshot {
        buttons: held(Button::Cross),
        ..InputSnapshot::new()
    };
    assert_eq!(
        ship_controls(&snapshot, ControlScheme::default()).thrust,
        1.0
    );
}

/// Cross is `activate` in the front end and thrust in a race. Circle is
/// neither, and mapping it to thrust is the single most likely slip here.
#[test]
fn circle_is_not_thrust() {
    let snapshot = InputSnapshot {
        buttons: held(Button::Circle),
        ..InputSnapshot::new()
    };
    assert_eq!(
        ship_controls(&snapshot, ControlScheme::default()).thrust,
        0.0
    );
}

/// Three of the four axes pass through, and the fourth is inverted.
///
/// `stick_y` is the only one that is not a pass-through, and it is the whole
/// point of this test: pushing the stick up pitches the nose down, measured
/// off the original in PPSSPP. This test asserted a pass-through until that
/// measurement existed.
#[test]
fn the_axes_pass_through_except_the_inverted_pitch() {
    let snapshot = InputSnapshot {
        stick_x: -0.5,
        stick_y: 0.25,
        airbrake_left: 1.0,
        airbrake_right: 0.5,
        ..InputSnapshot::new()
    };
    let controls = ship_controls(&snapshot, ControlScheme::default());
    assert_eq!(controls.steer_x, -0.5);
    assert_eq!(controls.steer_y, -0.25, "stick up is nose down");
    assert_eq!(controls.airbrake_left, 1.0);
    assert_eq!(controls.airbrake_right, 0.5);
}

/// The gesture fields are what a scheme selects, and they are exclusive.
///
/// Holding `L` is a flick modifier on novice and a left-airbrake tap on
/// veteran, and it must never be both: the two gesture machines run side by
/// side in the physics crate and rely on this to stay dormant.
#[test]
fn a_scheme_fills_only_its_own_gesture_fields() {
    let snapshot = InputSnapshot {
        buttons: held(Button::L),
        ..InputSnapshot::new()
    };

    let novice = ship_controls(&snapshot, ControlScheme::Novice);
    assert!(novice.shift_modifier, "L is the novice sideshift button");
    assert!(!novice.shift_tap_left);
    assert!(!novice.shift_tap_right);

    let veteran = ship_controls(&snapshot, ControlScheme::Veteran);
    assert!(!veteran.shift_modifier, "veteran has no sideshift button");
    assert!(veteran.shift_tap_left, "L is the veteran left airbrake");
    assert!(!veteran.shift_tap_right);
}

/// Novice's airbrake reads `R` alone, per action 4 (`OPT_CTRL_AIRBRAKES`)
/// in `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
///
/// `L` is a dedicated sideshift button in novice (action 7,
/// `OPT_CTRL_SS`) and contributes no airbrake at all. Veteran is untouched
/// by this: its table binds `L`/`R` to the two airbrakes directly, which
/// `the_axes_pass_through_except_the_inverted_pitch` already covers.
#[test]
fn novices_airbrake_reads_r_alone_and_l_arms_only_the_sideshift() {
    let right_only = InputSnapshot {
        buttons: held(Button::R),
        airbrake_right: 1.0,
        ..InputSnapshot::new()
    };
    let controls = ship_controls(&right_only, ControlScheme::Novice);
    assert_eq!(controls.airbrake_left, 1.0, "R centred drives both");
    assert_eq!(controls.airbrake_right, 1.0);
    assert!(!controls.shift_modifier);

    let left_only = InputSnapshot {
        buttons: held(Button::L),
        airbrake_left: 1.0,
        ..InputSnapshot::new()
    };
    let controls = ship_controls(&left_only, ControlScheme::Novice);
    assert_eq!(controls.airbrake_left, 0.0, "L is not an airbrake");
    assert_eq!(controls.airbrake_right, 0.0);
    assert!(controls.shift_modifier, "L arms the sideshift instead");
}

/// Novice's `R` follows the steering: the rows PPSSPP measured off the
/// original's `player_input` record, the steer column over `100`.
///
/// Full lock (`-100`/`100`, which the d-pad also writes) and the curve's
/// `-10.9375`/`10.9375` pick one side; `-3.0`, `3.0` and centred pick both.
/// Plus the edge itself, which the original's strict `< -10`/`> 10` leaves
/// on both. Falsifier for a return to "R is both airbrakes".
#[test]
fn novices_airbrake_picks_its_side_off_the_steering() {
    let rows = [
        (-1.0, (1.0, 0.0)),
        (-0.109_375, (1.0, 0.0)),
        (-0.1, (1.0, 1.0)),
        (-0.03, (1.0, 1.0)),
        (0.0, (1.0, 1.0)),
        (0.03, (1.0, 1.0)),
        (0.1, (1.0, 1.0)),
        (0.109_375, (0.0, 1.0)),
        (0.523_925_8, (0.0, 1.0)),
        (1.0, (0.0, 1.0)),
    ];
    for (stick_x, expected) in rows {
        let snapshot = InputSnapshot {
            buttons: held(Button::R),
            airbrake_right: 1.0,
            stick_x,
            ..InputSnapshot::new()
        };
        let controls = ship_controls(&snapshot, ControlScheme::Novice);
        assert_eq!(
            (controls.airbrake_left, controls.airbrake_right),
            expected,
            "stick {stick_x}"
        );
    }
}

/// No button, no airbrake, however far the stick leans; and the steering
/// never moves a veteran's airbrakes.
#[test]
fn steering_alone_drives_no_airbrake() {
    for scheme in ControlScheme::ALL {
        let snapshot = InputSnapshot {
            stick_x: -1.0,
            ..InputSnapshot::new()
        };
        let controls = ship_controls(&snapshot, scheme);
        assert_eq!(
            (controls.airbrake_left, controls.airbrake_right),
            (0.0, 0.0)
        );
    }
    let veteran = InputSnapshot {
        airbrake_right: 1.0,
        stick_x: -1.0,
        ..InputSnapshot::new()
    };
    let controls = ship_controls(&veteran, ControlScheme::Veteran);
    assert_eq!(
        (controls.airbrake_left, controls.airbrake_right),
        (0.0, 1.0)
    );
}

/// An analogue trigger's travel carries onto the chosen side unscaled
/// (chosen, not measured: the original's button is digital).
#[test]
fn a_novice_trigger_carries_its_travel_onto_the_chosen_side() {
    assert_eq!(novice_airbrakes(0.4, -0.5), (0.4, 0.0));
    assert_eq!(novice_airbrakes(0.4, 0.0), (0.4, 0.4));
    assert_eq!(novice_airbrakes(0.4, 0.5), (0.0, 0.4));
}

/// A held airbrake is one tap, not one per tick.
///
/// `is_pressed` and not `is_held` is the whole of what stops a veteran
/// pilot leaning on an airbrake from sideshifting continuously.
#[test]
fn a_veteran_tap_is_an_edge_and_not_a_level() {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::L.bit());
    let first = InputSnapshot {
        buttons,
        ..InputSnapshot::new()
    };
    assert!(ship_controls(&first, ControlScheme::Veteran).shift_tap_left);

    buttons.begin_frame(Button::L.bit());
    let second = InputSnapshot {
        buttons,
        ..InputSnapshot::new()
    };
    assert!(!ship_controls(&second, ControlScheme::Veteran).shift_tap_left);
}

/// The direct request stays a caller's business, never a pilot's.
///
/// A real shift arrives through the gesture machine in `oag_physics`, so
/// this field staying `None` is the boundary between the two and not a gap.
#[test]
fn no_scheme_produces_a_direct_sideshift_request() {
    let snapshot = InputSnapshot {
        buttons: held(Button::L),
        airbrake_left: 1.0,
        stick_x: 1.0,
        ..InputSnapshot::new()
    };
    for scheme in [ControlScheme::Novice, ControlScheme::Veteran] {
        assert_eq!(ship_controls(&snapshot, scheme).sideshift, Sideshift::None);
    }
}

/// The tokens are the game's own, and they round-trip.
///
/// Worth a test rather than being obvious: these strings are what a settings
/// file and a `--scheme` flag carry, so a rename here silently invalidates
/// every saved profile. They are also `Control_Type`'s literal values in the
/// binary, which is the reason they are not `Beginner`/`Expert` or anything
/// else that reads better.
#[test]
fn every_scheme_round_trips_through_its_token() {
    for scheme in ControlScheme::ALL {
        assert_eq!(ControlScheme::from_name(scheme.name()), Some(scheme));
        assert_eq!(scheme.to_string().parse(), Ok(scheme));
    }
    assert_eq!(ControlScheme::from_name("custom"), None, "not a scheme");
    assert_eq!(ControlScheme::ALL[0], ControlScheme::default());
}

#[test]
fn no_input_produces_no_controls() {
    for scheme in [ControlScheme::Novice, ControlScheme::Veteran] {
        assert_eq!(
            ship_controls(&InputSnapshot::new(), scheme),
            ShipControls::default()
        );
    }
}
