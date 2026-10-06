//! Provocation: what losing a place does to a driver, and how a provoked
//! one drives.
//! One theme of `driver.rs`'s tests, split by subject; fixtures stay in [`super`].

use super::*;

/// Losing a place stings, and then wears off.
#[test]
fn a_driver_that_loses_a_place_is_provoked_and_calms_down_again() {
    let line = straight_with_corridor();
    let hot = Personality {
        provocation_ticks: 300.0,
        ..Personality::NEUTRAL
    };
    let mut driver = Driver::seeded(3);

    // Settle on a place first, so the detector has something to compare to.
    stewing(&mut driver, &line, &placed(3), &hot);
    assert_eq!(driver.provocation, 0);

    // Passed: third becomes fourth.
    stewing(&mut driver, &line, &placed(4), &hot);
    assert!(driver.provocation > 250, "got {}", driver.provocation);

    // And it cools, one tick at a time.
    let stung = driver.provocation;
    for _ in 0..100 {
        stewing(&mut driver, &line, &placed(4), &hot);
    }
    assert_eq!(driver.provocation, stung - 100);
}

#[test]
fn a_driver_that_gains_a_place_is_not_provoked() {
    let line = straight_with_corridor();
    let hot = Personality {
        provocation_ticks: 300.0,
        ..Personality::NEUTRAL
    };
    let mut driver = Driver::seeded(3);
    stewing(&mut driver, &line, &placed(4), &hot);
    stewing(&mut driver, &line, &placed(3), &hot);
    assert_eq!(driver.provocation, 0);
}

/// The `place == 0` case, which naively reads as an overtake on tick one
/// for every craft on the grid.
#[test]
fn a_driver_that_has_never_been_placed_is_not_provoked_by_its_first_placing() {
    let line = straight_with_corridor();
    let hot = Personality {
        provocation_ticks: 300.0,
        ..Personality::NEUTRAL
    };
    let mut driver = Driver::seeded(3);
    assert_eq!(driver.place, 0);
    // Eighth on the grid, placed for the first time. Not an overtake.
    stewing(&mut driver, &line, &placed(8), &hot);
    assert_eq!(driver.provocation, 0);
    assert_eq!(driver.place, 8);
}

#[test]
fn provocation_never_exceeds_its_ceiling_however_often_a_driver_is_passed() {
    let line = straight_with_corridor();
    let hot = Personality {
        provocation_ticks: 600.0,
        ..Personality::NEUTRAL
    };
    let mut driver = Driver::seeded(3);
    stewing(&mut driver, &line, &placed(1), &hot);
    for place in 2..=8u8 {
        for _ in 0..20 {
            stewing(&mut driver, &line, &placed(place), &hot);
        }
    }
    assert!(driver.provocation <= PROVOCATION_MAX);
}

/// Provocation makes a driver harder to pass, not faster - the axis it must
/// not touch is `commitment`.
#[test]
fn a_provoked_driver_covers_harder_than_a_calm_one() {
    let line = straight_with_corridor();
    let mean = Personality {
        defence: 0.5,
        ..Personality::NEUTRAL
    };
    let pressing = rival_behind(4.0, 20.0, 20.0);

    let calm = Driver::seeded(11);
    let mut cross = Driver::seeded(11);
    cross.provocation = PROVOCATION_MAX;

    let tuning = Tuning::default();
    let aim = line.aim(0, 40.0);
    let lateral = aim.corridor.expect("fixture has a corridor").lateral;
    let lean = |driver: &Driver, field: &Field| {
        driver
            .drift(
                &aim,
                40.0,
                &Context {
                    line: &line,
                    tuning: &tuning,
                    pilot: &Pilot::BALANCED,
                    field,
                    yaw_ceiling: None,
                    plan: None,
                },
                &mean,
            )
            .dot(lateral)
    };
    let calm_lean = lean(&calm, &pressing) - lean(&calm, &Field::EMPTY);
    let cross_lean = lean(&cross, &pressing) - lean(&cross, &Field::EMPTY);
    assert!(
        cross_lean.abs() > calm_lean.abs() * 1.5,
        "a provoked driver should cover harder: {cross_lean} against {calm_lean}"
    );
}
