use super::*;

fn at(x: f32) -> Vec3 {
    Vec3::new(x, 0.0, 0.0)
}

/// Lays a crumb every 25 units along `+x`, fully grounded, on a record with no
/// flag: one push each.
fn laid(count: usize) -> Breadcrumbs {
    let mut crumbs = Breadcrumbs::seeded(at(0.0));
    for step in 1..=count {
        crumbs.lay(at(step as f32 * 25.0), 80.0, 1.0, 0);
    }
    crumbs
}

#[test]
fn a_reset_reads_the_oldest_of_seven_crumbs() {
    // Ten pushes after the seed: eleven laid, and the ring keeps the last
    // seven, so the oldest is the fifth laid (x = 100).
    let mut crumbs = laid(10);
    assert_eq!(crumbs.newest(), at(250.0));
    assert_eq!(crumbs.take_oldest(), at(100.0));
}

#[test]
fn a_young_ring_resets_to_the_grid() {
    let mut crumbs = laid(3);
    assert_eq!(crumbs.take_oldest(), at(0.0));
}

#[test]
fn a_reset_forgets_every_newer_crumb() {
    let mut crumbs = laid(10);
    let first = crumbs.take_oldest();
    // `FUN_0883ef30` moves the head to just past the oldest: the crumb it
    // returned is now the newest, and a second reset with nothing laid
    // between comes back to the same place.
    assert_eq!(crumbs.newest(), first);
    assert_eq!(crumbs.take_oldest(), first);
}

#[test]
fn a_crumb_needs_twenty_units_on_one_axis() {
    let mut crumbs = Breadcrumbs::seeded(at(0.0));
    // 20 on x and y and z is not past 20 on any of them.
    crumbs.lay(Vec3::new(20.0, 20.0, -20.0), 80.0, 1.0, 0);
    assert_eq!(crumbs.newest(), at(0.0));
    crumbs.lay(Vec3::new(0.0, 0.0, 20.5), 80.0, 1.0, 0);
    assert_eq!(crumbs.newest(), Vec3::new(0.0, 0.0, 20.5));
}

#[test]
fn no_crumb_on_a_flagged_record_or_off_the_ground() {
    let mut crumbs = Breadcrumbs::seeded(at(0.0));
    crumbs.lay(at(50.0), 80.0, 1.0, NO_CRUMB_FLAG);
    crumbs.lay(at(50.0), 80.0, 0.5, 0);
    crumbs.lay(at(50.0), 80.0, 0.0, 0);
    assert_eq!(crumbs.newest(), at(0.0));
    // Other bits of `flags` do not gate it.
    crumbs.lay(at(50.0), 80.0, 1.0, 0x02);
    assert_eq!(crumbs.newest(), at(50.0));
}

#[test]
fn the_launch_speed_latches_on_every_fully_grounded_tick() {
    let mut crumbs = Breadcrumbs::seeded(at(0.0));
    assert_eq!(crumbs.launch_speed(), 50.0);
    // Latched even where no crumb may be laid: the latch is outside
    // `FUN_0883d9ec`.
    crumbs.lay(at(1.0), 92.0, 1.0, NO_CRUMB_FLAG);
    assert_eq!(crumbs.launch_speed(), 92.0 * 0.25 + 50.0);
    // Not on a half contact or in the air.
    crumbs.lay(at(2.0), 30.0, 0.5, 0);
    crumbs.lay(at(3.0), 10.0, 0.0, 0);
    assert_eq!(crumbs.launch_speed(), 92.0 * 0.25 + 50.0);
}

#[test]
fn the_relocation_is_the_width_midpoint_five_up_launched_down_the_track() {
    // Down is -y and lateral +z, so the lift is +3 y and the midpoint sits
    // (right - left) / 2 = 4 units toward +z.
    let spline = Spline::from_track(&crate::tests::straight_track_with_widths(4.0, 12.0));
    let relocation = relocate(&spline, Vec3::new(20.0, 4.0, 0.0), 73.0).expect("a spline");
    let position = relocation.pose.position;
    assert!((position.y - 8.0).abs() < 1e-4, "{position:?}");
    assert!((position.z - 4.0).abs() < 1e-4, "{position:?}");
    assert!((position.x - 20.0).abs() < 2.0, "{position:?}");
    let forward = relocation.pose.orientation * Vec3::NEG_Z;
    assert!((relocation.velocity - forward * 73.0).length() < 1e-4);
    assert!(forward.x > 0.99, "faces down the track: {forward:?}");
}
