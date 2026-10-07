//! A projectile is born riding the firing craft's up, not world up.

use super::*;
use oag_core::math::quat_from_axis_angle;

fn rolled_craft() -> Vec<crate::test_craft::Ship> {
    let mut ships = ships(&[(true, Vec3::new(0.0, 10.0, 0.0))]);
    ships[0].physics.body.orientation = quat_from_axis_angle(Vec3::NEG_Z, 60.0_f32.to_radians());
    ships
}

fn assert_riding_up(projectile: &Projectile, ship: &crate::test_craft::Ship) {
    let up = ship.physics.body.up();
    assert!(up.y < 0.6, "the fixture is banked");
    assert!(
        (projectile.surface - up).length() < 1e-4,
        "{:?} is not the craft's up {up:?}",
        projectile.surface
    );
}

#[test]
fn a_plasma_bolt_is_released_riding_the_craft_up_whatever_the_table() {
    let ships = rolled_craft();
    let mut projectiles = Projectiles::new();
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z, 0, 0.05));
    for _ in 0..8 {
        projectiles.advance(
            1.0 / 60.0,
            &empty_world(),
            &ships,
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
        );
    }
    let bolt = projectiles.slots[0];
    assert_eq!(bolt.charge, 0.0, "released");
    assert_riding_up(&bolt, &ships[0]);
}

#[test]
fn a_missile_fired_from_a_banked_craft_rides_the_craft_up() {
    let ships = rolled_craft();
    let mut projectiles = Projectiles::new();
    assert!(projectiles.spawn_guided(
        Weapon::Missile,
        Vec3::ZERO,
        Vec3::Z,
        0,
        None,
        600.0,
        ships[0].physics.body.up(),
    ));
    assert_riding_up(&projectiles.slots[0], &ships[0]);
}

#[test]
fn a_shuriken_thrown_from_a_banked_craft_rides_the_craft_up() {
    let ships = rolled_craft();
    let stats = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Shuriken"><Stats absorb="1" rhicochetForce="2" blastForce="3"
               blastradius="4" rhicochetdamage="5" blastdamage="6" slowdown_time="7"
               venomspeed="500" flashspeed="600" rapierspeed="700" phantomspeed="800"
               launchSpeed="8" fuse="2"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .shuriken()
    .expect("a Shuriken");
    let mut projectiles = Projectiles::new();
    let mut rng = oag_core::rng::Rng::new(1);
    let thrown = shuriken::fire(
        &mut projectiles,
        &ships[0].physics,
        &ships[0].handling.dimensions,
        &stats,
        "VENOM",
        &mut rng,
        0,
    );
    assert_eq!(thrown, Some(true));
    assert_riding_up(&projectiles.slots[0], &ships[0]);
}
