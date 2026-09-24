//! What the Quake wave's advance, wrap and hit test in [`super`] are asserted
//! to do.
//!
//! Split out under the 200-line cap on inline `#[cfg(test)]` modules; see
//! `scripts/check-file-size.py`.

use super::*;
use crate::world::Ship;
use oag_physics::DamageRules;

fn stats() -> QuakeStats {
    QuakeStats {
        absorb: 0.0,
        damage: 5.0,
        radius: 10.0,
        slowdown_time: 2.0,
    }
}

fn ship_at(progress: f32) -> Ship {
    let mut ship = Ship {
        active: true,
        ..Ship::default()
    };
    ship.standing.progress = Some(progress);
    ship
}

#[test]
fn a_wave_launches_at_the_firing_crafts_own_progress() {
    let wave = Wave::launch(0, 42.0, 1.0, &stats());
    assert_eq!(wave.progress, 42.0);
    assert_eq!(wave.owner, 0);
    assert_eq!(wave.damage, 5.0);
    assert_eq!(wave.radius, 10.0);
    assert_eq!(wave.slowdown_time, 2.0);
    assert!(wave.hit.iter().all(|&h| !h));
}

#[test]
fn the_direction_sign_follows_the_dot_product() {
    assert_eq!(Wave::launch(0, 0.0, 1.0, &stats()).direction, 1.0);
    assert_eq!(Wave::launch(0, 0.0, -1.0, &stats()).direction, -1.0);
    // A perpendicular forward (dot == 0.0) takes the positive direction
    // rather than stalling - chosen, not measured, see `Wave::launch`'s doc
    // comment.
    assert_eq!(Wave::launch(0, 0.0, 0.0, &stats()).direction, 1.0);
}

#[test]
fn the_wave_advances_at_the_recovered_speed_and_wraps() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    wave.advance(1000.0, 1.0);
    assert_eq!(wave.progress, SPEED_UNITS_PER_SECOND);

    // Backward direction subtracts, and a negative result wraps into
    // `0.0..length` rather than going negative.
    let mut backward = Wave::launch(0, 10.0, -1.0, &stats());
    backward.advance(1000.0, 1.0);
    assert_eq!(backward.progress, 1000.0 + 10.0 - SPEED_UNITS_PER_SECOND);
}

#[test]
fn advancing_a_degenerate_course_is_a_no_op() {
    let mut wave = Wave::launch(0, 5.0, 1.0, &stats());
    wave.advance(0.0, 1.0);
    assert_eq!(wave.progress, 5.0);
    wave.advance(-1.0, 1.0);
    assert_eq!(wave.progress, 5.0);

    // The *age* is not a no-op, on purpose: a wave that cannot move still has
    // to expire, or the once-per-race guard never re-opens. See
    // `Wave::advance`'s own doc comment.
    assert_eq!(wave.age, 2.0);
}

#[test]
fn a_wave_expires_after_the_recovered_lifetime_and_not_before() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    assert!(!wave.expired(), "a wave is not born expired");

    // One tick short of the bound, at the tick rate the simulation actually
    // runs at, is still alive.
    let dt = 1.0 / 60.0;
    let ticks = (LIFETIME_SECONDS / dt) as u32;
    for _ in 0..ticks {
        wave.advance(1000.0, dt);
    }
    assert!(
        !wave.expired(),
        "expired at {:.4}s, inside the recovered {LIFETIME_SECONDS}s",
        wave.age
    );

    wave.advance(1000.0, dt);
    assert!(
        wave.expired(),
        "still alive at {:.4}s, past the recovered {LIFETIME_SECONDS}s",
        wave.age
    );
}

