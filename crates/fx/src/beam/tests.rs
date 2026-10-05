use super::*;
use oag_core::math::Vec3;

const DT: f32 = 1.0 / 60.0;

fn rng() -> Rng {
    Rng::new(1)
}

/// A beam fired down `-Z` by an upright craft, seen by a camera behind it.
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

/// The straight-line chain: no track to locate on.
fn build(ribbon: &Ribbon, frame: &Frame) -> Vec<GpuVertex> {
    super::build(ribbon, frame, &|_| None)
}

fn position(vertex: &GpuVertex) -> Vec3 {
    Vec3::from_array(vertex.position)
}

#[test]
fn segment_count_matches_the_recovered_formula() {
    // `ceil((6.0/range)*min(distance,range)*6.0)`, `range = 250` (Race table).
    assert_eq!(segment_count(0.0, 250.0), 1);
    assert_eq!(segment_count(250.0, 250.0), 36);
    // Past range, the clamp holds it at the same 36 rather than growing.
    assert_eq!(segment_count(10_000.0, 250.0), 36);
    assert!(segment_count(125.0, 250.0) < segment_count(250.0, 250.0));
}

#[test]
fn a_non_positive_range_does_not_divide_by_zero() {
    assert_eq!(segment_count(10.0, 0.0), 1);
    assert_eq!(segment_count(10.0, -5.0), 1);
}

#[test]
fn coincident_craft_draw_nothing() {
    let ribbon = Ribbon::new(&mut rng());
    assert!(build(&ribbon, &frame(Vec3::ZERO)).is_empty());
}

#[test]
fn the_vertex_count_is_submit_strips_own() {
    let ribbon = Ribbon::new(&mut rng());
    let target = Vec3::new(0.0, 0.0, -100.0);
    let segments = segment_count(100.0, 250.0) as usize;
    let vertices = build(&ribbon, &frame(target));
    // `(segment_count * 2 + 2) * 2`.
    assert_eq!(vertices.len(), (segments * 2 + 2) * 2);
    assert!(vertices.len() <= MAX_VERTICES);
}

#[test]
fn the_two_strips_widen_along_the_cameras_right_and_up() {
    let ribbon = Ribbon::new(&mut rng());
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -100.0)));
    let segments = segment_count(100.0, 250.0) as usize;
    // Pair 1 of the first strip, and pair 1 of the second.
    let first = position(&vertices[3]) - position(&vertices[2]);
    let second = 2 * (segments + 2);
    let second = position(&vertices[second + 1]) - position(&vertices[second]);
    assert!(
        (first - Vec3::X * 2.0 * HALF_WIDTH).length() < 1e-4,
        "{first}"
    );
    assert!(
        (second - Vec3::Y * 2.0 * HALF_WIDTH).length() < 1e-4,
        "{second}"
    );
}

#[test]
fn both_strips_share_one_displaced_chain() {
    let ribbon = Ribbon::new(&mut rng());
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -100.0)));
    let segments = segment_count(100.0, 250.0) as usize;
    for i in 0..segments.saturating_sub(1) {
        let a = (position(&vertices[2 * i]) + position(&vertices[2 * i + 1])) / 2.0;
        let pair = segments + 1 + i;
        let b = (position(&vertices[2 * pair]) + position(&vertices[2 * pair + 1])) / 2.0;
        assert!((a - b).length() < 1e-4, "point {i}: {a} vs {b}");
    }
}

#[test]
fn the_chain_is_displaced_off_the_straight_line_but_starts_on_the_shooter() {
    let ribbon = Ribbon::new(&mut rng());
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -250.0)));
    let segments = segment_count(250.0, 250.0) as usize;
    let centre =
        |pair: usize| (position(&vertices[2 * pair]) + position(&vertices[2 * pair + 1])) / 2.0;
    assert!(centre(0).length() < 1e-4);
    // Off the Z axis somewhere, and never by more than the two amplitudes
    // together allow.
    let offsets: Vec<f32> = (1..segments)
        .map(|i| centre(i).truncate().length())
        .collect();
    assert!(offsets.iter().any(|&d| d > 0.05), "{offsets:?}");
    let bound = AMPLITUDE_MAX * (1.0 + SECOND_AXIS_SCALE) + 1e-3;
    assert!(offsets.iter().all(|&d| d <= bound), "{offsets:?}");
}

#[test]
fn alpha_is_zero_at_the_first_and_second_to_last_points_and_full_at_the_target() {
    let ribbon = Ribbon::new(&mut rng());
    let mut f = frame(Vec3::new(0.0, 0.0, -300.0));
    f.alpha = 0.42;
    let vertices = build(&ribbon, &f);
    let n = segment_count(300.0, 250.0) as usize;
    let alpha = |pair: usize| vertices[2 * pair].colour[3];
    for strip in 0..2 {
        let base = strip * (n + 1);
        assert_eq!(alpha(base), 0.0);
        assert_eq!(alpha(base + 1), 0.42);
        assert_eq!(alpha(base + n - 1), 0.0);
    }
    // Only the second strip reaches the last point, at the caller's alpha.
    assert_eq!(alpha(2 * n + 1), 0.42);
}

