//! What [`super::drain`] is asserted to do.

use super::*;

/// A world of `n` active craft, all at the origin and all unshielded.
fn world(count: u8) -> World {
    let mut world = World::new(1);
    world.ship_count = count;
    for slot in 0..count as usize {
        world.ships[slot].active = true;
    }
    world
}

/// The plain case: a credited slot arms the timer and empties itself.
#[test]
fn a_pending_credit_arms_the_timer_and_clears_the_slot() {
    let mut world = world(1);
    world.ships[0].pending_slowdown = 0.5;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 0.5);
    assert_eq!(world.ships[0].pending_slowdown, 0.0);
}

/// The clamp is the one in `oag_physics::slowdown::add`, and the drain hands it
/// the whole pending figure rather than metering it out.
#[test]
fn the_drain_clamps_through_the_authored_ceiling() {
    let mut world = world(1);
    world.ships[0].pending_slowdown = 9.0;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 2.0);
    assert_eq!(world.ships[0].pending_slowdown, 0.0);
}

/// **The shield gate discards, it does not defer.** A hit landed one tick
/// before a shield expires is lost - which is the whole point of asserting the
/// slot is zero here rather than merely that the timer did not move.
#[test]
fn a_shielded_craft_takes_no_slowdown_and_loses_the_credit_anyway() {
    let mut world = world(1);
    world.ships[0].pending_slowdown = 1.0;
    world.ships[0].physics.shield_pickup_timer = 1.0;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 0.0);
    assert_eq!(
        world.ships[0].pending_slowdown, 0.0,
        "the pending figure was banked rather than discarded"
    );

    // And the next tick, with the shield still up but nothing new credited, is
    // not a second chance at it.
    world.ships[0].physics.shield_pickup_timer = 0.0;
    drain(&mut world, Some(2.0));
    assert_eq!(world.ships[0].physics.slowdown_timer, 0.0);
}

/// A craft's energy pool is not the gate; the fired Shield pickup is.
#[test]
fn an_empty_energy_pool_does_not_gate_the_drain() {
    let mut world = world(1);
    world.ships[0].pending_slowdown = 1.0;
    world.ships[0].physics.shield = 0.0;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 1.0);
}

/// No table, no drain - and no credit either, so nothing is stranded.
#[test]
fn a_race_with_no_weapon_table_drains_nothing() {
    let mut world = world(1);
    world.ships[0].pending_slowdown = 1.0;

    drain(&mut world, None);

    assert_eq!(world.ships[0].physics.slowdown_timer, 0.0);
    assert_eq!(world.ships[0].pending_slowdown, 1.0);
}

/// Every craft in the field, not just the player's slot.
#[test]
fn the_drain_reaches_every_active_craft() {
    let mut world = world(3);
    world.ships[0].pending_slowdown = 0.25;
    world.ships[2].pending_slowdown = 0.75;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 0.25);
    assert_eq!(world.ships[1].physics.slowdown_timer, 0.0);
    assert_eq!(world.ships[2].physics.slowdown_timer, 0.75);
}

/// The residue `oag_physics::forces::evaluate` leaves below zero survives the
/// drain, because the drain is a pass-through to the same one-sided clamp.
#[test]
fn a_negative_residue_is_carried_into_the_next_hit() {
    let dt = 1.0 / 60.0;
    let mut world = world(1);
    world.ships[0].physics.slowdown_timer = -dt;
    world.ships[0].pending_slowdown = 1.0;

    drain(&mut world, Some(2.0));

    assert_eq!(world.ships[0].physics.slowdown_timer, 1.0 - dt);
}
