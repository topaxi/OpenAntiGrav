//! What the Quake wave's advance, wrap and hit test in [`super`] are asserted
//! to do.
//!
//! Split out under the 200-line cap on inline `#[cfg(test)]` modules; see
//! `scripts/check-file-size.py`.

use super::*;
use crate::test_craft::Ship;
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

const DT: f32 = 1.0 / 60.0;
const RING: f32 = 1000.0;

fn hits() -> [crate::projectile::WeaponHit; crate::MAX_SHIPS] {
    [crate::projectile::WeaponHit::default(); crate::MAX_SHIPS]
}

/// A wave of this `age` whose bump centre sits at course distance 115 (it was
/// launched at 100, moving forward, and `age` is already advanced).
fn wave_aged(age: f32) -> Wave {
    let mut wave = Wave::launch(0, 100.0, 1.0, &stats());
    wave.age = age;
    wave
}

#[test]
fn the_lift_decays_by_one_minus_eight_dt_with_no_wave() {
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(5.0);
    ships[1].quake_lift = 1.0;
    Wave::update_lift(None, &mut ships, 2, RING, DT);
    assert!((ships[1].quake_lift - (1.0 - 8.0 * DT)).abs() < 1e-6);
    assert!((ships[1].quake_lift - 0.866_666_7).abs() < 1e-6);
    assert_eq!(smooth_lift(1.0, 0.0, DT), ships[1].quake_lift);
}

#[test]
fn the_lift_chases_the_raw_height_at_eight_per_second() {
    assert_eq!(smooth_lift(0.0, 12.0, DT), 12.0 * 8.0 * DT);
    assert_eq!(smooth_lift(3.0, 3.0, DT), 3.0);
}

#[test]
fn a_craft_at_the_bump_centre_gets_the_amplitude_of_the_lagged_age() {
    let wave = wave_aged(1.0);
    assert_eq!(wave.raw_lift(115.0, RING, DT), amplitude(1.0 + DT));
}

#[test]
fn the_bump_reaches_half_of_its_half_width_and_no_further() {
    let wave = wave_aged(1.0);
    let reach = half_width(1.0 + DT) * 0.5;
    assert_eq!(
        wave.raw_lift(115.0 + 0.49 * half_width(1.0 + DT), RING, DT),
        amplitude(1.0 + DT)
    );
    // The flat `radius` test of old would have hit here: the law is half of it.
    assert_eq!(
        wave.raw_lift(115.0 + 0.51 * half_width(1.0 + DT), RING, DT),
        0.0
    );
    assert_eq!(wave.raw_lift(115.0 - reach - 0.01, RING, DT), 0.0);
}

#[test]
fn the_bump_is_ahead_of_the_wave_in_its_direction_and_wraps_the_ring() {
    let mut backward = Wave::launch(0, 100.0, -1.0, &stats());
    backward.age = 1.0;
    assert_eq!(backward.raw_lift(85.0, RING, DT), amplitude(1.0 + DT));
    // A bump centred just before the start line reaches a craft just past it.
    let mut wrapping = Wave::launch(0, RING - 20.0, 1.0, &stats());
    wrapping.age = 1.0;
    assert_eq!(wrapping.raw_lift(5.0, RING, DT), amplitude(1.0 + DT));
}

#[test]
fn a_wave_in_flight_lifts_a_craft_under_it_and_no_other() {
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    ships[2] = ship_at(500.0);
    let wave = wave_aged(1.0);
    Wave::update_lift(Some(&wave), &mut ships, 3, RING, DT);
    assert_eq!(
        ships[1].quake_lift,
        smooth_lift(0.0, amplitude(1.0 + DT), DT)
    );
    assert_eq!(ships[2].quake_lift, 0.0);
}

#[test]
fn the_hit_lands_once_on_the_first_tick_the_lift_passes_a_tenth() {
    // At the peak the first tick already gives 12 * 8 / 60 = 1.6: a first-frame hit.
    let mut wave = wave_aged(0.3);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    ships[1].handling.dimensions.shield = 100.0;
    ships[1].physics.shield = 50.0;
    let mut absorbed = hits();
    Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
    assert!(ships[1].quake_lift > LIFT_HIT_THRESHOLD);
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 2.0);
    assert_eq!(ships[1].physics.shield, 45.0);

    // Still above the gate: the latch holds and no second hit is credited.
    Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert_eq!(ships[1].pending_slowdown, 2.0);
    assert_eq!(ships[1].physics.shield, 45.0);
}