#[test]
fn the_bridge_between_the_strips_has_zero_area() {
    let ribbon = Ribbon::new(&mut rng());
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -100.0)));
    let n = segment_count(100.0, 250.0) as usize;
    assert_eq!(vertices[2 * n].position, vertices[2 * n - 1].position);
    assert_eq!(vertices[2 * n + 1].position, vertices[2 * n + 2].position);
}

#[test]
fn u_flips_every_pair_and_v_runs_across_the_strip() {
    let ribbon = Ribbon::new(&mut rng());
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -100.0)));
    for (pair, rails) in vertices.chunks(2).enumerate() {
        let u = (pair & 1) as f32;
        assert_eq!(rails[0].texcoord, [u, 0.0], "pair {pair}");
        assert_eq!(rails[1].texcoord, [u, 1.0], "pair {pair}");
    }
}

#[test]
fn the_scroll_phase_slides_u_and_stays_in_zero_one() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    for _ in 0..1000 {
        ribbon.advance(DT, 100.0, 250.0, &mut r);
        assert!((0.0..1.0).contains(&ribbon.scroll_phase));
    }
    let vertices = build(&ribbon, &frame(Vec3::new(0.0, 0.0, -100.0)));
    assert_eq!(vertices[0].texcoord[0], ribbon.scroll_phase);
}

#[test]
fn the_kinks_travel_one_segment_a_tick() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    let f = frame(Vec3::new(0.0, 0.0, -250.0));
    // Past the first pulse, whose re-roll would change a bucket under us.
    ribbon.advance(DT, 250.0, 250.0, &mut r);
    ribbon.advance(DT, 250.0, 250.0, &mut r);
    let before = build(&ribbon, &f);
    ribbon.advance(DT, 250.0, 250.0, &mut r);
    let after = build(&ribbon, &f);
    // The first axis is `X` for this frame: point `i + 1`'s push last tick is
    // point `i`'s now, less the step between them.
    let push =
        |v: &[GpuVertex], i: usize| (position(&v[2 * i]).x + position(&v[2 * i + 1]).x) / 2.0;
    for i in 1..30 {
        assert!(
            (push(&after, i) - push(&before, i + 1)).abs() < 1e-4,
            "point {i}"
        );
    }
}

#[test]
fn the_pulse_block_runs_every_wrap_and_rearms_the_strength_once_a_second() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    let segments = segment_count(50.0, 250.0);
    let mut pulses = Vec::new();
    let mut rearmed = Vec::new();
    let mut last = ribbon.last_pulse;
    for tick in 0..240 {
        if ribbon.advance(DT, 50.0, 250.0, &mut r) {
            pulses.push(tick);
        }
        if ribbon.last_pulse != last {
            rearmed.push(tick);
            last = ribbon.last_pulse;
        }
    }
    assert_eq!(pulses[0], 0, "the first tick is a pulse");
    assert!(
        pulses.windows(2).all(|w| w[1] - w[0] == segments),
        "{pulses:?}"
    );
    assert_eq!(rearmed[0], 0);
    for w in rearmed.windows(2) {
        assert!(w[1] - w[0] > 60, "{rearmed:?}");
        assert!(w[1] - w[0] <= 60 + segments, "{rearmed:?}");
    }
}

#[test]
fn the_pulse_strength_is_the_hull_overlays_own_ramp() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    // The pulse tick itself draws nothing (`fade > 0` gate).
    ribbon.advance(DT, 50.0, 250.0, &mut r);
    assert_eq!(ribbon.pulse_strength(), None);
    for _ in 0..30 {
        ribbon.advance(DT, 50.0, 250.0, &mut r);
    }
    let strength = ribbon.pulse_strength().expect("inside the window");
    assert!((strength - 1.0).abs() < 1e-3, "{strength}");
}

#[test]
fn the_energy_point_walks_from_the_target_to_the_shooter() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    let target = Vec3::new(0.0, 0.0, -50.0);
    let segments = segment_count(50.0, 250.0);
    let step = 50.0 / segments as f32;
    let mut seen = Vec::new();
    for _ in 0..segments {
        ribbon.advance(DT, 50.0, 250.0, &mut r);
        seen.push(
            -ribbon
                .energy_point(Vec3::ZERO, target, 250.0, &|_| None)
                .unwrap()
                .z,
        );
    }
    assert!((seen[0] - (50.0 - step)).abs() < 1e-3, "{seen:?}");
    assert!(seen.last().unwrap().abs() < 1e-3, "{seen:?}");
    assert!(seen.windows(2).all(|w| w[1] < w[0]), "{seen:?}");
}

#[test]
fn only_a_short_beam_rerolls_and_only_one_fixed_bucket() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    let before = ribbon.amplitudes;
    // Full range: 36 segments, bucket `ceil(35 / 3) + 1 = 13` is past the
    // table, so the original re-rolls nothing.
    for _ in 0..200 {
        ribbon.advance(DT, 250.0, 250.0, &mut r);
    }
    assert_eq!(before, ribbon.amplitudes);
    // Thirty segments: bucket `ceil(29 / 3) + 1 = 11`, and only it.
    let distance = 29.5 / 36.0 * 250.0;
    assert_eq!(segment_count(distance, 250.0), 30);
    for _ in 0..200 {
        ribbon.advance(DT, distance, 250.0, &mut r);
    }
    for (i, (was, now)) in before.iter().zip(ribbon.amplitudes).enumerate() {
        assert_eq!(*was != now, i == 11, "bucket {i}");
    }
}

