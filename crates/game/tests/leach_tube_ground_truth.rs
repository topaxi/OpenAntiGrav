//! The LeachBeam's track tube on Talon's Junction's own spline.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(leach_tube_ground_truth)'
//! ```
//!
//! `LeachBeam_KeepInTrack` bends the chain so it does not cut a corner through
//! the wall. This finds, on the real circuit, the pair of spline points a beam
//! could span whose straight line leaves the road by the most, and checks that
//! the chain the renderer builds from [`oag_raceplay::Spline::tube_frame`]
//! stays in the tube where the straight line does not. It fails if the
//! locator stops being wired (the bent chain would equal the straight one) or
//! if the tube stops holding.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_fx::beam::tube;
use oag_raceplay as race;
use oag_raceplay::Spline;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// How far `point` sits outside the tube's walls, `0` when inside: the
/// lateral distance past `half_width - 2.0` on the nearest sample's frame.
fn overshoot(spline: &Spline, point: Vec3) -> f32 {
    let Some(frame) = spline.tube_frame(point) else {
        return 0.0;
    };
    let across = (point - frame.pos).dot(frame.lateral);
    let wall = if across > 0.0 {
        frame.half_width_right
    } else {
        frame.half_width_left
    } - tube::EDGE_MARGIN;
    (across.abs() - wall).max(0.0)
}

fn worst(spline: &Spline, chain: &[Vec3]) -> f32 {
    chain
        .iter()
        .map(|&point| overshoot(spline, point))
        .fold(0.0, f32::max)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_beam_across_a_real_corner_stays_in_the_tube() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        ..race::Options::default()
    })
    .expect("the default track loads");
    let spline = &loaded.setup.spline;

    // Every chord of about 100 units between two samples on one path; keep the
    // one whose straight line is furthest out of the road.
    let segments = oag_fx::beam::segment_count(100.0, 250.0);
    let mut worst_chord: Option<(f32, usize, usize)> = None;
    for from in 0..spline.len() {
        let Some(a) = spline.sample(from) else {
            continue;
        };
        let owner = Spline::track_sample(a).position;
        let Some(to) = (from + 1..spline.len().min(from + 80)).find(|&to| {
            spline.path_of(to) == spline.path_of(from)
                && spline
                    .sample(to)
                    .is_some_and(|b| (Spline::track_sample(b).position - owner).length() >= 100.0)
        }) else {
            continue;
        };
        let target = Spline::track_sample(spline.sample(to).unwrap()).position;
        let straight = tube::walk(owner, target, segments, &|_| None);
        let out = worst(spline, &straight);
        if worst_chord.is_none_or(|(best, _, _)| out > best) {
            worst_chord = Some((out, from, to));
        }
    }
    let (straight_out, from, to) = worst_chord.expect("the circuit has samples");
    let owner = Spline::track_sample(spline.sample(from).unwrap()).position;
    let target = Spline::track_sample(spline.sample(to).unwrap()).position;

    let straight = tube::walk(owner, target, segments, &|_| None);
    let bent = tube::walk(owner, target, segments, &|p| spline.tube_frame(p));
    let bent_out = worst(spline, &bent);
    println!(
        "worst chord {from}->{to}: straight leaves the tube by {straight_out:.2}, bent by {bent_out:.2}"
    );

    assert!(
        straight_out > 2.0,
        "the circuit should have a corner this beam cuts: {straight_out}"
    );
    assert_ne!(bent, straight, "the locator must reach the chain");
    assert!(
        bent_out < straight_out * 0.5 && bent_out < 1.5,
        "the bend should hold the chain in the tube: {bent_out} vs {straight_out}"
    );
}
