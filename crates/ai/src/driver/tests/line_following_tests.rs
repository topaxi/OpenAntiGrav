//! Following the line: thrust, steering lock, the corner-speed brake and the
//! braking differential.
//!
//! One theme of `driver.rs`'s tests, split by subject (an inline module of 1,185
//! lines broke both caps in `scripts/check-file-size.py`); shared fixtures stay
//! in [`super`].

use super::*;

#[test]
fn no_line_means_no_input() {
    let mut driver = Driver::default();
    let controls = driver.drive(
        &craft(Vec3::ZERO, 0.0),
        &Context::new(&Line::default(), &Tuning::default()),
    );
    assert_eq!(controls, ShipControls::default());
}

#[test]
fn a_craft_on_a_straight_holds_full_thrust_and_no_steering() {
    let mut driver = Driver::default();
    let controls = driver.drive(
        &craft(Vec3::ZERO, 50.0),
        &Context::new(&straight(), &Tuning::default()),
    );
    assert_eq!(controls.thrust, 1.0);
    assert!(controls.steer_x.abs() < 1e-3, "steer {}", controls.steer_x);
    assert_eq!(controls.airbrake_left, 0.0);
    assert_eq!(controls.airbrake_right, 0.0);
}

/// The sign is the whole thing: a craft to the *right* of its line must steer
/// *left*, and backwards drives the field into the outside wall on the first
/// corner, the trap the grid's lateral offset sprang once. See
/// `docs/ghidra/functions/psp-pulse-usa/grid.md`.
#[test]
fn a_craft_beside_the_line_steers_back_toward_it() {
    let tuning = Tuning::default();

    let mut driver = Driver::default();
    let right_of_line = driver.drive(
        &craft(Vec3::new(6.0, 0.0, 0.0), 40.0),
        &Context::new(&straight(), &tuning),
    );
    assert!(
        right_of_line.steer_x < 0.0,
        "steer {} should be left of centre",
        right_of_line.steer_x
    );

    let mut driver = Driver::default();
    let left_of_line = driver.drive(
        &craft(Vec3::new(-6.0, 0.0, 0.0), 40.0),
        &Context::new(&straight(), &tuning),
    );
    assert!(
        left_of_line.steer_x > 0.0,
        "steer {} should be right of centre",
        left_of_line.steer_x
    );
}

/// The damping, and the reason the rewrite happened: a craft already turning
/// the way the geometry wants must be asked for *less* lock, not the same.
#[test]
fn a_craft_already_turning_is_asked_for_less_lock() {
    let tuning = Tuning::default();
    let mut state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);

    let mut driver = Driver::default();
    let still = driver.drive(&state, &Context::new(&straight(), &tuning));

    // Yawing left, which is positive about the craft's own up axis, and is
    // the direction the controller wants to go.
    state.body.angular_velocity = Vec3::new(0.0, 0.3, 0.0);
    let mut driver = Driver::default();
    let turning = driver.drive(&state, &Context::new(&straight(), &tuning));
    assert!(
        turning.steer_x > still.steer_x,
        "already turning: {} should be less left lock than {}",
        turning.steer_x,
        still.steer_x
    );
}

/// Braking uses both sides while the steering loop still has authority. A
/// single airbrake yaws the craft, so tying one to the steering below
/// saturation would close a second loop around it - see [`trail`].
#[test]
fn braking_is_symmetric_until_the_steering_loop_saturates() {
    let tuning = Tuning::default();
    let line = Line::new(
        (0..128)
            .map(|step| {
                let angle = std::f32::consts::TAU * step as f32 / 128.0;
                Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
            })
            .collect(),
    );
    // Well over the corner's target, so the brakes are on throughout; the
    // craft's turn rate is swept to find the one leaving the steering loop short
    // of lock (`actual` is `-angular_velocity.y` with identity orientation, so
    // this sweeps the rate error through zero).
    let mut unsaturated = 0;
    for step in -200..=200 {
        let mut state = craft(line.point(0), 200.0);
        state.body.angular_velocity = Vec3::new(0.0, step as f32 * 0.02, 0.0);
        let controls = Driver::default().drive(&state, &Context::new(&line, &tuning));
        if controls.steer_x.abs() >= tuning.trail_saturation {
            continue;
        }
        unsaturated += 1;
        assert_eq!(
            controls.airbrake_left, controls.airbrake_right,
            "unsaturated at steer {}, but the airbrakes differ",
            controls.steer_x
        );
        assert!(
            controls.airbrake_left > 0.0,
            "a 60-unit circle at 200 needs brakes"
        );
    }
    assert!(
        unsaturated > 0,
        "the sweep never left the loop unsaturated, so this proves nothing"
    );
}

