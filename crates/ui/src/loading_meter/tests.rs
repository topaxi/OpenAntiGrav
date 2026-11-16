use super::*;

const ROW: Meter = Meter {
    at: [895.0, 475.0],
    pitch: 30.0,
    outer: 28.0,
    inner: 14.0,
    drop: 14.0,
};

#[test]
fn the_seven_centres_run_left_from_895_with_the_odd_ones_fourteen_lower() {
    let xs: Vec<f32> = (0..COUNT).map(|i| ROW.centre(i)[0]).collect();
    assert_eq!(xs, [895.0, 865.0, 835.0, 805.0, 775.0, 745.0, 715.0]);
    for i in 0..COUNT {
        let want = if i % 2 == 1 { 489.0 } else { 475.0 };
        assert!((ROW.centre(i)[1] - want).abs() < 1e-4, "{i}");
    }
}

#[test]
fn the_fill_runs_in_from_the_left_one_seventh_a_triangle() {
    let hollow =
        |fraction: f32| -> Vec<f32> { (0..COUNT).map(|i| Meter::hollow(fraction, i)).collect() };
    assert_eq!(hollow(0.0), [1.0; 7], "all hollow at the start");
    // 20 %: (1 - 0.2) * 7 = 5.6: five hollow, one part filled, one solid.
    let at_a_fifth = hollow(0.2);
    assert_eq!(at_a_fifth[..5], [1.0; 5]);
    assert!((at_a_fifth[5] - 0.6).abs() < 1e-5);
    assert!(at_a_fifth[6].abs() < 1e-6, "the left-most is solid first");
    // 98 %: every triangle solid but the first, almost.
    let at_98 = hollow(0.98);
    assert!((at_98[0] - 0.14).abs() < 1e-5);
    assert!(at_98[1..].iter().all(|h| *h == 0.0));
}

#[test]
fn the_breath_ramps_over_ten_steps_holds_and_wraps_at_64() {
    assert_eq!(Meter::pulse(0, 0), 0.0);
    assert!((Meter::pulse(5, 0) - 0.5).abs() < 1e-6);
    assert_eq!(Meter::pulse(10, 0), 1.0);
    assert_eq!(Meter::pulse(54, 0), 1.0);
    assert!((Meter::pulse(59, 0) - 0.5).abs() < 1e-6);
    assert_eq!(Meter::pulse(64, 0), 0.0, "the counter wraps");
    // Seven steps apart: triangle 1 at step 0 is where triangle 0 is at step 7.
    assert_eq!(Meter::pulse(0, 1), Meter::pulse(7, 0));
}

fn points(draw: &Draw) -> Vec<[[f32; 2]; 3]> {
    match draw {
        Draw::SolidTriangles { triangles, .. } => triangles.clone(),
        other => panic!("not triangles: {other:?}"),
    }
}

#[test]
fn a_triangle_is_three_quads_and_an_even_one_points_down() {
    let triangles = points(&ROW.draw(0.0, 20, [0.0, 0.0, 1.0, 1.0]));
    assert_eq!(
        triangles.len(),
        COUNT * 6,
        "three quads, two triangles each"
    );
    // Triangle 0 at full breath: outer corners (0, +28) below the centre, two above it.
    let outer = &triangles[0];
    assert!((outer[0][0] - 895.0).abs() < 1e-3 && (outer[0][1] - 503.0).abs() < 1e-3);
    // The odd triangle's lowest outer corner is a flat base: apex up at 489 - 28 = 461.
    let apex_y = triangles[6..12]
        .iter()
        .flatten()
        .map(|p| p[1])
        .fold(f32::MAX, f32::min);
    assert!((apex_y - 461.0).abs() < 1e-3, "{apex_y}");
}

#[test]
fn a_solid_triangle_has_its_inner_corner_a_pixel_past_the_centre() {
    // Fraction 1 makes every triangle solid: inner radius 14 u 0 - 1 = -1.
    let triangles = points(&ROW.draw(1.0, 20, [0.0; 4]));
    let inner = triangles[1][2];
    assert!((inner[0] - 895.0).abs() < 1.0);
    assert!(
        (inner[1] - 474.0).abs() < 1e-3,
        "{inner:?}: radius -1 mirrors the corner through the centre"
    );
}
