//! A Cannon round leaves the craft's own nose and flies straight, banked or not.

use super::*;
use crate::projectile::{Projectiles, TriggerRadii};
use crate::test_craft::Ship;
use oag_core::math::quat_from_axis_angle;
use oag_physics::params::Dimensions;
use oag_physics::{CollisionWorld, Surface, TriangleSoup};
use oag_tables::weapons::Weapon;

const DT: f32 = 1.0 / 60.0;

fn floor_at_y(y: f32) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-5000.0, y, -5000.0],
            [-5000.0, y, 5000.0],
            [5000.0, y, 5000.0],
            [5000.0, y, -5000.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

fn dimensions() -> Dimensions {
    Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    }
}

/// A craft 10 units over a flat floor, rolled `degrees` about its own nose.
fn rolled_craft(degrees: f32) -> ShipState {
    let mut state = ShipState::default();
    state.body.position = Vec3::new(0.0, 10.0, 0.0);
    state.body.orientation = quat_from_axis_angle(Vec3::NEG_Z, degrees.to_radians());
    state.body.linear_velocity = state.body.forward() * 50.0;
    state
}

fn advance(projectiles: &mut Projectiles, world: &CollisionWorld) {
    let ships: Vec<Ship> = Vec::new();
    projectiles.advance(
        DT,
        world,
        &ships,
        None,
        None,
        None,
        TriggerRadii::default(),
        "VENOM",
    );
}

/// The bug this pins: a round born with world up as its ridden normal probed
/// straight down on a bank, found the floor under the *world* and was snapped
/// to it, a few units off the muzzle. `Cannon_UpdateRound` has no probe.
#[test]
fn a_round_from_a_banked_craft_stays_on_its_own_muzzle_line() {
    let world = floor_at_y(0.0);
    for degrees in [0.0_f32, 30.0, 60.0, 90.0, 135.0] {
        for left in [false, true] {
            let state = rolled_craft(degrees);
            let (position, velocity) = launch(&state, &dimensions(), left);
            let mut projectiles = Projectiles::new();
            assert!(projectiles.spawn(Weapon::Cannon, position, velocity, 0));
            for tick in 1..=6 {
                advance(&mut projectiles, &world);
                let round = projectiles.slots[0];
                let expected = position + velocity * (DT * tick as f32);
                assert!(
                    (round.position - expected).length() < 1e-3,
                    "roll {degrees} left {left} tick {tick}: {:?} vs {expected:?}",
                    round.position
                );
            }
            let offset = projectiles.slots[0].position - state.body.position;
            let width = dimensions().width;
            let sideways = offset.dot(state.body.right());
            let flew = velocity.length() * DT * 6.0;
            assert!(
                (sideways.abs() - width * MUZZLE_SPACING).abs() < 1e-3,
                "roll {degrees}: sideways {sideways}"
            );
            assert!(
                (offset.dot(state.body.forward()) - flew).abs() > 0.0,
                "it left the nose"
            );
            let heading = projectiles.slots[0].velocity.normalize();
            assert!(
                heading.dot(state.body.forward()) > 0.9999,
                "roll {degrees}: not along the nose: {heading:?}"
            );
        }
    }
}

/// `Cannon_UpdateRound`'s third arm: a floor is reflected off, not ridden.
#[test]
fn a_round_that_meets_a_floor_is_reflected_and_pushed_off_by_three() {
    let mut projectiles = Projectiles::new();
    let speed = 300.0;
    let down = Vec3::new(0.0, -1.0, 1.0).normalize() * speed;
    projectiles.spawn(Weapon::Cannon, Vec3::new(0.0, 2.0, 0.0), down, 0);
    let world = floor_at_y(0.0);
    let mut bounced = false;
    for _ in 0..10 {
        advance(&mut projectiles, &world);
        let round = projectiles.slots[0];
        assert_eq!(round.kind, Some(Weapon::Cannon), "a glance is not a hit");
        if round.velocity.y > 0.0 {
            bounced = true;
            assert!(
                (round.position.y - BOUNCE_PUSH_OFF).abs() < 1e-3,
                "{round:?}"
            );
            assert!((round.velocity.length() - speed).abs() < 1e-2, "{round:?}");
            break;
        }
    }
    assert!(bounced, "it should have turned off the floor");
}
