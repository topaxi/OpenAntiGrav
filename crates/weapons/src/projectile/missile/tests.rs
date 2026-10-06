//! What the Missile's lock, guidance and speed ramp are asserted to do. Fixture
//! numbers are invented (ADR-0006). These pin the recovered arithmetic against the
//! shape it was recovered as; whether a missile gets anywhere on a real circuit is
//! `crates/game/tests/missile_ground_truth.rs`'s job.

use super::*;

use oag_tables::weapons::Weapon;

/// A stats block with a wide, unambiguous lock window.
fn stats() -> MissileStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Missile"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="100"
               lock_min_dist="10" lock_max_dist="200"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .missile()
    .expect("a Missile")
}

/// A grid of parked craft at the given positions, all active.
fn ships(positions: &[Vec3]) -> Vec<crate::test_craft::Ship> {
    positions
        .iter()
        .map(|&position| {
            let mut ship = crate::test_craft::Ship {
                active: true,
                ..crate::test_craft::Ship::default()
            };
            ship.physics.body.position = position;
            ship
        })
        .collect()
}

#[test]
fn the_speed_ramp_starts_at_the_launch_speed_and_ends_at_the_class_speed() {
    assert_eq!(speed_kmh(200.0, 800.0, 0.0), 200.0);
    assert_eq!(speed_kmh(200.0, 800.0, 0.5), 500.0);
    // At and past the window it is the class speed flat, with no overshoot.
    assert_eq!(speed_kmh(200.0, 800.0, 1.0), 800.0);
    assert_eq!(speed_kmh(200.0, 800.0, 9.0), 800.0);
}

/// The blend is over a code-literal second, not the authored `slowdown_time`; both
/// read 1.0 on the disc, so this asserts the constant itself.
#[test]
fn the_ramp_window_is_one_second_and_not_the_authored_slowdown_time() {
    assert_eq!(SPEED_RAMP_SECONDS, 1.0);
    // Just inside the window still blends; just outside is flat.
    assert!(speed_kmh(0.0, 100.0, 0.99) < 100.0);
    assert_eq!(speed_kmh(0.0, 100.0, 1.01), 100.0);
}

/// The guidance clamp is on a chord of the unit heading, so a missile pointed away
/// turns at a bounded rate rather than snapping round.
#[test]
fn guidance_turns_at_a_bounded_rate_when_the_error_is_large() {
    let dt = 1.0 / 60.0;
    let speed = 100.0;
    // Flying +Z, target dead astern: the error chord is the maximum, 2.0.
    let out = steer(Vec3::Z * speed, Vec3::ZERO, -Vec3::Z * 300.0, dt, speed);
    let heading = out.normalize();
    // It turned, but nowhere near round.
    assert!(
        heading.dot(Vec3::Z) > 0.0,
        "it reversed in one tick: {out:?}"
    );
    // The correction applied is the clamped chord and not the whole error.
    let applied = (heading - Vec3::Z).length();
    assert!(
        applied < dt * TURN_CHORD_PER_SECOND + 1e-4,
        "turned by {applied}, past the {}-per-second chord",
        TURN_CHORD_PER_SECOND
    );
}

/// When the error is small the clamp does not bite and the whole error is taken in
/// one tick (the `else` arm of `|step|^2 <= |err|^2`); otherwise it would circle.
#[test]
fn guidance_takes_the_whole_error_when_it_is_smaller_than_one_tick_of_turn() {
    let dt = 1.0 / 60.0;
    let speed = 100.0;
    // A target a whisker off the nose - error chord far under `dt * 4.0`.
    let target = Vec3::new(0.01, 0.0, 300.0);
    let out = steer(Vec3::Z * speed, Vec3::ZERO, target, dt, speed);
    let desired = target.normalize();
    assert!(
        out.normalize().dot(desired) > 0.999_9,
        "a small error should be taken whole, got {out:?}"
    );
}

