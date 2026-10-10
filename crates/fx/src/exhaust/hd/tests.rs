use super::*;

fn full_tube(step: Vec3) -> Tube {
    let mut tube = Tube::new();
    for k in 0..SAMPLES {
        tube.push(step * k as f32, Vec3::Y, 0.0);
    }
    tube
}

#[test]
fn the_first_push_seeds_a_full_bunched_ring() {
    // Measured at a live race load (engine-trail.md, e0..e4): the earliest
    // readable ring is already full, every sample bunched at the grid slot,
    // and the SPU extrudes all 324 vertices from it. No fill gate exists.
    let mut tube = Tube::new();
    assert!(!tube.ready());
    assert!(tube.vertices(1.0, 0.0, 0.0).is_empty());
    tube.push(Vec3::X, Vec3::Y, 0.0);
    assert!(tube.ready());
    let vertices = tube.vertices(1.0, 0.0, 0.0);
    assert_eq!(vertices.len(), VERTICES_PER_CRAFT);
    // The seeded ring is a clump at the sample: every vertex within the fin
    // half-width of it, exactly like the live grid-start buffers.
    for v in &vertices {
        let p = Vec3::from_array(v.position);
        assert!((p - Vec3::X).length() <= FIN_HALF_WIDTH + 1e-6);
    }
}

