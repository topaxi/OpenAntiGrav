//! What the Plasma's wind-up in [`super`] is asserted to do.
//!
//! Its own file rather than an inline `#[cfg(test)] mod`, because
//! `projectile/tests.rs` - where a flight test would otherwise go - was 999
//! lines when this landed, one under `scripts/check-file-size.py`'s cap.

use super::*;
use crate::projectile::{MAX_FLIGHT_SECONDS, Projectiles};
use oag_core::math::Quat;
use oag_physics::CollisionWorld;
use oag_tables::weapons::Weapon;

fn craft_at(position: Vec3, shield: f32, mass: f32) -> crate::world::Ship {
    let mut ship = crate::world::Ship {
        active: true,
        ..crate::world::Ship::default()
    };
    ship.handling.dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        shield,
        ..Dimensions::default()
    };
    ship.physics.shield = shield;
    ship.physics.body.mass = mass;
    ship.physics.body.position = position;
    ship
}

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

fn one_ship(position: Vec3, yaw: f32) -> Vec<crate::world::Ship> {
    let mut ship = crate::world::Ship {
        active: true,
        ..crate::world::Ship::default()
    };
    ship.physics.body.position = position;
    ship.physics.body.orientation = Quat::from_rotation_y(yaw);
    ship.handling.dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };
    vec![ship]
}

/// A charging bolt rides the craft that is charging it, and leaves along
/// wherever that craft is pointing when the wind-up ends - not along where it
/// pointed when the button went down.
///
/// **The whole point of the recovered charge.** `Plasma_UpdateCharge`
/// (`0x0885c170`) copies the firing craft's weapon-node world matrix onto the
/// entity every tick of the hold, and `Plasma_Launch` (`0x0885bf84`) re-reads
/// that same matrix at release. A bolt that flew off along the press-time
/// heading would be this engine's own simplification, and a visible one: a
/// second is a long time in a corner.
#[test]
fn a_charging_bolt_rides_the_craft_and_leaves_along_its_new_heading() {
    let geometry = CollisionWorld::new();
    let mut ships = one_ship(Vec3::ZERO, 0.0);
    let mut projectiles = Projectiles::new();

    let (nose, forward) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    assert!(projectiles.charge_up(nose, forward * 100.0, 0, CHARGE_SECONDS));

    // The whole wind-up, with the craft turning through it. The hold is
    // counted rather than assumed: `charge` is a float subtracted `dt` at a
    // time, so a one-second hold at a 60 Hz tick lands on 60 or 61 ticks
    // depending on where the rounding falls, and the original subtracts the
    // same way. Pinning 60 exactly would be pinning the rounding.
    let mut tick = 0;
    while projectiles.slots[0].charge > 0.0 {
        ships[0].physics.body.orientation = Quat::from_rotation_y(0.4 * TICK * tick as f32);
        ships[0].physics.body.position = Vec3::Z * (0.5 * TICK * tick as f32);
        projectiles.advance(
            TICK,
            &geometry,
            &ships,
            None,
            None,
            crate::projectile::TriggerRadii::default(),
            "VENOM",
        );
        tick += 1;
        let bolt = &projectiles.slots[0];
        assert_eq!(bolt.kind, Some(Weapon::Plasma), "the slot stays taken");
        // The reseat runs on the releasing tick too - the original calls
        // `Plasma_UpdateCharge` before it tests the countdown - so this holds
        // for every tick of the loop including the last.
        let (nose_now, _) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
        assert!(
            bolt.position.distance(nose_now) < 1e-3,
            "tick {tick}: the bolt sits on the nose, not where it was fired"
        );
        assert_eq!(
            bolt.lifetime, MAX_FLIGHT_SECONDS,
            "tick {tick}: a held bolt does not age - the original's age at +0x54 is \
             Plasma_Update's to advance and Plasma_Update does not run"
        );
        assert!(tick < 120, "the hold has to end");
    }
    assert!(
        (60..=61).contains(&tick),
        "a one-second hold is a second at 60 Hz, give or take the rounding: {tick} ticks"
    );

    let bolt = projectiles.slots[0];
    assert_eq!(bolt.charge, 0.0, "the hold is over after CHARGE_SECONDS");
    let heading = bolt.velocity.normalize();
    let (_, forward_now) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    assert!(
        heading.distance(forward_now) < 1e-3,
        "the bolt leaves along the craft's heading at release: {heading:?} against {forward_now:?}"
    );
    assert!(
        heading.distance(forward) > 0.1,
        "the craft turned far enough for release-time and press-time headings to differ"
    );
    assert!(
        (bolt.velocity.length() - 100.0).abs() < 1e-3,
        "the hold turns the shot without changing its speed"
    );

    // And the tick after the hold it is flying: it leaves the nose behind.
    let (nose_at_release, _) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    projectiles.advance(
        TICK,
        &geometry,
        &ships,
        None,
        None,
        crate::projectile::TriggerRadii::default(),
        "VENOM",
    );
    assert!(
        projectiles.slots[0].position.distance(nose_at_release) > 1.0,
        "once the hold is over the bolt flies"
    );
    assert!(
        projectiles.slots[0].lifetime < MAX_FLIGHT_SECONDS,
        "and only then does it start ageing"
    );
}

