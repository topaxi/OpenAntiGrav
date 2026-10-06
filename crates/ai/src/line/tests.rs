use super::*;

/// A unit-ish circle, counter-clockwise in the XZ plane.
fn circle(radius: f32, points: usize) -> Line {
    Line::new(
        (0..points)
            .map(|step| {
                let angle = std::f32::consts::TAU * step as f32 / points as f32;
                Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
            })
            .collect(),
    )
}

#[test]
fn an_empty_line_answers_everything_with_the_origin() {
    let line = Line::default();
    assert!(line.is_empty());
    assert_eq!(line.point(7), Vec3::ZERO);
    assert_eq!(line.nearest(Vec3::ONE, 3, 20), 0);
    assert_eq!(line.ahead(0, 50.0), (0, Vec3::ZERO, 0.0));
    assert_eq!(line.curvature(0, 10.0), 0.0);
}

#[test]
fn indices_wrap() {
    let line = circle(10.0, 8);
    assert_eq!(line.point(0), line.point(8));
    assert_eq!(line.point(3), line.point(11));
}

#[test]
fn nearest_finds_the_point_under_a_position() {
    let line = circle(100.0, 64);
    let target = line.point(20);
    assert_eq!(line.nearest(target, 18, 16), 20);
}

/// The window is the whole point: a position on the far side of the ring is
/// *not* found from here, and that is what stops a progress index jumping
/// across a track that passes near itself.
#[test]
fn nearest_does_not_look_across_the_ring() {
    let line = circle(100.0, 64);
    let opposite = line.point(40);
    assert_ne!(line.nearest(opposite, 0, 8), 40);
}

#[test]
fn ahead_walks_the_requested_distance() {
    let line = Line::new(vec![
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 20.0),
        Vec3::new(0.0, 0.0, 30.0),
    ]);
    // Interpolated: 15 units along is halfway between points 1 and 2.
    let (index, point, travelled) = line.ahead(0, 15.0);
    assert_eq!(index, 1);
    assert!((point.z - 15.0).abs() < 1e-4, "point {point:?}");
    assert!((travelled - 15.0).abs() < 1e-4, "travelled {travelled}");
    assert_eq!(line.ahead(0, 0.0).0, 0);
}

/// A tighter circle bends harder. The absolute value depends on the sampling,
/// so the ordering is what is asserted.
#[test]
fn a_tighter_circle_reads_as_more_curved() {
    let tight = circle(50.0, 64);
    let wide = circle(400.0, 64);
    assert!(tight.curvature(0, 10.0) > wide.curvature(0, 10.0));
}

/// Four points a fixed distance apart, with a corridor that widens along
/// them, so an interpolated bound is distinguishable from a snapped one.
fn widening() -> Line {
    let points: Vec<Vec3> = (0..4)
        .map(|step| Vec3::new(0.0, 0.0, 10.0 * step as f32))
        .collect();
    let corridor = (0..4)
        .map(|step| Frame {
            lateral: Vec3::X,
            left: -(step as f32),
            right: 1.0 * step as f32,
        })
        .collect();
    Line::with_corridor(points, corridor)
}

#[test]
fn a_line_can_have_no_corridor() {
    let line = Line::new(vec![Vec3::ZERO, Vec3::X]);
    assert!(!line.has_corridor());
    assert_eq!(line.aim(0, 0.5).corridor, None);
}

/// A corridor not matching the points is dropped, not half-used: a truncated
/// one would silently give the back of the track no room.
#[test]
fn a_mismatched_corridor_is_dropped() {
    let line = Line::with_corridor(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        vec![Frame::default(), Frame::default()],
    );
    assert!(!line.has_corridor());
}

/// **The bound is interpolated onto the same segment fraction as the aim
/// point**, not snapped: snapped, it steps a sample's worth each crossing and
/// the driver's drift clamp puts that step into the commanded turn rate.
#[test]
fn the_corridor_is_interpolated_rather_than_snapped() {
    let line = widening();
    let frame = line.aim(0, 15.0).corridor.expect("a corridor");
    assert!((frame.right - 1.5).abs() < 1e-4, "right {}", frame.right);
    assert!((frame.left + 1.5).abs() < 1e-4, "left {}", frame.left);
    assert!((frame.lateral - Vec3::X).length() < 1e-4);
}

