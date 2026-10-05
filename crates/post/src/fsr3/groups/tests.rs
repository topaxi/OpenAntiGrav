//! What [`super::Cache`] is asserted to do, which is a *cost* and not a
//! picture.
//!
//! Its own file beside `groups.rs` rather than a block inside it, under the
//! 200-line rule in `scripts/check-file-size.py` - and beside `groups.rs`
//! rather than in `fsr3/tests.rs`, because the contract it pins is this
//! module's own: two sets of bind groups, built when something that is in one
//! changes and not once a frame. `use super::*` still reaches every private
//! item.

use super::*;

/// The dispatch `fsr3/tests.rs` uses, restated rather than reached for: a
/// `#[cfg(test)] mod` is not visible from a sibling one, and the fields that
/// matter here are the two extents.
fn dispatch(render: (u32, u32), upscale: (u32, u32)) -> Dispatch {
    Dispatch {
        render,
        max_render: render,
        upscale,
        jitter: (0.25, -0.125),
        phase_count: crate::jitter::phases(render.0, upscale.0),
        camera: super::super::readback::CAMERA,
        delta_time: 1.0 / 60.0,
        reset: true,
        sample_count: 1,
        sharpness: crate::fsr1::Sharpness::DEFAULT,
    }
}

/// The bind groups are built once per allocation, not once per frame.
///
/// **Nothing else can see this.** A cache hit and a rebuild produce identical
/// pixels, so a regression that put `Cache::build` back in the frame path would
/// pass every other test in this module - see [`Fsr3::group_rebuilds`]. What is
/// pinned is the contract: one build for the first frame, none for the frames
/// after it whatever the ping-pong parity, and one more when the scene's own
/// views move.
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn the_bind_groups_are_built_once_and_not_once_a_frame() {
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
    let mut fsr3 = Fsr3::new(&device).expect("the fsr3 pipelines");

    let render = (16u32, 8u32);
    let upscale = (32u32, 16u32);
    let texture = |label: &str, size: (u32, u32), format, usage| {
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
            .create_view(&Default::default())
    };
    let scene = |label: &str| {
        (
            texture(
                label,
                render,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            texture(
                label,
                render,
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            texture(
                label,
                render,
                oag_gpu::formats::VELOCITY_FORMAT,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
        )
    };
    let (colour, depth, velocity) = scene("first");

    // A function rather than a closure: a closure capturing `fsr3` mutably
    // would hold that borrow across the `group_rebuilds()` reads between calls.
    fn run(
        fsr3: &mut Fsr3,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render: (u32, u32),
        upscale: (u32, u32),
        views: (&wgpu::TextureView, &wgpu::TextureView, &wgpu::TextureView),
    ) {
        let mut encoder = device.create_command_encoder(&Default::default());
        fsr3.render(
            device,
            queue,
            &mut encoder,
            Frame {
                colour: views.0,
                depth: views.1,
                velocity: views.2,
                dispatch: dispatch(render, upscale),
            },
            None,
        );
        queue.submit(Some(encoder.finish()));
    }
    let first = (&colour, &depth, &velocity);

    run(&mut fsr3, &device, &queue, render, upscale, first);
    assert_eq!(fsr3.group_rebuilds(), 1, "the first frame builds both sets");

    // Six more frames, which is three of each parity: the parity picks between
    // two sets that already exist rather than building either.
    for _ in 0..6 {
        run(&mut fsr3, &device, &queue, render, upscale, first);
    }
    assert_eq!(
        fsr3.group_rebuilds(),
        1,
        "no frame after the first may rebuild - that is the whole cache"
    );

    // A new scene target at the same size is a different resource with the same
    // dimensions, so `Targets::fits` would say nothing is wrong. The views are
    // what changed and the bindings point at them.
    let (colour2, depth2, velocity2) = scene("second");
    let second = (&colour2, &depth2, &velocity2);
    run(&mut fsr3, &device, &queue, render, upscale, second);
    assert_eq!(
        fsr3.group_rebuilds(),
        2,
        "a scene view that moved must invalidate the cache"
    );
    run(&mut fsr3, &device, &queue, render, upscale, second);
    assert_eq!(fsr3.group_rebuilds(), 2, "and settle again after it");
}
