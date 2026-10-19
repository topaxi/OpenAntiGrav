use super::*;

/// The kernel is the eleven bytes at `DAT_08ab2348`, in order, and it is
/// symmetric. A future edit that "tidies" it into a Gaussian would change
/// a recovered value into a fitted one.
#[test]
fn the_kernel_is_the_recovered_bytes() {
    assert_eq!(BLUR_WEIGHTS, [20, 30, 40, 50, 64, 64, 64, 50, 40, 30, 20]);
    let reversed: Vec<u8> = BLUR_WEIGHTS.iter().copied().rev().collect();
    assert_eq!(reversed, BLUR_WEIGHTS, "the kernel is symmetric");
    assert_eq!(
        BLUR_WEIGHTS.len(),
        (BLUR_RADIUS * 2 + 1) as usize,
        "eleven taps span -5..+5"
    );
}

/// **The gain is the point.** `bloom.md` records `472/255 = 1.85` per axis
/// and `3.43x` over both; normalising the kernel would silently remove it.
#[test]
fn the_kernel_deliberately_brightens() {
    let sum: u32 = BLUR_WEIGHTS.iter().map(|&w| u32::from(w)).sum();
    assert_eq!(sum, 472);
    let gain = sum as f32 / 255.0;
    assert!(
        (gain - 1.851).abs() < 0.001,
        "per-axis gain {gain}, expected the recovered 1.85"
    );
    assert!(
        (gain * gain - 3.427).abs() < 0.01,
        "two-axis gain {}, expected the recovered 3.43",
        gain * gain
    );
}

/// `0xaf / 255`, the composite's `GU_FIX` source factor.
#[test]
fn the_composite_strength_is_the_recovered_byte() {
    assert_eq!(COMPOSITE_STRENGTH, 0xaf);
    let strength = f32::from(COMPOSITE_STRENGTH) / 255.0;
    assert!(
        (strength - 0.6863).abs() < 0.0001,
        "strength {strength}, expected the recovered 0.686"
    );
}

