use super::*;

const SIZE: (u32, u32) = (512, 272);

/// The target after `stamp_glow` of `stamps` over black: the red byte at `(x, y)` for each probe,
/// or `None` with no adapter to run on.
fn glow_at(stamps: &Stamps<'_>, probes: &[(u32, u32)]) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::ordered(&instance)
        .into_iter()
        .next()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stamp test target"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    queue.write_texture(
        target.as_image_copy(),
        &vec![0u8; (SIZE.0 * SIZE.1 * 4) as usize],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE.0 * 4),
            rows_per_image: Some(SIZE.1),
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    let bloom = Bloom::new(&device, format).expect("the pipelines build");
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    bloom.stamp_glow(
        &device,
        &queue,
        &mut encoder,
        &view,
        (0.0, 0.0),
        SIZE,
        stamps,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stamp test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
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
    queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    Some(
        probes
            .iter()
            .map(|&(x, y)| data[((y * SIZE.0 + x) * 4) as usize])
            .collect(),
    )
}

fn stamps(rects: &[[f32; 4]], value: u8) -> Stamps<'_> {
    Stamps {
        rects,
        colour: [0.49, 0.94, 0.75],
        opacity: 1.0,
        value,
    }
}

/// A stamped rectangle glows inside and a little past its edge, and nowhere far from it; a mask of
/// `51` glows less than `255`, and no rectangle or a mask of nothing adds nothing.
#[test]
fn a_stamp_glows_in_its_rectangle_and_scales_with_its_mask() {
    let rect = [[0.25, 0.25, 0.5, 0.25]];
    let probes = [(256, 100), (125, 100), (2, 2), (256, 271)];
    let Some(full) = glow_at(&stamps(&rect, 255), &probes) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(full[0] > 100, "inside the stamp: {full:?}");
    assert!(full[1] > 0, "just past its left edge: {full:?}");
    assert_eq!((full[2], full[3]), (0, 0), "far from it: {full:?}");
    let weak = glow_at(&stamps(&rect, 51), &probes).expect("the same adapter");
    assert!(
        weak[0] > 0 && weak[0] < full[0] / 2,
        "{weak:?} against {full:?}"
    );
    assert_eq!(glow_at(&stamps(&rect, 0), &probes).unwrap(), [0; 4]);
    assert_eq!(glow_at(&stamps(&[], 255), &probes).unwrap(), [0; 4]);
}
