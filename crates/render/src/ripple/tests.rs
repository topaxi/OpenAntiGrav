use super::*;

#[test]
fn the_envelope_rises_for_three_tenths_of_a_second_and_is_gone_at_five() {
    assert_eq!(amplitude(0.0), 0.0);
    assert!((amplitude(0.15) - 6.0).abs() < 1e-5);
    assert!((amplitude(RISE_SECONDS) - PEAK_AMPLITUDE).abs() < 1e-5);
    assert!((amplitude(2.65) - 6.0).abs() < 1e-4);
    assert_eq!(amplitude(LIFETIME_SECONDS), 0.0);
    assert_eq!(amplitude(6.0), 0.0);
    assert_eq!(amplitude(-0.1), 0.0);
}

#[test]
fn the_bump_widens_from_25_to_75_units() {
    assert_eq!(half_width(0.0), 25.0);
    assert_eq!(half_width(LIFETIME_SECONDS), 75.0);
}

#[test]
fn the_profile_is_a_raised_cosine() {
    assert_eq!(height(0.0, 12.0, 40.0), 12.0);
    assert!((height(20.0, 12.0, 40.0) - 6.0).abs() < 1e-5);
    assert!((height(-20.0, 12.0, 40.0) - 6.0).abs() < 1e-5);
    assert_eq!(height(40.0, 12.0, 40.0), 0.0);
    assert_eq!(height(-55.0, 12.0, 40.0), 0.0);
}

#[test]
fn a_wave_is_born_ahead_of_the_craft_in_its_own_direction() {
    assert_eq!(Wave::at(100.0, 1.0, 0.0).centre, 115.0);
    assert_eq!(Wave::at(100.0, -1.0, 0.0).centre, 85.0);
}