#[test]
fn a_frame_measures_the_room_on_the_side_being_asked_about() {
    let frame = Frame {
        lateral: Vec3::X,
        left: -2.0,
        right: 6.0,
    };
    assert_eq!(frame.room(1.0), 6.0);
    assert_eq!(frame.room(-1.0), 2.0);
    assert_eq!(frame.clamp(9.0), 6.0);
    assert_eq!(frame.clamp(-9.0), -2.0);
    assert_eq!(frame.clamp(3.0), 3.0);
}

#[test]
fn a_straight_line_has_no_curvature() {
    let line = Line::new(
        (0..16)
            .map(|step| Vec3::new(0.0, 0.0, 10.0 * step as f32))
            .collect(),
    );
    assert!(line.curvature(0, 10.0).abs() < 1e-6);
}

/// A straight line stays straight even where a segment is longer than the span.
///
/// **This failed at 28.6 radians per unit before the walk was rooted at
/// `index`**: every circuit ships a seam (two authored paths meet, samples bunch
/// to a tenth of a unit, one segment jumps the gap). `06_Track`'s jump is 14.78
/// units, the only one on the disc over ten, hence the only circuit an AI could
/// be parked on by a corner that is not there.
#[test]
fn a_long_segment_is_still_straight() {
    let mut points = Vec::new();
    let mut x = 0.0f32;
    // Long enough that three spans past the last index tested still land
    // well short of the wrap - a `Line` is closed, and the join of an open
    // run of points is a genuine 180-degree turn that would mask this.
    for step in 0..140 {
        points.push(Vec3::new(x, 0.0, 0.0));
        // Two samples a tenth of a unit apart, then the jump: the seam.
        x += if step == 40 {
            0.1
        } else if step == 41 {
            14.78
        } else {
            1.5
        };
    }
    let line = Line::new(points);
    for span in [4.0f32, 8.0, 10.0, 14.0, 24.0] {
        for index in 36..46 {
            let measured = line.curvature(index, span);
            assert!(
                measured < 1e-3,
                "span {span} at index {index} measured {measured} on a straight line"
            );
        }
    }
}

/// A circle's curvature is `1 / radius`, and this estimator should land near
/// it - it is a speed target's input, so being wrong by a factor would be
/// wrong by a factor everywhere downstream.
#[test]
fn curvature_approximates_one_over_the_radius() {
    let line = circle(100.0, 128);
    let measured = line.curvature(0, 20.0);
    assert!(
        (measured - 0.01).abs() < 0.002,
        "measured {measured}, expected about 0.01"
    );
}

/// A deck that drops 60 degrees onto a lower one, running along `-z` - the
/// shape of `01_Track`'s lip - with the dropping points marked unsupported
/// when `masked`.
fn drop_line(masked: bool) -> Line {
    let mut points = Vec::new();
    let mut unsupported = Vec::new();
    let (mut z, mut y) = (0.0f32, 30.0f32);
    for step in 0..60 {
        points.push(Vec3::new(0.0, y, z));
        let pitched = (20..30).contains(&step);
        unsupported.push(pitched);
        z -= if pitched { 0.75 } else { 1.5 };
        y -= if pitched { 1.3 } else { 0.0 };
    }
    let line = Line::new(points);
    if masked {
        line.with_unsupported(unsupported)
    } else {
        line
    }
}

/// Chosen, not measured (maintainer decision): a bend over a gap is not a
/// corner to brake for, because the craft is flying there.
#[test]
fn a_bend_over_a_gap_reads_straight() {
    let masked = drop_line(true);
    let raw = drop_line(false);
    let at = 16;
    assert!(
        raw.curvature(at, 3.0) > 0.05,
        "the fixture has no bend to remove: {}",
        raw.curvature(at, 3.0)
    );
    assert_eq!(masked.curvature(at, 3.0), 0.0);
    // Well clear of the gap on both sides, the two lines agree.
    assert_eq!(masked.curvature(40, 3.0), raw.curvature(40, 3.0));
}

/// A mask of the wrong length is dropped rather than half-used.
#[test]
fn a_mismatched_unsupported_mask_is_dropped() {
    let line = drop_line(false).with_unsupported(vec![true; 3]);
    assert!(!line.is_unsupported(0));
    assert!(line.curvature(16, 3.0) > 0.05);
}

