//! What a *temporal* upscaler costs a frame that is not a picture.
//!
//! Its own file beside `temporal.rs` rather than in `upscale/tests.rs`, under
//! the 1,000-line rule in `scripts/check-file-size.py` - and it is the right
//! seam either way: what is asserted here is the one thing that distinguishes a
//! temporal resolve from a spatial one at the call site, which is that it does
//! not want a spatial pass in front of it.

use crate::upscale::{Framebuffer, Presentation, Temporal};
use oag_display::display::{Brightness, Gamma, Reconstruction};

/// A frame FSR 3.1 resolved must never have built an FXAA or SMAA pass.
///
/// **The finding this pins is a cost, not a picture.** FSR 3.1 deliberately
/// reads `self.perceptual` rather than a spatial pass's output - a temporal
/// reconstruction reasons about the edges a blur has already removed. Both
/// arrangements produce identical pixels, so nothing but
/// [`Framebuffer::built_spatial_anti_aliasing`] can tell them apart: the
/// passes are lazy, so "was it ever built" is "did a frame ever run it".
///
/// **Since [ADR-0041] the two cannot both be selected**, so what this guards
/// has narrowed and is still worth guarding: `resolve_scene` matches on one
/// enum now, and an arm that fell through to the FXAA branch would be a
/// full-screen pass paid for a frame nobody reads. The control is the same
/// call asking for FXAA, which must build it - or this would pass against a
/// `resolve_scene` that had stopped anti-aliasing altogether.
///
/// [ADR-0041]: ../../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn fsr3_skips_the_spatial_anti_aliasing_pass_rather_than_discarding_it() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no adapter; skipping");
        return;
    };
    if !oag_post::fsr3::supported(&adapter) {
        eprintln!("no compute shaders; skipping");
        return;
    }
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;

    // Half scale into a 16x8 rectangle, which is a real upscale and small
    // enough that the whole chain is a handful of workgroups.
    let render = (8u32, 4u32);
    let whole = (0.0, 0.0, 16.0, 8.0);
    let attachment = |label: &str, texture_format, usage| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: render.0,
                    height: render.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: texture_format,
                usage,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    let depth = attachment(
        "depth",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let velocity = attachment(
        "velocity",
        oag_gpu::formats::VELOCITY_FORMAT,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );

    let resolve = |reconstruction: Reconstruction| {
        let mut framebuffer = Framebuffer::new(&device, format, render).expect("the pipeline");
        framebuffer.resize_output(&device, (16, 8));
        let mut encoder = device.create_command_encoder(&Default::default());
        let encoded = framebuffer.resolve_scene(
            &device,
            &queue,
            &mut encoder,
            whole,
            &Presentation {
                reconstruction,
                sharpness: 0.2,
                brightness: Brightness::NEUTRAL,
                gamma: Gamma::NEUTRAL,
            },
            Some(Temporal {
                depth: &depth,
                velocity: &velocity,
                sample_count: 1,
                frames: vec![super::ViewFrame {
                    view: None,
                    camera: oag_post::fsr3::Camera::default(),
                    jitter: (0.0, 0.0),
                    phase_count: oag_post::jitter::phases(render.0, 16),
                    reset: true,
                }],
            }),
            None,
        );
        queue.submit(Some(encoder.finish()));
        (
            framebuffer.built_spatial_anti_aliasing(),
            framebuffer.temporal_upscaler_viable(),
            encoded,
        )
    };

    let (built_aa, viable, encoded) = resolve(Reconstruction::Fsr3);
    assert!(
        !built_aa,
        "FSR 3.1 resolved, so the FXAA pass must never have been built"
    );
    // The frame loop claims a timestamp pair on this predicate, and a pair
    // claimed for a pass that will never run reads back whatever the slot last
    // held - see `Session::upscale_timer`. A build that succeeded must say so.
    assert!(
        viable,
        "the pipelines built, so the temporal path stays claimable"
    );
    // The other half of the same claim: a build that succeeded and actually
    // ran `fsr.render` must say so too, or a caller has no way to tell "wrote
    // the timestamp" from "gave up before trying" - see `resolve_scene`'s own
    // return-value doc and the bug it was added to close, where nothing ever
    // told a claimed-but-unwritten slot to give itself back.
    assert!(encoded, "the pipelines built and ran, so the pass encoded");

    let (built_aa, viable, encoded) = resolve(Reconstruction::Fxaa);
    assert!(
        built_aa,
        "the control: asked for FXAA and nothing else, the pass must run"
    );
    // Nothing has attempted a build, which is not the same as one having
    // failed - and the timestamp claim keys on the difference.
    assert!(viable, "an unattempted build has not failed");
    assert!(
        !encoded,
        "FXAA was asked for, not FSR 3.1 - nothing here should have run it"
    );
}

