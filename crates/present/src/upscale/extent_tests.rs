//! The allocation-versus-extent split of
//! [ADR-0037](../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md):
//! that the extent is clamped and reset where it has to be, that a full extent
//! is bit-for-bit the arrangement that predates the split, and that a short one
//! reads only the pixels that were drawn.
//!
//! Its own file rather than more of `tests.rs`, which is 874 lines against the
//! 1,000-line ratchet `just check-size` enforces.

use super::*;

/// A device, or `None` so the caller skips - the same shape `tests.rs` uses.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

#[test]
fn the_whole_source_is_exactly_one_and_not_an_inset_that_rounds_to_it() {
    assert_eq!(Source::WHOLE.uv_scale, [1.0, 1.0]);
    assert_eq!(Source::WHOLE.uv_max, [1.0, 1.0]);
    // The point of the constant: an unconditional `(extent - 0.5) / allocation`
    // would put `0.99987` here at 1920 wide, which compresses the read by half
    // a texel across the whole destination and shifts every interior sample -
    // silently changing the bytes of every `--presented` capture for a change
    // that is meant to be structurally inert.
    assert_eq!(Source::of((1920, 1080), (1920, 1080)), Source::WHOLE);
    assert_eq!(
        Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false).uv_scale,
        [1.0, 1.0]
    );
}

#[test]
fn a_short_axis_is_inset_by_half_a_texel_and_a_full_one_is_not() {
    // Only one axis short, which `target_size` can produce on its own: it
    // clamps each axis independently against the device's maximum texture
    // dimension, so a wide window at a high scale can hit the ceiling on one
    // and not the other.
    let source = Source::of((960, 1080), (1920, 1080));
    assert_eq!(source.uv_scale, [0.5, 1.0]);
    assert_eq!(source.uv_max, [(960.0 - 0.5) / 1920.0, 1.0]);
    // Half a texel inside, not a whole one: at `uv_max` the sampler lands
    // exactly on the last drawn texel's centre, so the linear tap gives the
    // texel outside the rectangle a weight of zero.
    assert!(source.uv_max[0] < source.uv_scale[0]);
}

#[test]
fn the_extent_cannot_be_set_past_the_allocation() {
    let Some((device, _)) = device() else {
        return;
    };
    let mut framebuffer = Framebuffer::new(&device, wgpu::TextureFormat::Rgba8Unorm, (64, 32))
        .expect("a framebuffer");
    assert_eq!(framebuffer.allocation(), (64, 32));
    assert_eq!(framebuffer.extent(), (64, 32));

    framebuffer.set_extent((32, 16));
    assert_eq!(framebuffer.extent(), (32, 16));
    assert_eq!(framebuffer.allocation(), (64, 32));

    // Clamped rather than trusted. An extent past the allocation sets a
    // viewport past the attachment, which is a validation error rather than a
    // bad picture.
    framebuffer.set_extent((999, 999));
    assert_eq!(framebuffer.extent(), (64, 32));
    // And never zero: a zero-width viewport is the same class of error.
    framebuffer.set_extent((0, 0));
    assert_eq!(framebuffer.extent(), (1, 1));
}

#[test]
fn moving_the_extent_reallocates_nothing_and_a_reallocation_resets_it() {
    let Some((device, _)) = device() else {
        return;
    };
    let mut framebuffer = Framebuffer::new(&device, wgpu::TextureFormat::Rgba8Unorm, (64, 32))
        .expect("a framebuffer");

    framebuffer.set_extent((32, 16));
    // The whole point of the split: `resize` is handed the same ceiling it was
    // built with, so it rebuilds nothing however far the extent has moved.
    assert!(!framebuffer.resize(&device, (64, 32)));
    assert_eq!(framebuffer.extent(), (32, 16));

    // A window resize that shrinks the allocation must not leave the larger
    // extent behind - a controller re-applies its own value next frame, which
    // costs it one frame at full size and costs nothing to reason about.
    framebuffer.set_extent((64, 32));
    assert!(framebuffer.resize(&device, (16, 8)));
    assert_eq!(framebuffer.extent(), (16, 8));
}