/// The other half of the pair above, and the feature itself: once the loop
/// is out of lock and the craft is over the corner's speed, the brakes stop
/// being symmetric and the extra goes on the side it is turning toward.
#[test]
fn a_saturated_driver_over_its_corner_speed_brakes_asymmetrically() {
    let tuning = Tuning::default();
    let line = Line::new(
        (0..128)
            .map(|step| {
                let angle = std::f32::consts::TAU * step as f32 / 128.0;
                Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
            })
            .collect(),
    );
    let controls =
        Driver::default().drive(&craft(line.point(0), 200.0), &Context::new(&line, &tuning));
    assert!(
        controls.steer_x.abs() >= tuning.trail_saturation,
        "a 60-unit circle at 200 should have the loop at lock, got {}",
        controls.steer_x
    );
    assert_ne!(controls.airbrake_left, controls.airbrake_right);
    // Steering left means the nose is going left, so the left side carries
    // the extra.
    if controls.steer_x < 0.0 {
        assert!(controls.airbrake_left > controls.airbrake_right);
    } else {
        assert!(controls.airbrake_right > controls.airbrake_left);
    }
}

/// A curvature that reads as a real corner against
/// [`Tuning::trail_curvature_floor`], well clear of it as Talon's Junction's
/// bends measured. Used as its own peak, so the exit gate reads "at the
/// tightest point", not "opening up".
const CORNER_CURVATURE: f32 = 0.02;

/// [`track_peak_curvature`] holds the high-water mark through a dip (the chord
/// estimate's ~5e-5 tick-to-tick noise mid-corner on Talon's Junction) and
/// resets only once the line goes straight.
#[test]
fn the_peak_curvature_holds_through_a_dip_and_resets_on_a_straight() {
    let tuning = Tuning::default();
    let rising = track_peak_curvature(CORNER_CURVATURE, 0.0, &tuning);
    assert_eq!(rising, CORNER_CURVATURE, "a new high-water mark is kept");

    let dipped = track_peak_curvature(CORNER_CURVATURE * 0.99, rising, &tuning);
    assert_eq!(
        dipped, rising,
        "a small dip below the peak must not lower it"
    );

    let reset = track_peak_curvature(0.0, dipped, &tuning);
    assert_eq!(reset, 0.0, "a genuine straight resets the mark");
}

/// The sign that `5ad69f3` shipped backwards for months. Positive rate
/// error is a craft wanting to turn further right; a nose-right yaw needs
/// `imbalance = L - R` negative, so the **right** side is braked.
#[test]
fn the_differential_brakes_the_side_the_nose_is_turning_toward() {
    let tuning = Tuning::default();
    let hard_right = Steer {
        command: 1.0,
        rate_error: 1.0,
    };
    let hard_left = Steer {
        command: -1.0,
        rate_error: -1.0,
    };
    // At the peak of a real corner, not recovering, so every gate but
    // saturation and deadband is open.
    assert!(
        trail(
            &hard_right,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ) > 0.0
    );
    assert!(
        trail(
            &hard_left,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ) < 0.0
    );

    let (left, right) = airbrakes(
        0.0,
        trail(
            &hard_right,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL,
        ),
        tuning.brake_floor,
    );
    assert!(right > left, "turning right brakes the right side");
}

