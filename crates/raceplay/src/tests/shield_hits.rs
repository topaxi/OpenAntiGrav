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

/// Slot 0 fires a locked LeachBeam at slot 1, 60 units down its nose, `shielded` slots running a
/// Shield with every shell up and settled. The beam is returned after the tick it was fired on.
fn leach_duel(shielded: [bool; 2]) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_leach_beam_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[1].handling.dimensions.shield = 100.0;
    race.sim.world.ships[1].physics.shield = 100.0;
    for (slot, shielded) in shielded.into_iter().enumerate() {
        race.sim.world.ships[slot].physics.shield_pickup_timer = if shielded { 20.0 } else { 0.0 };
        race.view.shield[slot].activate();
        for _ in 0..60 {
            race.view.shield[slot].advance(1.0 / 60.0);
        }
    }
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[1].physics.body.position =
        race.sim.world.ships[0].physics.body.position + forward * 60.0;
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);
    race
}

/// The highest swell slot 1's shell reaches over the first `ticks` ticks of `race`, a LeachBeam
/// pressed on the second.
fn peak_swell_under_a_leach_beam(race: &mut Race, ticks: usize) -> f32 {
    let mut buttons = Buttons::new();
    let mut peak = 0.0_f32;
    for tick in 0..ticks {
        let snapshot = buttons.tick(if tick == 1 { SQUARE } else { 0 });
        race.tick(&PlayerInputs::single(snapshot));
        peak = peak.max(race.shield_of(1).swell());
    }
    peak
}

/// `LeachBeam_UpdatePool` marks a link on a Shield-holding target disconnected and still calls
/// `LeachBeam_Drain` on that tick and through the linger, so the target's pending damage meets
/// `Ship_ApplyPendingWeaponDamage`'s shield arm, `ShipShield_Hit`. Goes red if the beam stops
/// reporting `absorbed` on a broken link.
#[test]
fn a_leach_beam_on_a_shielded_target_bulges_the_shell_through_its_linger() {
    let mut race = leach_duel([false, true]);
    let peak = peak_swell_under_a_leach_beam(&mut race, 20);

    assert!(
        race.sim.world.leach_beams[0].is_some(),
        "the beam never fired, so the linger was never reached"
    );
    assert!(
        peak > SWELL_REST + 0.05,
        "a LeachBeam link on a shielded target left the shell at rest: {peak}"
    );
    assert_eq!(
        race.sim.world.ships[1].physics.shield, 100.0,
        "the broken link drained a running Shield"
    );
}

/// `LeachBeam_Drain` returns on the owner's own Shield, so nothing is posted to the target.
#[test]
fn a_leach_beam_from_a_shielded_owner_leaves_the_targets_shell_at_rest() {
    let mut race = leach_duel([true, true]);
    let peak = peak_swell_under_a_leach_beam(&mut race, 20);

    assert!(
        race.sim.world.leach_beams[0].is_some(),
        "the beam never fired, so the guard asserted nothing"
    );
    assert!(
        (peak - SWELL_REST).abs() < 0.01,
        "the shooter's Shield did not gate the drain: {peak}"
    );
}

/// [`bump`] with both pools full at 100 and the race's weapons switch at `weapons`.
fn charged_bump(shielded: [bool; 2], weapons: bool) -> Race {
    let mut race = bump(shielded);
    race.sim.weapons_on = weapons;
    for slot in 0..2 {
        let ship = &mut race.sim.world.ships[slot];
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 100.0;
    }
    race
}

/// The pool each slot holds after one pair pass, and the contact's impulse.
fn pools_after_a_bump(race: &mut Race) -> ([f32; 2], f32) {
    race.resolve_craft_pairs();
    assert_eq!(race.pair_contacts, [(0, 1)], "the craft never touched");
    let impulse = race.pair_impulses[0];
    assert!(impulse > 0.0, "a touch with no impulse charges nothing");
    let pools = [0, 1].map(|slot| race.sim.world.ships[slot].physics.shield);
    (pools, impulse)
}

/// `Ship_Damage(0.035 * |p|)` on both hulls of an unshielded pair
/// (`contact-response.md`, "Measured 2026-11-11: an unshielded craft-to-craft bump charges
/// the energy pool"). Goes red if the pair pass stops charging, charges one hull, or charges
/// only in the drawing build.
#[test]
fn an_unshielded_bump_charges_both_pools_the_wall_law_on_the_pair_impulse() {
    let mut race = charged_bump([false, false], true);
    let (pools, impulse) = pools_after_a_bump(&mut race);
    let lost = impulse * oag_physics::damage::CONTACT_DAMAGE_SCALE;
    for (slot, pool) in pools.into_iter().enumerate() {
        assert_eq!(pool, 100.0 - lost, "slot {slot}, impulse {impulse}");
    }
}

#[test]
fn an_unshielded_bump_with_weapons_off_charges_half() {
    let mut race = charged_bump([false, false], false);
    let (pools, impulse) = pools_after_a_bump(&mut race);
    let lost = impulse
        * oag_physics::damage::CONTACT_DAMAGE_SCALE
        * oag_physics::damage::NO_WEAPONS_DAMAGE_SCALE;
    for (slot, pool) in pools.into_iter().enumerate() {
        assert_eq!(pool, 100.0 - lost, "slot {slot}, impulse {impulse}");
    }
}

/// A shielded hull takes the shell bulge instead of the charge; its unshielded partner
/// still pays.
#[test]
fn a_shielded_hull_pays_nothing_for_a_bump_and_its_partner_still_does() {
    let mut race = charged_bump([true, false], true);
    let (pools, _) = pools_after_a_bump(&mut race);
    assert_eq!(pools[0], 100.0, "a running Shield let the bump through");
    assert!(pools[1] < 100.0, "the unshielded partner was not charged");

    let mut race = charged_bump([true, true], true);
    let (pools, _) = pools_after_a_bump(&mut race);
    assert_eq!(pools, [100.0, 100.0], "two Shields let the bump through");
}

/// A hull still exploding is solid to other craft but outside `Ship_Damage`'s racing gate.
#[test]
fn a_bump_leaves_the_pool_of_a_hull_that_is_not_racing_alone() {
    let mut race = charged_bump([false, false], true);
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Destroyed;
    race.sim.world.ships[0].physics.state_timer = oag_physics::damage::DESTROYED_DURATION;
    let (pools, _) = pools_after_a_bump(&mut race);
    assert_eq!(pools[0], 100.0, "a destroyed hull was charged");
    assert!(pools[1] < 100.0, "the racing partner was not charged");
}
