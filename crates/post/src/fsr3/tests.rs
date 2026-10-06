//! What the FSR 3.1 port in [`super`] is asserted to do: the constants against
//! upstream's arithmetic, the layout against the WGSL that reads it, and the
//! ported passes compiled and dispatched on a real device.
//!
//! Its own file rather than an inline `#[cfg(test)]` block, per the 200-line
//! rule in `scripts/check-file-size.py`.

use super::*;

use super::readback::{self, Scene};

/// The camera the race actually draws with, near enough: `oag-render`'s own
/// `camera::perspective` is built from exactly these three numbers.
fn camera() -> Camera {
    readback::CAMERA
}

pub(super) fn dispatch(render: (u32, u32), upscale: (u32, u32)) -> Dispatch {
    Dispatch {
        render,
        max_render: render,
        upscale,
        jitter: (0.25, -0.125),
        phase_count: crate::jitter::phases(render.0, upscale.0),
        camera: camera(),
        delta_time: 1.0 / 60.0,
        reset: true,
        sample_count: 1,
        sharpness: crate::fsr1::Sharpness::DEFAULT,
    }
}

#[test]
fn the_constants_are_the_size_the_wgsl_expects() {
    // 37 scalars in upstream's `Fsr3UpscalerConstants` - 148 bytes - rounded up
    // to the 16-byte multiple a uniform buffer's size must be. Written out
    // rather than derived, because the whole reason this is checkable is that
    // both sides state the number independently: get it wrong and every pass
    // reads a field one row out, which produces a picture rather than an error.
    assert_eq!(Constants::SIZE, 160);
    assert_eq!(size_of::<Constants>() % 16, 0);
}

#[test]
fn a_first_frame_has_no_previous_frame_and_says_so() {
    // Upstream's `resetAccumulation` path. The four `previousFrame*` fields
    // describe *this* frame rather than a frame that never happened, because a
    // reprojection through a zeroed previous size would divide by zero.
    let c = Constants::new(dispatch((960, 540), (1920, 1080)), None);
    assert_eq!(c.render_size, [960, 540]);
    assert_eq!(c.previous_frame_render_size, c.render_size);
    assert_eq!(c.upscale_size, [1920, 1080]);
    assert_eq!(c.previous_frame_upscale_size, c.upscale_size);
    assert_eq!(c.previous_frame_jitter_offset, c.jitter_offset);
    assert_eq!(c.frame_index, 0.0);
}

#[test]
fn the_second_frame_carries_the_first_s_sizes_and_jitter() {
    let first = Constants::new(dispatch((960, 540), (1920, 1080)), None);
    let mut second = dispatch((640, 360), (1920, 1080));
    second.jitter = (-0.5, 0.5);
    let c = Constants::new(second, Some(&first));
    assert_eq!(c.render_size, [640, 360]);
    assert_eq!(c.previous_frame_render_size, [960, 540]);
    assert_eq!(c.jitter_offset, [-0.5, 0.5]);
    assert_eq!(c.previous_frame_jitter_offset, [0.25, -0.125]);
    assert_eq!(c.frame_index, 1.0);
}

#[test]
fn the_downscale_factor_is_render_over_presentation() {
    let c = Constants::new(dispatch((960, 540), (1920, 1080)), None);
    assert_eq!(c.downscale_factor, [0.5, 0.5]);
    let native = Constants::new(dispatch((1920, 1080), (1920, 1080)), None);
    assert_eq!(native.downscale_factor, [1.0, 1.0]);
}

#[test]
fn the_motion_vector_scale_reverses_this_renderer_s_velocity() {
    // `shaders/velocity.wesl`'s `velocity_of` stores current-minus-previous in uv units;
    // FSR 3.1 reprojects with `uv + motionVector` and therefore wants
    // previous-minus-current. The whole conversion is the sign, because the
    // units already agree - and the jitter cancellation is zero because
    // ADR-0039 cancels it out of the buffer before it is written.
    let c = Constants::new(dispatch((960, 540), (1920, 1080)), None);
    assert_eq!(c.motion_vector_scale, [-1.0, -1.0]);
    assert_eq!(c.motion_vector_jitter_cancellation, [0.0, 0.0]);
}

