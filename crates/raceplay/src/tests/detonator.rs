//! The lap survey Detonator's placement reads: lanes across the road, the ground under each,
//! and the start zone.

use super::*;
use oag_physics::{Surface, TriangleSoup};

/// A strip of floor `x` in `-10..=80`, `z` in `-5..=5`, so the lanes past `z = 5` have no ground.
fn strip() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-10.0, 0.0, -5.0],
            [-10.0, 0.0, 5.0],
            [80.0, 0.0, 5.0],
            [80.0, 0.0, -5.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

fn survey_of(collision: &CollisionWorld) -> oag_weapons::detonator::Survey {
    let spline = Spline::from_track(&straight_track());
    let order: Vec<u32> = (0..spline.len() as u32).collect();
    crate::detonator::survey(
        &spline,
        &order,
        None,
        collision,
        &oag_tables::detonator_layout::GamePlayValues::default(),
    )
}

#[test]
fn one_entry_is_laid_out_per_control_point() {
    assert_eq!(survey_of(&strip()).entries.len(), 8);
}

#[test]
fn a_road_twenty_four_wide_fans_five_lanes_about_a_centre_off_the_middle() {
    let entry = survey_of(&strip()).entries[2];
    assert_eq!(entry.lanes, 5, "trunc(0.9 * 24 / 3.5) = 6, made odd");
    // `start - lateral * 12 + lateral * 0.5 * 21.6`.
    assert!((entry.centre.z - -1.2).abs() < 1e-3, "{:?}", entry.centre);
    assert!((entry.centre.x - 20.0).abs() < 1e-3);
    assert_eq!(entry.up, Vec3::Y);
    assert_eq!(entry.lateral, Vec3::Z);
}

#[test]
fn a_lane_with_no_ground_under_it_is_blocked() {
    // Lanes sit at z = -1.2 + 3.5 * j for j = -2..=2: -8.2 and 5.8 are off the strip.
    let entry = survey_of(&strip()).entries[2];
    assert_eq!(entry.blocked, 0b10001);
}

#[test]
fn a_short_straight_is_all_start_zone() {
    // The whole fixture lies within 150 of its first entry.
    assert!(survey_of(&strip()).entries.iter().all(|e| !e.usable));
}

#[test]
fn a_track_with_no_ground_at_all_is_unusable_everywhere() {
    // Every centre probe finds no ground: a jump gap from the first entry on.
    let survey = survey_of(&CollisionWorld::new());
    assert!(survey.entries.iter().all(|e| !e.usable));
}
