//! The fly-by's plane set, drawn by a renderer built with no front-end video.

use super::*;

const SIZE: u32 = 8;

/// The renderer's colour at `(x, y)` of an 8x8 target after drawing `list`.
fn drawn(
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    list: &[Draw],
) -> [Vec<u8>; 2] {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fly-by test"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fly-by test readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render(
        device,
        queue,
        &mut encoder,
        &view,
        list,
        (0.0, 0.0, SIZE as f32, SIZE as f32),
        None,
    );
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
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
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let mapped = slice.get_mapped_range().expect("the readback");
    let pixel = |x: usize, y: usize| {
        let at = y * row as usize + x * 4;
        mapped[at..at + 4].to_vec()
    };
    [pixel(4, 4), pixel(0, 0)]
}

/// A clip drawn from planes of its own: nothing until a frame is in them, the
/// frame inside the rect and not outside it, and nothing again once cleared.
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn the_fly_by_draws_its_own_planes_only_once_they_hold_a_frame() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_space(Space {
        size: (SIZE as f32, SIZE as f32),
        display_aspect: 1.0,
    });
    let list = [Draw::Video {
        rect: [2.0, 2.0, 4.0, 4.0],
        frame: 0,
        position: 0,
        source: oag_ui::frontend::Video::FlyBy,
    }];

    renderer
        .set_fly_by(
            &device,
            VideoFormat {
                width: 2,
                height: 2,
                chroma_width: 1,
                chroma_height: 1,
            },
        )
        .expect("the fly-by planes");
    let [inside, _] = drawn(&mut renderer, &device, &queue, &list);
    assert_eq!(
        inside[..3],
        [0, 0, 0],
        "planes nothing has filled draw nothing"
    );

    // Limit-range white: Y 235, Cb and Cr 128.
    let frame = crate::movie::VideoFrame {
        bytes: [vec![235u8; 4], vec![128], vec![128]].concat(),
        ..Default::default()
    };
    renderer
        .upload_fly_by_frame(&queue, &frame)
        .expect("a frame of the planes' shape");
    let [inside, outside] = drawn(&mut renderer, &device, &queue, &list);
    assert!(
        inside[..3].iter().all(|channel| *channel > 230),
        "{inside:?}"
    );
    assert_eq!(outside[..3], [0, 0, 0], "the frame stays in its rect");

    renderer.clear_fly_by();
    let [inside, _] = drawn(&mut renderer, &device, &queue, &list);
    assert_eq!(inside[..3], [0, 0, 0], "a cleared panel draws nothing");
}