/// Blits a target whose top-left `extent` is green and whose remainder is
/// black, reading `source` of it, and returns the four resulting pixels.
///
/// The pattern is drawn with the blit pipeline itself: `present` clears the
/// whole attachment black and then draws a `1x1` green texture into the
/// viewport it is given, which is the extent. So "outside the extent" holds
/// black - the stand-in for whatever a larger earlier frame left there.
fn blit_a_short_extent(source: Source) -> Option<[[u8; 4]; 4]> {
    let (device, queue) = device()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut framebuffer = Framebuffer::new(&device, format, (4, 4)).expect("a framebuffer");
    framebuffer.set_extent((2, 2));

    let green = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("green"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        green.as_image_copy(),
        &[0, 255, 0, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let green_view = green.create_view(&Default::default());
    let green_bind = framebuffer.source(&device, &green_view);

    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface"),
        size: wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let surface_view = surface.create_view(&Default::default());
    let row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row) * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    // The pattern, into the scene target: black everywhere, green in the
    // extent.
    framebuffer.present(
        &mut encoder,
        framebuffer.view(),
        (0.0, 0.0, 2.0, 2.0),
        Some(&green_bind),
    );
    // And the blit under test, reading `source` of that target.
    framebuffer.set_blit(&queue, Brightness::NEUTRAL, Gamma::NEUTRAL, false, source);
    framebuffer.present(&mut encoder, &surface_view, (0.0, 0.0, 2.0, 2.0), None);
    encoder.copy_texture_to_buffer(
        surface.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(2),
            },
        },
        wgpu::Extent3d {
            width: 2,
            height: 2,
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
        <[u8; 4]>::try_from(&mapped[at..at + 4]).expect("four bytes")
    };
    let read = [pixel(0, 0), pixel(1, 0), pixel(0, 1), pixel(1, 1)];
    drop(mapped);
    readback.unmap();
    Some(read)
}

/// The blit reads the drawn rectangle and nothing outside it.
///
/// **A bare `Framebuffer` and no `Scene`, deliberately**: there is no post
/// chain here to reach at all. No pass in one is viewport-aware yet - not the
/// upscaler, not FXAA or SMAA, and not bloom, which takes no size or viewport
/// whatsoever - so a sub-extent is a test-only thing until Phase 5.
/// `Framebuffer::set_extent` says so on itself.
#[test]
fn a_short_extent_is_read_without_the_pixels_outside_it() {
    let Some(read) = blit_a_short_extent(Source::of((2, 2), (4, 4))) else {
        return;
    };
    for pixel in read {
        assert_eq!(pixel, [0, 255, 0, 255], "read: {read:?}");
    }
}

/// The control, and the reason the test above means anything.
///
/// Told to read the whole texture instead, the same blit pulls in the three
/// quarters that were never drawn and comes back mostly black. Without this,
/// the assertion above would stay green against a shader that ignored
/// `uv_scale` entirely, on a target that happened to be uniform.
#[test]
fn reading_the_whole_allocation_instead_pulls_in_what_was_not_drawn() {
    let Some(read) = blit_a_short_extent(Source::WHOLE) else {
        return;
    };
    assert!(
        read.iter().any(|pixel| pixel[1] < 255),
        "reading the whole 4x4 should not come back uniformly green: {read:?}"
    );
}

/// FXAA and SMAA hand back a scene-sized frame, and the blit has to know that.
///
/// The bug this pins is invisible to every instrument this project has: both
/// arms of [`resolved_source`] agree while the extent is the allocation, which
/// is every frame the game draws until a controller moves it, so a
/// `--presented` capture passes either way. It would surface the first time
/// dynamic resolution stepped the scale - as a picture stretched by the
/// allocation-to-extent ratio - long after the change that caused it.
#[test]
fn only_an_upscaled_frame_is_read_whole() {
    // FSR 1 resolved to the presentation rectangle: its output is exactly the
    // size it was asked for, whatever the scene was drawn at.
    assert_eq!(
        resolved_source(true, (960, 544), (1440, 816)),
        Source::WHOLE
    );
    // FXAA or SMAA, or nothing at all: an allocation-sized target with the
    // drawn rectangle in its corner.
    assert_eq!(
        resolved_source(false, (960, 544), (1440, 816)),
        Source::of((960, 544), (1440, 816))
    );
    // And at the ceiling the two arms agree, which is exactly why the case
    // above needs a test of its own rather than a screenshot.
    assert_eq!(
        resolved_source(false, (1440, 816), (1440, 816)),
        resolved_source(true, (1440, 816), (1440, 816))
    );
}
