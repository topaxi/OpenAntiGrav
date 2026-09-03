use super::*;

fn rng() -> Rng {
    Rng::new(1)
}

#[test]
fn a_fresh_shake_is_inactive_and_identity() {
    let shake = Shake::new();
    assert!(!shake.active());
    assert_eq!(shake.rotation(), Quat::IDENTITY);
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
    assert_eq!(shake.rotation(), Quat::IDENTITY);
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
    // Zero magnitude means zero envelope and zero oscillator at every tick,
    // so the rotation stays identity for the shake's whole, otherwise-active
    // life - a hit clamped to zero severity should look like no hit at all.
    for _ in 0..36 {
        assert_eq!(shake.rotation(), Quat::IDENTITY);
        shake.advance(1.0 / 60.0);
    }
}

#[test]
fn a_stronger_hit_rotates_further_at_the_moment_of_impact() {
    let mut weak = Shake::new();
    weak.arm(0.2, Side::Elsewhere, &mut rng());
    let mut strong = Shake::new();
    strong.arm(1.0, Side::Elsewhere, &mut rng());
    // At progress 0 the oscillator term (`sin(0) = 0`) drops out and only the
    // envelope survives, so the two rotations compare directly by angle.
    let weak_angle = weak.rotation().to_axis_angle().1;
    let strong_angle = strong.rotation().to_axis_angle().1;
    assert!(strong_angle > weak_angle);
}

#[test]
fn every_rotation_stays_about_the_documented_axis() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Ahead, &mut rng());
    for _ in 0..36 {
        let (axis, angle) = shake.rotation().to_axis_angle();
        // `to_axis_angle` picks whichever sign makes `angle` non-negative, so
        // the axis can come back flipped; either way it must lie along `AXIS`.
        if angle > 1e-6 {
            assert!(axis.abs_diff_eq(AXIS, 1e-4) || axis.abs_diff_eq(-AXIS, 1e-4));
        }
        shake.advance(1.0 / 60.0);
    }
}

#[test]
fn arming_again_restarts_the_timer_rather_than_extending_it() {
    let mut shake = Shake::new();
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    shake.advance(DURATION_SECONDS * 0.5);
    shake.arm(1.0, Side::Elsewhere, &mut rng());
    assert_eq!(shake.timer, DURATION_SECONDS);
}
