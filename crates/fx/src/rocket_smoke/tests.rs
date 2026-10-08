use super::*;

/// A ramp whose entry is its own index, so a test reads which entry the law
/// picked rather than what the disc stores there.
fn identity_ramp() -> OpacityRamp {
    let mut tga = vec![0xeeu8; RAMP_OFFSET];
    tga.extend((0..=255u8).collect::<Vec<_>>());
    OpacityRamp::from_tga(&tga).expect("long enough")
}

const DT: f32 = 1.0 / 60.0;

fn basis() -> (Vec3, Vec3) {
    (Vec3::Z, Vec3::Y)
}

fn pushed(n: usize) -> Trail {
    let mut rng = Rng::new(7);
    let mut trail = Trail::new();
    let (right, up) = basis();
    for i in 0..n {
        trail.push(
            Vec3::new(i as f32 * 3.6, 0.0, 0.0),
            right,
            up,
            UNLIT_RGB,
            &mut rng,
        );
        trail.advance(DT);
    }
    trail
}

#[test]
fn the_table_starts_two_bytes_past_the_tga_header() {
    // 18 header bytes, then two pixels the engine skips, then the table.
    let mut tga = vec![0u8; 18];
    tga.extend([0xaa, 0xbb, 7, 8]);
    tga.resize(RAMP_OFFSET + 256, 0);
    let ramp = OpacityRamp::from_tga(&tga).unwrap();
    assert_eq!(ramp.table()[0], 7);
    assert_eq!(ramp.table()[1], 8);
    assert!(OpacityRamp::from_tga(&tga[..RAMP_OFFSET + 255]).is_none());
}

/// The live dump's own nodes: life after the update, the half-width and the
/// ramp entry its alpha came from (`0xf1` = entry 2, `0xdf` = 11, `0xc0` = 20
/// on the disc's table).
#[test]
fn half_width_and_ramp_index_follow_the_dumped_nodes() {
    let ramp = identity_ramp();
    for (life_before, half_width, entry) in [
        (1.8333f32, 0.4143f32, 2u8),
        (1.7661, 0.4726, 11),
        (1.6999, 0.5298, 20),
    ] {
        let mut trail = Trail::new();
        let mut rng = Rng::new(1);
        // Three nodes so the middle one keeps its alpha.
        for _ in 0..3 {
            trail.push(Vec3::ZERO, Vec3::Z, Vec3::Y, UNLIT_RGB, &mut rng);
        }
        trail.nodes[1].life = life_before;
        trail.advance(DT);
        let node = trail.nodes[1];
        assert!(
            (node.half_width - half_width).abs() < 2e-4,
            "{life_before}: {}",
            node.half_width
        );
        assert_eq!(trail.alpha(1, &ramp), entry, "{life_before}");
    }
}

#[test]
fn a_new_node_is_born_at_the_birth_width_and_both_ends_are_clear() {
    let ramp = identity_ramp();
    let trail = pushed(10);
    let nodes: Vec<_> = trail.nodes().copied().collect();
    let newest = nodes.last().unwrap();
    assert!((newest.half_width - HALF_WIDTH_BIRTH).abs() < 1e-6);
    assert!((newest.life - (LIFETIME - DT)).abs() < 1e-6);
    assert_eq!(trail.alpha(nodes.len() - 1, &ramp), 0);
    assert_eq!(trail.alpha(0, &ramp), 0);
    assert!((1..nodes.len() - 1).all(|i| trail.alpha(i, &ramp) > 0));
}

#[test]
fn u_is_frozen_arc_length_from_the_first_node() {
    let trail = pushed(5);
    let nodes: Vec<_> = trail.nodes().copied().collect();
    assert_eq!(nodes[0].u, Some(0.0));
    for pair in nodes.windows(2) {
        let step = pair[1].u.unwrap() - pair[0].u.unwrap();
        let spacing = (pair[1].position - pair[0].position).length();
        assert!(
            (step - spacing * U_PER_UNIT).abs() < 1e-4,
            "{step} vs {spacing}"
        );
    }
}

#[test]
fn a_ribbon_outlives_its_rocket_by_its_lifetime_and_then_empties() {
    let mut trail = pushed(20);
    let mut ticks = 0;
    while !trail.is_empty() {
        trail.advance(DT);
        ticks += 1;
        assert!(ticks < 200, "never emptied");
    }
    // The last node laid lives 1.85 s, plus the one undrawn update it
    // survives at a negative life.
    assert!((110..=113).contains(&ticks), "{ticks}");
}

#[test]
fn jitter_stays_inside_its_amplitude() {
    let mut rng = Rng::new(3);
    let mut trail = Trail::new();
    for _ in 0..500 {
        trail.push(Vec3::ZERO, Vec3::Z, Vec3::Y, UNLIT_RGB, &mut rng);
    }
    assert!(
        trail
            .nodes()
            .all(|n| n.position.abs().max_element() <= JITTER)
    );
    assert!(trail.nodes().any(|n| n.position.length() > 0.0));
}

/// The three fins the interpreter run of `RibbonBuilder_StripSide` printed on
/// an identity basis (right = X, up = Y).
#[test]
fn fins_are_sixty_degrees_apart_from_up_toward_minus_right() {
    let expect = [
        ((0.0, 1.0), (1.0, 0.0)),
        ((-0.866, 0.5), (0.5, 0.866)),
        ((-0.866, -0.5), (-0.5, 0.866)),
    ];
    for (k, ((sx, sy), (nx, ny))) in expect.into_iter().enumerate() {
        let (side, normal) = fin_axes(k, Vec3::X, Vec3::Y);
        assert!(
            (side - Vec3::new(sx, sy, 0.0)).length() < 1e-3,
            "fin {k} side {side}"
        );
        assert!(
            (normal - Vec3::new(nx, ny, 0.0)).length() < 1e-3,
            "fin {k} normal {normal}"
        );
    }
}

#[test]
fn vertices_run_newest_first_two_half_widths_apart() {
    let trail = pushed(4);
    let mut out = Vec::new();
    trail.extend_vertices(&mut out, &identity_ramp());
    // Three segments, three fins, two triangles each.
    assert_eq!(out.len(), 3 * FINS * 6);
    let newest = trail.nodes().last().unwrap();
    let (a0, a1) = (Vec3::from(out[0].position), Vec3::from(out[1].position));
    assert!(((a0 + a1) * 0.5 - newest.position).length() < 1e-5);
    assert!(((a1 - a0).length() - 2.0 * HALF_WIDTH_BIRTH).abs() < 1e-5);
    assert_eq!(out[0].texcoord[1], 1.0);
    assert_eq!(out[1].texcoord[1], 0.0);
    // The draw's own constant ambient is folded into the vertex colour.
    assert!((out[0].colour[0] - AMBIENT).abs() < 1e-6);
}

#[test]
fn a_spent_node_is_not_drawn() {
    let ramp = identity_ramp();
    let mut trail = pushed(3);
    for node in &mut trail.nodes {
        node.life = 0.001;
    }
    trail.advance(DT);
    let mut out = Vec::new();
    trail.extend_vertices(&mut out, &ramp);
    assert!(out.is_empty());
    assert_eq!(trail.nodes().count(), 3, "freed only on the next update");
    trail.advance(DT);
    assert!(trail.is_empty());
}
