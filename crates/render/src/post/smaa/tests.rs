//! What the SMAA passes in [`super`] are asserted to do: the packed lookup
//! textures decode to the bytes upstream ships, and all three passes draw on a
//! real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `smaa.rs`: the tests are 296 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

#[test]
fn the_lookup_textures_decode_to_exactly_the_bytes_upstream_ships() {
    assert_eq!(AREA_TEX.len(), (AREA_WIDTH * AREA_HEIGHT * 2) as usize);
    assert_eq!(SEARCH_TEX.len(), (SEARCH_WIDTH * SEARCH_HEIGHT) as usize);
}

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

/// Compiles all three shaders and runs all three passes on a real device.
///
/// This is where a WGSL syntax mistake actually shows up: `cargo check`
/// only compiles the Rust half, and `include_str!` hands the shader text
/// to the device unexamined until a real `create_shader_module` parses
/// and validates it. **Skips when there is no adapter.**
#[test]
fn all_three_passes_build_and_draw_on_a_real_device() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };

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

    let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
    assert!(smaa.output().is_none(), "no output before the first render");

    let mut encoder = device.create_command_encoder(&Default::default());
    smaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (64, 64),
        },
    );
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");

    assert!(smaa.output().is_some(), "the output target must exist");
}

/// A flat colour has no edges anywhere, so pass 1 discards every pixel,
/// pass 2 sees an all-zero edges texture and pass 3 must be a pure
/// pass-through - the whole three-pass pipeline reduces to an identity
/// on a picture with nothing to anti-alias.
///
/// **Skips when there is no adapter.**
#[test]
fn a_flat_colour_is_untouched() {
    const SIZE: u32 = 32;
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };

    // `Rgba8Unorm`, not sRGB: checking the pipeline's arithmetic against
    // a known input, the same reason `fxaa`'s equivalent test does.
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

    let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
    let mut encoder = device.create_command_encoder(&Default::default());
    smaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (SIZE, SIZE),
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
        smaa.output_texture().expect("an output").as_image_copy(),
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

/// **A perfectly straight edge is exactly the case SMAA leaves alone.**
/// Unlike `fxaa`, which blurs any sufficiently sharp edge indiscriminately
/// along its axis, SMAA's blending weight only comes from `smaa_area`,
/// and that lookup returns zero unless the search along the edge finds a
/// *crossing* edge - a corner - within `SMAA_MAX_SEARCH_STEPS` texels. A
/// perfectly vertical or horizontal one-pixel transition has no corner
/// anywhere, so it is already the correct pixel-grid representation of a
/// straight line and SMAA (correctly) does nothing to it. This is one of
/// SMAA's real advantages over FXAA - it does not soften clean UI edges -
/// and it is why this test uses a one-step staircase rather than a
/// straight edge: a straight edge would prove nothing here.
///
/// **Skips when there is no adapter.**
#[test]
fn a_staircase_step_blends_at_the_corner_and_nowhere_else() {
    const SIZE: u32 = 64;
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("staircase"),
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
    // One step: rows 0..32 have their black/white split at x=28, rows
    // 32..64 at x=32 - a single corner at the row-32 boundary, and
    // nothing but straight vertical edge everywhere else.
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        let split = if y < SIZE / 2 { 28 } else { 32 };
        for x in 0..SIZE {
            let value = if x < split { 0 } else { 255 };
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

    let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
    let mut encoder = device.create_command_encoder(&Default::default());
    smaa.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            size: (SIZE, SIZE),
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
        smaa.output_texture().expect("an output").as_image_copy(),
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
    let at = |x: u32, y: u32| mapped[padded * y as usize + (x * 4) as usize];

    // Far from the corner (more than `SMAA_MAX_SEARCH_STEPS` = 8 rows
    // away, at row 10), the vertical search cannot reach the step, and
    // the straight edge at x=28 must survive exactly as drawn: no
    // crossing edge means no blend weight, per the doc comment above.
    assert_eq!(at(26, 10), 0, "far from the corner, the black side moved");
    assert_eq!(at(30, 10), 255, "far from the corner, the white side moved");
    assert_eq!(
        at(28, 10),
        255,
        "far from the corner, the seam itself moved"
    );

    // At row 32 - immediately below the step, well inside the search
    // radius - the corner gives `smaa_area` a real crossing edge to
    // measure, and the texels between the two edge positions (x=28 and
    // x=32) must show a real antialiased ramp rather than a hard 0/255
    // cut.
    let ramp: Vec<u8> = (28..32).map(|x| at(x, 32)).collect();
    assert!(
        ramp.iter().any(|&v| v > 0 && v < 255),
        "no antialiasing near the staircase corner: {ramp:?}"
    );
}