#[test]
fn the_depth_transform_inverts_this_project_s_own_projection() {
    // The claim that makes `GetViewSpaceDepth` mean anything: run a known
    // view-space depth through the real projection, then back through the two
    // constants, and land on the number started with. This is the arithmetic
    // most easily got subtly wrong - a sign or a swapped near and far still
    // produces a plausible-looking depth - and it is checkable without a GPU.
    use oag_core::math::{Vec4, camera as cam};

    let c = Constants::new(dispatch((1920, 1080), (1920, 1080)), None);
    let projection = cam::perspective(camera().fov_y, 16.0 / 9.0, camera().near, camera().far);
    for view_z in [0.5f32, 5.0, 50.0, 500.0] {
        // A point `view_z` in front of a right-handed camera is at -z.
        let clip = projection * Vec4::new(0.0, 0.0, -view_z, 1.0);
        let device_depth = clip.z / clip.w;
        let recovered = c.device_to_view_depth[1] / (device_depth - c.device_to_view_depth[0]);
        assert!(
            (recovered - view_z).abs() < view_z * 1e-3,
            "device depth {device_depth} recovered {recovered}, wanted {view_z}"
        );
    }
}

#[test]
fn the_phase_count_walks_to_its_target_one_frame_at_a_time() {
    // Upstream ramps rather than jumping, so that a render-scale change does
    // not re-index the jitter sequence discontinuously mid-accumulation. A
    // straight assignment here would pass every other test in this file.
    let mut previous = Constants::new(dispatch((1920, 1080), (1920, 1080)), None);
    assert_eq!(previous.jitter_phase_count, 8.0);
    // Now ask for 32, which is what a 50 % render scale wants.
    let wanted = dispatch((960, 540), (1920, 1080));
    assert_eq!(wanted.phase_count, 32);
    for expected in [9.0, 10.0, 11.0] {
        previous = Constants::new(wanted, Some(&previous));
        assert_eq!(previous.jitter_phase_count, expected);
    }
    // And back down again, one at a time.
    let back = dispatch((1920, 1080), (1920, 1080));
    previous = Constants::new(back, Some(&previous));
    assert_eq!(previous.jitter_phase_count, 10.0);
}

#[test]
fn a_projection_round_trips_through_the_camera_it_was_built_from() {
    use oag_core::math::camera as cam;

    let want = camera();
    let projection = cam::perspective(want.fov_y, 16.0 / 9.0, want.near, want.far);
    let got = camera_from_projection(projection);
    assert!((got.near - want.near).abs() < 1e-3, "near {}", got.near);
    assert!((got.fov_y - want.fov_y).abs() < 1e-4, "fov {}", got.fov_y);
    // **A tenth of a percent on `far`, not an exact match**, and the loss is
    // real rather than a loose bound hiding a bug: `far` is recovered as
    // `e / (c + 1)` where `c + 1` is `near / (near - far)`, a small number
    // formed by adding two that nearly cancel. At 0.1 and 1000.0 in `f32` that
    // costs three digits. It matters here and nowhere else - the depth
    // transform this file also tests uses `far / (near - far)`, which is
    // insensitive to it, and passes to a thousandth.
    assert!(
        (got.far - want.far).abs() < want.far * 2e-3,
        "far {}",
        got.far
    );
}

#[test]
fn the_pass_list_is_upstream_s_dispatch_order() {
    // The port's spine. A pass reads what the ones before it wrote, so the
    // order is a correctness property rather than presentation - and the table
    // in `docs/rendering/fsr3.md` has to keep saying the same thing.
    let names: Vec<&str> = Pass::ALL.iter().map(|pass| pass.name()).collect();
    assert_eq!(
        names,
        [
            "prepare_inputs",
            "luma_pyramid",
            "shading_change_pyramid",
            "shading_change",
            "prepare_reactivity",
            "luma_instability",
            "accumulate",
            "rcas",
        ]
    );
}

