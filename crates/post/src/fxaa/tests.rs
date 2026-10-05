//! What the FXAA pass in [`super`] is asserted to do: the uniform's layout, and
//! the pass drawn on a real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `fxaa.rs`: the tests are 288 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

#[test]
fn the_uniform_is_thirty_two_bytes_and_holds_the_inverse_size() {
    assert_eq!(std::mem::size_of::<Constants>(), 32);
    let c = Constants::new((1920, 1080), (1920, 1080));
    assert_eq!(c.inv_size, [1.0 / 1920.0, 1.0 / 1080.0]);
    assert_eq!(c.threshold, Constants::THRESHOLD);
    assert_eq!(c.relative_threshold, Constants::RELATIVE_THRESHOLD);
    // Exactly one, not an inset that evaluates close to it: this is what
    // keeps the pass byte-identical while nothing moves the render extent.
    assert_eq!(c.uv_scale, [1.0, 1.0]);
    assert_eq!(c.uv_max, [1.0, 1.0]);
}

/// A sub-rectangle scales the read and pulls the clamp inside it.
///
/// The tap offsets stay texels of the *resource* - a 5-tap cross has to step
/// one real texel whatever fraction of the texture is drawn, or the edge test
/// would widen as the render scale fell.
#[test]
fn a_drawn_sub_rectangle_scales_the_read_and_not_the_tap() {
    let c = Constants::new((960, 544), (1440, 816));
    assert_eq!(c.inv_size, [1.0 / 1440.0, 1.0 / 816.0]);
    assert_eq!(c.uv_scale, [960.0 / 1440.0, 544.0 / 816.0]);
    assert_eq!(c.uv_max, [959.5 / 1440.0, 543.5 / 816.0]);
}

#[test]
fn a_degenerate_size_cannot_divide_by_zero() {
    let c = Constants::new((0, 0), (0, 0));
    assert!(c.inv_size.iter().all(|v| v.is_finite()), "{:?}", c.inv_size);
}

/// Compiles the shader and runs the pass on a real device.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence
/// that it ran. Run it locally on real hardware.
#[test]
fn the_pass_builds_and_draws_on_a_real_device() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[format.remove_srgb_suffix()],
    });
    let source = scene.create_view(&wgpu::TextureViewDescriptor {
        format: Some(format.remove_srgb_suffix()),
        ..Default::default()
    });

    let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
    assert!(fxaa.output().is_none(), "no output before the first render");

    let mut encoder = device.create_command_encoder(&Default::default());
    fxaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (64, 64),
            viewport: (64, 64),
        },
    );
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");

    assert!(fxaa.output().is_some(), "the output target must exist");
}

/// A flat colour has no local contrast anywhere, so every pixel takes the
/// early-out and the pass must be a pure pass-through.
///
/// **Skips when there is no adapter.**
#[test]
fn a_flat_colour_is_untouched() {
    const SIZE: u32 = 32;
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    // `Rgba8Unorm`, not sRGB: this is checking the blend's arithmetic
    // against a known input, and an encode either way would put a
    // transfer function between what is written and what is asserted.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("flat"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = vec![128u8; (SIZE * SIZE * 4) as usize];
    for pixel in pixels.chunks_mut(4) {
        pixel[3] = 255;
    }
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: Some(SIZE),
        },
        scene.size(),
    );
    let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
    let mut encoder = device.create_command_encoder(&Default::default());
    fxaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (SIZE, SIZE),
            viewport: (SIZE, SIZE),
        },
    );

    let unpadded = (SIZE * 4) as usize;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * SIZE as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        fxaa.output_texture().expect("an output").as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
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
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");

    let row = SIZE / 2;
    let at = padded * row as usize + (SIZE / 2 * 4) as usize;
    assert_eq!(
        &mapped[at..at + 4],
        &[128, 128, 128, 255],
        "a flat colour must pass through untouched"
    );
}

/// A hard edge blends near the seam and leaves both flat sides alone -
/// the whole point of the contrast threshold existing at all.
///
/// **Skips when there is no adapter.**
#[test]
fn a_hard_edge_blends_only_near_the_seam() {
    const SIZE: u32 = 32;
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("edge"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let value = if x < SIZE / 2 { 0 } else { 255 };
            let at = ((y * SIZE + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: Some(SIZE),
        },
        scene.size(),
    );
    let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
    let mut encoder = device.create_command_encoder(&Default::default());
    fxaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (SIZE, SIZE),
            viewport: (SIZE, SIZE),
        },
    );

    let unpadded = (SIZE * 4) as usize;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * SIZE as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        fxaa.output_texture().expect("an output").as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
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
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");

    let row = SIZE / 2;
    let at = |x: u32| mapped[padded * row as usize + (x * 4) as usize];

    // Far from the seam, on either side, the pass leaves the flat plain
    // alone - no neighbour ever disagrees enough to cross the threshold.
    assert_eq!(at(2), 0, "the black side is not black");
    assert_eq!(at(SIZE - 3), 255, "the white side is not white");
    // At the seam itself, the two texels immediately either side see a
    // neighbour on the far side of the edge and must move off their
    // extreme value.
    let left_of_seam = at(SIZE / 2 - 1);
    let right_of_seam = at(SIZE / 2);
    assert!(
        left_of_seam > 0 || right_of_seam < 255,
        "neither texel at the seam moved: {left_of_seam} / {right_of_seam}"
    );
}
