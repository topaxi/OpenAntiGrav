//! What the orbit camera's pure half is asserted to do: the key-held
//! integration, the clamps, and the screen-plane basis panning moves along.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `orbit.rs`:
//! the tests are past the 200 lines an inline test module may hold. See
//! `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

fn held(set: impl FnOnce(&mut Held)) -> Held {
    let mut held = Held::default();
    set(&mut held);
    held
}

#[test]
fn no_keys_held_leaves_the_camera_still() {
    let start = Orbit {
        yaw: 0.1,
        pitch: 0.2,
        zoom: 1.5,
        pan: Vec3::ZERO,
    };
    assert_eq!(advance(start, &Held::default(), 1.0), start);
}

#[test]
fn zero_dt_leaves_the_camera_still_even_with_keys_held() {
    let start = Orbit {
        yaw: 0.1,
        pitch: 0.2,
        zoom: 1.5,
        pan: Vec3::ZERO,
    };
    let held = held(|h| {
        h.yaw_pos = true;
        h.pitch_pos = true;
        h.zoom_in = true;
    });
    assert_eq!(advance(start, &held, 0.0), start);
}

#[test]
fn yaw_left_and_right_move_opposite_ways() {
    let start = Orbit {
        yaw: 0.0,
        pitch: 0.0,
        zoom: 1.0,
        pan: Vec3::ZERO,
    };
    let right = advance(start, &held(|h| h.yaw_pos = true), 1.0);
    let left = advance(start, &held(|h| h.yaw_neg = true), 1.0);
    assert!(right.yaw > start.yaw);
    assert!(left.yaw < start.yaw);
    assert_eq!(right.yaw, -left.yaw);
}

#[test]
fn opposite_keys_held_together_cancel() {
    let start = Orbit {
        yaw: 0.3,
        pitch: 0.0,
        zoom: 1.0,
        pan: Vec3::ZERO,
    };
    let held = held(|h| {
        h.yaw_neg = true;
        h.yaw_pos = true;
    });
    assert_eq!(advance(start, &held, 1.0), start);
}

#[test]
fn pitch_clamps_short_of_vertical_however_long_held() {
    let start = Orbit {
        yaw: 0.0,
        pitch: 0.0,
        zoom: 1.0,
        pan: Vec3::ZERO,
    };
    let up = advance(start, &held(|h| h.pitch_pos = true), 1000.0);
    assert_eq!(up.pitch, PITCH_LIMIT);
    let down = advance(start, &held(|h| h.pitch_neg = true), 1000.0);
    assert_eq!(down.pitch, -PITCH_LIMIT);
}

#[test]
fn zoom_clamps_to_its_range_however_long_held() {
    let start = Orbit {
        yaw: 0.0,
        pitch: 0.0,
        zoom: 1.0,
        pan: Vec3::ZERO,
    };
    let close = advance(start, &held(|h| h.zoom_in = true), 1000.0);
    assert_eq!(close.zoom, *ZOOM_RANGE.start());
    let far = advance(start, &held(|h| h.zoom_out = true), 1000.0);
    assert_eq!(far.zoom, *ZOOM_RANGE.end());
}

/// The two axes panning moves along are a right-handed screen frame, at
/// every angle the camera can reach.
///
/// Unit length and perpendicular is the whole correctness claim: a `right`
/// that shortened with pitch would make a horizontal drag slow down as the
/// camera rose, and a pair that stopped being perpendicular would skew the
/// pan off the plane the viewer is looking at.
#[test]
fn the_pan_basis_is_orthonormal_at_every_reachable_angle() {
    for yaw_step in -8..=8 {
        for pitch_step in -8..=8 {
            let orbit = Orbit {
                yaw: yaw_step as f32 * 0.4,
                pitch: (pitch_step as f32 * 0.2).clamp(-PITCH_LIMIT, PITCH_LIMIT),
                ..Orbit::default()
            };
            let (right, up) = orbit.basis();
            let at = format!("yaw {} pitch {}", orbit.yaw, orbit.pitch);
            assert!((right.length() - 1.0).abs() < 1e-5, "right at {at}");
            assert!((up.length() - 1.0).abs() < 1e-5, "up at {at}");
            assert!(right.dot(up).abs() < 1e-5, "perpendicular at {at}");
            assert!(
                right.y.abs() < 1e-6,
                "right must stay on the horizon at {at}"
            );
        }
    }
}

/// Panning right then left returns the look-at point to where it started.
#[test]
fn opposite_pans_cancel_and_the_pan_keys_move_opposite_ways() {
    let start = Orbit {
        yaw: 0.7,
        pitch: 0.3,
        ..Orbit::default()
    };
    let right = advance(start, &held(|h| h.pan_right = true), 1.0);
    let left = advance(start, &held(|h| h.pan_left = true), 1.0);
    assert!(right.pan.length() > 0.0);
    assert!((right.pan + left.pan).length() < 1e-6);

    let both = advance(
        start,
        &held(|h| {
            h.pan_left = true;
            h.pan_right = true;
        }),
        1.0,
    );
    assert_eq!(both, start);
}

/// A pan is along the *camera's* plane, not the world axes.
///
/// The failure this rules out is the easy one: panning along world X and Y
/// regardless of where the camera is, which looks right at yaw zero and
/// wrong everywhere else.
#[test]
fn panning_follows_the_camera_rather_than_the_world_axes() {
    let facing = Orbit::default().panned(1.0, 0.0);
    let turned = Orbit {
        yaw: std::f32::consts::FRAC_PI_2,
        ..Orbit::default()
    }
    .panned(1.0, 0.0);

    assert!(
        facing.pan.dot(turned.pan).abs() < 1e-5,
        "a quarter turn should send the same drag somewhere perpendicular, \
         got {:?} and {:?}",
        facing.pan,
        turned.pan
    );
}

/// Rotating after panning keeps the look-at point where it was put.
///
/// This is what `Orbit::pan` being a 3-D offset rather than two screen
/// numbers buys, and it is the difference between "pick a corner and look
/// at it from anywhere" and "the corner slides away as you turn".
#[test]
fn rotating_after_panning_does_not_drag_the_look_at_point() {
    let panned = Orbit::default().panned(0.8, 0.4);
    let turned = advance(panned, &held(|h| h.yaw_pos = true), 1.0);
    assert_eq!(turned.pan, panned.pan);
    assert!(turned.yaw > panned.yaw);
}

#[test]
fn pan_clamps_to_its_limit_however_long_held() {
    let far = advance(Orbit::default(), &held(|h| h.pan_right = true), 1000.0);
    assert!((far.pan.length() - PAN_LIMIT).abs() < 1e-5);
}

/// Recentring puts the look-at point back and leaves the angle alone.
#[test]
fn recentring_moves_the_target_and_nothing_else() {
    let moved = Orbit {
        yaw: 0.5,
        pitch: -0.2,
        zoom: 2.0,
        ..Orbit::default()
    }
    .panned(1.0, -0.5);
    let home = moved.recentred();

    assert_eq!(home.pan, Vec3::ZERO);
    assert_eq!((home.yaw, home.pitch, home.zoom), (0.5, -0.2, 2.0));
}
