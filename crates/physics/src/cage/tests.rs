use super::*;

/// Assegai VENOM's `<Misc>` box.
fn handling() -> Handling {
    let mut handling = Handling::default();
    handling.dimensions.width = 5.5;
    handling.dimensions.height = 3.5;
    handling.dimensions.length = 13.0;
    handling
}

fn level_section() -> Section {
    // A control point at the origin on level ground: tangent `-z`, down `-y`, lateral `+x`.
    Section::new(Vec3::ZERO, Vec3::NEG_Z, Vec3::NEG_Y, Vec3::X, [0; 8])
}

#[test]
fn an_authored_byte_is_a_signed_square_times_250() {
    assert_eq!(offset(0), 0.0);
    assert!((offset(127) - 250.0).abs() < 1e-3);
    assert!((offset(-127) + 250.0).abs() < 1e-3);
    // Half a byte is a quarter of the reach: the square, not the line.
    assert!((offset(-42) - -(42.0f32 / 127.0).powi(2) * 250.0).abs() < 1e-3);
}

#[test]
fn the_unauthored_section_spans_25_across_and_30_up_and_down() {
    let s = level_section();
    assert_eq!(s.left, Vec3::new(-25.0, 2.5, 0.0));
    assert_eq!(s.right, Vec3::new(25.0, 2.5, 0.0));
    assert_eq!(s.top, Vec3::new(0.0, 27.0, 0.0));
    assert_eq!(s.bottom, Vec3::new(0.0, -33.0, 0.0));
}

#[test]
fn a_point_inside_the_tube_makes_no_contact_and_one_outside_is_pushed_back_in() {
    let s = level_section();
    assert!(!s.test(Vec3::new(10.0, 3.0, 7.0)).0);
    let (outside, boundary, normal) = s.test(Vec3::new(26.0, 2.5, 7.0));
    assert!(outside);
    // Level with the centre the projection is straight across, at the same `z`.
    assert!(
        (boundary - Vec3::new(25.0, 2.5, 7.0)).length() < 1e-4,
        "{boundary}"
    );
    assert!((normal - Vec3::NEG_X).length() < 1e-4, "{normal}");
}

#[test]
fn the_tube_is_free_along_the_track() {
    let s = level_section();
    // However far along `z`, the cross-section is the same.
    assert!(!s.test(Vec3::new(0.0, 2.5, 500.0)).0);
}

/// A craft sitting outside the right wall of the tube is pushed back in, along the inward
/// normal, with no tangential loss: friction `0`.
#[test]
fn a_corner_outside_the_tube_is_resolved_with_no_friction() {
    let handling = handling();
    let mut state = ShipState::default();
    state.body.mass = 1.0;
    state.body.position = Vec3::new(24.5, 2.5, 0.0);
    state.body.linear_velocity = Vec3::new(10.0, 0.0, -100.0);
    let located = Located {
        point: Vec3::ZERO,
        section: level_section(),
        sibling: None,
    };
    let centre = state.body.position;
    let response = resolve(
        &mut state,
        &handling,
        centre,
        Quat::IDENTITY,
        Some(&located),
    );
    assert!(response.contacts > 0);
    assert!(!response.reset);
    // Pushed in along -x, and the inbound speed turned back.
    assert!(response.escape.x < 0.0);
    assert!(state.body.linear_velocity.x < 0.0);
    // The tangential part is untouched by friction: only the lever arm's spin can move it.
    assert!(
        response.velocity_delta.z.abs() < 1e-3,
        "{}",
        response.velocity_delta
    );
}

#[test]
fn a_point_inside_the_sibling_section_makes_no_contact() {
    let handling = handling();
    let mut state = ShipState::default();
    state.body.mass = 1.0;
    state.body.position = Vec3::new(26.0, 2.5, 0.0);
    let located = Located {
        point: Vec3::ZERO,
        section: level_section(),
        sibling: Some(Section::new(
            Vec3::new(30.0, 0.0, 0.0),
            Vec3::NEG_Z,
            Vec3::NEG_Y,
            Vec3::X,
            [0; 8],
        )),
    };
    let centre = state.body.position;
    let response = resolve(
        &mut state,
        &handling,
        centre,
        Quat::IDENTITY,
        Some(&located),
    );
    assert_eq!(response.contacts, 0);
}

#[test]
fn a_point_too_far_away_is_a_reset_and_no_push() {
    let handling = handling();
    let mut state = ShipState::default();
    state.body.position = Vec3::new(0.0, 0.0, 201.0);
    let located = Located {
        point: Vec3::ZERO,
        section: level_section(),
        sibling: None,
    };
    let before = state.body;
    let centre = state.body.position;
    let response = resolve(
        &mut state,
        &handling,
        centre,
        Quat::IDENTITY,
        Some(&located),
    );
    assert!(response.reset);
    assert_eq!(response.contacts, 0);
    assert_eq!(state.body, before);
}

#[test]
fn a_corner_deeper_than_15_stops_the_pass_as_a_reset() {
    let handling = handling();
    let mut state = ShipState::default();
    state.body.mass = 1.0;
    state.body.position = Vec3::new(45.0, 2.5, 0.0);
    let located = Located {
        point: Vec3::ZERO,
        section: level_section(),
        sibling: None,
    };
    let before = state.body;
    let centre = state.body.position;
    let response = resolve(
        &mut state,
        &handling,
        centre,
        Quat::IDENTITY,
        Some(&located),
    );
    assert!(response.reset);
    assert_eq!(state.body, before);
}

#[test]
fn the_corners_start_lower_left_front() {
    let mut handling = Handling::default();
    handling.dimensions.width = 4.0;
    handling.dimensions.height = 4.0;
    handling.dimensions.length = 4.0;
    let c = corners(Vec3::ZERO, Quat::IDENTITY, &handling);
    let h = 4.0 * crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    // Left is -x (`body.right()` is +x), front is -z.
    assert_eq!(c[0], Vec3::new(-h, -h, -h));
    assert_eq!(c[2], Vec3::new(h, -h, -h));
    assert_eq!(c[7], Vec3::new(h, h, h));
}
