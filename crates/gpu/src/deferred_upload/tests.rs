use super::*;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    copy: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::Buffer),
    len: u64,
) -> Vec<u8> {
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deferred readback"),
        size: len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    copy(&mut encoder, &out);
    queue.submit([encoder.finish()]);
    out.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("polls");
    out.slice(..).get_mapped_range().expect("range").to_vec()
}

/// A pattern no two chunks share, so a chunk written twice or not at all shows.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i / 251 + i) as u8).collect()
}

#[test]
fn a_buffer_waits_for_its_drain_and_lands_whole() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let source = pattern(CHUNK * 3 + 4096);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deferred"),
        size: source.len() as u64,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    assert!(!active(), "no scope, no deferral");
    let scope = Scope::open(&queue);
    assert!(active());
    defer_buffer(&buffer, 0, Arc::new(source.clone()), 0..source.len());
    assert_eq!(pending_bytes(), source.len() as u64);

    // One chunk per call: the budget is asked after every chunk.
    let mut steps = 0;
    let mut last = pending_bytes();
    while drain(|| false) > 0 {
        steps += 1;
        let now = pending_bytes();
        assert!(last - now <= CHUNK as u64, "a step wrote more than a chunk");
        last = now;
    }
    assert_eq!(steps, 3, "four chunks, the last one finishing the drain");
    assert_eq!(pending_bytes(), 0);
    let seen = read_back(
        &device,
        &queue,
        |e, out| e.copy_buffer_to_buffer(&buffer, 0, out, 0, source.len() as u64),
        source.len() as u64,
    );
    assert_eq!(seen, source);
    drop(scope);
    assert!(!active());
}

#[test]
fn a_texture_lands_in_row_bands() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let (width, height) = (256u32, 640u32);
    let source = pattern((width * height * 4) as usize);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("deferred"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let _scope = Scope::open(&queue);
    let level = Level {
        mip: 0,
        width,
        bytes_per_row: width * 4,
        rows: height,
        block_height: 1,
    };
    defer_texture(&texture, level, Arc::new(source.clone()), 0..source.len());
    // 640 KiB at 256 KiB a chunk: three bands, the last one short.
    let mut steps = 0;
    while drain(|| false) > 0 {
        steps += 1;
    }
    assert_eq!(steps, 2);
    let seen = read_back(
        &device,
        &queue,
        |e, out| {
            e.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: out,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        },
        source.len() as u64,
    );
    assert_eq!(seen, source);
}

#[test]
fn a_small_value_waits_in_a_scope_and_is_written_at_once_outside_one() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deferred value"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let value = [1.0f32, 2.0, 3.0, 4.0];
    write_value(&queue, &buffer, &value);
    assert_eq!(pending_bytes(), 0, "no scope: written at once");
    let scope = Scope::open(&queue);
    write_value(&queue, &buffer, &[5.0f32, 6.0, 7.0, 8.0]);
    assert_eq!(pending_bytes(), 16, "a scope parks even 16 bytes");
    assert_eq!(drain(|| true), 0);
    let seen = read_back(
        &device,
        &queue,
        |e, out| e.copy_buffer_to_buffer(&buffer, 0, out, 0, 16),
        16,
    );
    assert_eq!(seen, bytemuck::cast_slice::<f32, u8>(&[5.0, 6.0, 7.0, 8.0]));
    drop(scope);
}
