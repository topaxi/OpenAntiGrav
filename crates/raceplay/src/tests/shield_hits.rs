//! The shell of a running Shield reacts to the hits the original's `ShipShield_Hit`
//! (`0x0885eb04`) answers: a craft-to-craft contact and an absorbed weapon hit. The
//! wall scrape's twin is `respawn::a_shielded_scrape_bulges_the_shell_and_throws_no_sparks`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_render::shield::{SWELL_ON_HIT, SWELL_REST};

/// Two craft a unit apart nose to nose and closing, a Shield running on `shielded` slots
/// and every shell up and settled at rest.
fn bump(shielded: [bool; 2]) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    for (slot, shielded) in shielded.into_iter().enumerate() {
        let ship = &mut race.sim.world.ships[slot].physics;
        ship.shield_pickup_timer = if shielded { 20.0 } else { 0.0 };
        race.view.shield[slot].activate();
        for _ in 0..60 {
            race.view.shield[slot].advance(1.0 / 60.0);
        }
    }
    let at = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = at + Vec3::new(0.0, 0.0, 1.0);
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(0.0, 0.0, -8.0);
    race
}

#[test]
fn a_craft_bump_bulges_the_shell_of_the_shielded_craft_alone() {
    let mut race = bump([true, false]);
    assert!(
        (race.shield_of(0).swell() - SWELL_REST).abs() < 0.01,
        "the fixture's shell did not settle"
    );

    race.resolve_craft_pairs();

    assert_eq!(race.pair_contacts, [(0, 1)], "the craft never touched");
    assert_eq!(
        race.shield_of(0).swell(),
        SWELL_ON_HIT,
        "a shielded craft's shell did not react to the bump"
    );
    assert!(
        (race.shield_of(1).swell() - SWELL_REST).abs() < 0.01,
        "an unshielded craft's shell reacted"
    );
}

#[test]
fn a_craft_bump_bulges_both_shells_when_both_craft_run_a_shield() {
    let mut race = bump([true, true]);

    race.resolve_craft_pairs();

    for slot in 0..2 {
        assert_eq!(race.shield_of(slot).swell(), SWELL_ON_HIT, "slot {slot}");
    }
}

#[test]
fn a_bump_on_a_craft_that_is_not_racing_leaves_the_shell_alone() {
    let mut race = bump([true, false]);
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Destroyed;
    race.sim.world.ships[0].physics.state_timer = 0.0;

    race.resolve_craft_pairs();

    assert!(
        (race.shield_of(0).swell() - SWELL_REST).abs() < 0.01,
        "the loop's racing-state gate was skipped"
    );
}

const ROCKET_DAMAGE: f32 = 14.0;

/// Slot 0 fires Rockets at slot 1, 15 units down its nose, under a running Shield with the
/// shell up and settled.
fn duel() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(
        oag_tables::weapons::parse(&format!(
            r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="11" blastforce="12" blastradius="13"
                   damage="{ROCKET_DAMAGE}" slowdown_time="15" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="16" spread="0.2"/></Weapon>
               </WeaponStats>"#,
        ))
        .expect("the fixture table must parse"),
    );
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[1].handling.dimensions.shield = 100.0;
    race.sim.world.ships[1].physics.shield = 100.0;
    race.sim.world.ships[1].physics.shield_pickup_timer = 20.0;
    race.view.shield[1].activate();
    race
}

/// One tick of the duel with both craft pinned; slot 0 fires a Rocket when `fire` is set.
/// Returns whether a Rocket reached a craft this tick.
fn step(race: &mut Race, buttons: &mut Buttons, fire: bool) -> bool {
    race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[0].physics.body.linear_velocity = forward * 60.0;
    race.sim.world.ships[1].physics.body.position = forward * 15.0;
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
    if fire {
        race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    }
    let snapshot = buttons.tick(if fire { SQUARE } else { 0 });
    race.tick(&PlayerInputs::single(snapshot));
    race.drain_cues()
        .into_iter()
        .any(|e| e.cue == oag_gameplay::cue::Cue::RocketHitShip)
}

#[test]
fn a_rocket_a_shield_swallows_bulges_the_shell() {
    let mut race = duel();
    let mut buttons = Buttons::new();
    for _ in 0..60 {
        step(&mut race, &mut buttons, false);
    }
    assert!(
        (race.shield_of(1).swell() - SWELL_REST).abs() < 0.01,
        "the fixture's shell did not settle"
    );

    let mut fire = true;
    let mut struck = false;
    for _ in 0..120 {
        struck = step(&mut race, &mut buttons, fire);
        fire = false;
        if struck {
            break;
        }
    }

    assert!(struck, "the Rocket never reached the craft");
    assert_eq!(
        race.sim.world.ships[1].physics.shield, 100.0,
        "a running Shield took the Rocket's damage"
    );
    assert!(
        race.shield_of(1).swell() > SWELL_REST + 0.05,
        "the swallowed Rocket left the shell at rest: {}",
        race.shield_of(1).swell()
    );
}
