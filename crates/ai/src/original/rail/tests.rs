use super::*;

fn own() -> TrackPoint {
    TrackPoint {
        position: Vec3::ZERO,
        tangent: Vec3::NEG_Z,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        bound_left: -10.0,
        bound_right: 10.0,
        racing_line: 0.0,
        progress: 0.0,
    }
}

fn pose(x: f32, speed: f32) -> Pose {
    Pose {
        position: Vec3::new(x, 3.0, 0.0),
        up: Vec3::Y,
        forward: Vec3::NEG_Z,
        left: Vec3::NEG_X,
        velocity: Vec3::new(speed * 0.1, 0.0, -speed),
        angular_velocity: Vec3::ZERO,
    }
}

const DT: f32 = 1.0 / 60.0;

#[test]
fn slow_crafts_are_left_alone() {
    let mut rail = Rail::default();
    assert_eq!(
        rail.snap(&pose(2.0, 40.0), &own(), 0.0, false, false, true, DT),
        None
    );
    assert_eq!(rail, Rail::default());
}

#[test]
fn the_blend_ramps_in_and_then_pulls_seventy_percent_of_the_lateral_error() {
    let mut rail = Rail::default();
    let mut last = None;
    for _ in 0..200 {
        last = rail.snap(&pose(2.0, 200.0), &own(), 0.0, false, false, true, DT);
    }
    assert_eq!(rail.blend, 1.0);
    let write = last.expect("fires at full blend");
    assert!((write.position.x - 0.6).abs() < 1e-5, "{write:?}");
    assert_eq!(write.position.y, 3.0, "only across the track");
    let before = pose(2.0, 200.0).velocity;
    assert!(
        (write
            .velocity
            .expect("Pulse's rail turns the velocity")
            .length()
            - before.length())
        .abs()
            < 1e-3,
        "speed kept"
    );
    assert!(
        write.velocity.is_some_and(|v| v.x < before.x),
        "sideways velocity turned away"
    );
}

#[test]
fn contact_drops_the_rail_at_once() {
    let mut rail = Rail::default();
    for _ in 0..200 {
        rail.snap(&pose(2.0, 200.0), &own(), 0.0, false, false, true, DT);
    }
    assert_eq!(
        rail.snap(&pose(2.0, 200.0), &own(), 0.0, true, false, true, DT),
        None
    );
    assert_eq!(rail, Rail::default());
}

#[test]
fn far_off_the_surface_the_rail_lets_go() {
    let mut rail = Rail::default();
    let mut high = pose(2.0, 200.0);
    high.position.y = 15.0;
    for _ in 0..200 {
        assert_eq!(rail.snap(&high, &own(), 0.0, false, false, true, DT), None);
    }
}

#[test]
fn a_finished_craft_blends_in_faster() {
    let (mut racing, mut finished) = (Rail::default(), Rail::default());
    racing.snap(&pose(2.0, 200.0), &own(), 0.0, false, false, true, DT);
    finished.snap(&pose(2.0, 200.0), &own(), 0.0, false, true, true, DT);
    assert_eq!(racing.blend, DT * 0.6);
    assert_eq!(finished.blend, DT * 1.5);
}

/// Pure's rail (`Ai_SnapToLane`, `0x08917b80`) passes `Body_SetPosition` the
/// three position floats and nothing else: the same position, no velocity.
#[test]
fn a_position_only_rail_moves_the_craft_and_leaves_its_velocity() {
    let mut pulse = Rail::default();
    let mut pure = Rail::default();
    let (mut a, mut b) = (None, None);
    for _ in 0..200 {
        a = pulse.snap(&pose(2.0, 200.0), &own(), 0.0, false, false, true, DT);
        b = pure.snap(&pose(2.0, 200.0), &own(), 0.0, false, false, false, DT);
    }
    let (a, b) = (a.expect("fires"), b.expect("fires"));
    assert_eq!(a.position, b.position);
    assert_eq!(b.velocity, None);
}
