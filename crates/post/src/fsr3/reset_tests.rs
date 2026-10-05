//! What a reset into targets that already hold a history does - the
//! race-restart case, which a fresh allocation cannot exercise because wgpu
//! zero-initialises it.
//!
//! Its own file rather than more of `tests.rs`, which is already past 900
//! lines against `just check-size`'s 1,000-line ratchet.

use super::readback::{self, Scene};
use super::tests::{FIXTURE, colour_pattern, depth_pattern};

#[test]
fn a_reset_reads_no_history_even_when_the_targets_still_hold_one() {
    // Three still frames into one set of targets, the third a reset. After
    // two frames the accumulation texture the third frame reads holds two
    // thirds of a frame at every texel - `prepare_reactivity` stores one
    // step ahead - and upstream's `resetAccumulation` clears that texture
    // before the frame runs. Forgetting the previous constants alone does
    // not: `frame_index == 0` stops `accumulate` reprojecting a history
    // *colour*, but the history *weight* still comes off that texture, and
    // a zeroed colour blended in at two thirds is a darkened frame.
    //
    // The direct statement is read off `prepare_reactivity`'s own output:
    // the accumulation channel of the masks the reset frame computed, which
    // is what `accumulate` then weights the history by.
    let depth = depth_pattern();
    let colour = colour_pattern();
    let still = readback::Input {
        depth: &depth,
        colour: &colour,
    };
    let Some(scene) = Scene::run_resetting(FIXTURE, &[still; 3], false, |index| {
        index == 0 || index == 2
    }) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let constants = scene.fsr3.constants().expect("a dispatched frame");
    assert_eq!(
        constants.frame_index, 0.0,
        "the third frame was a reset, so it is frame zero of a new sequence"
    );
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let step = 1.0 / 255.0;
    for (index, masks) in scene
        .read_rgba_unorm(&targets.dilated_reactive_masks.texture)
        .iter()
        .enumerate()
    {
        assert!(
            masks[3] < step,
            "texel {index} read {} frames of history on a reset frame - the \
             previous sequence's accumulation survived the reset",
            masks[3]
        );
    }

    // And the picture is the one a genuinely fresh first frame produces: the
    // same three inputs run on a fresh allocation, read at *its* first frame.
    // Both are initial-sample frames of the same input, so `accumulate` takes
    // the same path in each and the two resolves must agree texel for texel.
    let Some(fresh) = Scene::run(FIXTURE, &[still]) else {
        return;
    };
    let read = |scene: &Scene| {
        let output = scene
            .fsr3
            .output_texture()
            .expect("a complete chain has an output");
        scene.read_rgba16(output)
    };
    let (reset, fresh) = (read(&scene), read(&fresh));
    let mut worst = 0.0f32;
    for (a, b) in reset.iter().zip(&fresh) {
        for channel in 0..3 {
            worst = worst.max((a[channel] - b[channel]).abs());
        }
    }
    assert!(
        worst < 1e-3,
        "a reset frame differs from a fresh first frame by {worst} at worst - \
         a history that should have been thrown away is being blended in"
    );
}
