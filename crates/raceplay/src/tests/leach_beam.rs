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
        oag_weapons::projectile::leach_beam::Kind::Unlocked,
        "nothing is in the window, so this must be the unlocked arm"
    );

    // Past the 0.75-second fizzle the unlocked arm runs on - which is an engine
    // literal rather than this fixture's `active_time`, so the loop is sized on
    // the constant itself rather than on the table.
    let ticks = (oag_weapons::projectile::leach_beam::UNLOCKED_FIZZLE_SECONDS / race.dt()).ceil()
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
    beamed.sim.world.leach_beam = Some(oag_weapons::projectile::leach_beam::Beam::locked(
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

/// `FUN_0883f540` lights `WO_LEACHBEAM_CHARGING` on **any** craft holding id
/// `10`, not only the player's: an opponent carrying the pickup charges, and
/// stops the moment it lets go. A hand-laid one-emitter stand-in for the effect,
/// so this asserts the gate and its per-slot bookkeeping, not the picture.
#[test]
fn an_opponent_holding_a_leach_beam_charges_and_the_player_who_is_not_does_not() {
    let mut race = race_with_a_grid();
    let blob = super::respawn::one_emitter_pob(
        crate::tests::respawn::trigger_name(Trigger::LeachbeamCharging),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::LeachbeamCharging, effect);

    race.sim.world.ships[1].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
    race.tick(&PlayerInputs::none());
    assert!(
        race.view.leach_charge_effect[1].is_some(),
        "the opponent holding a LeachBeam must charge"
    );
    assert!(
        race.view.leach_charge_effect[0].is_none(),
        "the player holds nothing, so carries no charge"
    );

    race.sim.world.ships[1].pickup.weapon = None;
    race.tick(&PlayerInputs::none());
    assert!(
        race.view.leach_charge_effect[1].is_none(),
        "the charge goes the moment the pickup does"
    );
}

/// A held LeachBeam's reticle runs the LeachBeam's own law when the title's
/// reticle dialect is Pulse's (`Race::set_sight_dialect`), and not Wipeout HD's: the arrowheads turn, and they lock on arrival rather than after the
/// Missile's hold.
///
/// **Through `Race::tick`** because the gate, the projection and the law meet
/// there: the held weapon decides there is a target, the camera projects it, and
/// `FUN_0881e8c8` turns what it sees into a figure. Without the flag the same
/// race keeps the Missile's law, which is what Wipeout HD's rings still run.
#[test]
fn a_held_leach_beam_spins_its_reticle_under_the_pulse_law_only() {
    let run = |pulse: bool| {
        let mut race = race_with_a_grid();
        race.sim.weapons = Some(one_leach_beam_table());
        if pulse {
            race.set_sight_dialect(oag_pulse::hud::ART.sights);
        } else {
            race.set_sight_dialect(oag_hd::hud::ART.sights);
        }
        let forward = race.sim.world.ships[0].physics.body.forward();
        let ahead = race.sim.world.ships[0].physics.body.position + forward * 60.0;
        race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
        let mut locked_at = None;
        for tick in 0..120 {
            race.sim.world.ships[1].physics.body.position = ahead;
            race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
            race.tick(&PlayerInputs::none());
            if locked_at.is_none() && race.sight().locked() {
                locked_at = Some(tick);
            }
        }
        (race.sight().brackets()[2].rotation, locked_at)
    };
    let (rotation, locked_at) = run(true);
    assert_ne!(
        rotation, 0.0,
        "the third arrowhead is a pure quarter-turn only when nothing spins"
    );
    let locked_at = locked_at.expect("the arrowheads never closed");
    assert!(
        locked_at < 40,
        "locked at tick {locked_at}: the Pulse law has no 0.8 s hold"
    );

    let (rotation, locked_at) = run(false);
    assert_eq!(rotation, 0.0, "without the dialect flag nothing spins");
    assert!(
        locked_at.expect("locked") >= 48,
        "the Missile's law holds for 0.8 s"
    );
}

/// Firing closes the reticle's gate on the fire tick - `Weapon_FireLeachBeam`
/// clears the held-weapon slot, and the sight's gate is that slot - so the
/// arrowheads open and are gone in a quarter of a second while the beam is
/// still live, rather than staying up over its target.
#[test]
fn firing_the_leach_beam_takes_the_reticle_down_while_the_beam_is_live() {
    let mut race = race_with_a_grid();
    // The fixture's half-second `active_time` outlasts the 16 ticks checked.
    let table = one_leach_beam_table();
    race.sim.weapons = Some(table);
    race.set_sight_dialect(oag_pulse::hud::ART.sights);
    let forward = race.sim.world.ships[0].physics.body.forward();
    let ahead = race.sim.world.ships[0].physics.body.position + forward * 60.0;
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
    let mut buttons = Buttons::new();
    buttons.tick(0);
    let tick = |race: &mut Race, input: oag_gameplay::InputSnapshot| {
        race.sim.world.ships[1].physics.body.position = ahead;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::single(input));
    };
    for _ in 0..60 {
        tick(&mut race, buttons.tick(0));
    }
    assert!(race.sight().locked(), "never locked before the shot");
    tick(&mut race, buttons.tick(SQUARE));
    assert!(race.sim.world.leach_beam.is_some(), "the shot did not fire");
    assert_eq!(
        race.sight_state(),
        oag_race::sight::State::Absent,
        "the reticle still holds its target on the fire tick"
    );
    for _ in 0..16 {
        tick(&mut race, buttons.tick(0));
    }
    assert!(
        race.sim.world.leach_beam.is_some(),
        "the beam ended before the check"
    );
    assert!(
        !race.sight().visible(),
        "the arrowheads are still up a quarter of a second into a live beam"
    );
}

/// Under the Pulse law the player's LeachBeam locks only once its reticle has:
/// `Ship_FireHeldWeapon` hands the fire request the target only when the sight's
/// flag (`entity+0x860 & 1`) is up, and `(0, -1)`, the unlocked fizzle, before.
/// A press at a target that is in the window but whose arrowheads have not
/// closed fires unlocked; the same press once they have is locked. Wipeout HD's
/// reticle is another function and keeps firing off the window alone.
#[test]
fn the_pulse_leach_beam_locks_only_once_its_reticle_has() {
    let kind_at = |pulse: bool, press: usize| {
        let mut race = race_with_a_grid();
        race.sim.weapons = Some(one_leach_beam_table());
        if pulse {
            race.set_sight_dialect(oag_pulse::hud::ART.sights);
        } else {
            race.set_sight_dialect(oag_hd::hud::ART.sights);
        }
        let forward = race.sim.world.ships[0].physics.body.forward();
        let ahead = race.sim.world.ships[0].physics.body.position + forward * 60.0;
        race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
        let mut buttons = Buttons::new();
        buttons.tick(0);
        for tick in 0..=press {
            race.sim.world.ships[1].physics.body.position = ahead;
            race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
            let input = buttons.tick(if tick == press { SQUARE } else { 0 });
            race.tick(&PlayerInputs::single(input));
        }
        race.sim.world.leach_beam.expect("the press must fire").kind
    };
    use oag_weapons::projectile::leach_beam::Kind;
    // The Pulse arrowheads close in about 0.34 s (20 ticks); 5 ticks is a first
    // sighting, 40 is well after it.
    let early = kind_at(true, 5);
    assert_eq!(early, Kind::Unlocked, "locked before the reticle had");
    let late = kind_at(true, 40);
    assert_eq!(
        late,
        Kind::Locked,
        "the reticle had locked, the beam did not"
    );
    let hd_early = kind_at(false, 5);
    assert_eq!(
        hd_early,
        Kind::Locked,
        "HD keeps firing off the window alone"
    );
}
