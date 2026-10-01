use super::*;
use crate::beam::{Frame, Ribbon, build, segment_count};
use oag_core::Rng;

/// A road along `-Z`, `10` either side of `x = 0`, its surface at `y = 0`
/// (`down` is `-Y`: on level ground the authored axis is `(0, -1, 0)`).
fn road() -> TubeFrame {
    TubeFrame {
        pos: Vec3::ZERO,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        half_width_left: 10.0,
        half_width_right: 10.0,
    }
}

/// The same road stretching the whole way: the frame under any point sits at
/// that point's own `z`.
fn straight_road(point: Vec3) -> Option<TubeFrame> {
    Some(TubeFrame {
        pos: Vec3::new(0.0, 0.0, point.z),
        ..road()
    })
}

fn kept(mut point: Vec3, target: Vec3, remaining: u32) -> (Vec3, Vec3) {
    let mut step = (target - point) / (remaining.max(1) as f32);
    keep_in_track(&road(), &mut point, &mut step, target, remaining);
    (point, step)
}

#[test]
fn a_point_inside_the_tube_is_left_alone() {
    let target = Vec3::new(0.0, 0.0, -50.0);
    let inside = Vec3::new(3.0, 1.0, -5.0);
    let mut point = inside;
    let mut step = Vec3::new(0.0, 0.0, -1.0);
    keep_in_track(&road(), &mut point, &mut step, target, 4);
    assert_eq!(point, inside);
    assert_eq!(step, Vec3::new(0.0, 0.0, -1.0));
}

#[test]
fn a_point_below_the_road_is_projected_onto_its_plane() {
    let (point, _) = kept(Vec3::new(2.0, -3.0, -5.0), Vec3::new(0.0, 0.0, -50.0), 4);
    assert_eq!(point, Vec3::new(2.0, 0.0, -5.0));
}

#[test]
fn a_point_above_the_road_is_not_touched() {
    // `KeepInTrack` tests the floor only: there is no ceiling.
    let above = Vec3::new(2.0, 40.0, -5.0);
    let (point, _) = kept(above, Vec3::new(0.0, 0.0, -50.0), 4);
    assert_eq!(point, above);
}

#[test]
fn the_right_wall_is_two_units_inside_the_edge() {
    let (point, _) = kept(Vec3::new(9.5, 0.0, 0.0), Vec3::new(0.0, 0.0, -50.0), 4);
    assert!((point.x - 8.0).abs() < 1e-5, "{point}");
    // Exactly on the wall is not past it.
    let (point, _) = kept(Vec3::new(8.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -50.0), 4);
    assert_eq!(point.x, 8.0);
}

#[test]
fn the_left_wall_uses_the_left_half_width() {
    let mut narrow_left = road();
    narrow_left.half_width_left = 4.0;
    let target = Vec3::new(0.0, 0.0, -50.0);
    let mut point = Vec3::new(-3.0, 0.0, 0.0);
    let mut step = Vec3::ZERO;
    keep_in_track(&narrow_left, &mut point, &mut step, target, 4);
    assert!((point.x + 2.0).abs() < 1e-5, "{point}");
    // The right half-width is still 10: the same point on the right is fine.
    let mut point = Vec3::new(3.0, 0.0, 0.0);
    keep_in_track(&narrow_left, &mut point, &mut step, target, 4);
    assert_eq!(point.x, 3.0);
}

#[test]
fn a_wall_pulls_towards_the_centre_line_not_along_lateral() {
    // Off the centre line in `y` and `z` too: the pull runs along
    // `normalize(pos - point)`, so the point moves in `y` as well.
    let outside = Vec3::new(12.0, 2.0, 0.0);
    let (point, _) = kept(outside, Vec3::new(0.0, 0.0, -50.0), 4);
    // It travels `|scale| = 4` along that slanted direction, so it lands a
    // little short of the wall plane rather than on it.
    let expected = outside + (-outside).normalize() * 4.0;
    assert!((point - expected).length() < 1e-4, "{point} vs {expected}");
    assert!(point.x > 8.0 && point.x < 12.0, "{point}");
    assert!(point.y < 2.0 && point.y > 0.0, "{point}");
}

