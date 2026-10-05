//! The camera shake taken out of the blur's velocities, on a real device.
//!
//! The scene is one white block on black whose velocity buffer holds nothing
//! but the shake's own screen motion, which is what a still camera under an
//! impact shake writes. With the shake map the blur must leave the block
//! alone; without it the same buffer smears it - that second half is what
//! keeps the first from passing for a blur that does nothing.

use super::*;
use oag_core::math::Vec2;

const SIZE: u32 = 64;

fn f32_to_half(value: f32) -> u16 {
    super::tests::f32_to_half(value)
}

/// A view-space turn of about three degrees about each of two axes, seen
/// through a 60 degree projection: the order of the shake's own jump.
fn shake_map() -> Mat4 {
    let projection = oag_core::math::camera::perspective(1.0, 1.0, 1.0, 100.0);
    let turn = Mat4::from_rotation_y(0.05) * Mat4::from_rotation_z(0.04);
    projection * turn * projection.inverse()
}

/// What the scene would write for a still camera under `map`: the pixel's
/// ndc minus where the map sends it, halved and y-flipped.
fn velocity_under(map: Mat4, x: u32, y: u32) -> [f32; 2] {
    let ndc = Vec2::new(
        (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0,
        1.0 - (y as f32 + 0.5) / SIZE as f32 * 2.0,
    );
    let moved = map * ndc.extend(0.5).extend(1.0);
    let v = (ndc - moved.truncate().truncate() / moved.w) * Vec2::new(0.5, -0.5);
    [v.x, v.y]
}

/// The red channel after the chain ran with `camera_shake`, `None` when the
/// machine has no adapter.
fn blurred_red(camera_shake: Mat4) -> Option<impl Fn(u32, u32) -> u8> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: None,
        rows_per_image: None,
    };
    let scene = texture(
        "scene",
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
    );
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 31..33 {
        for x in 40u32..42 {
            let at = ((y * SIZE + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[255; 4]);
        }
    }
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            bytes_per_row: Some(SIZE * 4),
            ..layout
        },
        extent,
    );
    let velocity = texture(
        "velocity",
        oag_gpu::formats::VELOCITY_FORMAT,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    let mut texels = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            for v in velocity_under(shake_map(), x, y) {
                texels.extend_from_slice(&f32_to_half(v).to_le_bytes());
            }
        }
    }
    queue.write_texture(
        velocity.as_image_copy(),
        &texels,
        wgpu::TexelCopyBufferLayout {
            bytes_per_row: Some(SIZE * 4),
            ..layout
        },
        extent,
    );
    let depth = texture(
        "depth",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let depth_view = depth.create_view(&Default::default());
    let mut clear = device.create_command_encoder(&Default::default());
    clear.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("depth clear"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.5),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    queue.submit(Some(clear.finish()));

    let mut blur = MotionBlur::new(&device, format).expect("building the pipelines");
    let mut encoder = device.create_command_encoder(&Default::default());
    blur.render(
        &device,
        &queue,
        &mut encoder,
        &Frame {
            scene: &scene.create_view(&Default::default()),
            velocity: &velocity.create_view(&Default::default()),
            depth: &depth_view,
            sample_count: 1,
            size: (SIZE, SIZE),
            viewport: (0.0, 0.0, SIZE as f32, SIZE as f32),
            strength: 1.0,
            camera_shake,
        },
        None,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                bytes_per_row: Some(SIZE * 4),
                ..layout
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the readback");
    let data = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapping")
        .to_vec();
    Some(move |x: u32, y: u32| data[((y * SIZE + x) * 4) as usize])
}

/// A frame whose velocity buffer is nothing but the shake's screen motion
/// blurs nothing once the blur is given the shake's map, and blurs the block
/// without it.
#[test]
fn the_shake_is_taken_out_of_the_velocity_the_blur_reads() {
    let Some(without) = blurred_red(Mat4::IDENTITY) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let with = blurred_red(shake_map()).expect("an adapter, found a moment ago");
    // The block's own pixels and the column either side of it.
    let smeared = (38..44).any(|x| without(x, 31) != if (40..42).contains(&x) { 255 } else { 0 });
    assert!(
        smeared,
        "the shake's motion did not smear the block, so the test proves nothing"
    );
    for x in 38..44 {
        let wanted = if (40..42).contains(&x) { 255 } else { 0 };
        assert!(
            with(x, 31).abs_diff(wanted) <= 1,
            "column {x} read {} with the shake map, wanted {wanted}",
            with(x, 31)
        );
    }
}