/// Compiles every ported pass and dispatches it on a real device.
///
/// The arithmetic above is CPU-side and says nothing about whether the WGSL
/// parses, whether the bind group layouts agree with their `@group`/`@binding`
/// declarations, or whether the storage-texture formats are ones the adapter
/// will actually accept - all of which are runtime failures in wgpu, not build
/// ones.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence that
/// it ran. Run it locally on real hardware.
#[test]
fn every_ported_pass_builds_and_dispatches_on_a_real_device() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    assert!(
        supported(&adapter),
        "this machine's adapter reports no compute shaders, which is what the \
         fallback to FSR 1 exists for - but then this test cannot run at all"
    );
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };

    let mut fsr3 = Fsr3::new(&device).expect("the FSR 3.1 shaders must compile");

    let render = (64u32, 32u32);
    let upscale = (128u32, 64u32);
    let texture = |label, size: (u32, u32), format, usage| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let colour = texture(
        "scene",
        render,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    // A real depth format, not a colour one standing in for it: binding a
    // depth view as an unfilterable float is exactly the arrangement that
    // would fail, and a colour texture here would not exercise it.
    let depth = texture(
        "depth",
        render,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let velocity = texture(
        "velocity",
        render,
        oag_gpu::formats::VELOCITY_FORMAT,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );

    let mut encoder = device.create_command_encoder(&Default::default());
    fsr3.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            colour: &colour,
            depth: &depth,
            velocity: &velocity,
            dispatch: dispatch(render, upscale),
        },
        None,
    );
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");

    assert_eq!(fsr3.constants().map(|c| c.render_size), Some([64, 32]));
    // **All eight passes are ported, so there is an output.** This assertion
    // was the opposite until `rcas` landed: an incomplete chain must hand back
    // nothing rather than a half-resolved intermediate, because a half-resolved
    // picture reads as a rendering bug rather than as unfinished work. It flips
    // with `Pass::ported`, which is what keeps the two honest.
    assert!(
        fsr3.output().is_some(),
        "every pass is ported, so the chain must produce a frame"
    );
}