fn vertex(y: f32) -> GpuVertex {
    GpuVertex {
        position: [0.0, y, 0.0],
        normal: [0.0, 1.0, 0.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        xform: 0,
        lightmap_texcoord: [0.0; 2],
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

/// One span of four vertices at parameters 0, 1/3, 2/3 and a pinned one,
/// its batch at vertices 10..14 of a 20-vertex model, placed at course
/// distance 100 with a 30-unit length on a 1000-unit ring.
fn one_span() -> (Ripple, Vec<GpuVertex>) {
    let span = Span {
        down_start: [0.0, -1.0, 0.0],
        down_end: [0.0, -1.0, 0.0],
        forward: [None, None],
        backward: [None, None],
        length: 30.0,
        batch: 0x40,
        first_position: 0x48,
        path: 0,
        t_start: 0.0,
        t_end: 0.1,
        parameters: vec![Some(0.0), Some(1.0 / 3.0), Some(2.0 / 3.0), None],
    };
    let placement = BatchPlacement {
        header: 0x40,
        vertices: 10..14,
        to_world: oag_vex::vex::IDENTITY,
    };
    let (ripple, built) =
        Ripple::build(&[span], &[placement], 1000.0, |_, p| Some(100.0 + p * 30.0));
    assert_eq!(
        built,
        Built {
            placed: 1,
            unplaced: 0,
            vertices: 3
        }
    );
    (ripple, (0..20).map(|_| vertex(5.0)).collect())
}

#[test]
fn the_road_rises_under_the_bump_and_settles_back_exactly() {
    let (mut ripple, base) = one_span();
    let mut scratch = Vec::new();
    let wave = Wave {
        centre: 110.0,
        amplitude: 12.0,
        half_width: 40.0,
    };
    let mut written = Vec::new();
    ripple.update(Some(wave), &base, &mut scratch, |first, vertices| {
        written.push((first, vertices.to_vec()));
    });
    assert_eq!(written.len(), 1);
    let (first, vertices) = &written[0];
    assert_eq!(*first, 10);
    // At 100, 110 and 120 against a centre at 110: 10 units either side and
    // the peak, and the pinned fourth vertex not at all.
    let expected = [
        5.0 + height(-10.0, 12.0, 40.0),
        17.0,
        5.0 + height(10.0, 12.0, 40.0),
        5.0,
    ];
    for (v, want) in vertices.iter().zip(expected) {
        assert!(
            (v.position[1] - want).abs() < 1e-4,
            "{} vs {want}",
            v.position[1]
        );
    }

    // Gone: the span is rewritten once more, undisplaced, then left alone.
    written.clear();
    ripple.update(None, &base, &mut scratch, |first, vertices| {
        written.push((first, vertices.to_vec()));
    });
    assert_eq!(written.len(), 1);
    assert!(written[0].1.iter().all(|v| v.position == [0.0, 5.0, 0.0]));
    written.clear();
    ripple.update(None, &base, &mut scratch, |first, vertices| {
        written.push((first, vertices.to_vec()));
    });
    assert!(written.is_empty());
}

#[test]
fn a_span_the_bump_does_not_reach_is_never_written() {
    let (mut ripple, base) = one_span();
    let mut scratch = Vec::new();
    let mut calls = 0;
    let wave = Wave {
        centre: 500.0,
        amplitude: 12.0,
        half_width: 40.0,
    };
    ripple.update(Some(wave), &base, &mut scratch, |_, _| calls += 1);
    assert_eq!(calls, 0);
}

#[test]
fn the_bump_reaches_across_the_start_line() {
    let (mut ripple, base) = one_span();
    let mut scratch = Vec::new();
    let mut calls = 0;
    // A lap on from 110: the same place, reached only by wrapping.
    let wave = Wave {
        centre: 1110.0,
        amplitude: 12.0,
        half_width: 40.0,
    };
    ripple.update(Some(wave), &base, &mut scratch, |_, _| calls += 1);
    assert_eq!(calls, 1);
}

#[test]
fn displace_agrees_with_what_update_wrote() {
    let (mut ripple, base) = one_span();
    let mut scratch = Vec::new();
    let wave = Wave {
        centre: 112.0,
        amplitude: 9.0,
        half_width: 30.0,
    };
    let mut written = Vec::new();
    ripple.update(Some(wave), &base, &mut scratch, |_, vertices| {
        written = vertices.to_vec()
    });
    for (k, want) in written.iter().enumerate() {
        let mut v = base[10 + k];
        ripple.displace(10 + k as u32, &mut v);
        assert_eq!(v.position, want.position);
    }
    // Outside every span, untouched.
    let mut v = base[3];
    ripple.displace(3, &mut v);
    assert_eq!(v.position, base[3].position);
}

/// Where paths meet, one batch belongs to two spans, and the original adds
/// both displacements on its vertices - measured live, to the short.
#[test]
fn two_spans_sharing_a_batch_add() {
    let span = |t_start: f32| Span {
        down_start: [0.0, -1.0, 0.0],
        down_end: [0.0, -1.0, 0.0],
        forward: [None, None],
        backward: [None, None],
        length: 30.0,
        batch: 0x40,
        first_position: 0x48,
        path: 0,
        t_start,
        t_end: t_start + 0.1,
        parameters: vec![Some(0.5)],
    };
    let placement = BatchPlacement {
        header: 0x40,
        vertices: 0..1,
        to_world: oag_vex::vex::IDENTITY,
    };
    let (mut ripple, built) =
        Ripple::build(&[span(0.0), span(0.5)], &[placement], 1000.0, |i, p| {
            Some(if i == 0 { 100.0 } else { 110.0 } + p * 30.0)
        });
    assert_eq!((built.placed, ripple.len()), (2, 1));
    let base = vec![vertex(0.0)];
    let wave = Wave {
        centre: 120.0,
        amplitude: 12.0,
        half_width: 40.0,
    };
    let mut y = 0.0;
    ripple.update(Some(wave), &base, &mut Vec::new(), |_, v| {
        y = v[0].position[1]
    });
    // One owner puts the vertex at 115, the other at 125: 5 units either side.
    let want = 2.0 * height(5.0, 12.0, 40.0);
    assert!((y - want).abs() < 1e-4, "{y} vs {want}");
}
