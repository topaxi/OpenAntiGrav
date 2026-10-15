//! What the split-view tests of the chains that grade a picture share: a device, a scene
//! target painted with known grey, a frame to resolve into and its red bytes read back.

pub(crate) const SIZE: (u32, u32) = (64, 64);

pub(crate) fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    let Some(adapter) = adapters.first() else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&Default::default()))
            .expect("a device with no extra features"),
    )
}

/// Writes `level(x, y)` (linear grey) into a scene target.
pub(crate) fn paint(queue: &wgpu::Queue, texture: &wgpu::Texture, level: impl Fn(u32, u32) -> f32) {
    let mut texels = Vec::new();
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let v = half(level(x, y));
            for channel in [v, v, v, half(0.0)] {
                texels.extend_from_slice(&channel);
            }
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE.0 * 8),
            rows_per_image: Some(SIZE.1),
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
}

/// An `f16` of a value in `0..=1`, as the scene target stores it.
fn half(value: f32) -> [u8; 2] {
    let table = [
        (0.0f32, 0x0000u16),
        (0.25, 0x3400),
        (0.5, 0x3800),
        (0.75, 0x3a00),
        (1.0, 0x3c00),
    ];
    let (_, bits) = table
        .iter()
        .find(|(v, _)| (*v - value).abs() < 1e-6)
        .expect("a level the table holds");
    bits.to_le_bytes()
}

/// Reads the red bytes of the frame `view` was resolved into.
pub(crate) fn read_red(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
) -> Vec<u8> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd split test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE.0 * 4),
                rows_per_image: Some(SIZE.1),
            },
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("the readback maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    let red = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|texel| texel[0])
        .collect();
    drop(data);
    readback.unmap();
    red
}

pub(crate) fn frame(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd split test frame"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The mean of `red` over the rows `rows` of the frame.
pub(crate) fn mean(red: &[u8], rows: std::ops::Range<u32>) -> f64 {
    let (from, to) = (rows.start * SIZE.0, rows.end * SIZE.0);
    let slice = &red[from as usize..to as usize];
    slice.iter().map(|&v| f64::from(v)).sum::<f64>() / slice.len() as f64
}