#[test]
fn a_live_wave_outlives_a_full_lap_of_a_short_ring_but_not_the_bound() {
    // The defect this bound closes: on a ring shorter than
    // `SPEED_UNITS_PER_SECOND * LIFETIME_SECONDS` the wave gets all the way
    // round, and before the bound existed it kept going for the whole race,
    // re-hitting the field once a lap.
    let length = SPEED_UNITS_PER_SECOND * LIFETIME_SECONDS / 2.0;
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let dt = 1.0 / 60.0;
    let mut ticks = 0u32;
    while !wave.expired() {
        wave.advance(length, dt);
        ticks += 1;
        assert!(ticks < 10_000, "the wave never expired");
    }
    // Two laps' worth of travel, then gone - not an unbounded circuit.
    assert!((wave.age - (f64::from(ticks) * f64::from(dt)) as f32).abs() < 1e-3);
    assert!(wave.age > LIFETIME_SECONDS);
}

#[test]
fn progress_delta_takes_the_short_way_round_the_ring() {
    let wave = Wave::launch(0, 5.0, 1.0, &stats());
    // Straight-line distance, no wrap needed.
    assert_eq!(wave.progress_delta(15.0, 1000.0), 10.0);
    // The other way round the ring is shorter for a craft just past the
    // start line from a wave that is just before it.
    let near_start = Wave::launch(0, 2.0, 1.0, &stats());
    assert_eq!(near_start.progress_delta(998.0, 1000.0), 4.0);
}

#[test]
fn a_craft_inside_radius_takes_the_hit_exactly_once() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let mut ships = [Ship::default(); crate::world::MAX_SHIPS];
    ships[1] = ship_at(5.0);
    ships[1].physics.shield = 50.0;
    ships[1].handling.dimensions.shield = 100.0;
    let mut absorbed = [crate::projectile::WeaponHit::default(); crate::world::MAX_SHIPS];

    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 2.0);
    assert_eq!(ships[1].physics.shield, 45.0);

    // A second tick still inside radius: the latch holds, and no second
    // hit is credited.
    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert_eq!(ships[1].pending_slowdown, 2.0);
    assert_eq!(ships[1].physics.shield, 45.0);
}

#[test]
fn a_craft_leaving_radius_clears_the_latch_so_a_second_pass_hits_again() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let mut ships = [Ship::default(); crate::world::MAX_SHIPS];
    ships[1] = ship_at(5.0);
    ships[1].handling.dimensions.shield = 100.0;
    let mut absorbed = [crate::projectile::WeaponHit::default(); crate::world::MAX_SHIPS];
    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);

    // The craft (or the wave) moves out of radius: the latch clears.
    ships[1].standing.progress = Some(500.0);
    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 2.0);

    // Back inside: a second, independent hit.
    ships[1].standing.progress = Some(5.0);
    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 4.0);
}

#[test]
fn the_firing_craft_is_excluded_outright() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let mut ships = [Ship::default(); crate::world::MAX_SHIPS];
    ships[0] = ship_at(0.0);
    let mut absorbed = [crate::projectile::WeaponHit::default(); crate::world::MAX_SHIPS];
    wave.apply_hits(&mut ships, 1, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[0]);
    assert_eq!(ships[0].pending_slowdown, 0.0);
}

#[test]
fn a_shielded_craft_takes_no_hit_and_absorbed_is_not_set() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let mut ships = [Ship::default(); crate::world::MAX_SHIPS];
    ships[1] = ship_at(5.0);
    ships[1].physics.shield_pickup_timer = 1.0;
    let mut absorbed = [crate::projectile::WeaponHit::default(); crate::world::MAX_SHIPS];
    wave.apply_hits(&mut ships, 2, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 0.0);
    assert!(!absorbed[1].absorbed);
}

#[test]
fn an_inactive_or_unlocated_ship_is_skipped() {
    let mut wave = Wave::launch(0, 0.0, 1.0, &stats());
    let mut ships = [Ship::default(); crate::world::MAX_SHIPS];
    // Slot 1: active but never located.
    ships[1] = Ship {
        active: true,
        ..Ship::default()
    };
    // Slot 2: located but inactive.
    ships[2] = ship_at(5.0);
    ships[2].active = false;
    let mut absorbed = [crate::projectile::WeaponHit::default(); crate::world::MAX_SHIPS];
    wave.apply_hits(&mut ships, 3, 1000.0, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert!(!wave.hit[2]);
}
