use super::*;

fn full_tube(step: Vec3) -> Tube {
    let mut tube = Tube::new();
    for k in 0..SAMPLES {
        tube.push(step * k as f32, Vec3::Y, 0.0);
    }
    tube
}

#[test]
fn empty_until_the_ring_fills() {
    let mut tube = Tube::new();
    for k in 0..SAMPLES - 1 {
        tube.push(Vec3::X * k as f32, Vec3::Y, 0.0);
        assert!(!tube.ready());
        assert!(tube.vertices(1.0, 0.0, 0.0).is_empty());
    }
    tube.push(Vec3::X * SAMPLES as f32, Vec3::Y, 0.0);
    assert!(tube.ready());
    assert_eq!(tube.vertices(1.0, 0.0, 0.0).len(), VERTICES_PER_CRAFT);
}

#[test]
fn vertex_count_matches_the_originals_index_count() {
    // 954 - the count `TrailEffectManager`'s own draw call submits.
    assert_eq!(VERTICES_PER_CRAFT, 954);
}

#[test]
fn edges_sit_half_a_unit_from_the_trail_line() {
    let tube = full_tube(Vec3::X);
    let verts = tube.vertices(1.0, 0.0, 0.0);
    // Segment vertices pair up across the trail line: each corner is at
    // exactly FIN_HALF_WIDTH from the sample position it was extruded from.
    for v in &verts {
        let p = Vec3::from_array(v.position);
        // The trail runs along X at multiples of 1; the offset is in YZ.
        let radial = Vec3::new(0.0, p.y, p.z).length();
        assert!(
            (radial - FIN_HALF_WIDTH).abs() < 1e-5,
            "radial {radial} at {p:?}"
        );
    }
}

#[test]
fn fin_zero_spans_the_up_axis_and_the_others_sit_at_sixty_degrees() {
    let tube = full_tube(Vec3::X);
    let verts = tube.vertices(1.0, 0.0, 0.0);
    let per_fin = VERTICES_PER_CRAFT / FINS;
    // Fins are emitted at -60, 0, +60 degrees from up; the middle block is
    // fin 0. Its first two vertices are the two edges of the head ring.
    let e0 = Vec3::from_array(verts[per_fin].position);
    let e1 = Vec3::from_array(verts[per_fin + 1].position);
    assert!((e0 - Vec3::new(53.0, -0.5, 0.0)).length() < 1e-4, "{e0:?}");
    assert!((e1 - Vec3::new(53.0, 0.5, 0.0)).length() < 1e-4, "{e1:?}");
    // The outer fins' edge directions make 60 degrees with up.
    for fin in [0, 2] {
        let a = Vec3::from_array(verts[fin * per_fin].position);
        let b = Vec3::from_array(verts[fin * per_fin + 1].position);
        let dir = (b - a).normalize();
        let cos = dir.dot(Vec3::Y).abs();
        assert!((cos - 0.5).abs() < 1e-4, "fin {fin} cos {cos}");
    }
}

#[test]
fn alpha_attacks_over_five_rings_and_falls_to_the_tail() {
    let tube = full_tube(Vec3::X);
    let verts = tube.vertices(0.717, 0.0, 0.0);
    // Ring k's first vertex within fin 0's strip is at (k * 6) of that block.
    let per_fin = VERTICES_PER_CRAFT / FINS;
    let alpha_at = |k: usize| verts[per_fin + k * 6].colour[3];
    assert_eq!(alpha_at(0), 0.0);
    // The measured peak: brightness * 1.0 * (1 - 6/54) at ring 6 = 0.637 for
    // brightness 0.717 - the dumped 162/255.
    let peak = alpha_at(6);
    assert!((peak - 0.717 * (1.0 - 6.0 / 54.0)).abs() < 1e-4, "{peak}");
    // Monotone fall after the attack.
    assert!(alpha_at(20) > alpha_at(40));
    let tail = alpha_at(52);
    assert!(tail > 0.0 && tail < 0.05, "{tail}");
}

#[test]
fn tint_reaches_pure_red_at_half_the_ring() {
    let tube = full_tube(Vec3::X);
    let verts = tube.vertices(1.0, 0.0, 1.0);
    let per_fin = VERTICES_PER_CRAFT / FINS;
    let colour_at = |k: usize| verts[per_fin + k * 6].colour;
    assert_eq!(colour_at(0)[1], 1.0);
    assert_eq!(colour_at(27)[1], 0.0);
    // Ring 53 exists only as segment 52's far corner (index 3 of its six).
    assert_eq!(verts[per_fin + 52 * 6 + 3].colour[1], 0.0);
    assert_eq!(colour_at(30)[0], 1.0);
    // And the mix flag rides `lit`.
    assert_eq!(verts[0].lit, 1.0);
}

#[test]
fn u_stretches_with_speed_and_falls_off_at_four_over_fiftyfour() {
    let tube = full_tube(Vec3::X);
    let phase = tube.phase_for_tests();
    let at_rest = tube.vertices(1.0, 0.0, 0.0);
    let at_speed = tube.vertices(1.0, 1.0, 0.0);
    let per_fin = VERTICES_PER_CRAFT / FINS;
    let u_at = |v: &[GpuVertex], k: usize| v[per_fin + k * 6].texcoord[0];
    assert!((u_at(&at_rest, 0) - (1.0 + phase)).abs() < 1e-5);
    assert!((u_at(&at_speed, 0) - (0.4 + phase)).abs() < 1e-5);
    // The measured law: u(k) = u_head * (1 - 4k/54) + phase.
    let expect = 1.0 * (1.0 - 4.0 * 20.0 / 54.0) + phase;
    assert!((u_at(&at_rest, 20) - expect).abs() < 1e-4);
}

#[test]
fn the_scroll_phase_advances_per_push_and_wraps() {
    let mut tube = Tube::new();
    for _ in 0..10 {
        tube.push(Vec3::ZERO, Vec3::Y, 0.0);
    }
    assert!((tube.phase_for_tests() - 0.8).abs() < 1e-5);
    for _ in 0..3 {
        tube.push(Vec3::ZERO, Vec3::Y, 1.0);
    }
    // 0.8 + 3 * 0.1 = 1.1, wrapped.
    assert!((tube.phase_for_tests() - 0.1).abs() < 1e-4);
}

#[test]
fn speed01_combines_speed_and_thrust_and_clamps() {
    assert_eq!(speed01(0.0, 0.0), 0.0);
    assert!((speed01(500.0, 0.0) - 0.5).abs() < 1e-6);
    assert!((speed01(500.0, 1.0) - 0.75).abs() < 1e-6);
    assert_eq!(speed01(1000.0, 1.0), 1.0);
}
