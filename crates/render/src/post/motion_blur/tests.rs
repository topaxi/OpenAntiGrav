//! What the motion blur pass in [`super`] is asserted to do: the uniform's
//! layout, the camera-pair rule, and the gather drawn on a real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `motion_blur.rs` for the reason `fxaa/tests.rs` is - the device test alone
//! is past the 200 lines an inline test module may hold. See
//! `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

use oag_core::math::{Vec3, vec3};

#[test]
fn the_uniform_is_304_bytes_with_no_implicit_padding() {
    // Two mat4s, three vec2s and two floats, a count padded to a row, and
    // eight focus spheres: WGSL and `repr(C)` agree on this layout exactly,
    // which is what the assertion pins.
    assert_eq!(std::mem::size_of::<Constants>(), 304);
}

#[test]
fn the_viewport_rectangle_is_mapped_into_uv_of_the_target() {
    // A 4:3 pillarboxed scene inside a 200x100 capture: rect x=40, 120 wide.
    let c = Constants::new(
        Mat4::IDENTITY,
        Mat4::IDENTITY,
        (40.0, 0.0, 120.0, 100.0),
        (200, 100),
        0.5,
        &[[0.5, 0.5, 8.0, 0.6]],
    );
    assert_eq!(c.rect_offset, [0.2, 0.0]);
    assert_eq!(c.rect_size, [0.6, 1.0]);
    assert_eq!(c.rect_pixels, [120.0, 100.0]);
    assert_eq!(c.strength, 0.5);
    assert_eq!(c.max_stretch, MAX_STRETCH);
    assert_eq!(c.focus_count, 1);
    assert_eq!(c.focus[0], [0.5, 0.5, 8.0, 0.6]);
    assert_eq!(c.focus[1], [0.0; 4], "unused slots stay zeroed");
}

#[test]
fn focus_spheres_past_the_uniforms_capacity_are_dropped_not_wrapped() {
    let spheres = vec![[0.1, 0.2, 3.0, 0.4]; MAX_FOCUS + 3];
    let c = Constants::new(
        Mat4::IDENTITY,
        Mat4::IDENTITY,
        (0.0, 0.0, 64.0, 64.0),
        (64, 64),
        0.5,
        &spheres,
    );
    assert_eq!(c.focus_count as usize, MAX_FOCUS);
}

#[test]
fn a_degenerate_size_cannot_divide_by_zero() {
    let c = Constants::new(
        Mat4::IDENTITY,
        Mat4::IDENTITY,
        (0.0, 0.0, 0.0, 0.0),
        (0, 0),
        0.5,
        &[],
    );
    for value in c
        .rect_offset
        .iter()
        .chain(&c.rect_size)
        .chain(&c.rect_pixels)
    {
        assert!(value.is_finite(), "{c:?}");
    }
}

#[test]
fn the_first_observation_seeds_both_cameras() {
    let mut camera = None;
    Camera::observe(&mut camera, Mat4::IDENTITY, 0);
    let camera = camera.expect("seeded");
    // Previous equals current, which is what `render` reads as "no travel"
    // and skips on - a scene's first frame must not smear.
    assert_eq!(camera.previous, camera.current);
}

#[test]
fn a_moved_camera_shifts_the_pair() {
    let moved = Mat4::from_translation(vec3(1.0, 0.0, 0.0));
    let mut camera = None;
    Camera::observe(&mut camera, Mat4::IDENTITY, 0);
    Camera::observe(&mut camera, moved, 1);
    let camera = camera.expect("seeded");
    assert_eq!(camera.previous, Mat4::IDENTITY);
    assert_eq!(camera.current, moved);
}

#[test]
fn a_frame_outrunning_the_tick_keeps_the_held_previous() {
    // Same camera, same stamp: a display rate above 60 Hz re-rendering an
    // unmoved simulation. The held previous survives, so the blur does not
    // flicker off on alternate frames.
    let moved = Mat4::from_translation(vec3(1.0, 0.0, 0.0));
    let mut camera = None;
    Camera::observe(&mut camera, Mat4::IDENTITY, 0);
    Camera::observe(&mut camera, moved, 1);
    Camera::observe(&mut camera, moved, 1);
    assert_eq!(camera.expect("seeded").previous, Mat4::IDENTITY);
}

