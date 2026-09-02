use super::*;
use oag_core::math::{Vec3, Vec4, camera};

/// A plain perspective to project through, standing in for a race camera.
fn view_projection() -> Mat4 {
    camera::perspective(60f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
        * Mat4::from_translation(Vec3::new(0.0, 0.0, -20.0))
}

fn ndc(matrix: Mat4, point: Vec3) -> (f32, f32) {
    let clip = matrix * Vec4::new(point.x, point.y, point.z, 1.0);
    (clip.x / clip.w, clip.y / clip.w)
}

#[test]
fn the_halton_terms_are_the_hand_computed_ones() {
    // 1-indexed, per `offset_pixels`. Base 2: 1/2, 1/4, 3/4, 1/8. Base 3:
    // 1/3, 2/3, 1/9, 4/9. Written out rather than generated so a change to
    // the indexing has something to fail against.
    for (index, expected) in [(1, 0.5), (2, 0.25), (3, 0.75), (4, 0.125)] {
        assert!(
            (halton(index, 2) - expected).abs() < 1e-6,
            "halton({index}, 2) = {}, wanted {expected}",
            halton(index, 2)
        );
    }
    for (index, expected) in [
        (1, 1.0 / 3.0),
        (2, 2.0 / 3.0),
        (3, 1.0 / 9.0),
        (4, 4.0 / 9.0),
    ] {
        assert!(
            (halton(index, 3) - expected).abs() < 1e-6,
            "halton({index}, 3) = {}, wanted {expected}",
            halton(index, 3)
        );
    }
}

#[test]
fn no_frame_lands_exactly_on_the_pixel_centre() {
    // Frame 0 taking `halton(0, _)` would, and a phase spent drawing the
    // unjittered image is a phase that reconstructs nothing.
    for frame in 0..PHASES {
        let (x, y) = offset_pixels(frame);
        assert!(
            x.abs() > 1e-6 || y.abs() > 1e-6,
            "frame {frame} sits on the centre"
        );
    }
}

#[test]
fn every_offset_is_inside_the_pixel_and_the_sequence_repeats() {
    for frame in 0..PHASES * 3 {
        let (x, y) = offset_pixels(frame);
        assert!((-0.5..0.5).contains(&x), "frame {frame} x = {x}");
        assert!((-0.5..0.5).contains(&y), "frame {frame} y = {y}");
        assert_eq!(
            offset_pixels(frame),
            offset_pixels(frame + PHASES),
            "the sequence did not repeat at frame {frame}"
        );
    }
}

#[test]
fn the_offsets_straddle_the_pixel_centre_rather_than_filling_one_side() {
    // Forget the centring subtraction and this mean lands near +0.5 rather
    // than near zero, and every jittered frame sits a consistent half-pixel
    // off the unjittered one. Not exactly zero: a 16-term prefix of a Halton
    // sequence is evenly spread, not balanced.
    let (x, y) = (0..PHASES).fold((0.0, 0.0), |(x, y), frame| {
        let (dx, dy) = offset_pixels(frame);
        (x + dx, y + dy)
    });
    let mean = (x / PHASES as f32, y / PHASES as f32);
    assert!(
        mean.0.abs() < 0.05,
        "mean x offset {} is not centred",
        mean.0
    );
    assert!(
        mean.1.abs() < 0.05,
        "mean y offset {} is not centred",
        mean.1
    );
}

#[test]
fn the_matrix_moves_a_point_by_exactly_its_offset_in_pixels() {
    let size = (1280.0, 720.0);
    let point = Vec3::new(3.0, -2.0, 0.0);
    let plain = view_projection();
    for frame in 0..PHASES {
        let (before_x, before_y) = ndc(plain, point);
        let (after_x, after_y) = ndc(matrix(frame, size) * plain, point);
        let (want_x, want_y) = offset_pixels(frame);
        // NDC spans 2 across the target, so a pixel is 2/size of it.
        let moved = (
            (after_x - before_x) * size.0 / 2.0,
            (after_y - before_y) * size.1 / 2.0,
        );
        assert!(
            (moved.0 - want_x).abs() < 1e-4 && (moved.1 - want_y).abs() < 1e-4,
            "frame {frame} moved {moved:?} pixels, wanted ({want_x}, {want_y})"
        );
    }
}

#[test]
fn depth_and_the_view_distance_come_through_untouched() {
    // `z` and `w` are what the depth test, the fog's view distance and the
    // Zone glow's own `clip.z` read. A translation in `x` and `y` must not
    // reach them, or jitter would silently move the depth buffer too.
    let plain = view_projection();
    let point = Vec4::new(3.0, -2.0, 5.0, 1.0);
    for frame in 0..PHASES {
        let before = plain * point;
        let after = (matrix(frame, (640.0, 360.0)) * plain) * point;
        assert!((after.z - before.z).abs() < 1e-5, "frame {frame} moved z");
        assert!((after.w - before.w).abs() < 1e-5, "frame {frame} moved w");
    }
}

#[test]
fn the_same_jitter_on_both_matrices_cancels_out_of_the_velocity() {
    // The claim the whole no-shader-change design rests on: a drawable's
    // screen-space velocity is the difference between its current and previous
    // clip positions after the divide, so shifting both by the same NDC amount
    // leaves that difference alone. Get this wrong - a different frame index on
    // each, or the offset applied to only one - and every pixel's motion vector
    // carries the jitter, which is exactly what a temporal upscaler must not be
    // handed.
    let size = (960.0, 544.0);
    let current = view_projection();
    // A camera that moved and a point that moved with it, so the velocity
    // under test is not trivially zero.
    let previous = camera::perspective(60f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
        * Mat4::from_translation(Vec3::new(0.4, -0.1, -19.3));
    let point = Vec3::new(2.5, 1.0, -4.0);

    let plain = {
        let (cx, cy) = ndc(current, point);
        let (px, py) = ndc(previous, point);
        (cx - px, cy - py)
    };
    for frame in 0..PHASES {
        let jitter = matrix(frame, size);
        let (cx, cy) = ndc(jitter * current, point);
        let (px, py) = ndc(jitter * previous, point);
        let jittered = (cx - px, cy - py);
        assert!(
            (jittered.0 - plain.0).abs() < 1e-5 && (jittered.1 - plain.1).abs() < 1e-5,
            "frame {frame}: velocity {jittered:?} against the unjittered {plain:?}"
        );
    }
}

#[test]
fn a_smaller_render_target_gets_a_proportionally_larger_offset() {
    // The offset is a half-pixel of the pixels actually being rasterised, so
    // halving the render scale doubles it in NDC. Passing the presentation
    // size here instead would under-jitter every scale below 100 %.
    let point = Vec3::new(1.0, 1.0, 0.0);
    let plain = view_projection();
    let (base_x, _) = ndc(plain, point);
    let (full_x, _) = ndc(matrix(3, (1280.0, 720.0)) * plain, point);
    let (half_x, _) = ndc(matrix(3, (640.0, 360.0)) * plain, point);
    let full = full_x - base_x;
    let half = half_x - base_x;
    assert!(
        (half - full * 2.0).abs() < 1e-6,
        "half-scale offset {half} is not twice the full-scale {full}"
    );
}
