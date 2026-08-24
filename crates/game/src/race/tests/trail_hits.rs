//! The trail-hit trigger's state machine: `Race::advance_trail_hits`.
//!
//! **What is measured and what is not.** That a craft flying through an engine
//! trail sparks is read out of `EBOOT.elf` - the flag, the consumer, the
//! spawner and the colour select are all on
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`. The *test geometry*
//! and the *rate* are this engine's approximations, which is exactly why they
//! are pinned here: an approximation nobody can see change is one that drifts.
//!
//! The ignition itself needs a `.pob`, so it is exercised by the disc-backed
//! `hd_engine_flare_ground_truth.rs` instead; what these assert is the part
//! that decides *when*.

use super::*;

/// A grid whose trails are HD's, with slot 0's ribbon laid along +X.
fn race_with_a_trail() -> Race {
    let mut race = race_with_a_grid();
    race.hd_trail_active = true;
    race.hull_radius = [2.0; oag_gameplay::MAX_SHIPS];
    for k in 0..oag_render::exhaust::hd::SAMPLES {
        race.hd_trail[0].push(Vec3::new(k as f32 * -4.0, 0.0, 0.0), Vec3::Y, 0.0);
    }
    race
}

/// Sits slot 1 at `at` and runs one tick of the trigger.
fn place_and_step(race: &mut Race, at: Vec3) {
    race.world.ships[1].physics.body.position = at;
    race.advance_trail_hits();
}

#[test]
fn a_craft_inside_the_ribbon_is_marked_and_one_outside_is_not() {
    let mut race = race_with_a_trail();
    // Well clear: 20 units off a ribbon of half-width 0.5 plus a 2.0 hull.
    place_and_step(&mut race, Vec3::new(-40.0, 20.0, 0.0));
    assert_eq!(race.trail_inside[0] & 0b10, 0);
    // On the line, between two stored samples - the case a sample-wise test
    // would miss and the segment-wise one catches.
    place_and_step(&mut race, Vec3::new(-42.0, 0.0, 0.0));
    assert_ne!(race.trail_inside[0] & 0b10, 0);
}

#[test]
fn the_reach_is_the_hull_radius_plus_the_measured_half_width() {
    let mut race = race_with_a_trail();
    let reach = 2.0 + oag_render::exhaust::hd::FIN_HALF_WIDTH;
    place_and_step(&mut race, Vec3::new(-40.0, reach - 0.01, 0.0));
    assert_ne!(race.trail_inside[0] & 0b10, 0, "just inside");
    place_and_step(&mut race, Vec3::new(-40.0, reach + 0.01, 0.0));
    assert_eq!(race.trail_inside[0] & 0b10, 0, "just outside");
}

/// **The rate approximation, pinned.** The original clears its flag every
/// frame and whether the SPU job re-raises it while a craft stays inside is
/// unread; firing per tick would be 60 ignitions a second. So this engine
/// fires on entry and re-arms on exit, and the bit is what says so.
#[test]
fn staying_inside_does_not_re_arm_but_leaving_does() {
    let mut race = race_with_a_trail();
    place_and_step(&mut race, Vec3::new(-40.0, 0.0, 0.0));
    assert_ne!(race.trail_inside[0] & 0b10, 0, "entered");
    place_and_step(&mut race, Vec3::new(-36.0, 0.0, 0.0));
    assert_ne!(race.trail_inside[0] & 0b10, 0, "still inside, still marked");
    place_and_step(&mut race, Vec3::new(-36.0, 50.0, 0.0));
    assert_eq!(race.trail_inside[0] & 0b10, 0, "left, so re-armed");
    place_and_step(&mut race, Vec3::new(-36.0, 0.0, 0.0));
    assert_ne!(race.trail_inside[0] & 0b10, 0, "entered again");
}

/// A craft is never in its own trail, however close the ring is - its nozzle
/// *is* the head sample, so a self-test would latch on tick one and never let
/// go.
#[test]
fn a_craft_is_never_inside_its_own_trail() {
    let mut race = race_with_a_trail();
    race.world.ships[0].physics.body.position = Vec3::ZERO;
    race.advance_trail_hits();
    assert_eq!(race.trail_inside[0] & 0b1, 0);
}

/// Off HD the whole path is inert, so no other title pays for it.
#[test]
fn a_non_hd_race_tests_nothing() {
    let mut race = race_with_a_trail();
    race.hd_trail_active = false;
    place_and_step(&mut race, Vec3::new(-40.0, 0.0, 0.0));
    assert_eq!(race.trail_inside[0], 0);
}
