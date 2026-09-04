//! The chain against a render extent smaller than its allocation - the shape
//! dynamic resolution actually runs, and the one nothing covered until the
//! raster stopped ignoring it.
//!
//! Its own file beside [`super::tests`] for the reason `fsr3/reset_tests.rs`
//! is one: `tests.rs` is already near the 1,000 lines a file may hold. See
//! `scripts/check-file-size.py`.
//!
//! **The work these pin has no pixel observable; the mistakes it invites
//! do.** A pass that rasterises the whole allocation and one that rasterises
//! the viewport produce byte-identical scenes - `fs_reconstruct` returns the
//! centre tap unchanged outside the rectangle and `fs_copy` samples a texel
//! centre exactly - so the waste was invisible in the output and only ever
//! visible in the time. What restricting the raster *can* break is visible,
//! and both halves are asserted below: a wrong tile rectangle loses the smear
//! inside the viewport, and a load operation left as a clear wipes everything
//! outside it, which is what the marker caught the first time this ran.

use super::*;

#[test]
fn a_reduced_rectangle_covers_every_tile_the_viewport_touches() {
    // A 37-pixel extent on an 8-pixel tile grid: five tiles, the last of them
    // only five pixels wide. Ceiling the far edge is what includes it.
    assert_eq!(
        reduced((0.0, 0.0, 37.0, 37.0), (8.0, 8.0)),
        (0.0, 0.0, 5.0, 5.0)
    );
    // The full-size axis of the tile-rows target is not reduced at all.
    assert_eq!(
        reduced((0.0, 0.0, 37.0, 37.0), (8.0, 1.0)),
        (0.0, 0.0, 5.0, 37.0)
    );
    // An offset rectangle - a pillarboxed capture - floors its origin, so the
    // tile that only partly contains the rectangle is still covered.
    assert_eq!(
        reduced((12.0, 0.0, 40.0, 32.0), (8.0, 8.0)),
        (1.0, 0.0, 6.0, 4.0)
    );
}

#[test]
fn a_rectangle_is_clamped_inside_the_target_it_is_set_on() {
    // `reduced`'s ceiling lands one texel past a target whose size did not
    // divide evenly, and `set_viewport` validates against the attachment.
    assert_eq!(
        clamped((0.0, 0.0, 5.0, 37.0), (5, 36)),
        (0.0, 0.0, 5.0, 36.0)
    );
    // A degenerate target cannot produce a zero-width viewport either.
    assert_eq!(clamped((0.0, 0.0, 0.0, 0.0), (1, 1)), (0.0, 0.0, 1.0, 1.0));
}

/// A tile edge that moves - which is every dynamic-resolution step, since the
/// edge is `MAX_STRETCH` of the *extent* - rebuilds the three tile targets and
/// keeps the two allocation-sized ones.
///
/// **Skips when there is no adapter**, the same as every device test here.
#[test]
fn a_moving_tile_edge_does_not_reallocate_the_full_size_targets() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let Ok((device, _queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };

    let mut blur =
        MotionBlur::new(&device, wgpu::TextureFormat::Rgba8Unorm).expect("building the pipelines");
    blur.resize(&device, (256, 256), 16);
    let prepared = blur.prepared.as_ref().expect("resized").view.clone();
    let scratch = blur.scratch.as_ref().expect("resized").view.clone();
    let tiles = blur.tile_a.as_ref().expect("resized").view.clone();

    // The extent fell, so the reach cap and the tile edge fell with it.
    blur.resize(&device, (256, 256), 12);
    assert_eq!(blur.prepared.as_ref().expect("resized").view, prepared);
    assert_eq!(blur.scratch.as_ref().expect("resized").view, scratch);
    assert_ne!(
        blur.tile_a.as_ref().expect("resized").view,
        tiles,
        "the tile grid kept a target sized for the previous edge"
    );

    // An allocation change still rebuilds everything.
    blur.resize(&device, (128, 128), 12);
    assert_ne!(blur.prepared.as_ref().expect("resized").view, prepared);
    assert_ne!(blur.scratch.as_ref().expect("resized").view, scratch);
}

/// The whole chain with `viewport` a strict sub-rectangle of `size`: the
/// smear inside the rectangle is what a wrong tile rectangle would lose, so
/// this is the guard on [`reduced`] being wired to the right target.
///
/// **Skips when there is no adapter**, the same as every device test here.
#[test]
fn the_chain_smears_inside_a_viewport_smaller_than_its_allocation() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        eprintln!("no device; skipping");
        return;
    };

    // 64x64 allocated, 37x37 drawn - a rectangle that divides into neither
    // the allocation nor the 8-pixel tile grid.
    const SIZE: u32 = 64;
    const EXTENT: f32 = 37.0;
    const MARKER: u8 = 200;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };

    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    // A 2x2 white block at (18, 18), inside the rectangle; a marker at
    // (50, 50), outside it.
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 18..20 {
        for x in 18..20 {
            let at = ((y * SIZE + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    let marker_at = ((50 * SIZE + 50) * 4) as usize;
    pixels[marker_at..marker_at + 4].copy_from_slice(&[MARKER, MARKER, MARKER, 255]);
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: None,
        },
        extent,
    );
    let scene_view = scene.create_view(&Default::default());

    // Rightward motion everywhere: 0.1 uv/tick over a 37-pixel rectangle at
    // full shutter is 3.7 pixels of reach, well inside the 8-pixel cap.
    let velocity = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velocity"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::mesh_render::VELOCITY_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let moving = super::tests::f32_to_half(0.1).to_le_bytes();
    let zero = super::tests::f32_to_half(0.0).to_le_bytes();
    let mut texels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _ in 0..SIZE * SIZE {
        texels.extend_from_slice(&moving);
        texels.extend_from_slice(&zero);
    }
    queue.write_texture(
        velocity.as_image_copy(),
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: None,
        },
        extent,
    );
    let velocity_view = velocity.create_view(&Default::default());

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
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
    assert!(blur.render(
        &device,
        &queue,
        &mut encoder,
        &Frame {
            scene: &scene_view,
            velocity: &velocity_view,
            depth: &depth_view,
            sample_count: 1,
            size: (SIZE, SIZE),
            viewport: (0.0, 0.0, EXTENT, EXTENT),
            strength: 1.0,
        },
        None,
    ));

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
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the readback");
    let data = readback.slice(..).get_mapped_range().expect("mapping");
    let red_at = |x: u32, y: u32| data[((y * SIZE + x) * 4) as usize];

    assert!(
        red_at(17, 18) > 0 || red_at(16, 18) > 0,
        "no smear trailing the block inside a sub-rectangle viewport"
    );
    assert!(
        red_at(20, 18) > 0 || red_at(21, 18) > 0,
        "no smear leading the block inside a sub-rectangle viewport"
    );
    assert!(
        red_at(18, 18) < 255,
        "the block kept full brightness, so nothing was gathered"
    );
    // Outside the rectangle: untouched. A load operation covers the whole
    // attachment however small the viewport is, so the copy pass onto the
    // caller's own scene must not clear - this read 0 against 200 when it
    // did.
    assert_eq!(
        red_at(50, 50),
        MARKER,
        "the marker outside the rectangle moved"
    );
}
