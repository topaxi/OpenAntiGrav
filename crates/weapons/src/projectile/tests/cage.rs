//! A probe or travel sweep that ends outside the collision cage is the original's code `4`
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "Code 4 is the collision cage"),
//! taken before the surface class.

use super::*;
use oag_physics::cage::Containment;

const DT: f32 = 1.0 / 60.0;

/// The tube's far side: everything past `x = limit` is outside, the boundary the plane.
struct EndsAtX(f32);

impl Containment for EndsAtX {
    fn outside(&self, point: Vec3) -> Option<(Vec3, Vec3)> {
        (point.x > self.0).then(|| (Vec3::new(self.0, point.y, point.z), Vec3::NEG_X))
    }
}

/// The tube's floor: everything below `y = limit` is outside, so only a probe that reaches the
/// ground is.
struct StartsAtY(f32);

impl Containment for StartsAtY {
    fn outside(&self, point: Vec3) -> Option<(Vec3, Vec3)> {
        (point.y < self.0).then(|| (Vec3::new(point.x, self.0, point.z), Vec3::Y))
    }
}

fn flying(kind: Weapon, position: Vec3, velocity: Vec3) -> Projectiles {
    let mut projectiles = Projectiles::new();
    projectiles.slots[0] = Projectile {
        kind: Some(kind),
        position,
        velocity,
        surface: Vec3::Y,
        lifetime: MAX_FLIGHT_SECONDS,
        ..Projectile::default()
    };
    projectiles
}

fn advance(
    projectiles: &mut Projectiles,
    world: &CollisionWorld,
    cage: &dyn Containment,
) -> [Option<Impact>; MAX_PROJECTILES] {
    let radii = TriggerRadii {
        mine: None,
        bomb: None,
    };
    projectiles.advance(
        DT,
        world,
        Some(cage),
        &ships(&[]),
        None,
        None,
        None,
        radii,
        "VENOM",
    )
}

#[test]
fn a_sweep_end_past_the_cage_ends_every_weapon_that_sweeps_at_the_boundary() {
    let from = Vec3::new(49.5, 10.0, 0.0);
    let velocity = Vec3::new(120.0, 0.0, 0.0);
    // The Rocket and the Plasma are caught by their probe first, which looks 6 and 12 units
    // below the segment's end; the others by the sweep itself.
    for (kind, y, blast) in [
        (Weapon::Rocket, 4.0, false),
        (Weapon::Plasma, -2.0, false),
        (Weapon::Cannon, 10.0, true),
        (Weapon::Missile, 10.0, false),
        (Weapon::Shuriken, 10.0, false),
    ] {
        let mut projectiles = flying(kind, from, velocity);
        let impacts = advance(&mut projectiles, &empty_world(), &EndsAtX(50.0));
        let impact = impacts[0].unwrap_or_else(|| panic!("{kind:?} flew on past the cage"));
        assert_eq!(impact.point, Vec3::new(50.0, y, 0.0), "{kind:?}");
        assert_eq!(impact.struck, None, "{kind:?}");
        assert_eq!(impact.blast, blast, "{kind:?}");
        assert_eq!(projectiles.slots[0].kind, None, "{kind:?} is gone");
    }
}

#[test]
fn without_a_cage_the_same_flight_goes_on() {
    let mut projectiles = flying(
        Weapon::Rocket,
        Vec3::new(49.5, 10.0, 0.0),
        Vec3::new(120.0, 0.0, 0.0),
    );
    let impacts = projectiles.advance(
        DT,
        &empty_world(),
        None,
        &ships(&[]),
        None,
        None,
        None,
        TriggerRadii {
            mine: None,
            bomb: None,
        },
        "VENOM",
    );
    assert!(impacts[0].is_none());
    assert!(projectiles.slots[0].kind.is_some());
}

#[test]
fn a_probe_that_reaches_the_ground_outside_the_cage_ends_a_rocket_and_a_plasma_there() {
    // A floor at y = 0 under a projectile riding 4 above it; the cage reaches no lower than 0.5, so
    // the probe's floor hit is outside it while the segment's end is inside.
    for kind in [Weapon::Rocket, Weapon::Plasma] {
        let mut projectiles = flying(kind, Vec3::new(0.0, 4.0, 0.0), Vec3::new(100.0, 0.0, 0.0));
        let impacts = advance(&mut projectiles, &floor_at_y(0.0), &StartsAtY(0.5));
        let impact = impacts[0].unwrap_or_else(|| panic!("{kind:?} rode a floor outside the cage"));
        assert_eq!(
            impact.point.y, 0.5,
            "{kind:?} ends at the boundary, not the hit"
        );
        assert!(!impact.blast, "{kind:?}");
    }
}

#[test]
fn a_probe_outside_the_cage_leaves_a_missile_and_a_shuriken_neither_riding_nor_falling() {
    for kind in [Weapon::Missile, Weapon::Shuriken] {
        let velocity = Vec3::new(100.0, -20.0, 0.0);
        let mut projectiles = flying(kind, Vec3::new(0.0, 4.0, 0.0), velocity);
        let impacts = advance(&mut projectiles, &floor_at_y(0.0), &StartsAtY(0.5));
        assert!(impacts[0].is_none(), "{kind:?} lives on");
        let after = projectiles.slots[0];
        // A ride would turn the velocity parallel to the floor and seat the projectile at 3.0;
        // a fall would add to its downward speed.
        assert_eq!(after.velocity, velocity, "{kind:?}");
        assert_eq!(
            after.position,
            Vec3::new(0.0, 4.0, 0.0) + velocity * DT,
            "{kind:?}"
        );
    }
}
