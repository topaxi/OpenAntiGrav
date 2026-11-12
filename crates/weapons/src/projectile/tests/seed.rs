//! A projectile is born riding the normal its launch carries: the track's up at the craft's
//! located point when known (`Craft::track_up`), else the craft's own up, never world up.

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
            None,
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
        ships[0].launch_normal(),
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
        ships[0].launch_normal(),
    );
    assert_eq!(thrown, Some(true));
    assert_riding_up(&projectiles.slots[0], &ships[0]);
}

/// With nothing under it a Plasma bolt falls along the normal it carries, as
/// `Plasma_Update`'s `0x7f` arm does (`velocity -= surface * dt * 50`), not along
/// world `-Y` as the Rocket and Shuriken do. On a craft rolled 60 degrees the two
/// are different directions.
#[test]
fn a_plasma_bolt_over_nothing_falls_along_its_carried_normal() {
    let ships = rolled_craft();
    let up = ships[0].physics.body.up();
    let mut projectiles = Projectiles::new();
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z * 100.0, 0, 0.05));
    let step = |projectiles: &mut Projectiles| {
        projectiles.advance(
            1.0 / 60.0,
            &empty_world(),
            None,
            &ships,
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
        );
    };
    for _ in 0..8 {
        step(&mut projectiles);
    }
    let released = projectiles.slots[0].velocity;
    for _ in 0..10 {
        step(&mut projectiles);
    }
    let gained = projectiles.slots[0].velocity - released;
    let expected = -up * (FALL_ACCELERATION * 10.0 / 60.0);
    assert!(
        (gained - expected).length() < 1e-2,
        "fell by {gained:?}, a fall along the carried normal is {expected:?}"
    );
}

/// The located track's up wins over the craft's: a craft banked 60 degrees on a
/// level track releases its bolt riding `(0, 1, 0)`. Drops if the charge's reseat
/// goes back to `body.up()`.
#[test]
fn a_plasma_bolt_is_released_riding_the_located_track_up() {
    let mut ships = rolled_craft();
    ships[0].track_up = Some(Vec3::Y);
    let mut projectiles = Projectiles::new();
    assert!(projectiles.charge_up(Vec3::ZERO, Vec3::Z, 0, 0.05));
    for _ in 0..8 {
        projectiles.advance(
            1.0 / 60.0,
            &empty_world(),
            None,
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
    assert_eq!(bolt.surface, Vec3::Y);
    assert_ne!(bolt.surface, ships[0].physics.body.up());
}

#[test]
fn a_launch_normal_is_the_track_up_when_known_and_the_craft_up_otherwise() {
    let mut ships = rolled_craft();
    assert_eq!(ships[0].launch_normal(), ships[0].physics.body.up());
    ships[0].track_up = Some(Vec3::Y);
    assert_eq!(ships[0].launch_normal(), Vec3::Y);
}
