//! The order a circuit's stages are drawn in, held where a refactor cannot move it
//! without a test saying so.

use crate::scene::{TRACK_STAGES, TrackStage, track_blended};

fn at(stage: TrackStage) -> usize {
    TRACK_STAGES
        .iter()
        .position(|&s| s == stage)
        .expect("every stage is drawn")
}

/// Ours drew the shine after the blended list, which put an additive veil over every
/// tree standing in front of a shine-mapped wall (Moa Therma, tick 160: a crown over
/// the grandstand read `122,141,134` against the original's `49,50,38`). The original
/// queues the `0x2000` pass with the `0x45`/`0x4a` sets and every list-B mesh at
/// `0x4d`, after them.
#[test]
fn the_shine_pass_is_drawn_before_the_blended_list() {
    assert!(at(TrackStage::Shine) < at(TrackStage::Blended));
    assert!(
        at(TrackStage::Solid) < at(TrackStage::Shine),
        "the shine is tested against the solid depth"
    );
}

/// The circuit's blended list is put in the original's back-to-front order from the
/// view's own depth plane; `None` would leave it in build order.
#[test]
fn the_circuits_blended_list_is_depth_ordered() {
    let plane = [0.0, 0.0, -1.0, 0.0];
    assert!(matches!(
        track_blended(plane),
        crate::Lists::Blended {
            depth_plane: Some(p)
        } if p == plane
    ));
}