#[test]
fn late_in_the_waves_life_the_hit_lands_a_tick_late() {
    // Amplitude 0.5: one tick gives 0.067, below the gate; the second 0.124.
    let mut wave = wave_aged(4.804);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    ships[1].handling.dimensions.shield = 100.0;
    let mut absorbed = hits();
    let mut landed = Vec::new();
    for tick in 0..3 {
        Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
        let before = ships[1].pending_slowdown;
        wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
        if ships[1].pending_slowdown > before {
            landed.push(tick);
        }
    }
    assert_eq!(landed, vec![1]);
}

#[test]
fn under_a_tenth_of_amplitude_the_wave_never_hits() {
    let mut wave = wave_aged(4.99);
    assert!(amplitude(4.99 + DT) < LIFT_HIT_THRESHOLD);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    let mut absorbed = hits();
    for _ in 0..30 {
        Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
        wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    }
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 0.0);
}

#[test]
fn a_lift_of_exactly_a_tenth_is_not_a_hit_and_clears_the_latch() {
    let mut wave = wave_aged(1.0);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(500.0);
    ships[1].handling.dimensions.shield = 100.0;
    let mut absorbed = hits();
    ships[1].quake_lift = 0.11;
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 2.0);

    ships[1].quake_lift = LIFT_HIT_THRESHOLD;
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 2.0);

    // The next rise is a second, independent hit.
    ships[1].quake_lift = 0.11;
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 4.0);
}

#[test]
fn the_firing_crafts_lift_integrates_but_its_latch_never_sets() {
    let mut wave = wave_aged(1.0);
    wave.owner = 1;
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    let mut absorbed = hits();
    Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
    assert!(ships[1].quake_lift > LIFT_HIT_THRESHOLD);
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 0.0);
}

#[test]
fn a_shielded_craft_takes_no_hit_and_absorbed_is_not_set() {
    let mut wave = wave_aged(1.0);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    ships[1] = ship_at(115.0);
    ships[1].physics.shield_pickup_timer = 1.0;
    let mut absorbed = hits();
    Wave::update_lift(Some(&wave), &mut ships, 2, RING, DT);
    wave.apply_hits(&mut ships, 2, DamageRules::default(), &mut absorbed);
    assert!(!wave.hit[1]);
    assert_eq!(ships[1].pending_slowdown, 0.0);
    assert!(!absorbed[1].absorbed);
}

#[test]
fn an_inactive_or_unlocated_ship_is_skipped() {
    let mut wave = wave_aged(1.0);
    let mut ships = [Ship::default(); crate::MAX_SHIPS];
    // Slot 1: active but never located.
    ships[1] = Ship {
        active: true,
        ..Ship::default()
    };
    // Slot 2: located but inactive.
    ships[2] = ship_at(115.0);
    ships[2].active = false;
    let mut absorbed = hits();
    Wave::update_lift(Some(&wave), &mut ships, 3, RING, DT);
    wave.apply_hits(&mut ships, 3, DamageRules::default(), &mut absorbed);
    assert_eq!(ships[1].quake_lift, 0.0);
    assert_eq!(ships[2].quake_lift, 0.0);
    assert!(!wave.hit[1]);
    assert!(!wave.hit[2]);
}

#[test]
fn the_envelope_rises_for_three_tenths_of_a_second_and_is_gone_at_five() {
    assert_eq!(amplitude(0.0), 0.0);
    assert!((amplitude(0.15) - 6.0).abs() < 1e-5);
    assert!((amplitude(RISE_SECONDS) - PEAK_AMPLITUDE).abs() < 1e-5);
    assert!((amplitude(2.65) - 6.0).abs() < 1e-4);
    assert_eq!(amplitude(LIFETIME_SECONDS), 0.0);
    assert_eq!(amplitude(6.0), 0.0);
    assert_eq!(amplitude(-0.1), 0.0);
}

#[test]
fn the_bump_widens_from_25_to_75_units() {
    assert_eq!(half_width(0.0), 25.0);
    assert_eq!(half_width(LIFETIME_SECONDS), 75.0);
}