#[test]
fn the_differential_is_dead_inside_its_deadband() {
    let tuning = Tuning::default();
    let saturated_but_settled = Steer {
        command: 1.0,
        rate_error: tuning.trail_deadband,
    };
    assert_eq!(
        trail(
            &saturated_but_settled,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
}

#[test]
fn the_differential_waits_for_the_steering_loop_to_run_out_of_lock() {
    let tuning = Tuning::default();
    let unsaturated = Steer {
        command: tuning.trail_saturation - 0.01,
        rate_error: 1.0,
    };
    assert_eq!(
        trail(
            &unsaturated,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
}

/// Curvature at or below [`Tuning::trail_curvature_floor`] is a straight
/// whatever the steering loop does; see [`trail`] for why `target` cannot say.
#[test]
fn the_differential_never_acts_on_a_straight() {
    let tuning = Tuning::default();
    let hard = Steer {
        command: 1.0,
        rate_error: 1.0,
    };
    assert_eq!(
        trail(
            &hard,
            tuning.trail_curvature_floor,
            tuning.trail_curvature_floor,
            false,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
    assert_eq!(
        trail(&hard, 0.0, 0.0, false, &tuning, &Personality::NEUTRAL),
        0.0
    );
}

/// Curvature fallen well below its recent peak is corner exit, where grip is
/// wanted for accelerating; see [`trail`] for the real-track measurement.
#[test]
fn the_differential_never_acts_on_corner_exit() {
    let tuning = Tuning::default();
    let hard = Steer {
        command: 1.0,
        rate_error: 1.0,
    };
    let past_apex = CORNER_CURVATURE * tuning.trail_exit_decay * 0.9;
    assert_eq!(
        trail(
            &hard,
            past_apex,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
}

/// A craft coasting off a weapon hit is not steering into anything - see
/// [`trail`]'s own doc for the real-track measurement that found this.
#[test]
fn the_differential_never_acts_while_recovering_from_a_hit() {
    let tuning = Tuning::default();
    let hard = Steer {
        command: 1.0,
        rate_error: 1.0,
    };
    assert_eq!(
        trail(
            &hard,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            true,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
}

/// The bug this crate was rewritten to fix: a craft below the corner's modelled
/// target speed, mid-entry, got no assistance. Reproduces Talon's Junction's
/// shape (saturated, past the deadband, curvature flat) with no target or speed
/// in sight.
#[test]
fn the_differential_acts_on_corner_entry_even_though_speed_is_still_below_target() {
    let tuning = Tuning::default();
    let hard = Steer {
        command: 1.0,
        rate_error: 1.0,
    };
    assert_ne!(
        trail(
            &hard,
            CORNER_CURVATURE,
            CORNER_CURVATURE,
            false,
            &tuning,
            &Personality::NEUTRAL
        ),
        0.0
    );
}

/// A yaw request must never drop a side to zero while braking: both sides
/// strictly positive is the only thing that engages `ShipState::brake`.
#[test]
fn a_differential_never_cancels_the_brake_it_is_layered_on() {
    let floor = Tuning::default().brake_floor;
    for brake in [floor, 0.5, 0.8, 1.0] {
        for differential in [-1.0f32, -0.6, -0.1, 0.0, 0.1, 0.6, 1.0] {
            let (left, right) = airbrakes(brake, differential, floor);
            assert!(
                left > 0.0 && right > 0.0,
                "brake {brake} with differential {differential} broke the \
                 both-held gate: {left}, {right}"
            );
            assert!((0.0..=1.0).contains(&left) && (0.0..=1.0).contains(&right));
        }
    }
}

/// The reason the interval slides rather than being clipped: a craft
/// braking flat out has no headroom above, and that is the corner it most
/// needs to rotate in.
#[test]
fn a_differential_survives_a_craft_already_braking_flat_out() {
    let floor = Tuning::default().brake_floor;
    let (left, right) = airbrakes(1.0, 0.6, floor);
    assert!((right - left - 0.6).abs() < 1.0e-6, "got {left}, {right}");
    assert_eq!(right, 1.0);
    assert!(left >= floor);
}

/// With the brake off, one side rises from nothing: yaw and no
/// deceleration, which is what a differential airbrake physically is.
#[test]
fn a_differential_with_no_brake_engages_no_brake() {
    let (left, right) = airbrakes(0.0, 0.4, Tuning::default().brake_floor);
    assert_eq!(left, 0.0);
    assert!((right - 0.4).abs() < 1.0e-6);
}

#[test]
fn a_straight_has_no_speed_limit() {
    let tuning = Tuning::default();
    let target = corner_target(0.0, &tuning, &Personality::NEUTRAL, None);
    assert_eq!(target, f32::INFINITY);
    assert_eq!(throttle(500.0, target, &tuning), (1.0, 0.0));
}

/// A corner too tight for the hull to rotate through is limited by the rotation,
/// not the grip. `07_Track`'s tightest arc, curvature 0.047: grip says
/// `sqrt(260 / 0.047)` = 74.4, yaw says `1.8 / 0.047` = 38.3; a craft taking the
/// first shed 34-35 shield a lap on the outside wall.
#[test]
fn a_corner_tighter_than_the_hull_can_rotate_is_limited_by_the_yaw_rate() {
    let tuning = Tuning::default();
    let target = corner_target(0.047, &tuning, &Personality::NEUTRAL, None);
    let grip = (tuning.lateral_accel / 0.047f32).sqrt();
    let yaw = tuning.max_turn_rate / 0.047;
    assert!(
        target < grip,
        "{target} should be under the grip limit {grip}"
    );
    assert!((target - yaw).abs() < 1.0e-3, "{target} against {yaw}");
}

/// And an open corner is still limited by the grip: the yaw term is a second
/// ceiling, not a replacement.
#[test]
fn an_open_corner_is_still_limited_by_the_grip() {
    let tuning = Tuning::default();
    // Under `max_turn_rate^2 / lateral_accel`, which is where the two cross.
    let target = corner_target(0.008, &tuning, &Personality::NEUTRAL, None);
    let grip = (tuning.lateral_accel / 0.008f32).sqrt();
    assert!((target - grip).abs() < 1.0e-2, "{target} against {grip}");
}

#[test]
fn a_corner_taken_too_fast_brakes_and_taken_slowly_does_not() {
    let tuning = Tuning::default();
    let target = corner_target(0.01, &tuning, &Personality::NEUTRAL, None);
    assert_eq!(throttle(target * 0.5, target, &tuning), (1.0, 0.0));
    assert_eq!(throttle(target * 2.0, target, &tuning), (0.0, 1.0));
}

/// The command level is not a deceleration, it is how much cornering grip the
/// deceleration is bought with. See [`Tuning::brake_floor`].
#[test]
fn a_brake_climbs_with_the_overspeed_and_never_starts_below_the_floor() {
    let tuning = Tuning::default();
    let target = 100.0;

    // Inside the margin: lift off, but do not touch the airbrakes.
    assert_eq!(throttle(target * 1.02, target, &tuning), (0.0, 0.0));

    // Just past it: braking begins at the floor, not at an epsilon.
    let (_, just_past) = throttle(target * 1.051, target, &tuning);
    assert!(
        (just_past - tuning.brake_floor).abs() < 0.01,
        "braking should start at the floor, got {just_past}"
    );

    // Further past it: more grip spent, monotonically, up to full.
    let (_, further) = throttle(target * 1.2, target, &tuning);
    assert!(further > just_past);
    assert_eq!(throttle(target * 3.0, target, &tuning).1, 1.0);
}

#[test]
fn a_driver_never_brakes_below_the_floor() {
    let tuning = Tuning::default();
    let target = 100.0;
    for step in 0..400 {
        let speed = target * (1.0 + step as f32 * 0.01);
        let (_, brake) = throttle(speed, target, &tuning);
        assert!(
            brake == 0.0 || brake >= tuning.brake_floor,
            "speed {speed} gave a brake of {brake}, between zero and the floor"
        );
    }
}

/// The turn rate the geometry asks for is bounded, or a craft thrown clear of
/// the track holds full lock through the whole recovery.
#[test]
fn the_requested_turn_rate_is_clamped() {
    let tuning = Tuning {
        rate_gain: 1.0,
        ..Tuning::default()
    };
    let mut driver = Driver::default();
    // Absurdly far off the line, at speed: the raw curvature is enormous.
    let controls = driver.drive(
        &craft(Vec3::new(500.0, 0.0, 0.0), 150.0),
        &Context::new(&straight(), &tuning),
    );
    assert!(
        controls.steer_x >= -1.0,
        "steer {} left the input range",
        controls.steer_x
    );
    assert!(controls.steer_x.abs() <= tuning.max_turn_rate * tuning.rate_gain + 1e-3);
}

/// A craft mid-flight holds full thrust and never brakes for a corner: there is
/// no lateral grip off the ground, so a symmetric brake is pure loss of the
/// speed a landing needs. Found chasing `13_Track`'s Novice jump pathology:
/// before the fix this fixture (a 60-unit circle at 200, tight enough to
/// saturate the loop and brake hard) produced **identical** controls whether
/// `time_airborne` was zero or not, the state read nowhere in `Driver::drive`.
/// See `docs/gameplay/ai.md#the-13_track-novice-pathology-chased`.
#[test]
fn a_craft_mid_flight_never_brakes_for_the_corner_ahead() {
    let tuning = Tuning::default();
    let line = Line::new(
        (0..128)
            .map(|step| {
                let angle = std::f32::consts::TAU * step as f32 / 128.0;
                Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
            })
            .collect(),
    );
    // On the ground this is
    // `a_saturated_driver_over_its_corner_speed_brakes_asymmetrically`'s
    // fixture: over the target speed, brakes on.
    let grounded = craft(line.point(0), 200.0);
    let controls_grounded = Driver::default().drive(&grounded, &Context::new(&line, &tuning));
    assert_eq!(controls_grounded.thrust, 0.0, "grounded should still brake");
    assert!(
        controls_grounded.airbrake_left > 0.0 || controls_grounded.airbrake_right > 0.0,
        "grounded should still brake"
    );

    let mut airborne = craft(line.point(0), 200.0);
    airborne.time_airborne = 0.3;
    let controls_airborne = Driver::default().drive(&airborne, &Context::new(&line, &tuning));
    assert_eq!(controls_airborne.thrust, 1.0, "airborne should hold thrust");
    // The low airbrake side is the symmetric brake `airbrakes` slid the
    // differential atop. Zero means no deceleration is bought, though the loop
    // may still spend a **differential** (one side alone decelerates nothing).
    assert_eq!(
        controls_airborne
            .airbrake_left
            .min(controls_airborne.airbrake_right),
        0.0,
        "airborne should not spend a symmetric brake it has no grip for"
    );
}
