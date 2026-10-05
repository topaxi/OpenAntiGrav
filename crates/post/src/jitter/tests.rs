use super::*;
use oag_core::math::{Vec3, Vec4, camera, frustum::Frustum};

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
    for frame in 0..DEFAULT_PHASES {
        let (x, y) = offset_pixels(frame, DEFAULT_PHASES);
        assert!(
            x.abs() > 1e-6 || y.abs() > 1e-6,
            "frame {frame} sits on the centre"
        );
    }
}

#[test]
fn every_offset_is_inside_the_pixel_and_the_sequence_repeats() {
    for frame in 0..DEFAULT_PHASES * 3 {
        let (x, y) = offset_pixels(frame, DEFAULT_PHASES);
        assert!((-0.5..0.5).contains(&x), "frame {frame} x = {x}");
        assert!((-0.5..0.5).contains(&y), "frame {frame} y = {y}");
        assert_eq!(
            offset_pixels(frame, DEFAULT_PHASES),
            offset_pixels(frame + DEFAULT_PHASES, DEFAULT_PHASES),
            "the sequence did not repeat at frame {frame}"
        );
    }
}

#[test]
fn the_offsets_straddle_the_pixel_centre_rather_than_filling_one_side() {
    // Forget the centring subtraction and this mean lands near +0.5 rather
    // than near zero, and every jittered frame sits a consistent half-pixel
    // off the unjittered one. Not exactly zero: a short prefix of a Halton
    // sequence is evenly spread, not balanced.
    //
    // **The bound is 0.06 because eight phases is a shorter prefix than the
    // sixteen this used to take.** Base 2's first eight terms sum to 3.5625,
    // a mean of -0.0547 once centred, where sixteen of them sum to 7.5 and
    // land on -0.03125. Base 3's first eight sum to exactly 4.0, so y is
    // exactly zero. Widening the bound is the honest response to a shorter
    // sequence, not a test being made to pass: what it is asserting is that
    // the offsets straddle the centre, and a twentieth of a pixel does.
    let (x, y) = (0..DEFAULT_PHASES).fold((0.0, 0.0), |(x, y), frame| {
        let (dx, dy) = offset_pixels(frame, DEFAULT_PHASES);
        (x + dx, y + dy)
    });
    let mean = (x / DEFAULT_PHASES as f32, y / DEFAULT_PHASES as f32);
    assert!(
        mean.0.abs() < 0.06,
        "mean x offset {} is not centred",
        mean.0
    );
    assert!(
        mean.1.abs() < 0.06,
        "mean y offset {} is not centred",
        mean.1
    );
}

#[test]
fn the_matrix_moves_a_point_by_exactly_its_offset_in_pixels() {
    let size = (1280.0, 720.0);
    let point = Vec3::new(3.0, -2.0, 0.0);
    let plain = view_projection();
    for frame in 0..DEFAULT_PHASES {
        let (before_x, before_y) = ndc(plain, point);
        let (after_x, after_y) = ndc(matrix(frame, DEFAULT_PHASES, size) * plain, point);
        let (want_x, want_y) = offset_pixels(frame, DEFAULT_PHASES);
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
    for frame in 0..DEFAULT_PHASES {
        let before = plain * point;
        let after = (matrix(frame, DEFAULT_PHASES, (640.0, 360.0)) * plain) * point;
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
    for frame in 0..DEFAULT_PHASES {
        let jitter = matrix(frame, DEFAULT_PHASES, size);
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
    let (full_x, _) = ndc(matrix(3, DEFAULT_PHASES, (1280.0, 720.0)) * plain, point);
    let (half_x, _) = ndc(matrix(3, DEFAULT_PHASES, (640.0, 360.0)) * plain, point);
    let full = full_x - base_x;
    let half = half_x - base_x;
    assert!(
        (half - full * 2.0).abs() < 1e-6,
        "half-scale offset {half} is not twice the full-scale {full}"
    );
}

#[test]
fn a_jittered_matrix_moves_the_culling_planes_which_is_why_the_frustum_is_built_first() {
    // The reason `race::Scene::render` builds its frustum *before* it applies
    // the offset, and the failure that ordering prevents: the planes move with
    // the matrix, so a bound sitting exactly on the screen edge would fall in
    // and out of the visible set at the sequence's period - geometry flickering
    // along the border, once every sixteen frames, only with jitter on.
    //
    // Exaggerated deliberately: a four-pixel-wide target makes a half-pixel
    // offset a quarter of NDC, which moves a plane far enough to flip a bound
    // that a real 1280-wide offset would move by a thousandth as much. The
    // *direction* of the argument is what is being pinned, not its magnitude.
    let plain = Frustum::from_view_projection(view_projection());
    let jittered =
        Frustum::from_view_projection(matrix(2, DEFAULT_PHASES, (4.0, 4.0)) * view_projection());
    // Frame 2 offsets by -0.25 px in x, which on a four-pixel target is -0.125
    // NDC - so the whole image shifts left and a bound off the left edge is
    // pushed out of view.
    let edge = (-100..100)
        .map(|step| Vec3::new(step as f32 * 0.2, 0.0, 0.0))
        .find(|&centre| {
            plain.intersects_sphere(centre, 0.05) != jittered.intersects_sphere(centre, 0.05)
        });
    assert!(
        edge.is_some(),
        "no bound along the horizon is classified differently, so the offset \
         never reached the frustum's planes"
    );
}

#[test]
fn the_phase_count_is_upstream_s_quadratic_ratio() {
    // `ffxFsr3UpscalerGetJitterPhaseCount`: 8 * (display / render)^2, truncated.
    // Written out at the ratios FSR 3.1's own quality modes produce rather than
    // recomputed from the formula, so a change to the formula has something
    // independent to fail against.
    assert_eq!(phases(1920, 1920), 8); // native, 1.0x
    assert_eq!(phases(1280, 1920), 18); // quality, 1.5x -> 8 * 2.25
    assert_eq!(phases(960, 1920), 32); // performance, 2.0x
    assert_eq!(phases(640, 1920), 72); // ultra performance, 3.0x
}

#[test]
fn a_supersampled_frame_still_gets_a_usable_sequence() {
    // Supersampling is the case upstream's quality modes cannot reach and this
    // renderer can. **It does not reach zero**, and that is worth pinning: the
    // deepest scale `Scale::OFFERED` holds is 200 %, a ratio of one half, which
    // truncates to 2 rather than to 0. Zero needs a ratio below 0.354 - past
    // 283 % - so the `max(1)` guards inputs no setting can produce.
    assert_eq!(phases(3840, 1920), 2);
    // A degenerate size can, though, and one reaches this every time a window
    // is created before it has been given an extent.
    assert_eq!(phases(1920, 0), 1);
    assert_eq!(phases(u32::MAX, 1920), 1);
    // And the guard holds at the point of use, not only at the point of
    // derivation - a zero passed straight in must not divide by zero.
    let _ = offset_pixels(7, 0);
}

#[test]
fn a_longer_sequence_repeats_at_its_own_period_and_not_the_default_one() {
    // The property `DEFAULT_PHASES` is checked for, at the count a 50 % render
    // scale actually asks for. A sequence that repeated at eight regardless
    // would pass every test above and reconstruct from a quarter of the samples
    // it was handed.
    let long = phases(960, 1920);
    assert_eq!(long, 32);
    assert_eq!(offset_pixels(0, long), offset_pixels(long, long));
    assert_ne!(offset_pixels(0, long), offset_pixels(DEFAULT_PHASES, long));
}
