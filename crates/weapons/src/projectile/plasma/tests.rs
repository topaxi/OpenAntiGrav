//! What the Plasma's wind-up in [`super`] is asserted to do. Its own file, as
//! `projectile/tests.rs` sat at the line cap when this landed.

use super::*;
use crate::projectile::{MAX_FLIGHT_SECONDS, Projectiles};
use oag_core::math::Quat;
use oag_physics::{CollisionWorld, Surface, TriangleSoup};
use oag_tables::weapons::Weapon;

fn craft_at(position: Vec3, shield: f32, mass: f32) -> crate::test_craft::Ship {
    let mut ship = crate::test_craft::Ship {
        active: true,
        ..crate::test_craft::Ship::default()
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

fn one_ship(position: Vec3, yaw: f32) -> Vec<crate::test_craft::Ship> {
    let mut ship = crate::test_craft::Ship {
        active: true,
        ..crate::test_craft::Ship::default()
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

/// A charging bolt rides the craft charging it and leaves along where that craft
/// points when the wind-up ends, not where it pointed at the press. The point of
/// the recovered charge: `Plasma_UpdateCharge` (`0x0885c170`) copies the craft's
/// weapon-node matrix every tick and `Plasma_Launch` (`0x0885bf84`) re-reads it at
/// release.
#[test]
fn a_charging_bolt_rides_the_craft_and_leaves_along_its_new_heading() {
    let geometry = CollisionWorld::new();
    let mut ships = one_ship(Vec3::ZERO, 0.0);
    let mut projectiles = Projectiles::new();

    let (nose, forward) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    assert!(projectiles.charge_up(nose, forward * 100.0, 0, CHARGE_SECONDS));

    // The wind-up with the craft turning. The hold is counted: `charge` is a float
    // minus `dt`, so one second at 60 Hz is 60 or 61 ticks as the rounding falls
    // (the original subtracts the same way); pinning 60 would pin the rounding.
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
            None,
            crate::projectile::TriggerRadii::default(),
            "VENOM",
        );
        tick += 1;
        let bolt = &projectiles.slots[0];
        assert_eq!(bolt.kind, Some(Weapon::Plasma), "the slot stays taken");
        // The reseat runs on the releasing tick too: the original calls
        // `Plasma_UpdateCharge` before testing the countdown.
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

/// A bolt that hits nothing detonates when its ten seconds are up, as a wall ends
/// it (ported 2026-09-16). **Recovered from `Plasmas_Update` (`0x0886b490`)** at
/// instruction level: `10.0 < age` sets the wall's destroy bit and both reach the
/// identical teardown (`Psys_Release_q`, `Plasma_SpawnDetonation`,
/// `PLASMAHITWALL`). Flown in an empty world so only the timer can end it.
#[test]
fn a_plasma_bolt_detonates_when_its_ten_seconds_are_up() {
    let geometry = CollisionWorld::new();
    let ships: Vec<crate::test_craft::Ship> = Vec::new();
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

    // Ten seconds as a literal, not the constant; see
    // `an_unguided_missile_detonates_when_its_three_seconds_are_up`.
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

/// And the timeout hurts nobody, however close they stand. **`Weapon_PostBlastImpulse`
/// (`0886794c`) has one caller in the binary**, `FUN_08867b50` (the Mine's chain,
/// by `get_xrefs_to`), and the Plasma's teardown is not it: a parked craft sees
/// the flash and takes nothing.
#[test]
fn a_timed_out_plasma_bolt_damages_nobody_standing_in_it() {
    let mut world = crate::test_craft::World::new(1);
    world.ship_count = 2;
    // Slot 1 is parked where a bolt fired down `+Z` runs out, well inside
    // `blastradius`.
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

    // Fired beside slot 1's line so the swept hull test cannot end the flight early.
    assert!(
        world
            .projectiles
            .charge_up(Vec3::X * 40.0, Vec3::Z * 200.0, 0, 0.0)
    );

    let empty = CollisionWorld::new();
    let mut ended = false;
    for _ in 0..700 {
        let impacts = crate::projectile::step(
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            TICK,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [crate::projectile::WeaponHit::default(); 2],
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

/// The plasma table these tests share: every offset `Plasma_HitCraft` and
/// `Plasma_ApplyBlastForce` read (`docs/ghidra/functions/psp-pulse-usa/plasma.md`,
/// "a craft hit is the third ending"): `damage`, `slowdown_time`, `blastradius`,
/// `blastforce`.
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

/// A direct hit is a craft hit, not a wall hit: `struck` carries the slot and
/// `blast` is `true`. **Recovered from `Plasma_SweepCraftHit` (`0x0886afb8`)**
/// calling `Plasma_HitCraft`/`Plasma_ApplyBlastForce` on a hull hit (read
/// 2026-09-16), a third ending the other tests do not exercise.
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

/// The struck craft takes full damage and slowdown unconditionally:
/// `Plasma_HitCraft` has no distance test, unlike [`blast`].
#[test]
fn a_plasma_bolt_that_strikes_a_craft_credits_it_directly() {
    let table = direct_hit_table();
    let mut world = crate::test_craft::World::new(1);
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
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            1.0 / 60.0,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [crate::projectile::WeaponHit::default(); 2],
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

/// Every other craft in `blastradius` except the firer takes only the falling-off
/// `blastforce` impulse, no damage or slowdown: `Plasma_ApplyBlastForce` excludes
/// `owner` outright.
#[test]
fn a_plasma_craft_hit_pushes_bystanders_but_spares_the_firer() {
    let table = direct_hit_table();
    let mut world = crate::test_craft::World::new(1);
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
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            1.0 / 60.0,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [crate::projectile::WeaponHit::default(); 3],
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

/// A fixture whose class speed and `launchSpeed` are round numbers, so a launch
/// reading differs from a class reading at a glance; `launchSpeed="0"` makes the
/// craft's speed the whole launch speed.
fn ramp_table() -> oag_tables::weapons::PlasmaStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Plasma"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" speed="600" launchSpeed="0"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .plasma()
    .expect("a Plasma")
}

/// A large horizontal floor, normal up: the ramp re-pins the bolt's speed only on
/// a tick the floor probe finds one; see [`crate::projectile::Projectiles::advance`].
fn floor_at_y(y: f32) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, y, -500.0],
            [-500.0, y, 500.0],
            [500.0, y, 500.0],
            [500.0, y, -500.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

/// The discriminating test. `Plasma_Launch` re-reads the craft's velocity when the
/// charge ends, not when the press started it: a player who presses still and then
/// accelerates through the hold leaves at the speed reached
/// (`docs/ghidra/functions/psp-pulse-usa/plasma.md`, "the charge is real"). A
/// wiring bug leaving [`Projectile::launch_speed_kmh`] at its default, or reading
/// the velocity at [`Projectiles::charge_up`], passes every formula-only test and
/// fails this.
#[test]
fn the_launch_speed_reads_the_crafts_velocity_at_release_not_at_the_press() {
    let stats = ramp_table();
    let geometry = CollisionWorld::new();
    let mut ships = one_ship(Vec3::ZERO, 0.0);
    let mut projectiles = Projectiles::new();
    let (nose, forward) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    // Still at the press: read now, the launch speed would be `launchSpeed`, `0.0`.
    assert!(projectiles.charge_up(nose, forward * 100.0, 0, CHARGE_SECONDS));

    let mut tick = 0;
    while projectiles.slots[0].charge > 0.0 {
        // Accelerating through the hold, so by release the craft is at 100 units/s
        // (360 km/h) on every tick the countdown could cross zero.
        ships[0].physics.body.linear_velocity = Vec3::Z * 100.0;
        projectiles.advance(
            TICK,
            &geometry,
            &ships,
            None,
            Some(&stats),
            None,
            crate::projectile::TriggerRadii::default(),
            "VENOM",
        );
        tick += 1;
        assert!(tick < 120, "the hold has to end");
    }

    let bolt = projectiles.slots[0];
    assert!(
        (bolt.launch_speed_kmh - 360.0).abs() < 1e-3,
        "expected 360 km/h from the craft's speed at release, got {}",
        bolt.launch_speed_kmh
    );
    let expected_units_per_second = 360.0 / KMH_PER_UNIT_PER_SECOND;
    assert!(
        (bolt.velocity.length() - expected_units_per_second).abs() < 1e-3,
        "the release tick's own velocity must already carry the ramp's age-0 \
         value: expected {expected_units_per_second}, got {}",
        bolt.velocity.length()
    );
}

/// A flying bolt's speed is strictly between launch and class speeds partway
/// through the one-second ramp and on the class speed after it
/// ([`missile::speed_kmh`] through real flight). Needs a floor: [`floor_at_y`].
#[test]
fn a_flying_plasma_bolts_speed_blends_from_launch_to_class_over_one_second() {
    let stats = ramp_table();
    let geometry = floor_at_y(0.0);
    let mut ships = one_ship(Vec3::new(0.0, crate::projectile::RIDE_HEIGHT, 0.0), 0.0);
    // 100 units/s == 360 km/h through charge and flight, so only the ramp can move
    // the speed.
    ships[0].physics.body.linear_velocity = Vec3::Z * 100.0;
    let mut projectiles = Projectiles::new();
    let (nose, forward) = muzzle(&ships[0].physics, &ships[0].handling.dimensions);
    assert!(projectiles.charge_up(nose, forward * 100.0, 0, CHARGE_SECONDS));

    let advance = |projectiles: &mut Projectiles, ships: &[crate::test_craft::Ship]| {
        projectiles.advance(
            TICK,
            &geometry,
            ships,
            None,
            Some(&stats),
            None,
            crate::projectile::TriggerRadii::default(),
            "VENOM",
        );
    };

    let mut tick = 0;
    while projectiles.slots[0].charge > 0.0 {
        advance(&mut projectiles, &ships);
        tick += 1;
        assert!(tick < 120, "the hold has to end");
    }
    assert!(
        (projectiles.slots[0].launch_speed_kmh - 360.0).abs() < 1e-3,
        "got {}",
        projectiles.slots[0].launch_speed_kmh
    );

    // Half a second in: strictly between 360 and 600.
    for _ in 0..30 {
        advance(&mut projectiles, &ships);
    }
    let mid = projectiles.slots[0].velocity.length() * KMH_PER_UNIT_PER_SECOND;
    assert!(
        (361.0..600.0).contains(&mid),
        "expected the bolt strictly between the launch (360) and class (600) \
         speed at age ~0.5s, got {mid}"
    );

    // Well past the one-second ramp: at the class speed and staying there.
    for _ in 0..90 {
        advance(&mut projectiles, &ships);
    }
    let after = projectiles.slots[0].velocity.length() * KMH_PER_UNIT_PER_SECOND;
    assert!(
        (after - 600.0).abs() < 1.0,
        "expected the class speed once the ramp is over, got {after}"
    );
}
