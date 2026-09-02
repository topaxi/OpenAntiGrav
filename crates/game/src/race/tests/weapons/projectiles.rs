//! What throwing or firing a projectile weapon puts in the air: the count, the
//! direction and the fuse, for the weapons that leave the nose.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` on the day the Shuriken landed and took that
//! file past it; a move, with no behaviour change. Shared fixtures live in
//! `race/tests.rs`, as they did before.
//!
//! The seam is a real one and it is the one the parent's own module doc
//! already drew: everything here is *what leaves the craft*, and what is left
//! behind is the pad, the inventory and the pickups that never leave it at
//! all. What a projectile then **shows** is [`super::visuals`]; where it ends
//! up on a real circuit is `crates/game/tests/*_ground_truth.rs`.

use super::*;

/// Throwing a Shuriken puts **one** in the air, off to one side, and the side
/// is the coin's.
///
/// **The angle is recovered and the side is random**, which is an awkward pair
/// to assert: a single run only ever sees one side, and asserting the *value*
/// of a seeded draw pins the generator rather than the weapon. So this asserts
/// the part that holds either way - the blade leaves at
/// [`shuriken::LAUNCH_ANGLE`] off the nose, magnitude exact - and a second test
/// below asserts that both sides are reachable.
#[test]
fn a_thrown_shuriken_leaves_at_the_recovered_angle() {
    use oag_gameplay::projectile::shuriken;

    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_shuriken_table(),
    );
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Shuriken)
    );

    let forward = race.ship().physics.body.forward();
    let up = race.ship().physics.body.up();
    race.tick(&buttons.tick(SQUARE));

    assert_eq!(race.ship_pickup(), None, "throwing must spend the pickup");
    assert_eq!(race.world.projectiles.live(), 1, "one press is one blade");

    let blade = race.world.projectiles.slots[0];
    assert_eq!(blade.kind, Some(oag_formats::weapons::Weapon::Shuriken));
    // The fuse, not the ten-second safety net every other flier gets. Measured
    // one tick after the throw - the blade is spawned and then flown inside the
    // same `tick`, so it has already spent `dt` of its own fuse by the time a
    // test can look at it.
    let spent = 0.5 - blade.lifetime;
    assert!(
        spent > 0.0 && spent <= 1.0 / 60.0 + 1e-4,
        "the blade carries {:.4} s of a 0.5 s fuse, having spent {spent:.4} - that is \
         not one tick's worth, so it is not the authored fuse it was given",
        blade.lifetime
    );

    // The angle, measured in the plane the rotation is about. `atan2` of the
    // components across and along the nose is exactly the launch angle's
    // magnitude, whichever side the coin picked.
    let direction = blade.velocity.normalize();
    let along = direction.dot(forward);
    let across = direction.dot(forward.cross(up));
    let angle = across.atan2(along).abs();
    assert!(
        (angle - shuriken::LAUNCH_ANGLE).abs() < 1e-3,
        "the blade left at {angle:.4} rad where the original throws at {:.4}",
        shuriken::LAUNCH_ANGLE
    );
}

/// Both sides of the coin are reachable, and the draw is the world's own seeded
/// generator rather than anything ambient.
///
/// **Ten throws from one race**, so the generator advances exactly as it would
/// in a real one. Asserting "both sides appear" rather than a specific sequence
/// keeps this a test of the weapon: a change to `Rng`'s internals should not
/// fail it, and a Shuriken that always veered the same way should.
#[test]
fn a_shurikens_side_is_drawn_and_both_sides_come_up() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_shuriken_table(),
    );
    race.tick(&InputSnapshot::default());

    let forward = race.ship().physics.body.forward();
    let up = race.ship().physics.body.up();
    let right = forward.cross(up);

    let mut sides = (0, 0);
    for _ in 0..10 {
        race.world.projectiles.clear();
        race.world.ships[0].pickup.weapon = Some(oag_formats::weapons::Weapon::Shuriken);
        let mut buttons = Buttons::new();
        buttons.tick(0);
        race.tick(&buttons.tick(SQUARE));
        let blade = race.world.projectiles.slots[0];
        assert_eq!(blade.kind, Some(oag_formats::weapons::Weapon::Shuriken));
        if blade.velocity.dot(right) > 0.0 {
            sides.0 += 1;
        } else {
            sides.1 += 1;
        }
    }
    assert!(
        sides.0 > 0 && sides.1 > 0,
        "ten throws all went the same way ({sides:?}) - the coin is not being flipped"
    );
}

