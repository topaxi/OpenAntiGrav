//! The LeachBeam through the composition root: the unlocked arm, the
//! whole-race busy check, and the victim's throttle.
//!
//! Split out of `weapons.rs` under the 1,000-line ceiling
//! (`scripts/check-file-size.py`); the fixtures are `super`'s.

use super::*;
use oag_gameplay::PlayerInputs;

/// A fired LeachBeam holds, drains its victim into its shooter, and lets go on
/// its own authored `active_time` - the whole weapon through the composition
/// root rather than through `oag_gameplay` alone.
///
/// **Fired with no lock on purpose.** The grid fixture puts no craft inside the
/// window, so this is the unlocked path: the pickup is spent, a beam exists,
/// and it expires without touching anybody. That is
/// `Weapon_FireLeachBeam`'s own behaviour - it claims the pool slot and clears
/// the fire bit before it ever branches on the lock - and it is the arm most
/// likely to be quietly wrong, because "fires and does nothing" and "did not
/// fire at all" look identical from outside unless the beam is checked for.
#[test]
fn a_leach_beam_fired_at_nobody_is_spent_and_expires_on_its_own_clock() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_leach_beam_table(),
    );
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);

    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));

    assert_eq!(race.ship_pickup(), None, "the beam must spend the pickup");
    let beam = race.sim.world.leach_beam.expect("a fired beam must exist");
    assert_eq!(beam.owner, 0);
    assert_eq!(
        beam.kind,
        oag_gameplay::projectile::leach_beam::Kind::Unlocked,
        "nothing is in the window, so this must be the unlocked arm"
    );

    // Past the 0.75-second fizzle the unlocked arm runs on - which is an engine
    // literal rather than this fixture's `active_time`, so the loop is sized on
    // the constant itself rather than on the table.
    let ticks = (oag_gameplay::projectile::leach_beam::UNLOCKED_FIZZLE_SECONDS / race.dt()).ceil()
        as usize
        + 2;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(buttons.tick(0)));
    }
    assert!(
        race.sim.world.leach_beam.is_none(),
        "an unlocked beam must retire rather than hold the whole race's only slot"
    );
}

/// The busy check is whole-race: a second press while a beam is up keeps the
/// pickup rather than spending it on nothing.
///
/// This is the gate that differs from every other weapon here - the original's
/// `pool->live` is a world cursor, not a per-craft cooldown - so it gets a test
/// of its own rather than riding on the fire test above.
#[test]
fn a_second_leach_beam_press_while_one_is_up_keeps_the_pickup() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_leach_beam_table(),
    );
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);

    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
    assert!(race.sim.world.leach_beam.is_some());

    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
    race.tick(&PlayerInputs::single(buttons.tick(0)));
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));

    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::LeachBeam),
        "a press into somebody else's live beam must keep the pickup"
    );
}

/// A beam's victim runs on `slowShipFactor` of its thrust, through the whole
/// tick: the drain arms `Ship::pending_thrust_scale`, and the next step's
/// `Environment::thrust_scale` spends it. Two identical grids, one with a beam
/// locked onto slot 1, and the beamed craft is the slower of the two.
#[test]
fn a_leach_beams_victim_is_throttled_by_the_authored_factor() {
    let table = one_leach_beam_table();
    // The fixture's `damage="42"` empties a 100-point pool in three ticks and
    // its half-second `active_time` is shorter than the run below; the beam is
    // built directly, so only the factor is taken from the table.
    let stats = oag_tables::weapons::LeachBeamStats {
        damage: 0.1,
        active_time: 3.0,
        ..table.leach_beam().expect("the fixture authors a LeachBeam")
    };
    let build = || {
        let mut race = race_with_a_grid();
        race.sim.weapons = Some(table.clone());
        // The grid fixture's craft have no engine; give the victim one so
        // there is a thrust to take a fifth of.
        let engine = &mut race.sim.world.ships[1].handling.engine;
        engine.amount = 20.0;
        engine.accelcap = 1000.0;
        while RaceState::thrust_gated(race.sim.world.tick) {
            race.tick(&PlayerInputs::none());
        }
        race
    };
    let mut control = build();
    let mut beamed = build();
    beamed.sim.world.leach_beam = Some(oag_gameplay::projectile::leach_beam::Beam::locked(
        0, 1, &stats,
    ));

    beamed.tick(&PlayerInputs::none());
    assert_eq!(
        beamed.sim.world.ships[1].pending_thrust_scale, 0.44,
        "the first draining tick must arm the victim's one-shot scale"
    );
    assert_eq!(beamed.sim.world.ships[0].pending_thrust_scale, 1.0);

    control.tick(&PlayerInputs::none());
    for _ in 0..60 {
        control.tick(&PlayerInputs::none());
        beamed.tick(&PlayerInputs::none());
    }
    assert!(
        beamed
            .sim
            .world
            .leach_beam
            .is_some_and(|beam| beam.connected()),
        "the link broke before the throttle could be measured"
    );
    let speed = |race: &Race| {
        let body = &race.sim.world.ships[1].physics.body;
        body.linear_velocity.dot(body.forward())
    };
    assert!(
        speed(&control) > 1.0,
        "the control craft never got going ({})",
        speed(&control)
    );
    assert!(
        speed(&beamed) < speed(&control) * 0.9,
        "the beamed craft ran at {} against {} unbeamed",
        speed(&beamed),
        speed(&control)
    );
}