#[test]
fn amplitude_draws_stay_in_the_recovered_range_and_the_extra_entry_is_the_half_width() {
    let ribbon = Ribbon::new(&mut rng());
    for amplitude in &ribbon.amplitudes[..AMPLITUDE_BUCKETS] {
        assert!((0.0..=AMPLITUDE_MAX).contains(amplitude));
    }
    assert_eq!(ribbon.amplitudes[AMPLITUDE_BUCKETS], HALF_WIDTH);
}

#[test]
fn a_beam_fired_along_the_shooters_own_axes_still_builds_finite_geometry() {
    // The step parallel to `up` makes the first axis degenerate - the
    // original's own zero-length guard - and nothing may go non-finite.
    let ribbon = Ribbon::new(&mut rng());
    for target in [Vec3::new(0.0, 100.0, 0.0), Vec3::new(100.0, 0.0, 0.0)] {
        let vertices = build(&ribbon, &frame(target));
        assert!(!vertices.is_empty());
        assert!(vertices.iter().flat_map(|v| v.position).all(f32::is_finite));
    }
}

mod hd_ball {
    use oag_core::math::Vec3;

    use crate::beam::hd_ball::{
        PERIOD_MAX_LENGTH, PERIOD_MAX_SECONDS, PERIOD_MIN_LENGTH, PERIOD_MIN_SECONDS, advance,
        period, position,
    };

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn period_is_clamped_at_both_ends_of_the_recovered_range() {
        assert_eq!(period(0.0), PERIOD_MIN_SECONDS);
        assert_eq!(period(PERIOD_MIN_LENGTH), PERIOD_MIN_SECONDS);
        assert_eq!(period(PERIOD_MAX_LENGTH), PERIOD_MAX_SECONDS);
        assert_eq!(period(10_000.0), PERIOD_MAX_SECONDS);
        // Halfway between the two lengths is halfway between the two
        // periods - the remap is linear, not eased.
        let midpoint = (PERIOD_MIN_LENGTH + PERIOD_MAX_LENGTH) / 2.0;
        let expected = (PERIOD_MIN_SECONDS + PERIOD_MAX_SECONDS) / 2.0;
        assert!((period(midpoint) - expected).abs() < 1e-6);
    }

    #[test]
    fn a_negative_length_reads_the_same_as_its_magnitude() {
        // `LeachBall_Advance` takes `ABS(param_4[4])` before the clamp.
        assert_eq!(period(-50.0), period(50.0));
    }

    #[test]
    fn advance_wraps_exactly_once_a_period_and_resets_the_remainder() {
        let mut elapsed = 0.0;
        let length = PERIOD_MIN_LENGTH; // period == PERIOD_MIN_SECONDS == 0.3s
        let ticks_per_period = (PERIOD_MIN_SECONDS / DT).round() as u32;
        let mut wraps = 0;
        for tick in 0..ticks_per_period * 3 {
            if advance(&mut elapsed, DT, length) {
                wraps += 1;
                // The remainder never grows past one tick's worth of
                // overshoot - the wrap subtracts a whole period, not
                // resetting to zero.
                assert!(elapsed < DT + 1e-6, "tick {tick}: elapsed {elapsed}");
            }
        }
        assert_eq!(wraps, 3);
    }

    #[test]
    fn position_reaches_the_owner_only_at_the_wrap_and_the_target_at_the_start() {
        let owner = Vec3::new(10.0, 0.0, 0.0);
        let target = Vec3::ZERO;
        let length = (owner - target).length();
        assert_eq!(position(0.0, length, owner, target), target);
        assert_eq!(position(period(length), length, owner, target), owner);
        let halfway = position(period(length) / 2.0, length, owner, target);
        assert!((halfway - (owner + target) / 2.0).length() < 1e-4);
    }

    #[test]
    fn the_arc_adds_its_fragment_alpha_into_the_frame_alpha_and_the_ribbon_does_not() {
        // `Rsx_SetBlendFunc` writes one factor for both channels, so the arc's
        // `ONE, ONE` is the alpha function too, and `MagStripArc_fp`'s
        // `a = vertex.a * tex.a` is what it adds (the glow mask HD's bloom gate
        // reads). The LeachBeam replaces alpha with its constant stamp instead.
        let arc = super::pipeline::Style::magstrip(8);
        assert!(arc.alpha_is_fragment);
        assert_eq!(arc.blend.alpha.src_factor, ::wgpu::BlendFactor::One);
        assert_eq!(arc.blend.alpha.dst_factor, ::wgpu::BlendFactor::One);
        assert_eq!(arc.blend.alpha.operation, ::wgpu::BlendOperation::Add);
        assert!(!super::pipeline::Style::BEAM.alpha_is_fragment);
    }
}