/// A split screen's frame resolves each view with a chain of its own, and a whole picture
/// after it drops them. Fails when a split is resolved as one picture (no chain per seat), and
/// when the seats' chains outlive the split.
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn a_split_resolves_each_view_with_a_chain_of_its_own() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no adapter; skipping");
        return;
    };
    if !oag_post::fsr3::supported(&adapter) {
        eprintln!("no compute shaders; skipping");
        return;
    }
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let render = (16u32, 8u32);
    let attachment = |label: &str, texture_format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: render.0,
                    height: render.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: texture_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    let depth = attachment("depth", wgpu::TextureFormat::Depth32Float);
    let velocity = attachment("velocity", oag_gpu::formats::VELOCITY_FORMAT);
    let mut framebuffer = Framebuffer::new(&device, format, render).expect("the pipeline");
    framebuffer.resize_output(&device, (32, 16));
    let frame = |view| super::ViewFrame {
        view,
        camera: oag_post::fsr3::Camera::default(),
        jitter: (0.0, 0.0),
        phase_count: oag_post::jitter::phases(render.0, 32),
        reset: true,
    };
    let presentation = Presentation {
        reconstruction: Reconstruction::Fsr3,
        sharpness: 0.2,
        brightness: Brightness::NEUTRAL,
        gamma: Gamma::NEUTRAL,
    };
    let mut resolve = |frames| {
        let mut encoder = device.create_command_encoder(&Default::default());
        let _ = framebuffer.resolve_scene(
            &device,
            &queue,
            &mut encoder,
            (0.0, 0.0, 32.0, 16.0),
            &presentation,
            Some(Temporal {
                depth: &depth,
                velocity: &velocity,
                sample_count: 1,
                frames,
            }),
            None,
        );
        queue.submit(Some(encoder.finish()));
        framebuffer.split_chains()
    };
    // Left and right, the layout's two-pixel gap between them.
    let split = vec![
        frame(Some((0, (0, 0, 7, 8)))),
        frame(Some((1, (9, 0, 7, 8)))),
    ];
    assert_eq!(resolve(split.clone()), 2, "a chain per seat");
    assert_eq!(resolve(split), 2, "the same chains the next frame");
    assert_eq!(resolve(vec![frame(None)]), 0, "a whole picture drops them");
}

#[test]
fn a_view_lands_where_the_blit_would_put_it() {
    // A 2:1 upscale of a 2h split: each edge doubles, the gap with it.
    let rect = (0.0, 0.0, 64.0, 32.0);
    assert_eq!(
        super::super::views::output_rect((0, 0, 32, 7), (32, 16), rect),
        (0.0, 0.0, 64.0, 14.0)
    );
    assert_eq!(
        super::super::views::output_rect((0, 9, 32, 7), (32, 16), rect),
        (0.0, 18.0, 64.0, 14.0)
    );
    // A letterboxed presentation rectangle offsets every view by its corner.
    assert_eq!(
        super::super::views::output_rect((0, 0, 32, 16), (32, 16), (10.0, 4.0, 48.0, 24.0)),
        (10.0, 4.0, 48.0, 24.0)
    );
}