/// A level run along `-z` that turns into a slope of `degrees` (a climb when
/// positive) at its middle, every point one unit apart along the path.
fn slope_line(degrees: f32) -> Line {
    let (sin, cos) = {
        let radians = degrees.to_radians();
        (radians.sin(), radians.cos())
    };
    let mut points = Vec::new();
    let (mut y, mut z) = (0.0f32, 0.0f32);
    for step in 0..80 {
        points.push(Vec3::new(0.0, y, z));
        if step < 40 {
            z -= 1.0;
        } else {
            z -= cos;
            y += sin;
        }
    }
    Line::new(points)
}

/// Chosen, not measured: the foot of a hill is concave, the road presses the
/// craft down and nothing asks its yaw for anything, so it is not a corner to
/// brake for. `05_Track`'s AI braked 7 units/s there and clipped the lip.
#[test]
fn the_foot_of_a_hill_is_not_a_corner() {
    let foot = slope_line(25.0);
    assert_eq!(foot.curvature(34, 3.0), 0.0);
}

/// The other half, and what `64da876e` lacked: over the top of a crest the craft
/// leaves the road, so the pitch still reads as a bend.
#[test]
fn the_top_of_a_crest_is_still_a_bend() {
    // Level road that turns into a descent: the road falls away under the craft.
    let crest = slope_line(-25.0);
    let raw = crest.curvature(34, 3.0);
    assert!(raw > 0.05, "a crest reads as {raw}");
}

/// The yaw half is unchanged on a line with no height in it.
#[test]
fn a_flat_circle_reads_exactly_as_it_did() {
    let ring = circle(100.0, 64);
    let measured = ring.curvature(0, 10.0);
    assert!((measured - 0.01).abs() < 0.002, "measured {measured}");
}

/// A level run along `-z` with a crest (the climb eases off) at step 40, then
/// `gap` unsupported samples from step 70, and a sideways kink of `yaw` radians
/// at step 50 when non-zero. One unit between points.
fn ramp_line(gap: usize, yaw: f32) -> Line {
    let mut points = Vec::new();
    let (mut x, mut y, mut z) = (0.0f32, 0.0f32, 0.0f32);
    let mut unsupported = Vec::new();
    for step in 0..200 {
        points.push(Vec3::new(x, y, z));
        unsupported.push((70..70 + gap).contains(&step));
        let climb = if step < 40 { 0.3f32 } else { 0.0 };
        let heading = if step >= 50 { yaw } else { 0.0 };
        y += climb;
        x += heading.sin();
        z -= heading.cos() * (1.0 - climb * climb).sqrt();
    }
    Line::new(points).with_unsupported(unsupported)
}

/// Chosen, not measured: the crest a craft launches off, a few dozen units
/// before a long gap, is not a corner to brake for. `05_Track`'s first jump.
#[test]
fn the_crest_before_a_long_gap_reads_straight() {
    let with_gap = ramp_line(50, 0.0);
    let no_gap = ramp_line(0, 0.0);
    let crest = 34;
    assert!(
        no_gap.curvature(crest, 4.0) > 0.01,
        "the fixture has no crest: {}",
        no_gap.curvature(crest, 4.0)
    );
    assert!(with_gap.is_takeoff(crest));
    assert_eq!(with_gap.curvature(crest, 4.0), 0.0);
}

/// Only the pitch is forgiven: a real turn on the run-up still brakes.
#[test]
fn a_turn_on_the_run_up_still_reads_as_a_corner() {
    let with_gap = ramp_line(50, 0.3);
    let no_gap = ramp_line(0, 0.3);
    assert!(with_gap.is_takeoff(50));
    let at = 42;
    assert!(no_gap.curvature(at, 4.0) > 0.01);
    assert!(with_gap.curvature(at, 4.0) > 0.01);
}

/// A short gap is a seam or a lip, not a jump: it marks no run-up.
#[test]
fn a_short_gap_has_no_run_up() {
    let line = ramp_line(5, 0.0);
    assert!(!line.is_takeoff(60));
    assert!(line.curvature(34, 4.0) > 0.01);
}