/// Every other weapon still spawns with no hold at all, so nothing else's
/// slot contents moved when the field landed.
#[test]
fn only_a_plasma_is_ever_held() {
    let mut projectiles = Projectiles::new();
    assert!(projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0));
    assert_eq!(projectiles.slots[0].charge, 0.0);
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z, 0, CHARGE_SECONDS));
    assert_eq!(projectiles.slots[1].charge, CHARGE_SECONDS);
}

/// A plasma bolt that hits nothing detonates when its ten seconds are up -
/// the same ending a wall gives it, ported 2026-09-16.
///
/// **Recovered from `Plasmas_Update` (`0x0886b490`), read at instruction level
/// with no elision.** `10.0 < age` sets the same destroy bit a wall does, and
/// both endings reach the *identical* teardown - `Psys_Release_q`,
/// `Plasma_SpawnDetonation`, `PLASMAHITWALL`. Flown in an empty world so
/// nothing but the timer can end it.
#[test]
fn a_plasma_bolt_detonates_when_its_ten_seconds_are_up() {
    let geometry = CollisionWorld::new();
    let ships: Vec<crate::world::Ship> = Vec::new();
    let mut projectiles = Projectiles::new();
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z * 200.0, 0, 0.0));

    let mut ticks = 0_usize;
    let mut ended: Option<crate::projectile::Impact> = None;
    while ended.is_none() {
        let impacts = projectiles.advance(
            TICK,
            &geometry,
            &ships,
            None,
            None,
            crate::projectile::TriggerRadii::default(),
            "VENOM",
        );
        ticks += 1;
        ended = impacts.into_iter().flatten().next();
        assert!(ticks < 700, "the bolt never ended");
    }

    let impact = ended.expect("an impact");
    assert_eq!(impact.struck, None, "an empty world struck a craft");
    assert!(
        !impact.blast,
        "the timeout spent a blast the original's teardown never reaches"
    );

    // Ten seconds as a literal, deliberately, and not the constant - see
    // `an_unguided_missile_detonates_when_its_three_seconds_are_up`'s own
    // doc comment for why.
    let flown = ticks as f32 / 60.0;
    assert!(
        (flown - 10.0).abs() <= 2.0 / 60.0,
        "it flew {flown}s, not the recovered 10.0s"
    );
    assert_eq!(
        MAX_FLIGHT_SECONDS, 10.0,
        "the constant and the number the flight is measured against have parted"
    );
}