/// Half of the PSP's own 480 x 272, and fixed rather than derived from the
/// scene size - see this module's docs for why.
#[test]
fn the_buffers_are_half_the_psp_framebuffer() {
    assert_eq!((BLOOM_WIDTH * 2, BLOOM_HEIGHT * 2), (480, 272));
}
/// Runs the chain over a flat `rgb` / `alpha` scene and returns the
/// composite's mean red delta, or `None` with no adapter to run on. With
/// `hud`, the target is overwritten with that flat grey (mask 4) between
/// [`Bloom::prepare`] and [`Bloom::composite`], and the delta is over it.
fn mean_delta(rgb: u8, alpha: u8, hud: Option<u8>) -> Option<f32> {
    const SIZE: (u32, u32) = (512, 272);
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
        .into_iter()
        .next()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bloom test scene"),
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
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let fill = |rgb: u8, alpha: u8| {
        let texels: Vec<u8> = (0..SIZE.0 * SIZE.1)
            .flat_map(|_| [rgb, rgb, rgb, alpha])
            .collect();
        queue.write_texture(
            scene.as_image_copy(),
            &texels,
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
    };
    fill(rgb, alpha);
    let bloom = Bloom::new(&device, format).expect("the pipelines build");
    let view = scene.create_view(&wgpu::TextureViewDescriptor::default());
    let frame = Frame {
        scene: &view,
        size: SIZE,
        origin: (0.0, 0.0),
        viewport: SIZE,
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    bloom.prepare(&device, &queue, &mut encoder, frame);
    if let Some(hud) = hud {
        queue.submit([encoder.finish()]);
        fill(hud, 4);
        encoder = device.create_command_encoder(&Default::default());
    }
    bloom.composite(&mut encoder, &view, (0.0, 0.0), SIZE);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bloom test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
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
    let sum: u64 = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| u64::from(p[0]))
        .sum();
    let mean = sum as f32 / (SIZE.0 * SIZE.1) as f32;
    Some(mean - f32::from(hud.unwrap_or(rgb)))
}

/// **The mask stamp of `4` brightens nothing.** The original truncates
/// every tap to a byte, so a texel of at most 4 vanishes in the blur:
/// measured outside the craft on Talon's Junction as `0.33` mean luma in
/// the original against `4.52` for a float chain. A glow byte of `0xaf`
/// on the same colour must still brighten the frame.
#[test]
fn the_opaque_stamp_adds_no_glow_and_a_glow_byte_does() {
    let Some(stamped) = mean_delta(150, 4, None) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(stamped.abs() < 0.01, "a stamp of 4 added {stamped}");
    let white = mean_delta(255, 4, None).expect("the same adapter");
    assert!(white.abs() < 0.01, "a stamp of 4 on white added {white}");
    let glowing = mean_delta(150, 0xaf, None).expect("the same adapter");
    assert!(glowing > 20.0, "a glow byte added only {glowing}");
}

/// **The haze lands on what was drawn after the bright pass read.** The
/// original's HUD sits between `Bloom_Draw`'s reads and its composite in
/// queue order (keys `0x52`..`0x6d` before `0x70`), and live, with the
/// composite poked off, the same HUD glyphs read `(111, 206, 168)` against
/// `(160, 251, 235)` with it on. A HUD drawn over a glowing scene after
/// [`Bloom::prepare`] must come out brighter than its own colour.
#[test]
fn the_composite_adds_over_a_hud_drawn_after_the_bright_pass() {
    let Some(over_hud) = mean_delta(150, 0xaf, Some(60)) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(over_hud > 20.0, "the HUD took only {over_hud} of glow");
}
/// The mean red delta the glow adds over the left and right halves of a scene whose
/// **right half alone** carries a glow byte, prepared as one rectangle at
/// `origin` and composited over the right half. `through_view` runs it through a
/// split-screen view's lane of `extent` instead of the whole picture's buffers.
/// `None` with no adapter to run on.
fn halves_delta(origin: f32, through_view: Option<(u32, u32)>) -> Option<(f32, f32)> {
    const SIZE: (u32, u32) = (512, 272);
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
        .into_iter()
        .next()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bloom test scene"),
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
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texels: Vec<u8> = (0..SIZE.0 * SIZE.1)
        .flat_map(|i| {
            [
                150,
                150,
                150,
                if i % SIZE.0 >= SIZE.0 / 2 { 0xaf } else { 0 },
            ]
        })
        .collect();
    let extent = wgpu::Extent3d {
        width: SIZE.0,
        height: SIZE.1,
        depth_or_array_layers: 1,
    };
    queue.write_texture(
        scene.as_image_copy(),
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE.0 * 4),
            rows_per_image: Some(SIZE.1),
        },
        extent,
    );
    let bloom = Bloom::new(&device, format).expect("the pipelines build");
    let view = scene.create_view(&wgpu::TextureViewDescriptor::default());
    let half = (SIZE.0 / 2, SIZE.1);
    let frame = Frame {
        scene: &view,
        size: SIZE,
        origin: (origin, 0.0),
        viewport: half,
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    let at = (SIZE.0 as f32 / 2.0, 0.0);
    match through_view {
        None => {
            bloom.prepare(&device, &queue, &mut encoder, frame);
            bloom.composite(&mut encoder, &view, at, half);
        }
        Some(lane) => {
            bloom
                .prepare_view(&device, &queue, &mut encoder, 1, lane, frame)
                .expect("the lane builds");
            bloom.composite_view(&mut encoder, 1, &view, at, half);
        }
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bloom test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE.0 * 4),
                rows_per_image: Some(SIZE.1),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    let mut sums = [0u64; 2];
    for (i, p) in data.as_chunks::<4>().0.iter().enumerate() {
        sums[usize::from(i as u32 % SIZE.0 >= SIZE.0 / 2)] += u64::from(p[0]);
    }
    let texels = (SIZE.0 / 2 * SIZE.1) as f32;
    Some((
        sums[0] as f32 / texels - 150.0,
        sums[1] as f32 / texels - 150.0,
    ))
}

