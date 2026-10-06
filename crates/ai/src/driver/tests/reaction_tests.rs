//! Reaction latency, through [`Driver::drive`] rather than [`Reflex`] alone.
//!
//! One theme of `driver.rs`'s tests (fixtures in [`super`]). `reflex::tests`
//! pins the clock; these pin that the clock reaches the controls, which a filter
//! nothing consumed would still pass.

use super::*;
use crate::pilot::Span;

/// A pilot that lifts hard for a craft closing ahead and does nothing else
/// unusual. [`Span::fixed`] still draws, so the frozen sequence is untouched.
fn cautious() -> Pilot {
    Pilot {
        caution: Span::fixed(0.9),
        ..Pilot::BALANCED
    }
}

/// A craft closing on this one, twenty units up the road.
fn closing_ahead() -> Field {
    Field {
        ahead: Some(Rival {
            slot: 4,
            gap: 20.0,
            offset: 0.0,
            closing: 8.0,
            range: 20.0,
            cos_bearing: 1.0,
        }),
        ..Field::EMPTY
    }
}

/// A craft on this one's tail, twenty units back: the mirror of
/// [`closing_ahead`], for a rear weapon. `cos_bearing` is `-1.0`: a hundred and
/// eighty degrees off the nose, what the forward cone rejects and a rear weapon
/// must not be gated on.
fn closing_behind() -> Field {
    Field {
        behind: Some(Rival {
            slot: 4,
            gap: 20.0,
            offset: 0.0,
            closing: 8.0,
            range: 20.0,
            cos_bearing: -1.0,
        }),
        ..Field::EMPTY
    }
}

/// Drives `ticks` ticks against a rival that is there the whole time, and
/// returns the thrust each tick.
fn thrust_over(reaction_ticks: u16, ticks: usize) -> Vec<f32> {
    let line = straight_with_corridor();
    let tuning = Tuning {
        reaction_ticks,
        ..Tuning::default()
    };
    let pilot = cautious();
    let field = closing_ahead();
    let mut driver = Driver::seeded(11);
    let state = craft(Vec3::ZERO, 100.0);
    (0..ticks)
        .map(|_| {
            driver
                .drive(
                    &state,
                    &Context {
                        line: &line,
                        tuning: &tuning,
                        pilot: &pilot,
                        field: &field,
                        yaw_ceiling: None,
                        plan: None,
                    },
                )
                .thrust
        })
        .collect()
}

/// The default is the driver that was here before this existed: it lifts on the
/// tick the rival appears.
#[test]
fn a_driver_with_no_latency_lifts_immediately() {
    let thrust = thrust_over(0, 4);
    assert!(
        thrust[0] < 1.0,
        "a cautious driver should lift for a craft closing ahead: {thrust:?}"
    );
}

/// And the same driver given a reaction time holds the throttle open until it
/// has noticed - the axis reaches the controls, not just the [`Reflex`].
#[test]
fn a_driver_with_a_latency_holds_the_throttle_until_it_notices() {
    const LATENCY: u16 = 30;
    let thrust = thrust_over(LATENCY, LATENCY as usize + 2);
    for (tick, &value) in thrust.iter().take(LATENCY as usize).enumerate() {
        assert_eq!(
            value, 1.0,
            "tick {tick} lifted for a rival this driver has not noticed yet: {thrust:?}"
        );
    }
    assert!(
        thrust[LATENCY as usize] < 1.0,
        "the driver should have noticed after {LATENCY} ticks: {thrust:?}"
    );
    assert_eq!(
        thrust[LATENCY as usize],
        thrust_over(0, 1)[0],
        "once noticed, a slow driver reacts exactly like a quick one"
    );
}

/// A driver cannot shoot at a craft it has not seen. `wants_to_fire` reads the
/// clock [`Driver::drive`] advances, so the two agree within a tick.
#[test]
fn a_driver_does_not_fire_at_a_rival_it_has_not_noticed() {
    let line = straight_with_corridor();
    let pilot = Pilot {
        trigger: Span::fixed(1.0),
        ..Pilot::BALANCED
    };
    let field = closing_ahead();
    let tuning = Tuning {
        reaction_ticks: 30,
        ..Tuning::default()
    };
    let mut driver = Driver::seeded(11);
    let state = craft(Vec3::ZERO, 100.0);
    let context = Context {
        line: &line,
        tuning: &tuning,
        pilot: &pilot,
        field: &field,
        yaw_ceiling: None,
        plan: None,
    };
    for tick in 0..30 {
        driver.drive(&state, &context);
        assert_eq!(
            driver.wants_to_fire(&context),
            None,
            "tick {tick} shot at a rival this driver has not noticed yet"
        );
    }
    // Not "it fires now": the trigger is a rate. What is asserted is that the
    // target is available to roll for, which the reflex was withholding.
    driver.drive(&state, &context);
    assert_eq!(driver.reflex.filter(&field).ahead, field.ahead);
}

/// **A rear weapon is never spent on a craft ahead.** The case is overtaking: a
/// driver reeling somebody in has a rival ahead and nobody behind, and a mine
/// laid then goes behind the *overtaker*, a pickup thrown away. Asserted over
/// three hundred ticks because the trigger is a **rate**: one `None` proves
/// nothing when the roll fails most ticks.
#[test]
fn a_driver_does_not_lay_mines_at_a_craft_ahead() {
    let line = straight_with_corridor();
    let pilot = Pilot {
        trigger: Span::fixed(1.0),
        ..Pilot::BALANCED
    };
    let field = closing_ahead();
    let tuning = Tuning::default();
    let mut driver = Driver::seeded(11);
    let state = craft(Vec3::ZERO, 100.0);
    let context = Context {
        line: &line,
        tuning: &tuning,
        pilot: &pilot,
        field: &field,
        yaw_ceiling: None,
        plan: None,
    };
    for tick in 0..300 {
        driver.drive(&state, &context);
        assert_eq!(
            driver.wants_to_drop(&context),
            None,
            "tick {tick}: laid mines while overtaking - the rival is ahead and the \
             cluster goes out of the back"
        );
    }
    // And the same driver, seed and budget *does* want to drop with the rival
    // behind; else the test above passes for a `wants_to_drop` that never
    // returns anything.
    let behind = closing_behind();
    let context = Context {
        field: &behind,
        ..context
    };
    let mut dropped = false;
    for _ in 0..300 {
        driver.drive(&state, &context);
        if driver.wants_to_drop(&context).is_some() {
            dropped = true;
            break;
        }
    }
    assert!(
        dropped,
        "the driver never laid mines in three hundred ticks with a rival on its \
         tail, so the test above proves nothing"
    );
}

/// The forward weapon's mirror: never fired at a craft *behind*. The cone
/// already rejects it (`cos_bearing` `-1.0`); this pins that it holds through
/// `Field::behind` being populated, the state a rear weapon introduced.
#[test]
fn a_driver_does_not_fire_forward_at_a_craft_behind() {
    let line = straight_with_corridor();
    let pilot = Pilot {
        trigger: Span::fixed(1.0),
        ..Pilot::BALANCED
    };
    let field = closing_behind();
    let tuning = Tuning::default();
    let mut driver = Driver::seeded(11);
    let state = craft(Vec3::ZERO, 100.0);
    let context = Context {
        line: &line,
        tuning: &tuning,
        pilot: &pilot,
        field: &field,
        yaw_ceiling: None,
        plan: None,
    };
    for tick in 0..300 {
        driver.drive(&state, &context);
        assert_eq!(
            driver.wants_to_fire(&context),
            None,
            "tick {tick}: fired a forward weapon at a craft behind it"
        );
    }
}
