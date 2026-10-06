//! What the magstrip attitude hold in [`super`] is asserted to do. Split out of `maglock.rs` under
//! the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::collide::{CollisionWorld, TriangleSoup};
use crate::ship::Body;

fn strip(surface: Surface) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        surface,
        0,
    ));
    world
}

/// A level section under a ship at the origin: the surface at `y = 0`, so the
/// lifted spline position sits at `y = 3`.
fn level_section() -> TrackSample {
    TrackSample {
        position: Vec3::new(0.0, SPLINE_LIFT, 0.0),
        down: Vec3::NEG_Y,
    }
}

fn env_on_strip() -> Environment {
    Environment {
        track_sample: Some(level_section()),
        ..Environment::default()
    }
}

fn state_at(height: f32) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(0.0, height, 0.0),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// Five frames up, and six back down, which is not a typo: `1.0 - 0.2` five times leaves
/// `2.98e-8` in `f32`, not zero, and the original's guard is `blend <= 0`, not a tolerance, so
/// the sixth frame clears it. Transcribed, not rounded: a `< 1e-6` clamp would decide the original
/// meant something tidier than it wrote.
#[test]
fn the_blend_ramps_to_full_lock_in_five_frames_and_out_again_in_six() {
    let mut blend = 0.0;
    for _ in 0..5 {
        blend = ramp(blend, true);
    }
    assert_eq!(blend, 1.0);
    // And it stays there.
    assert_eq!(ramp(blend, true), 1.0);

    for _ in 0..5 {
        blend = ramp(blend, false);
    }
    assert!(blend > 0.0 && blend < 1e-6, "the residue was {blend}");
    assert_eq!(ramp(blend, false), 0.0);
    assert_eq!(ramp(0.0, false), 0.0);
}

#[test]
fn the_probe_accepts_a_mag_floor_and_nothing_else() {
    let state = state_at(4.0);
    let env = env_on_strip();

    assert!(probe(&state, &env, &strip(Surface::MagFloor)).is_some());
    assert!(probe(&state, &env, &strip(Surface::Floor)).is_none());
    assert!(probe(&state, &env, &strip(Surface::Wall)).is_none());
}

/// Without a spline sample there is no direction to cast along, so the probe
/// declines rather than guessing one.
#[test]
fn the_probe_needs_a_track_sample() {
    let state = state_at(4.0);
    assert!(probe(&state, &Environment::default(), &strip(Surface::MagFloor)).is_none());
}

/// Ordinary track: the blend is zero, so the function returns before it can
/// touch anything at all. This is what keeps every non-magstrip trajectory in
/// the crate exactly as it was.
#[test]
fn a_ship_off_a_strip_is_not_touched() {
    let mut state = state_at(4.0);
    state.body.linear_velocity = Vec3::new(10.0, -3.0, 1.0);
    state.body.orientation = Quat::from_rotation_z(0.3);
    let before = state;

    let hold = update(&mut state, &Environment::default(), None, 5.0);

    assert!(hold.is_none());
    assert_eq!(state, before);
}

/// Five frames of contact and the ship's own up axis **is** the surface
/// normal, whatever it was before. This is the mechanism.
#[test]
fn a_full_lock_slaves_the_up_axis_to_the_surface_normal() {
    let env = env_on_strip();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    // Rolled and pitched well away from the surface.
    state.body.orientation = Quat::from_rotation_z(0.5) * Quat::from_rotation_x(-0.4);

    for _ in 0..5 {
        update(&mut state, &env, contact, 5.0);
    }

    assert_eq!(state.mag_lock_blend, 1.0);
    let up = state.body.up();
    assert!(
        (up - Vec3::Y).length() < 1e-5,
        "up came out {up:?}, not the surface normal"
    );
    // And the basis is still a basis.
    let right = state.body.right();
    let forward = state.body.forward();
    assert!((right.length() - 1.0).abs() < 1e-5);
    assert!(right.dot(up).abs() < 1e-5);
    assert!(forward.dot(up).abs() < 1e-5);
}

/// The load-bearing property, asserted directly rather than inferred: the hold
/// rotates the basis and leaves the angular velocity column alone. A torque
/// implementation fails this, which is the whole point of pinning it.
#[test]
fn the_hold_rotates_the_basis_without_touching_the_angular_velocity() {
    let env = env_on_strip();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    state.body.orientation = Quat::from_rotation_z(0.5);
    state.body.angular_velocity = Vec3::new(0.1, -0.2, 0.3);
    state.body.torque = Vec3::new(1.0, 2.0, 3.0);
    let before = state.body;

    update(&mut state, &env, contact, 5.0);

    assert_ne!(state.body.orientation, before.orientation);
    assert_eq!(state.body.angular_velocity, before.angular_velocity);
    assert_eq!(state.body.torque, before.torque);
    assert_eq!(state.body.force, before.force);
}

/// A ship held **inverted** under a ceiling strip: the same code, and the only
/// mechanism in the crate that can hold one there at all.
#[test]
fn an_inverted_strip_holds_an_inverted_ship() {
    // A ceiling at y = 0 whose normal points down, so the section's `down`
    // axis points up out of it.
    let section = TrackSample {
        position: Vec3::new(0.0, -SPLINE_LIFT, 0.0),
        down: Vec3::Y,
    };
    let env = Environment {
        track_sample: Some(section),
        ..Environment::default()
    };
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::NEG_Y,
    });

    let mut state = state_at(-4.0);
    // Nearly inverted, but not exactly, so there is something to correct.
    state.body.orientation = Quat::from_rotation_z(std::f32::consts::PI - 0.3);

    for _ in 0..5 {
        update(&mut state, &env, contact, 5.0);
    }

    let up = state.body.up();
    assert!(
        (up - Vec3::NEG_Y).length() < 1e-5,
        "an inverted hold left up at {up:?}"
    );
}

