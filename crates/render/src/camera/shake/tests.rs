use super::*;

fn rng() -> Rng {
    Rng::new(1)
}

/// A representative pair of basis rows, orthogonal so the two rotations this
/// module composes are easy to reason about by hand. No claim that these are
/// the original's own numbers - see the module documentation's "modeling
/// choice" section for why the physical meaning of `row1`/`row2` is open.
fn rows() -> (Vec3, Vec3) {
    (Vec3::Y, Vec3::Z)
}

#[test]
fn a_fresh_shake_is_inactive_and_identity() {
    let shake = Shake::new();
    assert!(!shake.active());
    let (row1, row2) = rows();
    assert_eq!(shake.rotation(row1, row2), Quat::IDENTITY);
}

#[test]
fn arming_makes_it_active_at_the_full_duration() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    assert!(shake.active());
    assert_eq!(shake.timer, DURATION_SECONDS);
}

#[test]
fn advancing_past_the_duration_deactivates_it() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    shake.advance(DURATION_SECONDS + 1.0);
    assert!(!shake.active());
    let (row1, row2) = rows();
    assert_eq!(shake.rotation(row1, row2), Quat::IDENTITY);
}

#[test]
fn the_timer_never_goes_negative() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    shake.advance(1000.0);
    assert_eq!(shake.timer, 0.0);
}

#[test]
fn magnitude_scales_with_severity() {
    let mut low = Shake::new();
    low.arm(0.5, Side::Elsewhere, &mut rng());
    let mut high = Shake::new();
    high.arm(1.0, Side::Elsewhere, &mut rng());
    assert_eq!(low.magnitude, 0.5 * MAGNITUDE_SCALE);
    assert_eq!(high.magnitude, 1.0 * MAGNITUDE_SCALE);
}

#[test]
fn the_envelope_walks_the_three_keys() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    let magnitude = MAGNITUDE_SCALE;
    // At progress 0 (the moment of impact) the envelope is `magnitude * 0.25`.
    assert_eq!(shake.envelope(0.0), magnitude * 0.25);
    // At progress 0.3 it has fallen to `magnitude * 0.125`.
    assert!((shake.envelope(0.3) - magnitude * 0.125).abs() < 1e-6);
    // At progress 1.0 (the shake's own end) it has decayed to nothing.
    assert!(shake.envelope(1.0).abs() < 1e-6);
    // Halfway between the second and third key it is the midpoint of the two.
    let mid = shake.envelope(0.65);
    assert!(mid > 0.0 && mid < magnitude * 0.125);
}

#[test]
fn zero_severity_arms_no_rotation_at_all() {
    let mut shake = Shake::new();
    shake.arm(0.0, Side::Elsewhere, &mut rng());
    let (row1, row2) = rows();
    // Zero magnitude means zero envelope and zero oscillator at every tick,
    // so the rotation stays identity for the shake's whole, otherwise-active
    // life - a hit clamped to zero severity should look like no hit at all.
    for _ in 0..36 {
        assert_eq!(shake.rotation(row1, row2), Quat::IDENTITY);
        shake.advance(1.0 / 60.0);
    }
}

#[test]
fn a_stronger_hit_rotates_further_at_the_moment_of_impact() {
    let mut weak = Shake::new();
    weak.arm(0.2, Side::Elsewhere, &mut rng());
    let mut strong = Shake::new();
    strong.arm(1.0, Side::Elsewhere, &mut rng());
    let (row1, row2) = rows();
    // At progress 0 the oscillator term (`sin(0) = 0`) drops out and only the
    // envelope survives, so the two rotations compare directly by angle.
    let weak_angle = weak.rotation(row1, row2).to_axis_angle().1;
    let strong_angle = strong.rotation(row1, row2).to_axis_angle().1;
    assert!(strong_angle > weak_angle);
}

/// At the moment of impact the oscillator (`sin(0 * BASE_FREQUENCY) = 0`) is
/// exactly zero, so the second call's angle is exactly zero and its rotation
/// is the identity - `rotation()` collapses to the single `row1` rotation by
/// the envelope term alone. Pins the per-mode angle arithmetic without
/// touching either open handedness question.
#[test]
fn at_the_moment_of_impact_only_row1_contributes() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    let (row1, row2) = rows();
    let expected = quat_from_axis_angle(row1, MAGNITUDE_SCALE * 0.25);
    assert!(shake.rotation(row1, row2).abs_diff_eq(expected, 1e-5),);
}

/// The two modes share every number except the sign of the first rotation's
/// angle - `Camera_ArmShake`'s own `mode = 3` vs `mode = 1` branch, both
/// still driven by the same envelope and oscillator terms.
#[test]
fn ahead_negates_only_the_first_angle() {
    let mut ahead = Shake::new();
    ahead.arm(1.0, Side::Ahead, &mut rng());
    let mut elsewhere = Shake::new();
    elsewhere.arm(1.0, Side::Elsewhere, &mut rng());
    let (row1, row2) = rows();
    // At progress 0 the oscillator drops out, so `Ahead`'s single surviving
    // rotation is exactly `Elsewhere`'s, negated.
    let ahead_angle = ahead.rotation(row1, row2).to_axis_angle();
    let elsewhere_angle = elsewhere.rotation(row1, row2).to_axis_angle();
    assert!((ahead_angle.1 - elsewhere_angle.1).abs() < 1e-5);
    assert!(ahead_angle.0.abs_diff_eq(-elsewhere_angle.0, 1e-4));
}

/// Two rotations about two different vectors do not commute - swapping which
/// row is read first changes the result. This is the property a single,
/// summed-angle axis (the approximation this module replaces) cannot have,
/// so it is the test that would fail if `rotation()` ever regressed to one.
#[test]
fn the_two_rotations_do_not_commute() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    // Advance somewhat so the oscillator term is nonzero and the second
    // rotation actually contributes something to compare against.
    shake.advance(DURATION_SECONDS * 0.1);
    let (row1, row2) = rows();
    let in_order = shake.rotation(row1, row2);
    let swapped = shake.rotation(row2, row1);
    assert_ne!(in_order, swapped);
}

/// Different live basis rows produce a different rotation - the whole point
/// of threading them in rather than reading a module-level constant.
#[test]
fn the_rotation_follows_the_basis_it_is_given() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    let with_y_z = shake.rotation(Vec3::Y, Vec3::Z);
    let with_x_z = shake.rotation(Vec3::X, Vec3::Z);
    assert_ne!(with_y_z, with_x_z);
}

#[test]
fn arming_again_restarts_the_timer_rather_than_extending_it() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    shake.advance(DURATION_SECONDS * 0.5);
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    assert_eq!(shake.timer, DURATION_SECONDS);
}