/// Firing a Plasma puts **one** in the air, ahead of the craft, straight down
/// its own forward axis - and spends the pickup.
///
/// One is recovered rather than chosen, twice over: `Weapon_FirePlasma`
/// (`0x0886a868`) takes one pool slot and clears its own request bit in the
/// same breath, and the `<Stats>` author no `spread` to fan a volley with. See
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
///
/// **Deliberately the Rocket's test with the count changed**, because that is
/// what the weapon is: one bolt where the Rocket throws three, off the same
/// flight model. The "no fan" assertion is the one that is not a mirror image -
/// a Plasma flying off-axis would mean the Rocket's `launch` had been reused
/// wholesale rather than the single-shot one written.
#[test]
fn a_fired_plasma_puts_exactly_one_projectile_in_the_air() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_plasma_table());
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Plasma)
    );
    assert_eq!(race.world.projectiles.live(), 0, "nothing before firing");

    let before = race.ship().physics.body.position;
    let forward = race.ship().physics.body.forward();
    race.tick(&buttons.tick(SQUARE));

    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.world.projectiles.live(),
        1,
        "one press is one bolt, not a volley"
    );

    let bolt = race.world.projectiles.slots[0];
    assert_eq!(bolt.kind, Some(oag_formats::weapons::Weapon::Plasma));
    assert_eq!(bolt.owner, 0);
    assert!(
        (bolt.position - before).dot(forward) > 0.0,
        "the bolt spawned behind the craft: {:?}",
        bolt.position
    );
    // Venom's authored speed plus `launchSpeed`, both km/h in the file, so the
    // velocity is the sum over `KMH_PER_UNIT_PER_SECOND` - spelled as the
    // arithmetic so the unit stays legible, as the Rocket's test does.
    let expected = (650.0 + 26.0) / oag_gameplay::projectile::KMH_PER_UNIT_PER_SECOND;
    assert!(
        (bolt.velocity.length() - expected).abs() < 1e-2,
        "expected (650 + 26) km/h as units per second, got {}",
        bolt.velocity.length()
    );
    // Straight down the nose: no fan, so no lateral component at all.
    let right = race.ship().physics.body.right();
    assert!(
        bolt.velocity.dot(right).abs() < 1e-2,
        "the bolt was fanned: lateral component {}",
        bolt.velocity.dot(right)
    );
    assert!(
        bolt.velocity.dot(forward) > 0.0,
        "the bolt must fly forwards"
    );
}

/// Firing a Rocket puts **three** in the air at once, ahead of the craft,
/// at the class's own speed - and spends the pickup.
///
/// Three is recovered, not chosen: `Weapon_FireRocket` (`0x0886e104`) makes
/// three literal spawn calls in one invocation. See
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
///
/// **The fan's geometry is pinned in `oag_gameplay::projectile`'s own
/// tests**, the same split the Shield's wiring test uses. What this owns is
/// the wiring: that the button reaches the array, three times, with the
/// disc's numbers, and that the slot empties.
#[test]
fn a_fired_rocket_puts_three_projectiles_in_the_air() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Rocket)
    );
    assert_eq!(race.world.projectiles.live(), 0, "nothing before firing");

    let before = race.ship().physics.body.position;
    let forward = race.ship().physics.body.forward();
    race.tick(&buttons.tick(SQUARE));

    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.world.projectiles.live(),
        oag_gameplay::projectile::ROCKET_SHOTS,
        "one press is a volley of three"
    );

    for slot in 0..oag_gameplay::projectile::ROCKET_SHOTS {
        let rocket = race.world.projectiles.slots[slot];
        assert_eq!(rocket.kind, Some(oag_formats::weapons::Weapon::Rocket));
        assert_eq!(rocket.owner, 0);
        assert!(
            (rocket.position - before).dot(forward) > 0.0,
            "rocket {slot} spawned behind the craft: {:?}",
            rocket.position
        );
        // Venom's authored speed plus `launchSpeed`, the same for all three:
        // the fan turns them, it does not slow them. Both figures are km/h
        // in the file, so the velocity is the sum over
        // `KMH_PER_UNIT_PER_SECOND` - spelled as the arithmetic so the unit
        // stays legible.
        let expected = (600.0 + 16.0) / oag_gameplay::projectile::KMH_PER_UNIT_PER_SECOND;
        assert!(
            (rocket.velocity.length() - expected).abs() < 1e-2,
            "rocket {slot}: expected (600 + 16) km/h as units per second, got {}",
            rocket.velocity.length()
        );
        assert!(
            rocket.velocity.dot(forward) > 0.0,
            "rocket {slot} must fly forwards"
        );
    }

    // And they really are fanned rather than three copies of one shot,
    // which the count alone would not catch.
    let right = race.ship().physics.body.right();
    let lateral: Vec<f32> = (0..oag_gameplay::projectile::ROCKET_SHOTS)
        .map(|slot| race.world.projectiles.slots[slot].velocity.dot(right))
        .collect();
    assert!(
        lateral.iter().any(|&l| l > 1.0) && lateral.iter().any(|&l| l < -1.0),
        "the volley did not fan: lateral components {lateral:?}"
    );
}