/// The reposition parks the craft at four fifths of the hover target, and does
/// it by displacement: the velocity is not what carries it there.
#[test]
fn the_hold_parks_the_craft_at_four_fifths_of_the_hover_target() {
    let env = env_on_strip();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });
    let target = 5.0;

    let mut state = state_at(1.0);
    for _ in 0..40 {
        update(&mut state, &env, contact, target);
    }

    assert!(
        (state.body.position.y - HOLD_HEIGHT_FRACTION * target).abs() < 1e-3,
        "parked at {}, not {}",
        state.body.position.y,
        HOLD_HEIGHT_FRACTION * target
    );
}

/// The velocity is turned, never shortened: the projection is renormalised to its starting
/// magnitude. `rigid-body.md` ruled this out as the missing linear resistance on that ground;
/// pinned so an "optimisation" dropping the renormalisation is caught.
#[test]
fn the_velocity_projection_preserves_speed() {
    let env = env_on_strip();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    state.body.linear_velocity = Vec3::new(30.0, 12.0, -4.0);
    let speed = state.body.linear_velocity.length();
    state.mag_lock_blend = 1.0;

    update(&mut state, &env, contact, 5.0);

    assert!((state.body.linear_velocity.length() - speed).abs() < 1e-3);
    // And it now lies in the surface plane.
    assert!(state.body.linear_velocity.y.abs() < 1e-4);
}

/// Two sections are weighted by how well each one's height agrees with the
/// height the ray measured, and the weights are normalised to sum to one.
#[test]
fn the_axis_blends_between_the_two_sections_by_height_agreement() {
    // Section 1 level, section 2 banked, the craft's measured height matching
    // section 1 exactly.
    let banked = TrackSample {
        position: Vec3::new(0.0, SPLINE_LIFT, 0.0),
        down: Vec3::new(0.6, -0.8, 0.0),
    };
    let env = Environment {
        track_sample: Some(level_section()),
        track_sample_next: Some(banked),
        ..Environment::default()
    };
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    state.mag_lock_blend = 1.0;
    let hold = update(&mut state, &env, contact, 5.0).expect("a locked ship gets a hold");

    // Both sections carry weight, so the axis is neither one of them: the
    // banked section's own normal leans to `-x`, and it pulls the blend that
    // way without taking it over.
    assert!(hold.axis.x < 0.0, "the banked section contributed nothing");
    assert!(hold.axis.y > 0.0);
    assert!(!hold.from_ray);
    // The primary agrees with the measurement exactly, so it wins the larger
    // share and the blended axis stays nearer to it.
    assert!(hold.axis.normalize().dot(Vec3::Y) > 0.9);
}

/// When the sections disagree with the ray by more than five units the spline
/// is abandoned and the ray's own normal is used. Pinned because it is the
/// branch a caller with no spline data at all also lands on.
#[test]
fn a_section_that_disagrees_with_the_ray_is_abandoned_for_it() {
    // A section twenty units below where the ray says the floor is.
    let env = Environment {
        track_sample: Some(TrackSample {
            position: Vec3::new(0.0, SPLINE_LIFT - 20.0, 0.0),
            down: Vec3::NEG_Y,
        }),
        ..Environment::default()
    };
    let normal = Vec3::new(0.0, 0.6, 0.8).normalize();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal,
    });

    let mut state = state_at(4.0);
    state.mag_lock_blend = 1.0;
    let hold = update(&mut state, &env, contact, 5.0).expect("a locked ship gets a hold");

    assert!(hold.from_ray);
    assert_eq!(hold.axis, normal);
}

/// A caller that supplies a contact but no spline still gets the mechanism, off the ray's own
/// hit: the original's fallback branch, not an invented rule. It is *not* a way for a caller with
/// no track data to get a hold by accident: [`probe`] needs a sample to cast along, so a caller
/// supplying neither gets no contact, blend or hold (`oag-trace`'s replay, per this module's note).
#[test]
fn a_caller_with_no_spline_falls_back_to_the_rays_own_normal() {
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    state.body.orientation = Quat::from_rotation_z(0.4);
    for _ in 0..5 {
        update(&mut state, &Environment::default(), contact, 5.0);
    }

    assert!((state.body.up() - Vec3::Y).length() < 1e-5);
}

/// The blend decays over five frames after the strip ends, and the hold keeps
/// using the last hit while it does - `craft+0x250` is only overwritten by a
/// hit, so the original reads a stale one here too.
#[test]
fn the_hold_fades_out_over_five_frames_on_the_last_known_contact() {
    let env = env_on_strip();
    let contact = Some(MagContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
    });

    let mut state = state_at(4.0);
    for _ in 0..5 {
        update(&mut state, &env, contact, 5.0);
    }
    assert_eq!(state.mag_lock_blend, 1.0);

    // Off the strip: no contact this frame, but the stored one is still there.
    let mut blends = Vec::new();
    for _ in 0..6 {
        update(&mut state, &Environment::default(), None, 5.0);
        blends.push(state.mag_lock_blend);
    }
    assert_eq!(blends.len(), 6);
    assert!((blends[0] - 0.8).abs() < 1e-6);
    // Five subtractions leave an `f32` residue rather than zero; see
    // `the_blend_ramps_to_full_lock_in_five_frames_and_out_again_in_six`.
    assert!(blends[4] > 0.0 && blends[4] < 1e-6);
    assert_eq!(blends[5], 0.0);
    assert!(state.mag_contact.is_some());
}