/// The direction is deliberately not renormalised before the speed scale, so a
/// turning missile flies slower than its ramp says. Pinned with the closed form:
/// the intuition runs the other way, and a first disassembly reading recorded that.
#[test]
fn a_turning_missile_flies_slower_than_its_pinned_speed() {
    let dt = 1.0 / 60.0;
    let speed = 100.0;
    // A right-angle turn.
    let out = steer(Vec3::Z * speed, Vec3::ZERO, Vec3::X * 300.0, dt, speed);
    let ratio = out.length() / speed;
    assert!(
        ratio < 1.0,
        "the direction was renormalised somewhere - got exactly {speed}"
    );

    // |v + s*u|^2 = 1 - 2*s*sin(t/2) + s^2, with s the clamped chord.
    let half_angle_sin = (std::f32::consts::FRAC_PI_4).sin();
    let step = dt * TURN_CHORD_PER_SECOND;
    let expected = (1.0 - 2.0 * step * half_angle_sin + step * step).sqrt();
    assert!(
        (ratio - expected).abs() < 1e-5,
        "speed ratio {ratio} against the predicted {expected}"
    );

    // It returns to exactly one on the line: the clamp takes the whole error.
    let lined_up = steer(
        Vec3::Z * speed,
        Vec3::ZERO,
        Vec3::new(0.001, 0.0, 300.0),
        dt,
        speed,
    );
    assert!((lined_up.length() / speed - 1.0).abs() < 1e-5);
}

/// A degenerate input leaves the velocity alone rather than producing a NaN.
#[test]
fn guidance_declines_rather_than_dividing_by_zero() {
    let dt = 1.0 / 60.0;
    assert_eq!(
        steer(Vec3::ZERO, Vec3::ZERO, Vec3::Z, dt, 100.0),
        Vec3::ZERO
    );
    // Target exactly on top of the missile.
    let flying = Vec3::Z * 5.0;
    assert_eq!(steer(flying, Vec3::ZERO, Vec3::ZERO, dt, 100.0), flying);
}

/// A missile never locks the craft that fired it (the blast does not exclude its
/// owner).
#[test]
fn the_lock_never_picks_the_firer() {
    let grid = ships(&[Vec3::ZERO, Vec3::Z * 50.0]);
    assert_eq!(lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats(), None), Some(1));
    // Firing backwards from slot 1, slot 0 is the only candidate and is picked;
    // asking slot 0 to lock with nobody else on the grid finds nothing.
    let alone = ships(&[Vec3::ZERO]);
    assert_eq!(lock(&alone, 0, Vec3::ZERO, Vec3::Z, &stats(), None), None);
}

/// The window is on the longitudinal distance: a craft alongside is out however
/// close, one past the far bound out however square-on.
#[test]
fn the_lock_window_is_measured_along_the_nose() {
    let stats = stats();
    // Dead alongside at 50 units: `along` is 0, under `lock_min_dist`.
    let alongside = ships(&[Vec3::ZERO, Vec3::X * 50.0]);
    assert_eq!(
        lock(&alongside, 0, Vec3::ZERO, Vec3::Z, &stats, None),
        None,
        "a craft with zero longitudinal distance is inside no window"
    );
    // Straight ahead but past `lock_max_dist`.
    let far = ships(&[Vec3::ZERO, Vec3::Z * 500.0]);
    assert_eq!(lock(&far, 0, Vec3::ZERO, Vec3::Z, &stats, None), None);
    // And too close.
    let near = ships(&[Vec3::ZERO, Vec3::Z * 5.0]);
    assert_eq!(lock(&near, 0, Vec3::ZERO, Vec3::Z, &stats, None), None);
}

/// Inside the window but outside the cone is refused. Two craft at the same
/// longitudinal distance, so only the bearing separates them.
#[test]
fn the_lock_refuses_a_craft_outside_the_cone() {
    let stats = stats();
    let along = 100.0;
    // 0.9 is about 26 degrees; 100 across at 100 along is 45.
    let wide = ships(&[Vec3::ZERO, Vec3::new(100.0, 0.0, along)]);
    assert_eq!(lock(&wide, 0, Vec3::ZERO, Vec3::Z, &stats, None), None);
    let narrow = ships(&[Vec3::ZERO, Vec3::new(10.0, 0.0, along)]);
    assert_eq!(lock(&narrow, 0, Vec3::ZERO, Vec3::Z, &stats, None), Some(1));
}

/// Two candidates inside every gate: the nearer along the nose wins, not the better
/// bearing. The nearer one is the more off-axis, so either key would not pass.
#[test]
fn the_lock_picks_the_nearest_by_longitudinal_distance() {
    let grid = ships(&[
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 150.0),
        Vec3::new(12.0, 0.0, 80.0),
    ]);
    assert_eq!(lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats(), None), Some(2));
}