#[test]
fn clear_forgets_the_pose_and_the_next_push_reseeds() {
    let mut tube = full_tube(Vec3::X);
    tube.clear();
    assert!(!tube.ready());
    assert!(tube.vertices(1.0, 0.0, 0.0).is_empty());
    tube.push(Vec3::Z * 100.0, Vec3::Y, 0.0);
    // Re-bunched at the new pose - nothing spans the teleport.
    for v in &tube.vertices(1.0, 0.0, 0.0) {
        let p = Vec3::from_array(v.position);
        assert!((p - Vec3::Z * 100.0).length() <= FIN_HALF_WIDTH + 1e-6);
    }
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
    // Fins are emitted at 0, 60, 120 degrees from up; block 0 is the
    // up-spanning fin. Its first two vertices are the head ring's edges.
    let e0 = Vec3::from_array(verts[0].position);
    let e1 = Vec3::from_array(verts[1].position);
    assert!((e0 - Vec3::new(53.0, -0.5, 0.0)).length() < 1e-4, "{e0:?}");
    assert!((e1 - Vec3::new(53.0, 0.5, 0.0)).length() < 1e-4, "{e1:?}");
    // The other fins' edge directions make 60 degrees with up either way.
    for fin in [1, 2] {
        let a = Vec3::from_array(verts[fin * per_fin].position);
        let b = Vec3::from_array(verts[fin * per_fin + 1].position);
        let dir = (b - a).normalize();
        let cos = dir.dot(Vec3::Y).abs();
        assert!((cos - 0.5).abs() < 1e-4, "fin {fin} cos {cos}");
    }
    // The single-sided facing fade is what makes the normal *directions*
    // load-bearing: with travel along +X and up +Y, the dumped normals are
    // -Z for the vertical fin and (0, +0.87, -+0.5) for the other two -
    // both tilted toward up, so a camera above the trail keeps two fins
    // lit. See the fin comment in `Tube::vertices`.
    let n0 = Vec3::from_array(verts[6].normal).normalize();
    assert!((n0 - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4, "{n0:?}");
    for (fin, expect_z) in [(1, -0.5), (2, 0.5)] {
        let n = Vec3::from_array(verts[fin * per_fin + 6].normal).normalize();
        let want = Vec3::new(0.0, 3.0f32.sqrt() / 2.0, expect_z);
        assert!((n - want).length() < 1e-4, "fin {fin}: {n:?} want {want:?}");
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
    // The measured law is `u(k) = u_head * (1 - 4k/54)` with no phase in it -
    // the original leaves the scroll to the draw-time `TrailSpeed` constant.
    // What this engine emits folds the two, which is why `+ phase` is here and
    // not in the dumped buffers. See `Tube::phase`.
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
fn speed01_combines_the_gained_speed_and_thrust_and_clamps() {
    assert_eq!(speed01(0.0, 0.0), 0.0);
    // 100 km/h -> field 150 -> 0.15; the dump's cross-check: a live field of
    // 456 under full throttle read back u_head = 1 - 0.6 * 0.706 = 0.576.
    assert!((speed01(100.0, 0.0) - 0.15).abs() < 1e-6);
    assert!((speed01(304.0, 1.0) - 0.706).abs() < 1e-6);
    assert_eq!(speed01(600.0, 1.0), 1.0);
}

#[test]
fn the_brightness_ramp_saturates_where_hd_races() {
    // The live fields: 456 -> ramp 0.7117 (exact), 248 -> 0.2959, and our
    // km/h is the field over 1.5.
    assert_eq!(speed_ramp(0.0), 0.0);
    assert!((speed_ramp(456.0 / 1.5) - 0.7117).abs() < 1e-3);
    assert!((speed_ramp(248.0 / 1.5) - 0.2959).abs() < 1e-2);
    assert_eq!(speed_ramp(400.0), 1.0);
}

/// The centre-line query the trail-hit test rides on.
///
/// The mechanism it serves is measured (`Trail_SpawnHitEffect` spawns
/// `WO_TRAIL_HITSHIP` on a craft inside a trail); this is only the geometry,
/// so what it asserts is the geometry alone.
#[test]
fn nearest_projects_onto_a_segment_rather_than_snapping_to_a_sample() {
    let tube = full_tube(Vec3::X * 10.0);
    // A point beside the ribbon, halfway between two stored samples 10 units
    // apart. Snapping to the nearest sample would answer 5-ish; projecting
    // onto the segment answers the perpendicular distance.
    let (on, distance) = tube
        .nearest(Vec3::new(25.0, 3.0, 0.0))
        .expect("a full ring");
    assert!((distance - 3.0).abs() < 1e-4, "{distance}");
    assert!((on - Vec3::new(25.0, 0.0, 0.0)).length() < 1e-4, "{on:?}");
    // Past the head, the answer clamps to the end of the line rather than
    // running off it.
    let (on, _) = tube
        .nearest(Vec3::new(-100.0, 0.0, 0.0))
        .expect("a full ring");
    assert!(on.x >= -1e-4, "{on:?}");
}

/// A tube nothing pushed to has no centre line, and says so.
#[test]
fn nearest_answers_none_before_the_first_push() {
    assert!(Tube::new().nearest(Vec3::ZERO).is_none());
}

/// A bunched ring - which is every ring at a race start - collapses each
/// segment to a point rather than dividing by its own zero length.
#[test]
fn nearest_survives_the_degenerate_ring_a_race_starts_with() {
    let mut tube = Tube::new();
    tube.push(Vec3::new(4.0, 0.0, 0.0), Vec3::Y, 0.0);
    let (on, distance) = tube.nearest(Vec3::new(4.0, 2.0, 0.0)).expect("seeded");
    assert!(
        distance.is_finite() && (distance - 2.0).abs() < 1e-4,
        "{distance}"
    );
    assert!((on - Vec3::new(4.0, 0.0, 0.0)).length() < 1e-4, "{on:?}");
}

fn walked_sprite() -> Sprite {
    // Ten ticks of a fixed stream: the walk has moved off its seed and the
    // jitter is a known draw.
    let mut sprite = Sprite::new();
    for _ in 0..10 {
        sprite.advance(|| 0.5);
    }
    sprite
}

#[test]
fn the_fade_is_full_inside_the_fadeout_distance_and_zero_past_its_range() {
    // `1 - saturate((d - 15) / 15)`: 1.0 up to 15 units, 0 from 30 on.
    let on_axis = 1.0;
    let walk = 0.7;
    let near = sprite_fade(0.0, on_axis, walk);
    let at_start = sprite_fade(SPRITE_FADEOUT_DIST, on_axis, walk);
    assert!((near - walk).abs() < 1e-6, "{near}");
    assert!((at_start - walk).abs() < 1e-6, "{at_start}");
    let halfway = sprite_fade(
        SPRITE_FADEOUT_DIST + SPRITE_FADEOUT_RANGE * 0.5,
        on_axis,
        walk,
    );
    assert!((halfway - walk * 0.5).abs() < 1e-6, "{halfway}");
    let gone = sprite_fade(SPRITE_FADEOUT_DIST + SPRITE_FADEOUT_RANGE, on_axis, walk);
    assert_eq!(gone, 0.0);
    assert_eq!(sprite_fade(1000.0, on_axis, walk), 0.0);
    // A craft at the fadeout distance through the state type: no quad at
    // all - the original leaves after the store when the fade is not
    // positive, before the quad is built. One unit inside, a quad.
    let sprite = walked_sprite();
    assert_eq!(
        sprite.fade(SPRITE_FADEOUT_DIST + SPRITE_FADEOUT_RANGE, on_axis),
        None
    );
    assert!(
        sprite
            .fade(SPRITE_FADEOUT_DIST + SPRITE_FADEOUT_RANGE - 1.0, on_axis)
            .is_some_and(|f| f > 0.0)
    );
}

#[test]
fn the_highlight_is_a_narrow_lobe_and_the_far_hemisphere_draws_nothing() {
    let sprite = walked_sprite();
    let walk = sprite.alpha_walk();
    // Dead on the axis the walk comes through whole (Boost is 1.0).
    let on = sprite.fade(1.0, 1.0).expect("on axis");
    assert!((on - walk).abs() < 1e-6);
    // `cos^32`: 30 degrees off is already a hundredth of the peak.
    let off = sprite
        .fade(1.0, 30f32.to_radians().cos())
        .expect("off axis");
    assert!(off < walk * 0.02, "{off} against {walk}");
    // The ceiling: a walk above `Flare Opacity Max` cannot exceed it.
    assert!(sprite_fade(0.0, 1.0, 5.0) <= SPRITE_OPACITY_MAX);
    // Side-on and behind: the original's `ble` before any of the math.
    assert_eq!(sprite.fade(1.0, 0.0), None);
    assert_eq!(sprite.fade(1.0, -0.5), None);
}

#[test]
fn the_view_dot_is_positive_looking_into_the_nozzle() {
    // Camera behind a craft that flies along +X: the view looks along +X and
    // the exhaust points back along -X, toward the eye.
    let forward = Vec3::X;
    let nozzle_axis = Vec3::NEG_X;
    assert!((sprite_view_dot(forward, nozzle_axis) - 1.0).abs() < 1e-6);
    // Head-on, the nozzle points away from the eye.
    assert!((sprite_view_dot(Vec3::NEG_X, nozzle_axis) + 1.0).abs() < 1e-6);
    // Unnormalised inputs are normalised, and a zero axis is a zero dot.
    assert!((sprite_view_dot(Vec3::X * 7.0, Vec3::NEG_X * 0.1) - 1.0).abs() < 1e-6);
    assert_eq!(sprite_view_dot(Vec3::X, Vec3::ZERO), 0.0);
}

#[test]
fn the_half_height_is_min_plus_radius_by_fade_plus_a_one_sided_jitter() {
    // Faded out entirely: the base term plus nothing.
    assert_eq!(sprite_half_height(0.0, 0.0), SPRITE_RADIUS_MIN);
    // Full fade, full jitter: every term at its top - 5.5 authored.
    assert_eq!(
        sprite_half_height(1.0, 1.0),
        SPRITE_RADIUS_MIN + SPRITE_RADIUS + SPRITE_RADIUS_JITTER
    );
    // The fade is clamped into the size; the jitter never subtracts.
    assert_eq!(
        sprite_half_height(3.0, 0.0),
        SPRITE_RADIUS_MIN + SPRITE_RADIUS
    );
    for jitter in [0.0, 0.25, 1.0] {
        assert!(sprite_half_height(0.5, jitter) >= sprite_half_height(0.5, 0.0));
    }
    // And the state type feeds its own tick's draw in.
    let sprite = walked_sprite();
    assert_eq!(sprite.jitter(), 0.5);
    assert_eq!(
        sprite.half_height(0.0),
        SPRITE_RADIUS_MIN + SPRITE_RADIUS_JITTER * 0.5
    );
}

#[test]
fn the_quad_is_four_times_wider_than_it_is_tall() {
    let half = 1.5;
    let quad = sprite_quad(Vec3::ZERO, Vec3::X, Vec3::Y, half, 0.3);
    let (mut max_x, mut max_y) = (0.0f32, 0.0f32);
    for v in &quad {
        max_x = max_x.max(v.position[0].abs());
        max_y = max_y.max(v.position[1].abs());
        assert_eq!(v.colour[3], 0.3, "alpha is the fade");
    }
    assert!((max_y - half).abs() < 1e-6, "{max_y}");
    assert!((max_x - half * SPRITE_ASPECT).abs() < 1e-6, "{max_x}");
    assert_eq!(SPRITE_ASPECT, 4.0, "1024 x 256 texels");
}

#[test]
fn the_vertex_ramp_follows_the_fury_flag_as_the_running_game_wrote_it() {
    // Read from the SPU output buffers of a live race (engine-trail.md, "The
    // vertex colour ramp is per skin"): u8 rgb at rings 0, 3 and 27+.
    let u8s = |c: [f32; 3]| c.map(|x| (x * 255.0).round() as i32);
    assert_eq!(u8s(tint_at(0, 1.0)), [255, 255, 255]);
    assert_eq!(u8s(tint_at(27, 1.0)), [255, 0, 0]);
    assert_eq!(u8s(tint_at(53, 1.0)), [255, 0, 0]);
    assert_eq!(u8s(tint_at(0, 0.0)), [63, 255, 255]);
    let ring3 = u8s(tint_at(3, 0.0));
    assert!((ring3[0] - 84).abs() <= 1 && (ring3[1] - 240).abs() <= 1 && ring3[2] == 255);
    assert_eq!(u8s(tint_at(27, 0.0)), [255, 127, 255]);
    assert_eq!(u8s(tint_at(53, 0.0)), [255, 127, 255]);
    // A classic ribbon never loses blue or green below half: no trip to red.
    let tube = full_tube(Vec3::NEG_Z);
    for v in tube.vertices(1.0, 0.0, 0.0) {
        assert!(v.colour[1] >= 0.49 && (v.colour[2] - 1.0).abs() < 1e-6);
    }
}
