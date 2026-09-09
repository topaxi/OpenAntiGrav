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
