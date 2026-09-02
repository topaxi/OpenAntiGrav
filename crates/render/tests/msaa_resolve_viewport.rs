//! Does a multisample resolve honour `set_viewport`, or resolve the whole
//! attachment?
//!
//! The question matters to dynamic resolution and to nothing else. Since
//! [ADR-0037] the scene target is allocated at the `render_scale` ceiling and a
//! sub-rectangle of it is drawn into; under `[graphics] anti_aliasing =
//! "msaa4x"` that drawing goes into a multisampled attachment and is resolved
//! into the scene texture at the end of the pass. If the resolve covers the
//! whole attachment, the region outside the drawn rectangle is written with
//! whatever the multisampled attachment's `LoadOp::Clear` left there - and
//! every downstream pass's UV clamp has to be what keeps that off the screen.
//! If instead it covered only the viewport, the region outside would hold the
//! *previous* frame's larger picture, which is the classic dynamic-resolution
//! stale fringe and a materially different thing to defend against.
//!
//! **Reasoning about this is not good enough**, which is why it is a test: it
//! is the one path where "the region outside the extent is the clear value" is
//! not obviously true, and no `--presented` capture can distinguish the two
//! answers, because a capture never moves the extent.
//!
//! An answer either way is a fact about wgpu rather than about this project,
//! so this test asserts what was found rather than what would be convenient.
//!
//! [ADR-0037]: ../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md

/// The attachment is this wide, and only the top-left quarter is drawn into.
const SIZE: u32 = 64;
const DRAWN: u32 = 32;

const SHADER: &str = r"
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
";

/// A resolve covers the whole attachment, so the region outside a short
/// viewport holds the clear rather than the last larger frame.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence that
/// it ran. Run it locally on real hardware.
#[test]
fn a_multisample_resolve_covers_the_whole_attachment_and_not_the_viewport() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = |label: &str, samples: u32, usage: wgpu::TextureUsages| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let multisampled = target("msaa", 4, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let resolved = target(
        "resolved",
        1,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("viewport probe"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viewport probe"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 4,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    });

    // Prime the resolve target with an unmistakable colour, so "the resolve
    // left it alone" and "the resolve wrote the clear" are distinguishable.
    // Without this both would read as *some* non-green value and the test
    // would not be asking anything.
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("prime"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &resolved.create_view(&Default::default()),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    // Now the real pass: a red clear over the whole multisampled attachment, a
    // green triangle over the top-left quarter of it, resolved into the primed
    // target. Exactly the shape `race/scene/frame.rs` builds under `msaa4x`.
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("viewport probe"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &multisampled.create_view(&Default::default()),
                depth_slice: None,
                resolve_target: Some(&resolved.create_view(&Default::default())),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_viewport(0.0, 0.0, DRAWN as f32, DRAWN as f32, 0.0, 1.0);
        pass.draw(0..3, 0..1);
    }

    let row = (SIZE * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        resolved.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapped");

    let pixel = |x: u32, y: u32| {
        let at = (y * row + x * 4) as usize;
        [mapped[at], mapped[at + 1], mapped[at + 2]]
    };

    // Inside the viewport: what was drawn.
    assert_eq!(
        pixel(8, 8),
        [0, 255, 0],
        "the drawn quarter is the triangle"
    );

    // Outside it: **the clear, not the primed blue.** The resolve covered the
    // whole attachment, so the region nobody drew into is the multisampled
    // attachment's own `LoadOp::Clear` - which in a race is black with alpha
    // zero, and which every sampler downstream keeps off the screen with the
    // half-texel clamp `post::sub_rectangle` hands it.
    //
    // Had this come back blue, the resolve would have been viewport-restricted
    // and the region outside the extent would hold the *previous* frame - the
    // classic stale fringe - which is a different and worse thing to defend
    // against.
    assert_eq!(
        pixel(48, 48),
        [255, 0, 0],
        "the undrawn region holds the pass's clear, not what was there before"
    );
    assert_eq!(pixel(40, 8), [255, 0, 0], "and the same just past the edge");
}