#[test]
fn a_camera_that_held_still_for_a_tick_stops_blurring() {
    // Same camera, new stamp: the simulation moved and the camera did not -
    // a pause, a craft parked on the grid. The previous collapses onto the
    // current, which `render` reads as "no travel".
    let moved = Mat4::from_translation(vec3(1.0, 0.0, 0.0));
    let mut camera = None;
    Camera::observe(&mut camera, Mat4::IDENTITY, 0);
    Camera::observe(&mut camera, moved, 1);
    Camera::observe(&mut camera, moved, 2);
    let camera = camera.expect("seeded");
    assert_eq!(camera.previous, camera.current);
}

/// Compiles the shaders and runs the gather on a real device: a white block
/// under a purely horizontal camera shift smears horizontally and not
/// vertically.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence
/// that it ran. Run it locally on real hardware:
///
/// ```sh
/// cargo nextest run -p oag-render -E 'test(smears_along)' --no-capture
/// ```
#[test]
fn the_gather_smears_along_the_camera_travel_and_not_across_it() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    const SIZE: u32 = 64;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };

    // The scene: black, with a 2x2 white block at the centre.
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
    // Two 2x2 white blocks: one free at the centre, one at x=16 that the
    // focus sphere below covers - a stand-in craft.
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 31..33 {
        for x in [31u32, 32, 16, 17] {
            let at = ((y * SIZE + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(SIZE * 4),
        rows_per_image: None,
    };
    queue.write_texture(scene.as_image_copy(), &pixels, layout, extent);
    let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());

    // A flat depth plane. Not uploaded - copying *to* a Depth32Float texture
    // is forbidden - but a clear writes any value to every texel without a
    // single pipeline: a pass with the depth attachment and no draws.
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
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
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

    // Current camera: the identity, so world space *is* NDC. Previous: the
    // same view shifted along x, so every static point travelled purely
    // horizontally on screen. Far past the cap on purpose - the cap preserves
    // direction, and a bounded smear is exactly what it promises.
    let current = Mat4::IDENTITY;
    let previous = Mat4::from_translation(Vec3::new(0.8, 0.0, 0.0));
    // A focus sphere over the block at x=16: centre in viewport uv, a 6 px
    // radius, and a depth bound past the flat 0.5 plane so the whole disc
    // counts as "the craft".
    let focus = [[17.0 / SIZE as f32, 32.0 / SIZE as f32, 6.0, 0.6]];
    let frame = |view_projection, stamp| Frame {
        scene: &scene_view,
        depth: &depth_view,
        size: (SIZE, SIZE),
        viewport: (0.0, 0.0, SIZE as f32, SIZE as f32),
        view_projection,
        stamp,
        strength: 0.5,
        focus: &focus,
    };

    let mut encoder = device.create_command_encoder(&Default::default());
    // Seeds the pair; previous == current, so this encodes nothing.
    blur.render(&device, &queue, &mut encoder, &frame(previous, 0));
    // The camera moved: this one gathers.
    blur.render(&device, &queue, &mut encoder, &frame(current, 1));

    // Read the result back.
    let bytes_per_row = SIZE * 4; // 256 already, aligned.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout,
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |result| {
        result.expect("mapping the readback");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the readback");
    let data = slice.get_mapped_range().expect("the mapped range");
    let red_at = |x: u32, y: u32| data[((y * SIZE + x) * 4) as usize];

    // The block bled horizontally: with the travel capped at 5% of 64 pixels,
    // the gather reaches about +-1.6 pixels, so the columns just outside the
    // block catch some of its taps.
    assert!(
        red_at(30, 31) > 0 || red_at(29, 31) > 0,
        "no smear to the left of the block"
    );
    assert!(
        red_at(33, 31) > 0 || red_at(34, 31) > 0,
        "no smear to the right of the block"
    );
    // And spent energy doing it: a smeared centre is dimmer than the source.
    assert!(
        red_at(31, 31) < 255,
        "the centre kept full brightness, so nothing was gathered"
    );
    // Across the travel, nothing: three rows up is past any vertical reach.
    assert_eq!(red_at(31, 28), 0, "the smear leaked vertically");
    assert_eq!(red_at(31, 36), 0, "the smear leaked vertically");
    // The focused block is the stand-in craft: sharp, undimmed, and smearing
    // into nothing - the pixels beside it inside the sphere gather nothing
    // either, so they stay the background they were.
    assert_eq!(red_at(16, 31), 255, "the focused block was dimmed");
    assert_eq!(red_at(14, 31), 0, "the focused block smeared left");
    assert_eq!(red_at(19, 31), 0, "the focused block smeared right");
}
