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
    race.view.hd_trail_active = true;
    race.view.hull_reach = [2.0; oag_gameplay::MAX_SHIPS];
    for k in 0..oag_fx::exhaust::hd::SAMPLES {
        race.view.hd_trail[0].push(Vec3::new(k as f32 * -4.0, 0.0, 0.0), Vec3::Y, 0.0);
    }
    race
}

/// Sits slot 1 at `at` and runs one tick of the trigger.
fn place_and_step(race: &mut Race, at: Vec3) {
    race.sim.world.ships[1].physics.body.position = at;
    race.advance_trail_hits();
}

#[test]
fn a_craft_inside_the_ribbon_is_marked_and_one_outside_is_not() {
    let mut race = race_with_a_trail();
    // Well clear: 20 units off a ribbon of half-width 0.5 plus a 2.0 hull.
    place_and_step(&mut race, Vec3::new(-40.0, 20.0, 0.0));
    assert_eq!(race.view.trail_inside[0] & 0b10, 0);
    // On the line, between two stored samples - the case a sample-wise test
    // would miss and the segment-wise one catches.
    place_and_step(&mut race, Vec3::new(-42.0, 0.0, 0.0));
    assert_ne!(race.view.trail_inside[0] & 0b10, 0);
}

/// The **fallback** only - a hull that authors no `Ship Collision Fx` locators,
/// which no HD craft is. The path every real race takes is the anchor test
/// below.
#[test]
fn the_fallback_reach_is_the_hull_reach_plus_the_measured_half_width() {
    let mut race = race_with_a_trail();
    let reach = 2.0 + oag_fx::exhaust::hd::FIN_HALF_WIDTH;
    place_and_step(&mut race, Vec3::new(-40.0, reach - 0.01, 0.0));
    assert_ne!(race.view.trail_inside[0] & 0b10, 0, "just inside");
    place_and_step(&mut race, Vec3::new(-40.0, reach + 0.01, 0.0));
    assert_eq!(race.view.trail_inside[0] & 0b10, 0, "just outside");
}

/// **The rate, pinned.** The burst follows the mask every tick rather than its
/// edge, which the asset is what settles: `WO_TRAIL_HITSHIP` is a one-shot -
/// duration 1 tick, `looping` false, five 0.2..0.5-unit streaks whose size is
/// down to a third by 17 % of its life. Fired once per entry that is all but
/// invisible, and this engine did exactly that until it was measured: 6 changed
/// pixels in a 1,175,040-pixel frame, against 7,592 once the rate was fixed.
///
/// So what this asserts is that the mask is *level*, not edge: set on every
/// tick a craft is inside and cleared the tick it leaves.
#[test]
fn the_mask_tracks_presence_every_tick_rather_than_only_its_edge() {
    let mut race = race_with_a_trail();
    place_and_step(&mut race, Vec3::new(-40.0, 0.0, 0.0));
    assert_ne!(race.view.trail_inside[0] & 0b10, 0, "entered");
    place_and_step(&mut race, Vec3::new(-36.0, 0.0, 0.0));
    assert_ne!(
        race.view.trail_inside[0] & 0b10,
        0,
        "still inside, still set"
    );
    place_and_step(&mut race, Vec3::new(-36.0, 50.0, 0.0));
    assert_eq!(race.view.trail_inside[0] & 0b10, 0, "left, so cleared");
    place_and_step(&mut race, Vec3::new(-36.0, 0.0, 0.0));
    assert_ne!(race.view.trail_inside[0] & 0b10, 0, "back inside");
}

/// A craft is never in its own trail, however close the ring is - its nozzle
/// *is* the head sample, so a self-test would latch on tick one and never let
/// go.
#[test]
fn a_craft_is_never_inside_its_own_trail() {
    let mut race = race_with_a_trail();
    race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
    race.advance_trail_hits();
    assert_eq!(race.view.trail_inside[0] & 0b1, 0);
}

/// Off HD the whole path is inert, so no other title pays for it.
#[test]
fn a_non_hd_race_tests_nothing() {
    let mut race = race_with_a_trail();
    race.view.hd_trail_active = false;
    place_and_step(&mut race, Vec3::new(-40.0, 0.0, 0.0));
    assert_eq!(race.view.trail_inside[0], 0);
}

/// **The burst sits on the hull, at `reach` from the centre - never on the
/// ribbon.** Got wrong twice: first by spawning at the contact point outright,
/// then by clamping to `min(distance, reach)`, which is the same thing whenever
/// the burst actually fires. So the property is asserted directly.
#[test]
fn the_burst_sits_on_the_hull_facing_the_contact() {
    use crate::effects::hull_contact_point;
    let centre = Vec3::new(10.0, 0.0, 0.0);
    // A contact *inside* the reach - the case every real firing is, and the one
    // the clamp got wrong.
    let inside = hull_contact_point(centre, Vec3::new(11.0, 0.0, 0.0), 4.0);
    assert!(
        ((inside - centre).length() - 4.0).abs() < 1e-5,
        "{inside:?} is not on the hull sphere"
    );
    assert!(inside.x > centre.x, "and it faces the contact");
    // A contact outside it lands on the same sphere, not at the contact.
    let outside = hull_contact_point(centre, Vec3::new(10.0, 40.0, 0.0), 4.0);
    assert!(
        ((outside - centre).length() - 4.0).abs() < 1e-5,
        "{outside:?}"
    );
    // Degenerate: a contact at the centre has no direction, so the centre it is.
    assert_eq!(hull_contact_point(centre, centre, 4.0), centre);
}

/// **The test a real race takes: the hull's own authored points, not a sphere.**
///
/// A player reported the sparks firing before the craft touched the trail. The
/// sphere is `hull_reach` wide - about half a hull *length* - so it reaches
/// past a hull side long before the hull does. An anchor a hull-width off the
/// ribbon must not fire; the same anchor on it must.
#[test]
fn an_authored_anchor_decides_the_hit_and_the_sphere_does_not() {
    let mut race = race_with_a_trail();
    // One anchor, on the craft's centreline. The ribbon runs along -X at y = 0.
    race.view.spark_anchors = vec![Vec::new(); oag_gameplay::MAX_SHIPS];
    race.view.spark_anchors[1] = vec![Vec3::ZERO];
    // Two units off the ribbon: inside the 2.0 + 0.5 sphere the fallback would
    // use, and well outside the ribbon's own 0.5 half-width.
    place_and_step(&mut race, Vec3::new(-40.0, 2.0, 0.0));
    assert_eq!(
        race.view.trail_inside[0] & 0b10,
        0,
        "the sphere would have fired here and the hull has not touched anything"
    );
    // On it, and it fires.
    place_and_step(&mut race, Vec3::new(-40.0, 0.2, 0.0));
    assert_ne!(race.view.trail_inside[0] & 0b10, 0);
}
