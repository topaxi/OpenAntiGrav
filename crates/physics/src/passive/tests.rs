//! What the passive drag, damping and gravity terms in [`super`] are asserted to do. Split out of
//! `passive.rs` under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::params::{Physical, Pitch};

#[test]
fn drag_opposes_travel_while_moving_forward() {
    let velocity = Vec3::new(0.0, 0.0, -40.0);
    let force = quadratic_drag(velocity, 40.0, true);
    assert!(force.z > 0.0, "force was {force:?}");
}

#[test]
fn drag_grows_with_the_square_of_speed() {
    let slow = quadratic_drag(Vec3::new(0.0, 0.0, -10.0), 10.0, true).length();
    let fast = quadratic_drag(Vec3::new(0.0, 0.0, -20.0), 20.0, true).length();
    assert!(
        (fast - slow * 4.0).abs() < 1e-4,
        "{fast} was not four times {slow}"
    );
}

/// The airborne coefficient is the **smaller** one. Pinned because the intuitive ordering is the
/// opposite and a reimplementation tidies it by accident.
#[test]
fn airborne_drag_is_weaker_than_grounded_drag() {
    let velocity = Vec3::new(0.0, 0.0, -40.0);
    let ground = quadratic_drag(velocity, 40.0, true).length();
    let air = quadratic_drag(velocity, 40.0, false).length();
    assert!(air < ground, "air {air} was not below ground {ground}");
}

#[test]
fn reversing_selects_a_much_larger_drag_coefficient() {
    let forward = quadratic_drag(Vec3::new(0.0, 0.0, -40.0), 40.0, true).length();
    let reversing = quadratic_drag(Vec3::new(0.0, 0.0, 40.0), -40.0, true).length();
    assert!(reversing > forward * 10.0);
}

#[test]
fn drag_is_zero_at_rest() {
    assert_eq!(quadratic_drag(Vec3::ZERO, 0.0, true), Vec3::ZERO);
}

#[test]
fn rolling_resistance_is_a_constant_magnitude_opposing_travel() {
    let slow = rolling_resistance(Vec3::new(0.0, 0.0, -1.0), 1.0);
    let fast = rolling_resistance(Vec3::new(0.0, 0.0, -100.0), 100.0);

    assert_eq!(slow, Vec3::new(0.0, 0.0, ROLLING_RESISTANCE));
    assert_eq!(fast, slow);
}

/// The gate is on the forward speed, not on the speed, so a ship falling
/// straight down or sliding sideways gets none of this.
#[test]
fn rolling_resistance_needs_forward_motion_and_is_never_nan() {
    assert_eq!(rolling_resistance(Vec3::ZERO, 0.0), Vec3::ZERO);
    assert_eq!(
        rolling_resistance(Vec3::new(0.0, -50.0, 0.0), 0.0),
        Vec3::ZERO
    );
    assert_eq!(
        rolling_resistance(Vec3::new(0.0, 0.0, 40.0), -40.0),
        Vec3::ZERO
    );
}

/// The nose is turned toward travel by this term alone, so its direction is
/// worth pinning: a ship sliding to its right must be yawed toward its right.
#[test]
fn the_weathervane_turns_the_nose_toward_the_direction_of_travel() {
    let forward = Vec3::NEG_Z;
    // Mostly forward, drifting to the right.
    let velocity = Vec3::new(10.0, 0.0, -40.0);
    let torque = weathervane(forward, velocity, true);

    // A right-handed rotation about -Y turns -Z toward +X, so the yaw component must be negative for
    // the nose to swing right (forward is -Z). With the page's sign this comes out positive and the
    // nose swings away from travel; see `WEATHERVANE_GROUND`.
    assert!(torque.y < 0.0, "torque was {torque:?}");
}

/// The same invariant stated the way the derivation does, so it holds for any
/// attitude and any velocity rather than only the case above.
#[test]
fn the_weathervane_torque_points_from_the_forward_axis_toward_the_velocity() {
    let forward = Vec3::new(0.0, 0.0, -1.0);

    for velocity in [
        Vec3::new(10.0, 0.0, -40.0),
        Vec3::new(-10.0, 0.0, -40.0),
        Vec3::new(0.0, -30.0, -40.0),
        Vec3::new(5.0, 5.0, -40.0),
    ] {
        for grounded in [true, false] {
            let torque = weathervane(forward, velocity, grounded);
            let aligning = forward.cross(velocity);
            assert!(
                torque.dot(aligning) > 0.0,
                "torque {torque:?} was not aligning for velocity {velocity:?}"
            );
        }
    }
}

