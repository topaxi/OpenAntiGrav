//! Personality: the inside line, the corridor drift, corner commitment, and
//! what a driver remembers between ticks.
//!
//! One theme of `driver.rs`'s tests, split by subject; fixtures stay in [`super`].

use super::*;

/// `inside` is the one axis that reads the geometry rather than the craft,
/// so it has to move the aim toward the inside of a real corner - and it
/// has to be zero on a straight, or it is just a second `line_bias`.
#[test]
fn an_inside_line_leans_into_the_corner_and_not_on_a_straight() {
    let tuning = Tuning::default();
    let driver = Driver::seeded(11);
    let plain = Personality {
        inside: 0.0,
        ..Personality::NEUTRAL
    };
    let keen = Personality {
        inside: 0.6,
        ..Personality::NEUTRAL
    };

    let corner = right_hand_corner(120.0);
    let aim = corner.aim(0, 40.0);
    let lateral = aim.corridor.expect("the fixture has a corridor").lateral;
    let across = |personality: &Personality| {
        driver
            .drift(&aim, 40.0, &Context::new(&corner, &tuning), personality)
            .dot(lateral)
    };
    assert!(
        across(&keen) > across(&plain) + 0.5,
        "an inside line should pull toward the corner: {} against {}",
        across(&keen),
        across(&plain)
    );

    // The same pilot on a straight has no corner to lean into.
    let straight = straight_with_corridor();
    let flat = straight.aim(0, 40.0);
    let flat_lateral = flat.corridor.expect("the fixture has a corridor").lateral;
    let on_a_straight = |personality: &Personality| {
        driver
            .drift(&flat, 40.0, &Context::new(&straight, &tuning), personality)
            .dot(flat_lateral)
    };
    assert!(
        (on_a_straight(&keen) - on_a_straight(&plain)).abs() < 1.0e-3,
        "an inside line should do nothing on a straight"
    );
}

/// Seed zero has to stay the driver every other test in this file measures.
#[test]
fn seed_zero_is_the_plain_line_follower() {
    assert_eq!(Personality::from_seed(0), Personality::NEUTRAL);
    assert_eq!(
        Driver::default().personality(&Pilot::BALANCED),
        Personality::NEUTRAL
    );
    assert_ne!(Personality::from_seed(1), Personality::NEUTRAL);
}

/// The seed is the whole character, so it must decide it completely.
#[test]
fn a_personality_is_a_pure_function_of_its_seed() {
    for seed in [1u32, 2, 99, 0xdead_beef] {
        assert_eq!(Personality::from_seed(seed), Personality::from_seed(seed));
    }
    assert_ne!(Personality::from_seed(1), Personality::from_seed(2));
}

/// Every axis has to land inside the range its documentation claims, or a
/// craft is handed a lookahead or a grip budget nothing tested.
#[test]
fn every_personality_stays_within_its_stated_range() {
    for seed in 1..2_000u32 {
        let p = Personality::from_seed(seed);
        assert!(
            (0.25..=0.85).contains(&p.line_bias.abs()),
            "seed {seed}: line_bias {}",
            p.line_bias
        );
        assert!((0.10..=0.30).contains(&p.wander), "seed {seed}");
        assert!(
            (1.0 / 420.0..=1.0 / 150.0).contains(&p.wander_rate),
            "seed {seed}"
        );
        assert!((0.85..=1.15).contains(&p.look), "seed {seed}");
        assert!((0.93..=1.05).contains(&p.commitment), "seed {seed}");
        assert!((0.85..=1.20).contains(&p.patience), "seed {seed}");
    }
}

/// Both sides of the line get used: a one-way bias queues the field on one side.
#[test]
fn the_field_leans_both_ways() {
    let left = (1..200u32)
        .filter(|&seed| Personality::from_seed(seed).line_bias < 0.0)
        .count();
    assert!((60..140).contains(&left), "{left} of 199 leant left");
}

/// The point of the whole change: two seeded craft in the same place aim at
/// different points, and a craft with no seed aims at the line.
#[test]
fn two_drivers_aim_at_different_parts_of_the_corridor() {
    let tuning = Tuning::default();
    let line = straight_with_corridor();
    let state = craft(Vec3::ZERO, 60.0);

    let offset = |seed: u32| {
        let mut driver = Driver::seeded(seed);
        driver.drive(&state, &Context::new(&line, &tuning));
        driver
            .drift(
                &line.aim(0, 40.0),
                40.0,
                &Context::new(&line, &tuning),
                &driver.personality(&Pilot::BALANCED),
            )
            .dot(Vec3::NEG_X)
    };

    let one = offset(1);
    let two = offset(2);
    assert!(
        (one - two).abs() > 1.0,
        "two seeded drivers aim {one} and {two} across the line, which is the same place"
    );
    assert_eq!(offset(0), 0.0, "an unseeded driver leaves the line alone");
}

