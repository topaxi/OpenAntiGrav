//! Does a second multisample pass that loads the attachment keep what the first
//! drew? Split screen draws one pass per view into one target and relies on it:
//! the later views `LoadOp::Load` the multisampled colour and resolve again, and
//! the resolve covers the whole attachment (`msaa_resolve_viewport`'s finding),
//! so a Load that did not keep the first view would erase it.
//!
//! Skips when there is no GPU adapter, so a green CI run is not evidence that it
//! ran.

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

/// Two passes, two viewports, one loading what the other kept.
#[test]
fn a_loading_multisample_pass_keeps_the_view_drawn_before_it() {
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

    let mut encoder = device.create_command_encoder(&Default::default());
    let msaa_view = multisampled.create_view(&Default::default());
    let resolved_view = resolved.create_view(&Default::default());
    let draw = |encoder: &mut wgpu::CommandEncoder, load: wgpu::LoadOp<wgpu::Color>, at: f32| {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("view"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &msaa_view,
                depth_slice: None,
                resolve_target: Some(&resolved_view),
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_viewport(at, at, DRAWN as f32, DRAWN as f32, 0.0, 1.0);
        pass.draw(0..3, 0..1);
    };
    draw(&mut encoder, wgpu::LoadOp::Clear(wgpu::Color::RED), 0.0);
    queue.submit(Some(encoder.finish()));
    let mut encoder = device.create_command_encoder(&Default::default());
    draw(&mut encoder, wgpu::LoadOp::Load, DRAWN as f32);

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

    assert_eq!(
        pixel(8, 8),
        [0, 255, 0],
        "the first view survived the second pass"
    );
    assert_eq!(pixel(48, 48), [0, 255, 0], "the second view is drawn");
    assert_eq!(
        pixel(8, 48),
        [255, 0, 0],
        "elsewhere is still the first clear"
    );
}
