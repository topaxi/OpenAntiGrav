use super::*;

fn tuning() -> Tuning {
    super::super::tests::tuning()
}

fn frame(boost: f32, tick: u64) -> Frame {
    Frame {
        boost,
        damage: 0.0,
        size: [1.0, 1.0],
        tick,
    }
}

/// A 64x64 frame cleared to `(0.2, 0.2, 0.2, 0.5)`, the host run once per `frames`
/// entry over `viewport` at `origin`, and the final frame's `(x, y)` pixels read
/// back. `None` with no adapter.
fn run(
    frames: &[Frame],
    origin: (u32, u32),
    viewport: (u32, u32),
    pixels: &[(u32, u32)],
) -> Option<Vec<[u8; 4]>> {
    let instance = wgpu::Instance::default();
    let adapters = oag_gpu::test_adapter::ordered(&instance);
    let adapter = adapters.first()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (64u32, 64u32);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("boost zoom host test frame"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("boost zoom host test readback"),
        size: u64::from(size.0 * size.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let host = Host::new(tuning(), format);
    for (n, frame) in frames.iter().enumerate() {
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("boost zoom host test scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
                        a: 0.5,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        host.run(
            &device,
            &queue,
            &mut encoder,
            &view,
            size,
            origin,
            viewport,
            frame,
        );
        if n + 1 == frames.len() {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size.0 * 4),
                        rows_per_image: Some(size.1),
                    },
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit(Some(encoder.finish()));
    }
    readback.slice(..).map_async(wgpu::MapMode::Read, |r| {
        r.expect("the readback maps");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    Some(
        pixels
            .iter()
            .map(|&(x, y)| {
                let at = ((y * size.0 + x) * 4) as usize;
                [data[at], data[at + 1], data[at + 2], data[at + 3]]
            })
            .collect(),
    )
}

/// A borrowed ring brightens the periphery of a finished frame, leaves the centre
/// and the frame's alpha (a bloom's glow mask) alone, and a quiet pulse draws nothing.
#[test]
fn the_host_draws_the_ring_over_a_finished_frame() {
    let probes = [(1, 1), (32, 32)];
    let quiet = [frame(0.0, 1)];
    let pulse = [frame(0.0, 1), frame(1.0, 2)];
    let (Some(off), Some(on)) = (
        run(&quiet, (0, 0), (64, 64), &probes),
        run(&pulse, (0, 0), (64, 64), &probes),
    ) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(on[1], off[1], "the centre moved");
    assert!(
        f32::from(on[0][0]) > 1.3 * f32::from(off[0][0]),
        "the rim did not brighten: {:?} -> {:?}",
        off[0],
        on[0]
    );
    assert_eq!(on[0][3], off[0][3], "the ring wrote the frame's alpha");
}

/// A scene drawn into a sub-rectangle gets its ring there and nothing outside it.
#[test]
fn the_ring_stays_inside_the_drawn_rectangle() {
    let probes = [(2, 2), (17, 17), (10, 32), (60, 60)];
    let quiet = [frame(0.0, 1)];
    let pulse = [frame(0.0, 1), frame(1.0, 2)];
    let (Some(off), Some(on)) = (
        run(&quiet, (16, 16), (32, 32), &probes),
        run(&pulse, (16, 16), (32, 32), &probes),
    ) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    for outside in [0, 2, 3] {
        assert_eq!(on[outside], off[outside], "outside the rectangle moved");
    }
    assert!(
        f32::from(on[1][0]) > 1.3 * f32::from(off[1][0]),
        "the rectangle's own rim did not brighten: {:?} -> {:?}",
        off[1],
        on[1]
    );
}
