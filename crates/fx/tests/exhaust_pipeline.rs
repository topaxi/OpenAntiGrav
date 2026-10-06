//! Builds the exhaust pipeline on a real device and draws through it.
//!
//! `#[ignore]`d because it needs a GPU adapter, the same reason
//! `collision_capture.rs` is. Run it by hand:
//!
//! ```sh
//! cargo nextest run -p oag-render --run-ignored all -E 'test(exhaust_pipeline)' --no-capture
//! ```
//!
//! The maths is covered by unit tests in `oag_fx::exhaust`, which need no
//! GPU. What is **only** checkable here is the part those cannot reach: that
//! `exhaust.wesl` compiles, that the vertex layout matches `GpuVertex`, and that
//! the pipeline's depth and blend state are accepted alongside the mesh
//! pipeline's depth format. Every one of those is a validation error at device
//! level rather than a compile error, so without this they are only caught by
//! running the game.
//!
//! It deliberately does **not** assert on pixels. The picture is checked through
//! `just play --race --screenshot`, against a real ship and the disc's own
//! texture; a synthetic golden image here would pin the placeholder glow instead.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::exhaust::{self, Exhaust, FlareTexture};
use oag_mesh::mesh_render::DEPTH_FORMAT;

/// The colour format the headless capture path uses, so this matches it.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

#[test]
#[ignore = "needs a GPU adapter"]
fn the_exhaust_pipeline_builds_and_draws_on_a_real_device() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("no GPU adapter available");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("exhaust pipeline test"),
        ..Default::default()
    }))
    .expect("requesting the device");

    // Any validation failure below - a shader that will not compile, a vertex
    // layout that disagrees with `GpuVertex`, a blend state the format rejects -
    // surfaces through this.
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));

    let flare = FlareTexture::placeholder(32);
    assert_eq!(flare.rgba.len(), 32 * 32 * 4, "placeholder must be RGBA8");
    let noise = FlareTexture::placeholder(16);
    let mut pipeline = exhaust::Pipeline::new(
        &device,
        &queue,
        FORMAT,
        &flare,
        &noise,
        None,
        exhaust::TRAIL_BLEND,
        1,
        oag_mesh::mesh_render::Velocity::None,
        true,
    );

    // A lit exhaust, so there is something to draw rather than an empty buffer.
    let mut state = Exhaust::new();
    let mut rng = Rng::new(3);
    for k in 0..300 {
        state.advance(1.0 / 60.0, 100.0, 150.0, &mut rng);
        // A moving craft, so consecutive samples differ and the ribbon is not
        // degenerate.
        state.push_trail(Vec3::new(0.0, 0.0, -(k as f32) * 2.5), -Vec3::Z);
    }
    assert!(state.intensity() > 0.0, "the test needs a lit exhaust");

    // The flare is one quad. **It used to be the whole buffer**, and this
    // assertion read `== MAX_VERTICES` until `exhaust::sprite` gave it company -
    // see that function and `exhaust::MAX_SPRITES`.
    let mut vertices = state.vertices(Vec3::ZERO, Vec3::X, Vec3::Y);
    assert_eq!(vertices.len(), 6, "the flare is one quad");

    // Fill the rest of the budget with caller sprites, so this exercises a
    // **full** vertex buffer on a real device rather than the six bytes the
    // flare alone needs. That is the case a too-small budget silently truncates,
    // and the case a buffer sized for six would fail validation on.
    while vertices.len() + 6 <= exhaust::MAX_VERTICES {
        let n = vertices.len() as f32;
        vertices.extend(exhaust::sprite(
            Vec3::new(n * 0.01, 0.0, 0.0),
            Vec3::X,
            Vec3::Y,
            0.5,
            1.0,
        ));
    }
    assert_eq!(vertices.len(), exhaust::MAX_VERTICES);

    // An identity view-projection: this test is about the pipeline, not framing.
    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    // A full ring, so the ribbon has geometry to draw as well as the sprite.
    assert!(
        state.trail_ready(),
        "the ring must be full to draw a ribbon"
    );
    // One craft's ribbon is a fraction of the buffer now that all eight craft
    // share it, so this cycles a full buffer's worth of it - the point of this
    // test is a **full** buffer on a real device, and a fraction of one would
    // stop exercising the case a too-small budget truncates. `cycle` rather
    // than `repeat_n(_, MAX_TRAILS)` because the budget is sized for the
    // larger of the two ribbon shapes - HD's 954-vertex tube, not this PSP
    // ribbon's 648 - so eight PSP ribbons no longer fill it exactly.
    let one = state.trail_vertices(Vec3::X, Vec3::Y);
    assert_eq!(one.len(), exhaust::TRAIL_VERTICES_PER_CRAFT);
    let trail: Vec<_> = one
        .iter()
        .copied()
        .cycle()
        .take(exhaust::MAX_TRAIL_VERTICES)
        .collect();
    assert_eq!(trail.len(), exhaust::MAX_TRAIL_VERTICES);
    pipeline.upload(&queue, &identity, Vec3::ZERO, &vertices, &trail);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("exhaust target"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    // The same depth format `race::Scene` uses, which is the compatibility this
    // test exists to pin: the exhaust shares that pass and that attachment.
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("exhaust depth"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("exhaust"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("exhaust"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pipeline.draw(&mut pass);
    }
    queue.submit(std::iter::once(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");

    // A cold exhaust uploads zero-alpha geometry rather than nothing, so the draw
    // must still be issued without complaint - the constant vertex count is what
    // keeps the buffer from ever resizing.
    let cold = Exhaust::new();
    pipeline.upload(
        &queue,
        &identity,
        Vec3::ZERO,
        &cold.vertices(Vec3::ZERO, Vec3::X, Vec3::Y),
        &cold.trail_vertices(Vec3::X, Vec3::Y),
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}
