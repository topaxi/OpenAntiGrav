use super::*;

fn craft() -> Craft {
    Craft {
        anchor: Vec3::new(0.0, 3.0, 0.0),
        position: Vec3::new(0.0, 3.0, 0.0),
        forward: Vec3::Z,
        up: Vec3::Y,
        speed: 150.0,
        with_track: true,
        lateral: 0.0,
    }
}

/// A flat, straight road along `+Z`, 20 wide, lifted `3` above the surface.
fn road(ahead: f32) -> Option<Placement> {
    Some(Placement {
        position: Vec3::new(0.0, 0.0, ahead),
        forward: Vec3::Z,
        lateral: Vec3::X,
        down: -Vec3::Y,
        half_width_left: 10.0,
        half_width_right: 10.0,
    })
}

#[test]
fn nothing_spawns_while_off_the_strip() {
    let mut wake = Wake::new(1);
    for _ in 0..60 {
        wake.advance(1.0 / 60.0, false, &craft(), &road);
    }
    assert_eq!(wake.live(), 0, "an inactive wake must not spawn");
}

#[test]
fn arcs_spawn_while_on_the_strip_and_age_out_after() {
    let mut wake = Wake::new(2);
    for _ in 0..30 {
        wake.advance(1.0 / 60.0, true, &craft(), &road);
    }
    assert!(wake.live() > 0, "an active wake spawns arcs");
    assert!(wake.live() <= ARCS);
    for _ in 0..120 {
        wake.advance(1.0 / 60.0, false, &craft(), &road);
    }
    assert_eq!(
        wake.live(),
        0,
        "life is at most 1.1 s, so two seconds clears it"
    );
}

#[test]
fn a_live_slot_is_never_overwritten() {
    let mut wake = Wake::new(3);
    wake.advance(0.0, true, &craft(), &road);
    let first: Vec<(usize, Vec3)> = (0..ARCS)
        .filter(|&i| wake.arcs()[i].life > 0.0)
        .map(|i| (i, wake.arcs()[i].end))
        .collect();
    wake.advance(0.0, true, &craft(), &road);
    for (slot, end) in first {
        assert_eq!(wake.arcs()[slot].end, end, "slot {slot} was overwritten");
    }
}

#[test]
fn an_arc_ends_on_the_road_within_its_width() {
    let mut wake = Wake::new(4);
    for _ in 0..40 {
        wake.advance(1.0 / 60.0, true, &craft(), &road);
    }
    for arc in wake.arcs().iter().filter(|a| a.life > 0.0) {
        assert!((arc.end.y + END_DROP).abs() < 1e-4, "{:?}", arc.end);
        assert!(arc.end.x.abs() <= 10.0 + 1e-4);
        assert!(arc.life <= 1.1);
    }
}

#[test]
fn each_live_arc_is_six_body_quads_and_one_contact_quad() {
    let mut wake = Wake::new(5);
    for _ in 0..20 {
        wake.advance(1.0 / 60.0, true, &craft(), &road);
    }
    let live = wake.live();
    let (mut atlas, mut contact) = (Vec::new(), Vec::new());
    wake.build(
        &craft(),
        Vec3::new(0.0, 6.0, -10.0),
        &mut atlas,
        &mut contact,
    );
    assert_eq!(atlas.len(), live * BODY_QUADS * 6);
    assert_eq!(contact.len(), live * 6);
    assert!(atlas.len() <= BODY_VERTICES && contact.len() <= CONTACT_VERTICES);
}

#[test]
fn the_body_samples_one_atlas_cell_and_ends_dim() {
    let mut wake = Wake::new(6);
    wake.advance(0.0, true, &craft(), &road);
    let arc = *wake.arcs().iter().find(|a| a.life > 0.0).expect("an arc");
    let (mut atlas, mut contact) = (Vec::new(), Vec::new());
    wake.build(
        &craft(),
        Vec3::new(0.0, 6.0, -10.0),
        &mut atlas,
        &mut contact,
    );
    let cell = |v: f32, base: f32| (base..=base + CELL + 1e-5).contains(&v);
    let (col, row) = ((arc.frame % 8) as f32 * CELL, (arc.frame / 8) as f32 * CELL);
    for v in &atlas[..BODY_QUADS * 6] {
        assert!(
            cell(v.texcoord[0], col) && cell(v.texcoord[1], row),
            "{v:?}"
        );
        assert_eq!(v.colour[3], BODY_ALPHA);
    }
    let last = &atlas[(BODY_QUADS - 1) * 6..BODY_QUADS * 6];
    let dimmest = last.iter().map(|v| v.colour[0]).fold(f32::MAX, f32::min);
    assert!((dimmest - arc.intensity * SOFT).abs() < 1e-5);
}