#[test]
fn the_step_is_measured_from_the_point_displaced_the_other_way() {
    // The original's quirk, pinned: after one re-aim the chain ends
    // `2 * scale * dir` off the target, not on it.
    let target = Vec3::new(0.0, 0.0, -50.0);
    let remaining = 4;
    let (point, step) = kept(Vec3::new(2.0, -3.0, -5.0), target, remaining);
    let end = point + step * remaining as f32;
    // scale = -3, dir = -down = +Y: scaled = (0, -3, 0).
    let expected = target - Vec3::new(0.0, -6.0, 0.0);
    assert!((end - expected).length() < 1e-4, "{end} vs {expected}");
    assert!((end - target).length() > 5.0);
}

#[test]
fn the_last_point_takes_its_step_undivided() {
    let target = Vec3::new(0.0, 0.0, -50.0);
    let below = Vec3::new(2.0, -3.0, -5.0);
    let (_, step) = kept(below, target, 0);
    assert_eq!(step, target - (below + Vec3::new(0.0, -3.0, 0.0)));
}

#[test]
fn a_missed_locate_leaves_the_straight_line() {
    let owner = Vec3::new(1.0, 2.0, 3.0);
    let target = Vec3::new(31.0, 2.0, -57.0);
    let bases = walk(owner, target, 6, &|_| None);
    assert_eq!(bases.len(), 7);
    let step = (target - owner) / 6.0;
    let mut expected = owner;
    for base in &bases[1..] {
        expected += step;
        assert_eq!(*base, expected);
    }
}

#[test]
fn a_wide_straight_road_changes_nothing() {
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(0.0, 0.0, -60.0);
    assert_eq!(
        walk(owner, target, 6, &straight_road),
        walk(owner, target, 6, &|_| None),
    );
}

#[test]
fn a_chain_aimed_past_the_wall_stays_inside_it() {
    let owner = Vec3::ZERO;
    // Well outside the road: the straight line leaves it at the third point.
    let target = Vec3::new(40.0, 0.0, -60.0);
    let straight = walk(owner, target, 6, &|_| None);
    assert!(straight[4].x > 8.0, "the straight line leaves the road");
    let bent = walk(owner, target, 6, &straight_road);
    for (i, base) in bent.iter().enumerate() {
        assert!(
            base.x <= 8.0 + 1e-3,
            "point {i} sits outside the wall: {base}"
        );
    }
    assert_ne!(bent, straight);
}

fn frame(target: Vec3) -> Frame {
    Frame {
        owner: Vec3::ZERO,
        target,
        owner_right: Vec3::X,
        owner_up: Vec3::Y,
        camera_right: Vec3::X,
        camera_up: Vec3::Y,
        range: 250.0,
        alpha: 1.0,
    }
}

#[test]
fn the_drawn_ribbon_bends_with_the_tube() {
    let mut rng = Rng::new(1);
    let mut ribbon = Ribbon::new(&mut rng);
    ribbon.advance(1.0 / 60.0, 70.0, 250.0, &mut rng);
    let f = frame(Vec3::new(40.0, 0.0, -60.0));
    let straight = build(&ribbon, &f, &|_| None);
    let bent = build(&ribbon, &f, &straight_road);
    assert_eq!(straight.len(), bent.len());
    assert_ne!(
        bytemuck::cast_slice::<_, u8>(&straight),
        bytemuck::cast_slice::<_, u8>(&bent),
    );
    // Both strips start at the same shooter origin.
    assert_eq!(straight[0].position, bent[0].position);
}

#[test]
fn the_energy_point_follows_the_bend() {
    let mut rng = Rng::new(1);
    let mut ribbon = Ribbon::new(&mut rng);
    let target = Vec3::new(40.0, 0.0, -60.0);
    let distance = target.length();
    ribbon.advance(1.0 / 60.0, distance, 250.0, &mut rng);
    let segments = segment_count(distance, 250.0);
    assert!(segments > 4);
    let point = ribbon
        .energy_point(Vec3::ZERO, target, 250.0, &straight_road)
        .expect("cursor at zero");
    assert!(point.x <= 8.0 + 1e-3, "{point}");
    let straight = ribbon
        .energy_point(Vec3::ZERO, target, 250.0, &|_| None)
        .unwrap();
    assert!(straight.x > 8.0, "{straight}");
}