/// And the corridor is a bound, not a suggestion.
#[test]
fn the_drift_stays_inside_the_corridor() {
    let tuning = Tuning::default();
    let line = straight_with_corridor();
    let aim = line.aim(0, 40.0);
    // `width` scales `corridor_use`, so the budget a driver may spend is
    // the shared fraction times the widest a pilot is allowed to be. The
    // corridor's own edge, below, is the one that must never be crossed.
    let room = 8.0 * tuning.corridor_use * Pilot::BALANCED.width.high;

    for seed in 1..500u32 {
        let mut driver = Driver::seeded(seed);
        let personality = driver.personality(&Pilot::BALANCED);
        // A minute of driving, at the ticks the wander is a function of.
        for tick in 0..3_600 {
            driver.phase = tick;
            let across = driver
                .drift(&aim, 40.0, &Context::new(&line, &tuning), &personality)
                .dot(Vec3::X);
            assert!(
                across.abs() <= room + 1e-3,
                "seed {seed} at tick {tick} aimed {across} across a budget of {room}"
            );
            assert!(
                across.abs() <= 8.0,
                "seed {seed} at tick {tick} aimed outside the corridor entirely, at {across}"
            );
        }
    }
}

/// A line with no corridor has no room to spend, and every craft on it
/// drives it exactly - which is what keeps the synthetic tests meaningful.
#[test]
fn a_line_without_a_corridor_is_driven_exactly() {
    let tuning = Tuning::default();
    let state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);

    let mut plain = Driver::default();
    let mut seeded = Driver::seeded(7);
    let a = plain.drive(&state, &Context::new(&straight(), &tuning));
    let b = seeded.drive(&state, &Context::new(&straight(), &tuning));
    assert_eq!(a.steer_x, b.steer_x);
}

/// The drift has to move, or the field is eight fixed lines rather than
/// eight drivers.
#[test]
fn a_driver_drifts_over_time() {
    let tuning = Tuning::default();
    let line = straight_with_corridor();
    let aim = line.aim(0, 40.0);

    let mut driver = Driver::seeded(3);
    let personality = driver.personality(&Pilot::BALANCED);
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for tick in 0..3_600 {
        driver.phase = tick;
        let across = driver
            .drift(&aim, 40.0, &Context::new(&line, &tuning), &personality)
            .dot(Vec3::X);
        low = low.min(across);
        high = high.max(across);
    }
    assert!(high - low > 0.5, "drifted over a range of {}", high - low);
}

/// Commitment is the axis that strings the field out, so it has to reach
/// the speed target.
#[test]
fn a_committed_driver_carries_more_speed_through_a_corner() {
    let tuning = Tuning::default();
    let timid = Personality {
        commitment: 0.9,
        ..Personality::NEUTRAL
    };
    let brave = Personality {
        commitment: 1.1,
        ..Personality::NEUTRAL
    };
    // A speed between the two targets: one lifts, the other does not.
    let speed = corner_target(0.01, &tuning, &Personality::NEUTRAL, None);
    let (brave_thrust, brave_brake) =
        throttle(speed, corner_target(0.01, &tuning, &brave, None), &tuning);
    let (timid_thrust, timid_brake) =
        throttle(speed, corner_target(0.01, &tuning, &timid, None), &tuning);
    assert_eq!((brave_thrust, brave_brake), (1.0, 0.0));
    assert_eq!(timid_thrust, 0.0, "the timid driver is over its own target");
    assert!(timid_brake > 0.0, "and far enough over it to brake");
}

/// The wander argument is the driver's own tick count, so it has to advance
/// when the driver drives and only then.
#[test]
fn the_phase_counts_the_ticks_this_driver_drove() {
    let mut driver = Driver::seeded(4);
    let state = craft(Vec3::ZERO, 40.0);
    for expected in 1..=5 {
        driver.drive(&state, &Context::new(&straight(), &Tuning::default()));
        assert_eq!(driver.phase, expected);
    }
    // No line, no drive, no tick.
    driver.drive(&state, &Context::new(&Line::default(), &Tuning::default()));
    assert_eq!(driver.phase, 5);
}

/// The index is the search seed, so it has to survive the call.
#[test]
fn the_driver_remembers_where_it_was() {
    let mut driver = Driver::default();
    let line = straight();
    driver.drive(
        &craft(Vec3::new(0.0, 0.0, -200.0), 40.0),
        &Context::new(&line, &Tuning::default()),
    );
    assert_eq!(driver.index, 20);
}

#[test]
fn a_personality_spent_on_a_plan_keeps_its_temper_and_loses_its_line() {
    // Seeded, so every driving axis is off neutral to begin with.
    let drawn = Personality::from_pilot_seed(&Pilot::BALANCED, 0x5eed);
    let none = drawn.spent(0.0);
    assert_eq!(none.line_bias, 0.0);
    assert_eq!(none.wander, 0.0);
    assert_eq!(none.inside, 0.0);
    assert_eq!(
        (none.look, none.patience, none.trail, none.width),
        (1.0, 1.0, 1.0, 1.0)
    );
    // What is about other craft, the roll and the margin is untouched.
    assert_eq!(none.commitment, drawn.commitment);
    assert_eq!(none.defence, drawn.defence);
    assert_eq!(none.courtesy, drawn.courtesy);
    assert_eq!(none.trigger, drawn.trigger);
    assert_eq!(none.roll_chance, drawn.roll_chance);
    // All of it spent is the drawn line again.
    let all = drawn.spent(1.0);
    assert!((all.line_bias - drawn.line_bias).abs() < 1e-6);
    assert!((all.look - drawn.look).abs() < 1e-6);
}
