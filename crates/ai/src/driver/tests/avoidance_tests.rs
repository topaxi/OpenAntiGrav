//! Steering around a laid charge: which way, how hard, and when not at all.
//!
//! One theme of `driver.rs`'s tests - see [`super`] for the fixtures.
//!
//! These drive [`Driver::avoidance`] directly rather than through
//! [`Driver::drive`]. That is deliberate and it is the opposite of what
//! `reaction_tests` does: the *lean* is one term among five that `drift` sums
//! and clamps, so a test that watched the steering command would be measuring
//! the sum, and a term that had stopped working would still leave a craft on a
//! plausible line. The one place the sum is worth watching is
//! [`the_lean_reaches_the_steering_command`], which is here so the term cannot
//! be correct and unwired at the same time.

use super::*;
use crate::Hazard;

/// A charge `distance` ahead and `offset` to the right.
fn charge(distance: f32, offset: f32) -> Option<Hazard> {
    Some(Hazard { distance, offset })
}

/// **Lean away from the side it is on.** The sign is the whole term.
#[test]
fn a_driver_leans_away_from_a_charge() {
    let driver = Driver::seeded(11);
    let right = driver.avoidance(charge(30.0, 3.0));
    let left = driver.avoidance(charge(30.0, -3.0));
    assert!(
        right < 0.0,
        "a charge on the right should push the craft left, got {right}"
    );
    assert!(
        left > 0.0,
        "a charge on the left should push the craft right, got {left}"
    );
    assert!(
        (right + left).abs() < f32::EPSILON,
        "the two sides are not mirror images: {right} and {left}"
    );
}

/// **Harder the nearer it is**, and nothing at all at the edge of the lookahead.
///
/// The ramp is what makes an instant *noticing* look like a gradual reaction -
/// see `Reflex::filter`, which passes a hazard through with no latency and says
/// why.
#[test]
fn the_lean_grows_as_the_charge_nears() {
    let driver = Driver::seeded(11);
    let far = driver
        .avoidance(charge(AVOIDANCE_LOOKAHEAD * 0.9, 3.0))
        .abs();
    let near = driver
        .avoidance(charge(AVOIDANCE_LOOKAHEAD * 0.1, 3.0))
        .abs();
    assert!(
        near > far,
        "a charge at a tenth of the lookahead ({near}) should pull harder than one \
         at nine tenths ({far})"
    );
    assert!(far > 0.0, "a charge inside the lookahead is ignored");
    assert_eq!(
        driver.avoidance(charge(AVOIDANCE_LOOKAHEAD, 3.0)),
        0.0,
        "a charge at exactly the lookahead is still being reacted to"
    );
}

/// **Nothing behind, and nothing when there is nothing.**
///
/// The caller already drops anything level or astern, so this is the belt to
/// that braces: a negative distance is meaningless here and must not produce a
/// lean of some arbitrary sign.
#[test]
fn a_driver_ignores_a_charge_it_has_passed() {
    let driver = Driver::seeded(11);
    assert_eq!(driver.avoidance(None), 0.0);
    assert_eq!(driver.avoidance(charge(-10.0, 3.0)), 0.0);
    assert_eq!(driver.avoidance(charge(0.0, 3.0)), 0.0);
}

/// **A charge dead ahead splits the field rather than sending it one way.**
///
/// Its offset's sign is noise at that range, so the side comes from the driver's
/// own seed. What matters is not which way any one craft goes but that they do
/// not all go the same way - a field diving as one is both a worse look and a
/// worse outcome, since they would then all still be in line astern.
#[test]
fn craft_meeting_a_charge_head_on_do_not_all_dive_the_same_way() {
    let sides: Vec<f32> = (0..8)
        .map(|slot| Driver::seeded(slot).avoidance(charge(30.0, 0.0)).signum())
        .collect();
    assert!(
        sides.iter().any(|&s| s > 0.0) && sides.iter().any(|&s| s < 0.0),
        "every driver dodged the same way: {sides:?}"
    );
    // And each one is consistent with itself, tick after tick - the side is a
    // property of the craft, not a coin flipped every frame.
    let driver = Driver::seeded(3);
    let once = driver.avoidance(charge(30.0, 0.0));
    assert_eq!(driver.avoidance(charge(30.0, 0.0)), once);
}

/// **The lean reaches the steering command.**
///
/// The term above could be perfectly correct and summed into nothing, so this
/// is the one test that goes through [`Driver::drive`]: two identical drivers on
/// one line, one of them with a charge to its right, and the one that can see it
/// must steer differently. Not *how* differently - that is the sum's business
/// and the corridor's - only that the term is wired.
#[test]
fn the_lean_reaches_the_steering_command() {
    let line = straight_with_corridor();
    let tuning = Tuning::default();
    let pilot = Pilot::BALANCED;
    let state = craft(Vec3::ZERO, 100.0);

    let clear = Context {
        line: &line,
        tuning: &tuning,
        pilot: &pilot,
        field: &Field::EMPTY,
        yaw_ceiling: None,
        plan: None,
    };
    let mined = Context {
        field: &Field {
            hazard: charge(20.0, 3.0),
            ..Field::EMPTY
        },
        ..clear
    };

    let mut a = Driver::seeded(11);
    let mut b = Driver::seeded(11);
    let without = a.drive(&state, &clear);
    let with = b.drive(&state, &mined);
    println!(
        "steering {:.4} with nothing on the road, {:.4} with a charge 20 units up \
         and 3 to the right",
        without.steer_x, with.steer_x
    );
    assert!(
        (without.steer_x - with.steer_x).abs() > 1e-4,
        "the avoidance term is computed and never reaches the steering command"
    );
}