/// And the timeout hurts nobody, however close they are standing - the same
/// half of the rule the Missile's self-detonation carries, and the one this
/// port exists to close for the Plasma.
///
/// **The original's blast (`Weapon_PostBlastImpulse`, `0886794c`) has exactly
/// one caller in the whole binary** - `FUN_08867b50`, the Mine's own chain,
/// confirmed by `get_xrefs_to` - **and the Plasma's teardown is not it.** So a
/// craft parked where a bolt runs out of time watches the flash and takes
/// nothing.
#[test]
fn a_timed_out_plasma_bolt_damages_nobody_standing_in_it() {
    let mut world = crate::World::new(1);
    world.ship_count = 2;
    // Slot 1 is parked near where a bolt fired down `+Z` runs out of time,
    // and well inside `blastradius` of it.
    for (slot, position) in [Vec3::ZERO, Vec3::Z * 500.0].into_iter().enumerate() {
        let ship = &mut world.ships[slot];
        ship.active = true;
        ship.handling.dimensions = Dimensions {
            length: 4.0,
            width: 2.0,
            height: 1.0,
            shield: 100.0,
            ..Dimensions::default()
        };
        ship.physics.shield = 100.0;
        ship.physics.body.mass = 1.0;
        ship.physics.body.position = position;
    }

    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Plasma"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" speed="600" launchSpeed="0"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses");

    // Fired from beside slot 1's line rather than at it, so the swept hull
    // test cannot end the flight early - this is about the timer, not about
    // a hit.
    assert!(
        world
            .projectiles
            .charge_up(Vec3::X * 40.0, Vec3::Z * 200.0, 0, 0.0)
    );

    let empty = CollisionWorld::new();
    let mut ended = false;
    for _ in 0..700 {
        let impacts = crate::projectile::step(
            &mut world,
            TICK,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [false; 2],
        );
        if let Some(impact) = impacts.into_iter().flatten().next() {
            assert!(!impact.blast, "the timer spent a blast");
            ended = true;
            break;
        }
    }
    assert!(ended, "the bolt never ran out of time");
    assert_eq!(
        world.ships[1].physics.shield, 100.0,
        "a craft standing in a timed-out bolt took damage the original never deals"
    );
    assert_eq!(
        world.ships[0].physics.shield, 100.0,
        "the firing craft took damage from its own expired bolt"
    );
}

/// The plasma table this section's tests share: a fixture that authors every
/// offset `Plasma_HitCraft`/`Plasma_ApplyBlastForce` read
/// (`docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit is the
/// third ending" section) - `damage` and `slowdown_time` for the direct
/// credit, `blastradius`/`blastforce` for the impulse sweep.
fn direct_hit_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Plasma"><Stats absorb="1" blastforce="10" blastradius="30"
               damage="25" slowdown_time="1" speed="600" launchSpeed="0"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
}

/// A direct hit is a craft hit, not a wall hit - `struck` carries the slot
/// and `blast` is `true`, unlike the wall and timeout endings this file
/// already covers.
///
/// **Recovered from `Plasma_SweepCraftHit` (`0x0886afb8`) calling
/// `Plasma_HitCraft`/`Plasma_ApplyBlastForce` on a hull hit, read
/// 2026-09-16** - a third ending neither of the other two tests in this file
/// exercises.
#[test]
fn a_plasma_bolt_that_strikes_a_craft_reports_it_struck_with_a_blast() {
    let geometry = CollisionWorld::new();
    let ships = vec![
        craft_at(Vec3::ZERO, 100.0, 1.0),
        craft_at(Vec3::Z * 30.0, 100.0, 1.0),
    ];
    let mut projectiles = Projectiles::new();
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z * 600.0, 0, 0.0));

    let mut impact = None;
    for _ in 0..20 {
        if let Some(hit) = projectiles
            .advance(
                1.0 / 60.0,
                &geometry,
                &ships,
                None,
                None,
                crate::projectile::TriggerRadii::default(),
                "VENOM",
            )
            .into_iter()
            .flatten()
            .next()
        {
            impact = Some(hit);
            break;
        }
    }
    let impact = impact.expect("a plasma bolt flew through a craft");
    assert_eq!(impact.struck, Some(1), "a plasma bolt hit its own launcher");
    assert_eq!(impact.owner, 0);
    assert!(
        impact.blast,
        "a craft hit spends a blast - only the wall and the timeout do not"
    );
}