#[test]
fn the_weathervane_is_three_times_stronger_in_the_air() {
    let forward = Vec3::NEG_Z;
    let velocity = Vec3::new(10.0, 0.0, -40.0);
    let ground = weathervane(forward, velocity, true);
    let air = weathervane(forward, velocity, false);
    assert_eq!(air, ground * 3.0);
}

#[test]
fn the_weathervane_is_zero_when_already_pointing_along_travel() {
    let torque = weathervane(Vec3::NEG_Z, Vec3::new(0.0, 0.0, -40.0), true);
    assert_eq!(torque, Vec3::ZERO);
}

#[test]
fn angular_damping_opposes_rotation_on_every_axis() {
    let handling = Handling {
        pitch: Pitch {
            pitch_damping: 3.0,
            ..Pitch::default()
        },
        ..Handling::ZERO
    };
    let damping = angular_damping(&handling, Vec3::new(1.0, 1.0, 1.0));

    assert!(damping.x < 0.0);
    assert!(damping.y < 0.0);
    assert!(damping.z < 0.0);
}

/// Only the pitch axis is per ship. Yaw and roll are the same for every craft
/// in the game, so a zeroed parameter set still damps them.
#[test]
fn only_the_pitch_axis_of_the_angular_damping_is_tunable() {
    let damping = angular_damping(&Handling::ZERO, Vec3::ONE);
    assert_eq!(damping.x, 0.0);
    assert_eq!(damping.y, YAW_DAMPING);
    assert_eq!(damping.z, ROLL_DAMPING);
}

#[test]
fn vertical_damping_opposes_motion_along_the_ships_own_up_axis() {
    let up = Vec3::Y;
    let falling = vertical_damping(up, Vec3::new(0.0, -10.0, 0.0), 0.0);
    assert!(falling.y > 0.0);

    // Sideways motion is not damped by this term at all.
    assert_eq!(
        vertical_damping(up, Vec3::new(10.0, 0.0, 0.0), 0.0),
        Vec3::ZERO
    );
}

/// `Ship_UpdateCraft`'s inline step 14 scales by `1 - craft+0x2b0`, so a craft
/// on both hover points gets nothing from this term at all.
#[test]
fn a_grounded_craft_gets_no_vertical_damping() {
    let falling = Vec3::new(0.0, -10.0, 0.0);
    assert_eq!(vertical_damping(Vec3::Y, falling, 1.0), Vec3::ZERO);

    // A half contact halves it, because the field is the 0/0.5/1 fraction
    // rather than a flag.
    let half = vertical_damping(Vec3::Y, falling, 0.5);
    let airborne = vertical_damping(Vec3::Y, falling, 0.0);
    assert_eq!(half, airborne * 0.5);
}

#[test]
fn gravity_acts_on_world_down_only() {
    let handling = Handling {
        physical: Physical {
            normal_gravity: 10.0,
            ..Physical::default()
        },
        ..Handling::ZERO
    };
    let force = gravity(&handling, 2.0, 1.0, 1.0);
    assert_eq!(force, Vec3::new(0.0, -20.0, 0.0));
}

/// `track_gravity` is in the hover spring's calibration and **not** in the gravity term (a
/// correction to what was inferred from the field names alone).
#[test]
fn track_gravity_is_not_part_of_the_gravity_force() {
    let mut handling = Handling::ZERO;
    handling.physical.track_gravity = 100.0;
    assert_eq!(gravity(&handling, 1.0, 1.0, 1.0), Vec3::ZERO);
    assert_eq!(gravity(&handling, 1.0, 1.0, 0.0), Vec3::ZERO);
}

#[test]
fn the_two_gravities_blend_by_groundedness_and_only_one_is_class_scaled() {
    let handling = Handling {
        physical: Physical {
            normal_gravity: 10.0,
            flight_gravity: 4.0,
            ..Physical::default()
        },
        ..Handling::ZERO
    };

    assert_eq!(gravity(&handling, 1.0, 1.0, 1.0).y, -10.0);
    assert_eq!(gravity(&handling, 1.0, 1.0, 0.0).y, -4.0);
    assert_eq!(gravity(&handling, 1.0, 1.0, 0.5).y, -7.0);

    // The per-class scale reaches `normal_gravity` and not `flight_gravity`.
    assert_eq!(gravity(&handling, 1.0, 2.0, 1.0).y, -20.0);
    assert_eq!(gravity(&handling, 1.0, 2.0, 0.0).y, -4.0);
}

#[test]
fn a_zero_parameter_set_has_no_gravity() {
    assert_eq!(gravity(&Handling::ZERO, 1.0, 1.0, 1.0), Vec3::ZERO);
    assert_eq!(gravity(&Handling::ZERO, 1.0, 1.0, 0.0), Vec3::ZERO);
}
