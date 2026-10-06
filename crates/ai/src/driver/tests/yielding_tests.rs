//! Yielding and blocking: what a driver does about the craft behind it.
//!
//! One theme of `driver.rs`'s tests, split by subject; fixtures stay in [`super`].

use super::*;

/// Shy yields: it moves to the *other* side from the craft behind it.
#[test]
fn a_shy_driver_moves_away_from_the_craft_behind_it() {
    let line = straight_with_corridor();
    let shy = Personality {
        courtesy: 0.8,
        ..Personality::NEUTRAL
    };
    // A rival to the right, closing hard and close.
    let alone = across(&shy, &line, &Field::EMPTY);
    let pressed = across(&shy, &line, &rival_behind(4.0, 20.0, 20.0));
    assert!(
        pressed < alone - 0.5,
        "a shy driver should move left of a rival on its right: {pressed} against {alone}"
    );
}

/// Aggressive covers: it moves *onto* the side the craft behind is on.
#[test]
fn an_aggressive_driver_covers_the_line_of_the_craft_behind_it() {
    let line = straight_with_corridor();
    let mean = Personality {
        defence: 0.8,
        ..Personality::NEUTRAL
    };
    let alone = across(&mean, &line, &Field::EMPTY);
    let pressed = across(&mean, &line, &rival_behind(4.0, 20.0, 20.0));
    assert!(
        pressed > alone + 0.5,
        "a defending driver should cover a rival on its right: {pressed} against {alone}"
    );
}

/// The two are one axis, so a pilot with equal measures of both does
/// nothing rather than jittering between them.
#[test]
fn equal_courtesy_and_defence_cancel_instead_of_fighting() {
    let line = straight_with_corridor();
    let torn = Personality {
        courtesy: 0.7,
        defence: 0.7,
        ..Personality::NEUTRAL
    };
    assert_eq!(
        across(&torn, &line, &rival_behind(4.0, 20.0, 20.0)),
        across(&torn, &line, &Field::EMPTY)
    );
}

#[test]
fn a_driver_ignores_a_rival_beyond_its_awareness_range() {
    let line = straight_with_corridor();
    let shy = Personality {
        courtesy: 0.8,
        ..Personality::NEUTRAL
    };
    let far = rival_behind(4.0, AWARENESS_RANGE + 1.0, 20.0);
    assert_eq!(
        across(&shy, &line, &far),
        across(&shy, &line, &Field::EMPTY)
    );
}

/// A rival sitting on the tail at matched pace still earns a lean - that is
/// when yielding matters most - but less than one actually catching up.
#[test]
fn a_rival_holding_station_earns_less_of_a_lean_than_one_closing() {
    let line = straight_with_corridor();
    let shy = Personality {
        courtesy: 0.8,
        ..Personality::NEUTRAL
    };
    let alone = across(&shy, &line, &Field::EMPTY);
    let holding = across(&shy, &line, &rival_behind(4.0, 20.0, 0.0));
    let catching = across(&shy, &line, &rival_behind(4.0, 20.0, 20.0));

    assert!(
        (holding - alone).abs() > 0.1,
        "a rival on the tail should still be yielded to: {holding} against {alone}"
    );
    assert!(
        (catching - alone).abs() > (holding - alone).abs(),
        "and one catching up should earn more: {catching} against {holding}"
    );
}

/// Blocking where the corridor is narrow and both craft are at the grip
/// limit is how two craft end up in the scenery.
#[test]
fn neither_yielding_nor_blocking_happens_mid_corner() {
    let mean = Personality {
        defence: 0.9,
        ..Personality::NEUTRAL
    };
    let pressing = rival_behind(4.0, 20.0, 20.0);
    let lean = |line: &Line| across(&mean, line, &pressing) - across(&mean, line, &Field::EMPTY);

    // The gate is proportional to the corner, so take both ends of it.
    let on_a_straight = lean(&straight_with_corridor());
    assert!(
        on_a_straight > 0.5,
        "a defending driver should cover on a straight, got {on_a_straight}"
    );

    let in_a_corner = lean(&right_hand_corner(35.0));
    assert!(
        in_a_corner.abs() < 0.05,
        "a corner worth the name should hold it off, got {in_a_corner}"
    );

    // And a gentle bend in between, so this is a gate and not a switch.
    let on_a_bend = lean(&right_hand_corner(120.0));
    assert!(
        on_a_bend.abs() < on_a_straight && on_a_bend.abs() > in_a_corner.abs(),
        "the gate should scale: straight {on_a_straight}, bend {on_a_bend}, corner {in_a_corner}"
    );
}

/// A rival dead astern has an offset whose sign is rounding noise, so the
/// side has to come from somewhere else.
#[test]
fn a_rival_squarely_behind_is_yielded_toward_the_outside_of_the_corner() {
    // A gentle bend, so the corner gate is open but `bend` still has a sign.
    let line = straight_with_corridor();
    let shy = Personality {
        courtesy: 0.8,
        ..Personality::NEUTRAL
    };
    // Dead astern on a straight: no side to pick from either source, so no
    // lean at all rather than a coin flip.
    let astern = rival_behind(0.0, 20.0, 20.0);
    assert_eq!(
        across(&shy, &line, &astern),
        across(&shy, &line, &Field::EMPTY)
    );
}

#[test]
fn a_driver_lifts_for_a_craft_it_is_closing_on() {
    let tuning = Tuning::default();
    let line = straight_with_corridor();
    let wary = Personality {
        caution: 0.8,
        ..Personality::NEUTRAL
    };
    let closing = Field {
        ahead: Some(Rival {
            slot: 1,
            gap: 10.0,
            offset: 0.0,
            closing: 20.0,
            range: 10.0,
            cos_bearing: 1.0,
        }),
        ..Field::EMPTY
    };
    let context = |field| Context {
        line: &line,
        tuning: &tuning,
        pilot: &Pilot::BALANCED,
        field,
        yaw_ceiling: None,
        plan: None,
    };
    let driver = Driver::default();
    assert_eq!(driver.caution(&context(&Field::EMPTY), &wary), 1.0);
    let lifted = driver.caution(&context(&closing), &wary);
    assert!(lifted < 0.5, "a wary driver should lift, got {lifted}");
    // And a driver with no caution at all keeps its foot in.
    assert_eq!(
        driver.caution(&context(&closing), &Personality::NEUTRAL),
        1.0
    );
}

/// The whole of stage 3 has to be invisible to a driver that cannot see
/// anybody, or every test written before it starts measuring something else.
#[test]
fn a_driver_that_sees_nobody_drives_exactly_the_line_it_did_before() {
    let tuning = Tuning::default();
    let line = straight_with_corridor();
    let state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);
    for (name, pilot) in Pilot::BUILT_IN {
        let mut with_sight = Driver::seeded(5);
        let mut blind = Driver::seeded(5);
        let seen = with_sight.drive(
            &state,
            &Context {
                line: &line,
                tuning: &tuning,
                pilot: &pilot,
                field: &Field::EMPTY,
                yaw_ceiling: None,
                plan: None,
            },
        );
        let unseen = blind.drive(
            &state,
            &Context {
                line: &line,
                tuning: &tuning,
                pilot: &pilot,
                field: &Field::default(),
                yaw_ceiling: None,
                plan: None,
            },
        );
        assert_eq!(seen, unseen, "{name}");
    }
}