/// The struck craft takes full damage and slowdown, unconditionally - the
/// original's `Plasma_HitCraft` has no distance test at all, unlike
/// [`blast`]'s own radius-and-falloff rule.
#[test]
fn a_plasma_bolt_that_strikes_a_craft_credits_it_directly() {
    let table = direct_hit_table();
    let mut world = crate::World::new(1);
    world.ship_count = 2;
    world.ships[0] = craft_at(Vec3::ZERO, 100.0, 1.0);
    world.ships[1] = craft_at(Vec3::Z * 30.0, 100.0, 1.0);
    assert!(
        world
            .projectiles
            .charge_up(Vec3::ZERO, Vec3::Z * 600.0, 0, 0.0)
    );

    let empty = CollisionWorld::new();
    let mut hit = false;
    for _ in 0..20 {
        let impacts = crate::projectile::step(
            &mut world,
            1.0 / 60.0,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [false; 2],
        );
        if impacts.iter().flatten().count() > 0 {
            hit = true;
            break;
        }
    }
    assert!(hit, "the bolt never reached the struck craft");
    assert_eq!(
        world.ships[1].physics.shield, 75.0,
        "the struck craft did not take the authored 25 damage"
    );
    assert_eq!(
        world.ships[1].pending_slowdown, 1.0,
        "the struck craft did not take the authored slowdown_time"
    );
}

/// Every other craft in `blastradius` - except the bolt's own firer - takes
/// only the falling-off `blastforce` impulse: no damage, no slowdown.
/// `Plasma_ApplyBlastForce` excludes `owner` outright, even though it sits
/// well inside the radius here.
#[test]
fn a_plasma_craft_hit_pushes_bystanders_but_spares_the_firer() {
    let table = direct_hit_table();
    let mut world = crate::World::new(1);
    world.ship_count = 3;
    // 0 fires, 1 is struck, 2 is a bystander inside `blastradius` (30) of the
    // hit but not on the flight path.
    world.ships[0] = craft_at(Vec3::ZERO, 100.0, 1.0);
    world.ships[1] = craft_at(Vec3::Z * 30.0, 100.0, 1.0);
    world.ships[2] = craft_at(Vec3::new(10.0, 0.0, 30.0), 100.0, 1.0);
    assert!(
        world
            .projectiles
            .charge_up(Vec3::ZERO, Vec3::Z * 600.0, 0, 0.0)
    );

    let empty = CollisionWorld::new();
    let mut hit = false;
    for _ in 0..20 {
        let impacts = crate::projectile::step(
            &mut world,
            1.0 / 60.0,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [false; 3],
        );
        if impacts.iter().flatten().count() > 0 {
            hit = true;
            break;
        }
    }
    assert!(hit, "the bolt never reached the struck craft");

    assert_eq!(
        world.ships[2].physics.shield, 100.0,
        "a bystander inside the radius took damage the direct hit never spends on it"
    );
    assert_eq!(
        world.ships[2].pending_slowdown, 0.0,
        "a bystander inside the radius took slowdown the direct hit never spends on it"
    );
    assert!(
        world.ships[2].physics.body.linear_velocity.length() > 0.0,
        "a bystander inside the radius took no impulse at all"
    );

    assert_eq!(
        world.ships[0].physics.shield, 100.0,
        "the firer took damage from its own bolt"
    );
    assert_eq!(
        world.ships[0].pending_slowdown, 0.0,
        "the firer took slowdown from its own bolt"
    );
    assert_eq!(
        world.ships[0].physics.body.linear_velocity.length(),
        0.0,
        "the firer is excluded from the impulse sweep, even though it sits inside the radius"
    );
}
