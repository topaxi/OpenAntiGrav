use super::*;

fn rec(anchor: Vec3, progress: f32) -> Record {
    Record {
        anchor,
        progress,
        row0: Vec3::X,
        row1: Vec3::Y,
    }
}

fn line(n: usize, step: f32) -> Vec<Record> {
    (0..n)
        .map(|i| rec(Vec3::new(i as f32 * step, 0.0, 0.0), i as f32))
        .collect()
}

#[test]
fn a_record_is_laid_only_past_the_spacing() {
    let mut trail = AnchorTrail::new();
    trail.record(rec(Vec3::ZERO, 0.0));
    trail.record(rec(Vec3::new(3.4, 0.0, 0.0), 1.0));
    assert_eq!(trail.len(), 1, "3.4 from the newest is not more than 3.4");
    trail.record(rec(Vec3::new(3.41, 0.0, 0.0), 1.0));
    assert_eq!(trail.len(), 2);
    let newest: Vec<f32> = trail.newest_first().map(|r| r.anchor.x).collect();
    assert_eq!(newest, vec![3.41, 0.0]);
}

#[test]
fn the_trail_wraps_and_saturates_below_the_ring() {
    let mut trail = AnchorTrail::new();
    for i in 0..(RING * 2) {
        trail.record(rec(Vec3::new(i as f32 * 4.0, 0.0, 0.0), i as f32));
    }
    assert_eq!(trail.len(), RING - 1);
    let first = trail.newest_first().next().unwrap();
    assert_eq!(first.progress, (RING * 2 - 1) as f32);
}

#[test]
fn the_anchor_is_pushed_along_row_one_by_minus_one_point_seven() {
    let a = anchor_point(Vec3::new(1.0, 2.0, 3.0), Vec3::Y);
    assert!((a - Vec3::new(1.0, 0.3, 3.0)).length() < 1e-5);
}

#[test]
fn the_walk_runs_back_to_the_shooters_progress_and_ends_interpolated() {
    let mut target = AnchorTrail::new();
    for i in 0..10 {
        target.record(rec(Vec3::new(i as f32 * 4.0, 0.0, 0.0), i as f32 * 4.0));
    }
    let leader = rec(Vec3::new(40.0, 0.0, 0.0), 40.0);
    let shooter_anchor = Vec3::new(10.0, 5.0, 0.0);
    let samples = walk(&target, leader, shooter_anchor, 10.0);
    assert_eq!(samples[0], leader);
    assert!(samples.len() >= 8);
    assert!((samples.last().unwrap().anchor.x - 10.0).abs() < 1e-4);

    let between = walk(&target, leader, shooter_anchor, 9.0);
    assert!(
        (between.last().unwrap().anchor.x - 9.0).abs() < 1e-4,
        "the last point sits at the shooter's progress: {between:?}"
    );
}

#[test]
fn the_bend_pins_both_ends_and_moves_the_middle_by_length_fraction() {
    let mut samples = line(5, 10.0);
    let shooter = Vec3::new(40.0, 8.0, 0.0);
    bend(&mut samples, shooter);
    assert_eq!(samples[0].anchor, Vec3::ZERO);
    assert!((samples[4].anchor - shooter).length() < 1e-5);
    assert!((samples[2].anchor.y - 4.0).abs() < 1e-5, "{:?}", samples[2]);
}

#[test]
fn the_timeline_is_ball_then_reveal_then_held() {
    assert_eq!(Phase::at(0.39).reveal(), None);
    let (reveal, window) = Phase::at(0.55).reveal().unwrap();
    assert!((reveal - 0.5).abs() < 1e-5 && window == REVEAL_WINDOW);
    assert_eq!(Phase::at(0.71).reveal(), Some((1.0, 0.0)));
}

#[test]
fn the_reveal_whitens_the_target_end_first() {
    let shades: Vec<u8> = (0..5).map(|i| colour(i, 5, 0.5, 0.001)).collect();
    assert_eq!(shades[0], 255);
    assert_eq!(shades[1], 255);
    assert!(shades[2] > 100 && shades[2] < 160, "{shades:?}");
    assert_eq!(shades[3], 0);
    assert_eq!(shades[4], 0);
    assert!((0..5).all(|i| colour(i, 5, 1.0, 0.0) == 255));
    assert!((0..5).all(|i| colour(i, 5, 0.0, 0.001) == 0));
}

#[test]
fn nodes_start_at_the_shooter_with_u_growing_by_a_twentieth_per_unit() {
    let samples = vec![
        rec(Vec3::new(0.0, 0.0, 0.0), 0.0),
        rec(Vec3::new(5.0, 0.0, 0.0), 1.0),
        rec(Vec3::new(10.0, 1.0, 0.0), 2.0),
    ];
    let out = nodes(&samples, 1.0, 0.0);
    assert_eq!(out[0].position, samples[2].anchor);
    assert_eq!(out[0].u, 0.0);
    let step = (samples[2].anchor - samples[1].anchor).length() * U_PER_UNIT;
    assert!((out[1].u - step).abs() < 1e-6);
    assert_eq!(
        out[2].u, 0.0,
        "the walked end restarts at 0, as the live draws do"
    );
    assert!(out[1].up.dot(out[1].right).abs() < 1e-5);
    assert!(out.iter().all(|n| n.shade == 255));
}

#[test]
fn three_fins_of_six_vertices_per_segment() {
    let out = nodes(&line(6, 4.0), 1.0, 0.0);
    let mut vertices = Vec::new();
    extend_vertices(&mut vertices, &out);
    assert_eq!(vertices.len(), 5 * FINS * 6);
    assert!(vertices.iter().all(|v| v.colour == [1.0; 4]));
}

/// The live capture's own numbers (`data/scratch/hd-leach-draw/c5`, beam of 40
/// samples): progress 0.58344 at sample 0 and 0.55055 at the last, phases all
/// 10.93334, and the stored last sines -0.75139, -0.99273 and 0.22650.
#[test]
fn the_wobble_sines_reproduce_the_live_beams_stored_values() {
    let arc = (0.550_550_34f32 - 0.583_440_6) * 150.0;
    let phase = 10.933_338f32;
    let sines: Vec<f32> = [3.0f32, 2.9, 2.58]
        .iter()
        .map(|f| ((arc + phase) * f).sin())
        .collect();
    assert!((sines[0] + 0.751_385).abs() < 1e-3, "{sines:?}");
    assert!((sines[1] + 0.992_730).abs() < 1e-3, "{sines:?}");
    assert!((sines[2] - 0.226_499).abs() < 1e-3, "{sines:?}");
}

#[test]
fn the_wobble_leaves_sample_zero_and_tapers_to_nothing_at_the_far_end() {
    let mut samples = line(21, 1.0);
    let before = samples.clone();
    let mut phases = [0.0; 3];
    advance_wobble(&mut phases);
    assert!((phases[0] - WOBBLE_PHASE_STEP).abs() < 1e-9);
    wobble(&mut samples, &phases, 100.0);
    assert_eq!(samples[0], before[0]);
    let moved: Vec<f32> = samples
        .iter()
        .zip(&before)
        .map(|(a, b)| (a.anchor - b.anchor).length())
        .collect();
    assert!(moved[10] > moved[1] && moved[10] > moved[20], "{moved:?}");
    assert!(
        moved[10] > 0.5,
        "the middle rises by about its taper: {moved:?}"
    );
}

#[test]
fn the_wobble_moves_the_middle_sample() {
    let before = line(21, 1.0);
    let mut moved = before.clone();
    wobble(&mut moved, &[0.0; 3], 100.0);
    assert!(moved[10].anchor != before[10].anchor);
}
