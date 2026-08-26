//! What a fired weapon *shows*: the flare gate, the bounce gate and the
//! Missile's own orbiting-anchor geometry - the pure functions
//! `crate::race::weapons::visuals` exposes for exactly this, testable with
//! no disc.
//!
//! Split out of `race/tests/weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Only the Rocket and the Missile ride a flare, and each rides its **own**:
/// `WO_ROCKET_FLARE` and `WO_MISSILE_HEAD` are two separate authored files,
/// not one generic "something is in the air" marker.
///
/// **Regression test for a real bug, not a speculative one.** This gate used
/// to be `kind.is_none()`, true for *every* live projectile, which attached
/// the Rocket's looping flare to a Mine or a Bomb the instant it was laid -
/// a fire riding a charge from the moment it landed, reported from play
/// against real Pulse and impossible to reproduce headlessly because the
/// test harness's `psys::Library` carries no loaded effects (see
/// `Race::advance_projectile_flares`'s doc comment for why the gate itself,
/// rather than the attach, is what is tested here).
#[test]
fn each_projectile_rides_only_its_own_flare() {
    use oag_formats::weapons::Weapon;

    for weapon in Weapon::ALL {
        let expected = match weapon {
            Weapon::Rocket => Some(crate::race::ROCKET_FLARE_EFFECT),
            Weapon::Missile => Some(crate::race::MISSILE_FLARE_EFFECT),
            _ => None,
        };
        assert_eq!(
            crate::race::weapons::flare_effect_for(Some(weapon)),
            expected,
            "{weapon:?} rides the wrong flare"
        );
    }
    assert_eq!(
        crate::race::weapons::flare_effect_for(None),
        None,
        "an empty slot rides nothing"
    );
}

/// Only a live Missile whose own counter just went up counts as a bounce -
/// not another kind sharing the same nonzero value, and not a Missile whose
/// counter fell (a fresh spawn landing in a slot the old one's `bounces`
/// snapshot was taken from).
#[test]
fn only_a_missiles_own_rising_counter_is_a_bounce() {
    use crate::race::weapons::bounced_this_tick;
    use oag_formats::weapons::Weapon;

    assert!(
        bounced_this_tick(Some(Weapon::Missile), 1, 2),
        "the counter rose on a live Missile"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Missile), 2, 2),
        "an unchanged counter is not a new bounce"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Missile), 3, 0),
        "a falling counter is a fresh spawn in the slot, never a bounce"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Rocket), 1, 2),
        "a Rocket never bounces, whatever a stray counter says"
    );
    assert!(
        !bounced_this_tick(None, 1, 2),
        "an empty slot cannot have bounced"
    );
}

/// At the instant of launch (`age == 0`), the primary anchor sits exactly on
/// the missile - `sin(0) == 0` and `vertical == 0` zero out every offset
/// term - and the second sits one `lateral` unit out along the missile's own
/// right vector. Both are read off `Missile_Init`'s two `Psys_Spawn_q` calls
/// via `Missile_Update`'s formula; see `missile_flare_anchors`'s doc comment.
#[test]
fn the_missiles_flare_anchors_coincide_with_it_at_the_instant_of_launch() {
    use crate::race::weapons::missile_flare_anchors;

    let position = Vec3::new(10.0, 2.0, -5.0);
    let velocity = Vec3::new(0.0, 0.0, -1.0);
    let (a, b) = missile_flare_anchors(position, velocity, 0.0);

    assert!(
        (a - position).length() < 1e-5,
        "the primary anchor must start exactly on the missile: {a:?}"
    );
    assert!(
        ((b - position).length() - 1.5).abs() < 1e-5,
        "the second anchor must start 1.5 units out along the right vector: {b:?}"
    );
}

/// The two anchors are always the same distance apart from `position` summed
/// together, whatever `θ` (the flight time) is - `sin(θ) + (1 - sin(θ))` and
/// `cos(θ) + (1 - cos(θ))` are both `1` for any `θ`. A rotation invariant
/// that holds without pinning a single trig value, so it exercises the whole
/// formula rather than one sampled angle.
#[test]
fn the_two_anchors_sum_to_a_theta_independent_point() {
    use crate::race::weapons::missile_flare_anchors;

    let position = Vec3::new(-4.0, 1.0, 8.0);
    let velocity = Vec3::new(3.0, 0.5, -2.0);
    let lateral = 1.5;

    // Recomputed the same way the function does, so this checks the
    // invariant against the function's own (unexported) geometry rather
    // than hardcoding a basis of the test's own.
    let back = (-velocity).try_normalize().unwrap();
    let up = (Vec3::Y - back * Vec3::Y.dot(back))
        .try_normalize()
        .unwrap();
    let right = up.cross(back);

    for age in [0.0_f32, 0.05, 0.3, 1.0, 2.7] {
        let (a, b) = missile_flare_anchors(position, velocity, age);
        let vertical = (age * 6.0).min(3.0);
        // a + b = 2*position + right*lateral + up*vertical, independent of θ.
        let sum = a + b;
        let expected = position * 2.0 + right * lateral + up * vertical;
        assert!(
            (sum - expected).length() < 1e-4,
            "age {age}: anchors summed to {sum:?}, expected {expected:?}"
        );
    }
}

/// A missile flying exactly parallel to world up must not produce a NaN
/// basis - the primary degenerate case `missile_flare_anchors` names in its
/// own doc comment.
#[test]
fn a_missile_flying_straight_up_gets_a_finite_basis() {
    use crate::race::weapons::missile_flare_anchors;

    let (a, b) = missile_flare_anchors(Vec3::ZERO, Vec3::Y, 0.4);
    assert!(a.is_finite(), "primary anchor went non-finite: {a:?}");
    assert!(b.is_finite(), "second anchor went non-finite: {b:?}");
}