/// **A rectangle is read from where it is, not from the target's corner.** The
/// right half alone carries a glow byte. Read as the rectangle that starts at its
/// corner, the bright pass sees the unmasked left half and the right half stays
/// dark - what a split screen's second view got when the bloom ignored the
/// frame's origin; read from `origin` 256 it sees the mask and glows. The left
/// half is outside the composite's rectangle and never changes.
#[test]
fn a_rectangle_glows_from_its_own_pixels() {
    let Some((left_at_zero, right_at_zero)) = halves_delta(0.0, None) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        right_at_zero.abs() < 0.01,
        "read from the corner: {right_at_zero}"
    );
    assert_eq!(left_at_zero, 0.0);
    let (left, right) = halves_delta(256.0, None).expect("the same adapter");
    assert!(
        right > 20.0,
        "read from its own corner it added only {right}"
    );
    assert_eq!(left, 0.0, "the composite stays inside its rectangle");
}

/// A view's lane does the same at any size of its own, and leaves the whole
/// picture's buffers to the whole picture.
#[test]
fn a_view_lane_glows_from_its_own_pixels_at_its_own_size() {
    for extent in [(240, 136), (120, 136), (435, 136), (240, 272)] {
        let Some((left, right)) = halves_delta(256.0, Some(extent)) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        assert!(right > 20.0, "{extent:?}: a lane added only {right}");
        assert_eq!(left, 0.0, "{extent:?}: the composite stays inside its view");
    }
}

/// The extent keeps the original's own buffer for a whole picture and scales it
/// per axis by a view's share of the field, within bounds.
#[test]
fn a_lane_is_the_original_size_for_a_whole_picture() {
    let base = (BLOOM_WIDTH, BLOOM_HEIGHT);
    assert_eq!(lane_extent(base, (1.0, 1.0)), base);
    assert_eq!(lane_extent(base, (2.0, 1.0)), (480, 136));
    assert_eq!(lane_extent(base, (0.5, 2.0)), (120, 272));
    assert_eq!(lane_extent(base, (100.0, 0.0001)), (960, 8));
    assert_eq!(lane_extent(base, (f32::NAN, f32::INFINITY)), base);
}

/// The bright pass's group is built once for a scene view however many frames
/// read it, again for a different view (a resized or swapped scene), and a lane
/// rebuilt for a new extent starts with none - so a stale group is never bound.
#[test]
fn the_bright_group_is_kept_until_the_scene_view_or_the_extent_changes() {
    let Some((device, queue)) = crate::split_fixture::device() else {
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let make = |side: u32| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("bloom group test scene"),
                size: wgpu::Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let bloom = Bloom::new(&device, format).expect("the pipelines build");
    let frame = |scene, side: u32| Frame {
        scene,
        size: (side, side),
        origin: (0.0, 0.0),
        viewport: (side, side),
    };
    let (a, b) = (make(64), make(128));
    let mut encoder = device.create_command_encoder(&Default::default());
    for _ in 0..3 {
        bloom.prepare(&device, &queue, &mut encoder, frame(&a, 64));
    }
    assert_eq!(
        bloom.base.bright_group.builds(),
        1,
        "three frames, one group"
    );
    bloom.prepare(&device, &queue, &mut encoder, frame(&b, 128));
    assert_eq!(
        bloom.base.bright_group.builds(),
        2,
        "a resized scene rebuilds it"
    );

    for _ in 0..2 {
        bloom
            .prepare_view(&device, &queue, &mut encoder, 1, (64, 64), frame(&a, 64))
            .expect("a lane");
    }
    let lane_builds = || bloom.views.borrow()[1].bright_group.builds();
    assert_eq!(lane_builds(), 1, "a view's lane keeps its own");
    bloom
        .prepare_view(&device, &queue, &mut encoder, 1, (32, 32), frame(&a, 64))
        .expect("a lane");
    assert_eq!(
        lane_builds(),
        1,
        "a new extent is a new lane with one build, not a stale group"
    );
    drop(encoder);
}
