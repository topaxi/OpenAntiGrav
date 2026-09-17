use super::*;
use oag_core::math::Vec3;

fn rng() -> Rng {
    Rng::new(1)
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
    let vertices = build(&ribbon, Vec3::ZERO, Vec3::ZERO, 250.0, 1.0);
    assert!(vertices.is_empty());
}

#[test]
fn the_strip_is_two_crossed_ribbons_of_matching_length() {
    let ribbon = Ribbon::new(&mut rng());
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(100.0, 0.0, 0.0);
    let range = 250.0;
    let vertices = build(&ribbon, owner, target, range, 1.0);

    let segments = segment_count((target - owner).length(), range) as usize;
    // Two strips, `segments + 1` pairs each - `LeachBeam_SubmitStrip`'s own
    // `(segment_count*2+2)*2` vertex count.
    assert_eq!(vertices.len(), 4 * (segments + 1));
    assert!(vertices.len() <= MAX_VERTICES);
}

#[test]
fn both_chain_endpoints_are_alpha_zero_on_every_strip() {
    let ribbon = Ribbon::new(&mut rng());
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(300.0, 0.0, 0.0);
    let vertices = build(&ribbon, owner, target, 250.0, 1.0);
    let segments = segment_count((target - owner).length(), 250.0) as usize;
    let pairs_per_strip = segments + 1;

    for strip in 0..2 {
        let base = strip * pairs_per_strip * 2;
        // First pair of this strip.
        assert_eq!(vertices[base].colour[3], 0.0);
        assert_eq!(vertices[base + 1].colour[3], 0.0);
        // Last pair of this strip.
        let last = base + (pairs_per_strip - 1) * 2;
        assert_eq!(vertices[last].colour[3], 0.0);
        assert_eq!(vertices[last + 1].colour[3], 0.0);
    }
}

#[test]
fn interior_vertices_carry_the_callers_alpha() {
    let ribbon = Ribbon::new(&mut rng());
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(300.0, 0.0, 0.0);
    let vertices = build(&ribbon, owner, target, 250.0, 0.42);
    let segments = segment_count((target - owner).length(), 250.0) as usize;
    assert!(
        segments > 2,
        "need an interior segment for this to test anything"
    );
    // Index 2/3 is the second pair of the first strip - interior when
    // `segments > 1`.
    assert_eq!(vertices[2].colour[3], 0.42);
    assert_eq!(vertices[3].colour[3], 0.42);
}

#[test]
fn the_two_rails_of_a_pair_straddle_the_spine_by_the_half_width() {
    let ribbon = Ribbon::new(&mut rng());
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(100.0, 0.0, 0.0);
    let vertices = build(&ribbon, owner, target, 250.0, 1.0);
    // A pair's two rails are `2 * HALF_WIDTH` apart, whatever the wiggle did
    // to the shared centre.
    let a = Vec3::from_array(vertices[0].position);
    let b = Vec3::from_array(vertices[1].position);
    assert!(((a - b).length() - 2.0 * HALF_WIDTH).abs() < 1e-4);
}

#[test]
fn a_beam_fired_in_any_direction_still_builds_two_finite_axes() {
    // The straight-up case is the one `cross_axes` has a special branch for.
    let ribbon = Ribbon::new(&mut rng());
    let owner = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(0.0, 100.0, 0.0);
    let vertices = build(&ribbon, owner, target, 250.0, 1.0);
    assert!(!vertices.is_empty());
    for vertex in &vertices {
        for component in vertex.position {
            assert!(component.is_finite());
        }
    }
}

#[test]
fn advance_wraps_the_scroll_phase_into_zero_one() {
    let mut ribbon = Ribbon::new(&mut rng());
    let mut r = rng();
    for _ in 0..1000 {
        ribbon.advance(1.0 / 60.0, &mut r);
        assert!((0.0..1.0).contains(&ribbon.scroll_phase));
    }
}

#[test]
fn advance_rerolls_every_bucket_at_least_once_given_enough_time() {
    let mut r = rng();
    let mut ribbon = Ribbon::new(&mut r);
    let before = ribbon.amplitudes;
    // Continuing the same stream rather than a fresh `Rng::new(1)` - two
    // instances seeded alike would replay construction's own draws and make
    // every reroll land on the value it is replacing, passing this test for
    // the wrong reason.
    //
    // AMPLITUDE_BUCKETS seconds at a one-second cadence covers every bucket
    // once, with a tick of slack for the timer's own rounding.
    let ticks = ((AMPLITUDE_BUCKETS as f32 + 1.0) / (1.0 / 60.0)) as u32;
    for _ in 0..ticks {
        ribbon.advance(1.0 / 60.0, &mut r);
    }
    assert_ne!(before, ribbon.amplitudes);
}

#[test]
fn amplitude_draws_stay_in_the_recovered_range() {
    let ribbon = Ribbon::new(&mut rng());
    for amplitude in ribbon.amplitudes {
        assert!((0.0..=AMPLITUDE_MAX).contains(&amplitude));
    }
}