#[test]
fn a_jittered_still_scene_converges_on_a_history_it_trusts() {
    // **The one property that distinguishes a temporal upscaler from a blit**,
    // and the reason this fixture is the only one that turns jitter on. Without
    // a sub-pixel offset there is one sample per pixel per frame and nothing to
    // reconstruct from, so every other test here would report a resolve and a
    // point sample as identical.
    //
    // With jitter, a still scene must *converge*: each frame lands its samples
    // somewhere new inside each pixel, the history absorbs them, and after a
    // full sequence the answer stops moving. Two things are asserted:
    //
    // 1. The last two frames' outputs agree - accumulation settles rather than
    //    oscillating, which a wrong history weight would break.
    // 2. The chain believes it has a deep, trusted history by then: the
    //    accumulation channel is full, and nothing calls the scene disoccluded
    //    or re-lit.
    //
    // (2) rather than "the settled output differs from the first frame's",
    // which was tried and is not a control at all here. On a smooth ramp a
    // single frame's nine-tap upsample already gets the answer nearly right, so
    // accumulation legitimately buys almost nothing and the two agree to within
    // the settling residual - measured at 0.0156 against 0.0176. Reconstruction
    // *gain* needs high-frequency detail to reconstruct, and high-frequency
    // detail is exactly what makes the rectification box too tight to converge
    // at this fixture's size. Measuring at the source sidesteps the conflict.
    //
    // **A gentler ramp than every other fixture here uses**, and the reason is
    // measured rather than guessed. A converged history is not frozen: each
    // frame's jittered upsample lands somewhere new, and where it falls outside
    // the rectification box the history is *snapped* to the box's surface
    // rather than blended a few percent towards it. The box is sized by the
    // neighbourhood's own standard deviation, so on a steep gradient it is
    // narrow and the snap is large. Measured on this 8x4 fixture: the standard
    // 2%-per-texel ramp settles to 0.0625 between the last two frames, a
    // 0.2%-per-texel ramp to 0.0176 - a tenth of the slope for a third of the
    // residual. Neither is a convergence failure; the accumulation reaches a
    // full 1.0 with no false shading change in both.
    let depth = depth_pattern();
    let colour: Vec<[f32; 3]> = (0..FIXTURE.0 * FIXTURE.1)
        .map(|i| {
            let t = 0.40 + 0.002 * i as f32;
            [t, t * 0.8, t * 0.6]
        })
        .collect();
    let still = readback::Input {
        depth: &depth,
        colour: &colour,
    };
    let phases = crate::jitter::phases(FIXTURE.0, FIXTURE.0 * 2) as usize;
    assert_eq!(phases, 32, "the fixture upscales by two");

    let run = |count: usize| {
        Scene::run_with(FIXTURE, &vec![still; count], true).map(|scene| {
            let output = scene
                .fsr3
                .output_texture()
                .expect("a complete chain has an output");
            scene.read_rgba16(output)
        })
    };

    let (Some(nearly), Some(settled)) = (run(phases - 1), run(phases)) else {
        eprintln!("no adapter; skipping");
        return;
    };

    let mut worst_settling = 0.0f32;
    for index in 0..settled.len() {
        for channel in 0..3 {
            worst_settling =
                worst_settling.max((settled[index][channel] - nearly[index][channel]).abs());
        }
    }

    // Not zero, for the reason above - and comfortably above the 0.0176
    // measured, so a real regression in the history weight has room to show up
    // rather than being absorbed by the bound.
    assert!(
        worst_settling < 0.03,
        "the resolve is still moving at frame {phases}: worst channel change \
         {worst_settling}"
    );

    // The direct statement, read off `prepare_reactivity`'s own output rather
    // than inferred from pixels: after a full sequence of a still scene the
    // history is as deep as it goes and nothing has knocked it back.
    let Some(scene) = Scene::run_with(FIXTURE, &vec![still; phases], true) else {
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    for (index, masks) in scene
        .read_rgba_unorm(&targets.dilated_reactive_masks.texture)
        .iter()
        .enumerate()
    {
        assert!(
            masks[3] > 0.99,
            "texel {index} accumulated only {} frames' worth under jitter",
            masks[3]
        );
        assert!(masks[1] < 0.01, "texel {index} disocclusion {}", masks[1]);
        assert!(
            masks[2] < 0.01,
            "texel {index} shading change {} - a moving sub-pixel offset is \
             being mistaken for the scene changing",
            masks[2]
        );
    }
}

/// The fixture's extent: small enough that a wrong mip count or a wrong
/// rounding shows up, and it already has - see `resources::mip_ceiling`.
pub(super) const FIXTURE: (u32, u32) = (8, 4);

/// A depth buffer with no two texels alike, so that a 3x3 neighbourhood's
/// nearest, its centre and its plain maximum are three different numbers
/// everywhere they can be.
pub(super) fn depth_pattern() -> Vec<f32> {
    (0..FIXTURE.0 * FIXTURE.1)
        // 0.10 .. 0.72, monotonic along the scanline and stepped between rows,
        // so a neighbourhood is never flat and never symmetric.
        .map(|i| 0.10 + 0.02 * i as f32)
        .collect()
}

/// A colour buffer with a luma gradient across it, so that the five-tap
/// neighbourhood the shading-change pass compares is never flat either.
pub(super) fn colour_pattern() -> Vec<[f32; 3]> {
    (0..FIXTURE.0 * FIXTURE.1)
        .map(|i| {
            let t = 0.15 + 0.02 * i as f32;
            [t, t * 0.8, t * 0.6]
        })
        .collect()
}

#[test]
fn the_farthest_depth_is_the_centre_s_own_and_never_a_neighbourhood_maximum() {
    // **The most transliteration-fragile line in `prepare_inputs`.** Upstream's
    // `FindDepthExtents` initialises `fFarthest` to the centre's depth and then
    // only ever `max`es it with a sample that was *nearer* than the running
    // nearest - and every such sample is, by construction, nearer than the
    // centre was. So `fFarthest` can never move: it is exactly the centre
    // texel's own depth, whatever the neighbourhood holds.
    //
    // That reads as a bug. It is not, it is upstream's, and it is the sort of
    // thing a future reader "fixes" into a real maximum without noticing that
    // the reconstruction was tuned against this. Pinned on hardware rather than
    // by a comment, because a comment cannot fail.
    let size = FIXTURE;
    let depth = depth_pattern();
    let colour = colour_pattern();
    let Some(scene) = Scene::run(
        size,
        &[readback::Input {
            depth: &depth,
            colour: &colour,
        }],
    ) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let farthest = scene.read_r32(&targets.farthest_depth.texture);

    for y in 0..size.1 {
        for x in 0..size.0 {
            let at = (y * size.0 + x) as usize;
            let want = scene.view_space_metres(depth[at]);
            assert!(
                (farthest[at] - want).abs() < want * 1e-4,
                "({x}, {y}): got {}, wanted the centre's own {want}",
                farthest[at]
            );
        }
    }

    // And the control that makes the above mean something: a plain
    // neighbourhood maximum would be a *different* number at almost every
    // texel, so a test that passed both ways would be testing nothing.
    let interior = (2 * size.0 + 3) as usize;
    let neighbourhood_max = [-1i32, 0, 1]
        .iter()
        .flat_map(|dy| [-1i32, 0, 1].iter().map(move |dx| (*dx, *dy)))
        .map(|(dx, dy)| depth[((2 + dy) as u32 * size.0 + (3 + dx) as u32) as usize])
        .fold(f32::MIN, f32::max);
    assert!(
        (neighbourhood_max - depth[interior]).abs() > 1e-3,
        "the fixture is flat, so it cannot tell the two readings apart"
    );
}

#[test]
fn the_luma_pyramid_reduces_the_farthest_depth_by_a_plain_box_average() {
    // `SpdReduce4`: `(v0 + v1 + v2 + v3) * 0.25` over the 2x2 below each
    // half-resolution texel. Checked against the CPU-side view-space transform
    // rather than against a second run of the same shader, so a wrong
    // `device_to_view_depth` cannot cancel itself out.
    let size = FIXTURE;
    let depth = depth_pattern();
    let colour = colour_pattern();
    let Some(scene) = Scene::run(
        size,
        &[readback::Input {
            depth: &depth,
            colour: &colour,
        }],
    ) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let mip1 = scene.read_rg16(&targets.farthest_depth_mip1.texture, 0);

    let half = (size.0 / 2, size.1 / 2);
    for y in 0..half.1 {
        for x in 0..half.0 {
            let tap = |dx: u32, dy: u32| {
                let at = ((y * 2 + dy) * size.0 + (x * 2 + dx)) as usize;
                scene.view_space_metres(depth[at])
            };
            let want = (tap(0, 0) + tap(1, 0) + tap(0, 1) + tap(1, 1)) * 0.25;
            let got = mip1[(y * half.0 + x) as usize].0;
            // **A tenth of a percent, not a ten-thousandth**, because this
            // target is `Rgba16Float`: it is sampled by `luma_instability`
            // through `SampleFarthestDepthMip1`, and a filterable format is
            // half-precision. The reduction is exact; the storage is not.
            assert!(
                (got - want).abs() < want * 1e-3,
                "({x}, {y}): got {got}, wanted {want}"
            );
        }
    }
}

#[test]
fn a_still_scene_reports_no_shading_change_at_any_pyramid_level() {
    // **Two frames of an unchanging scene, which is the sharpest property this
    // pass has.** The neighbourhood it compares is identical either side, and
    // upstream's `ComputeMinimumDifference` handles that through a value that
    // looks like the opposite of "no change": a pair matching to within
    // `FP16_MIN` sets `fMinDiff` to `FP16_MAX`, and the *final multiply* is
    // what turns that back into a zero. Drop that multiply - it reads like a
    // no-op guard - and a perfectly still frame reports the largest shading
    // change representable, at every level of the pyramid.
    //
    // The first frame is excluded from the claim by construction: it has no
    // previous luma, so its output is whatever the ping-pong's unwritten half
    // held. Only the second frame's is asserted.
    let depth = depth_pattern();
    let colour = colour_pattern();
    let still = readback::Input {
        depth: &depth,
        colour: &colour,
    };
    let Some(scene) = Scene::run(FIXTURE, &[still, still]) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");

    let levels = targets.spd_mips.mips.len() as u32;
    assert!(levels >= 2, "a one-level pyramid tests no reduction");
    for level in 0..levels {
        for (index, (difference, sign_sum)) in scene
            .read_rg16(&targets.spd_mips.texture, level)
            .iter()
            .enumerate()
        {
            assert_eq!(
                (*difference, *sign_sum),
                (0.0, 0.0),
                "level {level} texel {index} reports a change in a still scene"
            );
        }
    }

    // And the pass that collapses those three levels into one answer says the
    // same thing. Not implied by the above: it multiplies the two channels and
    // takes a maximum across levels, and a sign error in either would still
    // read zero from zeros - which is why the control below exists.
    for (index, change) in scene
        .read_unorm(&targets.shading_change.texture)
        .iter()
        .enumerate()
    {
        assert_eq!(
            *change, 0.0,
            "texel {index} reports a shading change in a still scene"
        );
    }
}

#[test]
fn a_changed_scene_reports_a_signed_shading_change() {
    // The control for the test above: with the second frame's colour actually
    // different, the pyramid must be non-zero - and *signed*, brightening
    // positive. Without this, a pass that wrote zeros unconditionally would
    // pass every other assertion in this file.
    // **Flat, where the still-scene test above uses a gradient**, and the
    // difference matters: `ComputeMinimumDifference` walks the two *sorted*
    // five-tap sets merge-style and keeps the smallest relative difference
    // between any pair drawn from them. Across a gradient, some dark tap is
    // nearer to some bright tap than the four-times ratio, and the answer is a
    // number nobody can predict by hand - measured at 0.357 for this fixture,
    // which is a fact about the neighbourhood rather than about the formula.
    // Flat neighbourhoods leave exactly one pair to find.
    let depth = depth_pattern();
    let texels = (FIXTURE.0 * FIXTURE.1) as usize;
    let dark = vec![[0.2f32, 0.2, 0.2]; texels];
    let bright = vec![[0.8f32, 0.8, 0.8]; texels];
    let Some(scene) = Scene::run(
        FIXTURE,
        &[
            readback::Input {
                depth: &depth,
                colour: &dark,
            },
            readback::Input {
                depth: &depth,
                colour: &bright,
            },
        ],
    ) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let level0 = scene.read_rg16(&targets.spd_mips.texture, 0);

    // Four times the luma is a `1 - min/max` of 0.75, and the sign summand is
    // +1 at every contributing texel - so the 2x2 average of each is that same
    // value. Not an approximation of upstream's formula: it *is* the formula,
    // at an input chosen so the answer is a round number.
    for (index, (difference, sign_sum)) in level0.iter().enumerate() {
        assert!(
            (*difference - 0.75).abs() < 0.01,
            "texel {index} difference {difference}, wanted 0.75"
        );
        assert!(
            (*sign_sum - 1.0).abs() < 0.01,
            "texel {index} sign sum {sign_sum}, wanted +1 for a brightening"
        );
    }

    // The collapse to one answer: `abs(x * y)` at each of three levels, then
    // the maximum. On a flat change every level holds the same pair, so the
    // answer is that pair's product - `0.75 * 1`. **Unsigned**, which is the
    // point of the `abs`: a darkening and a brightening of the same magnitude
    // are the same amount of shading change, and only the pyramid keeps the
    // direction.
    for (index, change) in scene
        .read_unorm(&targets.shading_change.texture)
        .iter()
        .enumerate()
    {
        assert!(
            (*change - 0.75).abs() < 0.01,
            "texel {index} shading change {change}, wanted 0.75"
        );
    }
}

#[test]
fn a_still_scene_accumulates_exactly_one_third_of_a_frame_per_frame() {
    // The feedback loop `accumulate` will weight its history by, and the reason
    // the `R8_UNORM` targets are held at `Rgba8Unorm` rather than widened: a
    // loop that quantises differently drifts differently.
    //
    // Two still frames. The first has no history and a previous luma of zero,
    // so its shading change reads as total and its accumulation is knocked to
    // nothing - then stored one third of a frame ahead. The second sees an
    // unchanged scene, so nothing knocks it back and it advances by another
    // third. That is upstream's own worked example in `UpdateAccumulation`,
    // run rather than read.
    let depth = depth_pattern();
    let colour = colour_pattern();
    let still = readback::Input {
        depth: &depth,
        colour: &colour,
    };
    let Some(scene) = Scene::run(FIXTURE, &[still, still]) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let frame = scene
        .fsr3
        .constants()
        .expect("a dispatched frame")
        .frame_index as u64;
    assert_eq!(frame, 1, "the fixture ran two frames");

    // One unorm step, which is the tightest a `0..1` byte can be held to.
    let step = 1.0 / 255.0;

    for (index, masks) in scene
        .read_rgba_unorm(&targets.dilated_reactive_masks.texture)
        .iter()
        .enumerate()
    {
        // **Nothing was revealed and nothing was re-lit.** A zero motion vector
        // reprojects onto the texel's own scattered depth, so the depth
        // difference is exactly zero and the disocclusion test rejects it -
        // which is the branch that matters, because a *positive* difference
        // there is what "something moved out of the way" means.
        assert!(masks[1] < step, "texel {index} disocclusion {}", masks[1]);
        assert!(masks[2] < step, "texel {index} shading change {}", masks[2]);
        // And the history is a third of a frame deep: the value the first
        // frame stored, read back by the second.
        assert!(
            (masks[3] - 1.0 / 3.0).abs() < 2.0 * step,
            "texel {index} accumulation {}, wanted a third of a frame",
            masks[3]
        );
    }

    // What the *next* frame would read: two thirds, one more step along.
    for (index, accumulation) in scene
        .read_unorm(&targets.accumulation.current(frame).texture)
        .iter()
        .enumerate()
    {
        assert!(
            (accumulation - 2.0 / 3.0).abs() < 2.0 * step,
            "texel {index} stored accumulation {accumulation}, wanted two thirds"
        );
    }
}

#[test]
fn a_still_scene_fills_the_luma_history_and_is_never_unstable() {
    // **Seven frames, because the pass is gated twice and both gates take
    // time.** The instability factor is only computed where the accumulation is
    // over 0.9 frames deep, which at a third of a frame each takes until the
    // fourth; the history is then four frames deep, so it takes three more to
    // fill. A shorter run would read zeros and prove nothing.
    //
    // Two properties, and the second is the one a reader would break:
    //
    // 1. A perfectly still scene is **never unstable**. Instability means the
    //    luma came back towards an older frame rather than going on, and
    //    nothing here ever moves.
    // 2. The history fills one slot per frame, oldest first out - so after
    //    enough frames every slot holds this frame's luma. Get the four shift
    //    assignments in the wrong order and the window becomes a smear, which
    //    property 1 would not notice.
    let depth = depth_pattern();
    let colour = colour_pattern();
    let still = readback::Input {
        depth: &depth,
        colour: &colour,
    };
    let Some(scene) = Scene::run(FIXTURE, &[still; 7]) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let targets = scene.fsr3.targets().expect("a dispatched frame");
    let frame = scene
        .fsr3
        .constants()
        .expect("a dispatched frame")
        .frame_index as u64;
    assert_eq!(frame, 6, "the fixture ran seven frames");

    for (index, instability) in scene
        .read_rgba16(&targets.luma_instability.texture)
        .iter()
        .enumerate()
    {
        assert_eq!(
            instability[0], 0.0,
            "texel {index} is called unstable in a scene that never moved"
        );
    }

    for (index, history) in scene
        .read_rgba16(&targets.luma_history.current(frame).texture)
        .iter()
        .enumerate()
    {
        // `rgb_to_luma` on the fixture's own colour, on the CPU.
        let [r, g, b] = colour[index];
        let want = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        for (slot, held) in history.iter().enumerate() {
            assert!(
                (held - want).abs() < want * 1e-2,
                "texel {index} history slot {slot} holds {held}, wanted {want}"
            );
        }
    }
}

#[test]
fn no_dispatch_dimension_can_reach_wgpu_s_ceiling() {
    // **A dispatch is capped at 65535 workgroups per dimension**, and this port
    // walked straight into it: the clear over
    // `reconstructed_previous_nearest_depth` was one-dimensional over
    // `width * height / 64`, which a 2880x1800 render extent turns into 81000
    // and a panic in a player's frame loop. Every fixture here runs at 8x4, and
    // the `--presented` captures at 720x408 come to 4590 - so nothing before a
    // real window at a real size could see it.
    //
    // Every dispatch is now two-dimensional over a texture extent, which makes
    // the bound a property of what this port can *allocate* rather than of the
    // arithmetic at any one call site. `max_texture_dimension_2d` is 8192 on
    // the WebGPU baseline and 16384 on most desktop adapters; both are checked,
    // because the guard has to hold for the largest thing that could ever be
    // bound rather than for the largest anyone has tried.
    const WGPU_MAX_WORKGROUPS_PER_DIMENSION: u32 = 65535;
    for dimension in [8192u32, 16384, 32768] {
        let dispatched = groups(dimension, GROUP);
        assert!(
            dispatched <= WGPU_MAX_WORKGROUPS_PER_DIMENSION,
            "a {dimension}-wide target dispatches {dispatched} workgroups"
        );
    }
    // And the shape that failed, stated so the reason is not lost: a flat
    // dispatch over the same target's texels exceeds the cap long before its
    // width does.
    let flat = groups(2880 * 1800, 64);
    assert!(
        flat > WGPU_MAX_WORKGROUPS_PER_DIMENSION,
        "the one-dimensional shape this replaced would have fitted at {flat}, \
         so this test is no longer describing the bug it was written for"
    );
}

#[test]
fn the_multisampled_build_binds_a_multisampled_scene() {
    // **The crash this test exists for was found by playing, not by testing.**
    // `[graphics] anti_aliasing` at either MSAA level makes the scene's depth
    // and velocity attachments multisampled, and a multisampled binding is a
    // different WGSL type - so `prepare_inputs` is built twice and the wrong
    // one is a `create_bind_group` validation failure, which is a panic in a
    // player's frame loop rather than anything the compile-only tests could
    // see.
    //
    // Only the two scene attachments are ever multisampled: everything after
    // `prepare_inputs` reads a target this port wrote itself.
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };

    let render = (64u32, 32u32);
    let sampled = |format, samples| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("fsr3 msaa test"),
                size: wgpu::Extent3d {
                    width: render.0,
                    height: render.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };

    let mut fsr3 = Fsr3::new(&device).expect("the FSR 3.1 shaders must compile");
    for samples in [1u32, 4] {
        let colour = sampled(wgpu::TextureFormat::Rgba16Float, 1);
        let depth = sampled(wgpu::TextureFormat::Depth32Float, samples);
        let velocity = sampled(oag_gpu::formats::VELOCITY_FORMAT, samples);

        let mut dispatch = dispatch(render, (render.0 * 2, render.1 * 2));
        dispatch.sample_count = samples;

        let mut encoder = device.create_command_encoder(&Default::default());
        fsr3.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                colour: &colour,
                depth: &depth,
                velocity: &velocity,
                dispatch,
            },
            None,
        );
        queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll");
    }
}