/// An inactive slot is not a target: it holds what the last race left in it.
#[test]
fn the_lock_skips_an_inactive_slot() {
    let mut grid = ships(&[Vec3::ZERO, Vec3::Z * 100.0]);
    grid[1].active = false;
    assert_eq!(lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats(), None), None);
}

/// The along-track screen rejects a craft near in space and far round the circuit
/// (the hairpin). With no circuit length it is skipped, as for a synthetic straight.
#[test]
fn the_lock_refuses_a_craft_that_is_close_in_space_and_far_round_the_lap() {
    let stats = stats();
    let mut grid = ships(&[Vec3::ZERO, Vec3::Z * 100.0]);
    grid[0].standing.progress = Some(0.0);
    // 900 of a 1000-unit lap away, which wraps to -100: |gap| 100 against a range
    // of 100 is a ratio of 1.0, inside the 1.4 bound.
    grid[1].standing.progress = Some(900.0);
    assert_eq!(
        lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats, Some(1000.0)),
        Some(1),
        "a wrapped gap must be measured the short way round"
    );

    // 400 units of tarmac for 100 of air: a ratio of 4.0.
    grid[1].standing.progress = Some(400.0);
    assert_eq!(
        lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats, Some(1000.0)),
        None
    );
    // And with no course to measure against, the screen does not apply.
    assert_eq!(lock(&grid, 0, Vec3::ZERO, Vec3::Z, &stats, None), Some(1));
}

/// A missile leaves at the firing craft's speed plus `launchSpeed`, which is what
/// `launchSpeed` turned out to be for.
#[test]
fn the_launch_speed_is_the_craft_speed_plus_the_authored_offset() {
    let stats = stats();
    let dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };

    let mut state = ShipState::default();
    state.body.position = Vec3::ZERO;
    // 100 units/s is 360 km/h, plus the fixture's launchSpeed of 100.
    state.body.linear_velocity = state.body.forward() * 100.0;
    let (_, velocity, launch_kmh) = launch(&state, &dimensions, &stats, "VENOM");
    assert!(
        (launch_kmh - 460.0).abs() < 1e-3,
        "launch speed {launch_kmh} km/h, expected 360 + 100"
    );
    assert!(
        (velocity.length() - launch_kmh * KMH_TO_UNITS_PER_SECOND).abs() < 1e-3,
        "the launch velocity does not match the launch speed"
    );

    // A parked craft still gets out of the tube: the floor applies.
    let parked = ShipState::default();
    let (_, slow, base) = launch(&parked, &dimensions, &stats, "VENOM");
    assert_eq!(base, stats.launch_speed, "the ramp blends from the raw sum");
    assert!(slow.length() > 0.0);
}

/// The spawn point is outside the firing hull, so the sweep need not special-case
/// a projectile starting inside a craft.
#[test]
fn a_missile_launches_from_the_nose_and_not_the_centre() {
    let dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };
    let state = ShipState::default();
    let (position, _, _) = launch(&state, &dimensions, &stats(), "VENOM");
    assert!(
        position.length() > 0.0,
        "the missile spawned inside the craft"
    );
}

/// The multiply carries the executable's own bit pattern, exactly `1.0 / 3.6`
/// (pinned because an earlier reading claimed otherwise; an edit toward a "more
/// correct" value fails here). The two conversions still differ as operations:
/// `x * (1/3.6)` and `x / 3.6` are different `f32` functions, each spent where the
/// original spends it.
#[test]
fn the_speed_conversion_is_the_executables_own_constant() {
    assert_eq!(KMH_TO_UNITS_PER_SECOND.to_bits(), 0x3e8e_38e4);
    assert_eq!(
        KMH_TO_UNITS_PER_SECOND.to_bits(),
        (1.0f32 / 3.6).to_bits(),
        "the executable's literal and the rounded reciprocal have stopped agreeing"
    );
    // Separate code paths; neither is derived from the other.
    let kmh = 800.0;
    assert!((speed_units_guided(kmh) - speed_units_on_surface(kmh)).abs() < 1e-3);
}

/// `Weapon::Missile` is what `Projectiles::advance` branches on; a rename splitting
/// the two would silently stop every missile homing.
#[test]
fn the_guided_weapon_is_the_missile() {
    assert_eq!(Weapon::Missile.as_type(), "Missile");
}