#[test]
fn an_arc_left_behind_is_shed_far_out() {
    let mut wake = Wake::new(7);
    wake.advance(0.0, true, &craft(), &road);
    let mut moved = craft();
    moved.position = Vec3::new(0.0, 3.0, 500.0);
    moved.forward = Vec3::Z;
    for _ in 0..4 {
        wake.advance(1.0 / 60.0, false, &moved, &road);
    }
    assert_eq!(wake.live(), 0, "an arc 500 units astern is shed at once");
}

#[test]
fn the_jitter_scales_are_the_measured_sine_bell() {
    for (k, scale) in JITTER_SCALE.iter().enumerate() {
        let want = 0.4 + 0.6 * (k as f64 * std::f64::consts::FRAC_PI_4).sin();
        assert!((f64::from(*scale) - want).abs() < 1e-6, "term {k}");
    }
}

#[test]
fn colours_are_the_brightness_itself_with_no_gain() {
    let mut wake = Wake::new(11);
    wake.advance(0.0, true, &craft(), &road);
    for _ in 0..40 {
        wake.advance(1.0 / 60.0, true, &craft(), &road);
    }
    let arc = *wake.arcs().iter().find(|a| a.life > 0.0).expect("an arc");
    let (mut atlas, mut contact) = (Vec::new(), Vec::new());
    wake.build(
        &craft(),
        Vec3::new(0.0, 6.0, -10.0),
        &mut atlas,
        &mut contact,
    );
    // `MagStripArc_fp` adds `vertex.rgb * tex.rgb * tex.a`: whatever scale the
    // vertex colour carries is the whole gain, so the port may not add one.
    assert_eq!(
        atlas[0].colour,
        [arc.intensity, arc.intensity, arc.intensity, BODY_ALPHA]
    );
    assert_eq!(
        contact[0].colour,
        [arc.glow, arc.glow, arc.glow, CONTACT_ALPHA]
    );
    assert_eq!((BODY_ALPHA, CONTACT_ALPHA), (0.3, 0.25));
}

#[test]
fn brightness_settles_inside_hds_sample_ranges() {
    let mut wake = Wake::new(12);
    wake.advance(0.0, true, &craft(), &road);
    for _ in 0..200 {
        wake.advance(1.0 / 60.0, true, &craft(), &road);
        for arc in wake.arcs().iter().filter(|a| a.life > 0.0) {
            // A slot may still be climbing from zero; it never overshoots the
            // top of its sample range.
            assert!(arc.intensity <= BODY_SAMPLE.1 + 1e-6, "{arc:?}");
            assert!(arc.glow <= CONTACT_SAMPLE.1 + 1e-6, "{arc:?}");
        }
    }
    let settled = wake
        .arcs()
        .iter()
        .filter(|a| a.life > 0.0)
        .map(|a| a.intensity)
        .fold(f32::MAX, f32::min);
    assert!(
        settled > 0.0,
        "a slot that ticks smooths up from its held value"
    );
}

#[test]
fn the_arc_draw_leaves_vertex_alpha_out_of_the_colour() {
    use crate::beam::pipeline::Style;
    assert!(!Style::magstrip(1).vertex_alpha_weights_colour);
    assert!(Style::BEAM.vertex_alpha_weights_colour);
    // No transfer function in the program: the arc adds gamma values as they
    // are, the beam keeps the decode every other additive draw has.
    assert!(!Style::magstrip(1).decodes_source && Style::BEAM.decodes_source);
    let shader = include_str!("../beam.wgsl");
    assert!(shader.contains("mix(1.0, in.colour.a, vertex_alpha_weight)"));
    assert!(shader.contains("linear_out * decode_source"));
}