#[test]
fn the_intermediates_are_allocated_at_the_ceiling_and_the_cost_is_reportable() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let Ok((device, _queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };

    let targets = Targets::new(&device, (960, 540), (1920, 1080));
    assert!(targets.fits((960, 540), (1920, 1080)));
    assert!(!targets.fits((640, 360), (1920, 1080)));

    let sizes = targets.sizes();
    assert!(sizes.render > 0 && sizes.half_render > 0 && sizes.upscale > 0);
    assert_eq!(
        sizes.total(),
        sizes.render + sizes.half_render + sizes.upscale
    );
    // The presentation-resolution set dominates, which is what makes the
    // *upscale* ceiling the number worth watching on a handheld rather than the
    // render one - and it stays true at every scale, because lowering the
    // render scale only shrinks the smaller half.
    assert!(
        sizes.upscale > sizes.render,
        "render {} vs upscale {}",
        sizes.render,
        sizes.upscale
    );

    // **The pyramid's mip chain is counted, not just its mip 0.** `spd_mips` is
    // the one multi-level target here, and a reduction chain over halving
    // extents adds a third again - so the half-resolution figure has to exceed
    // what the three targets' base levels come to. Counting mip 0 alone
    // under-reported by that third, in the one number the widened-format
    // argument is checked with.
    let half = ((960 / 2) as u64, (540 / 2) as u64);
    // `farthest_depth_mip1` and `spd_mips` at 8 bytes a texel, `shading_change`
    // at 4 - each target's base level and nothing else.
    let base_levels = half.0 * half.1 * (8 + 8 + 4);
    assert!(
        sizes.half_render > base_levels,
        "half_render {} must exceed the base levels' {base_levels} by the \
         pyramid's own chain",
        sizes.half_render
    );
}
