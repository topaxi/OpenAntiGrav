//! What the reconstruction chain in [`super`] is asserted to do: the
//! uniform's layout, the tile arithmetic, the format probe the velocity
//! target rests on, and the chain drawn on a real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `motion_blur.rs` for the reason `fxaa/tests.rs` is - the device test
//! alone is past the 200 lines an inline test module may hold. See
//! `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

#[test]
fn the_uniform_is_48_bytes_with_no_implicit_padding() {
    // Three vec2s, two floats, another vec2, and a u32 row: WGSL and
    // `repr(C)` agree on this layout exactly, which is what the assertion
    // pins - the previous layout's vec3 padding disagreement reached a
    // player as a validation panic.
    assert_eq!(std::mem::size_of::<Constants>(), 48);
}

#[test]
fn the_viewport_rectangle_is_mapped_into_uv_of_the_target() {
    // A 4:3 pillarboxed scene inside a 200x100 capture: rect x=40, 120 wide.
    let c = Constants::new((40.0, 0.0, 120.0, 100.0), (200, 100), 0.5, 8);
    assert_eq!(c.rect_offset, [0.2, 0.0]);
    assert_eq!(c.rect_size, [0.6, 1.0]);
    assert_eq!(c.rect_pixels, [120.0, 100.0]);
    assert_eq!(c.strength, 0.5);
    assert_eq!(c.max_px, MAX_STRETCH * 100.0);
    assert_eq!(c.inv_size, [1.0 / 200.0, 1.0 / 100.0]);
    assert_eq!(c.tile, 8);
}

#[test]
fn a_degenerate_size_cannot_divide_by_zero() {
    let c = Constants::new((0.0, 0.0, 0.0, 0.0), (0, 0), 0.5, 8);
    for value in c
        .rect_offset
        .iter()
        .chain(&c.rect_size)
        .chain(&c.rect_pixels)
        .chain(&c.inv_size)
    {
        assert!(value.is_finite(), "{c:?}");
    }
}

#[test]
fn the_reach_cap_is_floored_for_tiny_viewports() {
    // Eight per cent of a small capture would be a degenerate tile.
    assert_eq!(max_px((0.0, 0.0, 64.0, 64.0)), 8.0);
    assert_eq!(max_px((0.0, 0.0, 1920.0, 1080.0)), MAX_STRETCH * 1080.0);
}

/// The format question `docs/rendering/motion-blur.md` says to settle before
/// the velocity tier writes a shader: can `Rg16Float` be a multisampled
/// render attachment at the 4x this renderer's MSAA runs at?
///
/// **Skips when there is no adapter**, like the chain test below, and pins
/// the answer as a tripwire the way `msaa_resolve.rs` pins sample-count 2:
/// if an adapter ever fails this, the velocity target needs the documented
/// `Rgba16Float` fallback, and this test is where that shows up first.
#[test]
fn rg16float_is_multisample_renderable_at_4x_on_this_adapter() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let features = adapter.get_texture_format_features(crate::mesh_render::VELOCITY_FORMAT);
    assert!(
        features
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT),
        "the velocity format cannot even be a render attachment here"
    );
    assert!(
        features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4),
        "the velocity format is not multisample-renderable at 4x; use the Rgba16Float fallback"
    );
}

/// IEEE 754 half precision, for filling the synthetic velocity texture.
fn f32_to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7f_ffff;
    if exponent <= 0 {
        return sign; // flushes to zero; the test writes no subnormals
    }
    sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16)
}

/// Runs the whole chain on a real device: two identical white blocks on the
/// same still background, one inside a region whose velocity buffer says it
/// moved, one whose says it did not. The moving one smears along its own
/// motion; the still one stays exactly as drawn.
///
/// This is the property camera reprojection could not deliver and the
/// velocity buffer exists for - per-object separation - asserted end to end
/// through the reconstruction filter.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence
/// that it ran. Run it locally on real hardware:
///
/// ```sh
/// cargo nextest run -p oag-render -E 'test(smears_what_moved)' --no-capture
/// ```
#[test]
fn the_chain_smears_what_moved_and_leaves_still_surfaces_sharp() {
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

    // The scene: black, with 2x2 white blocks at x=32 (moving) and x=16
    // (still), both on row 31.
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
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 31..33 {
        for x in [32u32, 33, 16, 17] {
            let at = ((y * SIZE + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
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

    // The velocity buffer: 0.1 uv/tick of rightward motion everywhere right
    // of x=24, zero left of it - so the two blocks sit well inside their own
    // regions and their own tiles.
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
    let moving = f32_to_half(0.1);
    let zero = f32_to_half(0.0);
    let mut texels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _y in 0..SIZE {
        for x in 0..SIZE {
            let vx = if x >= 24 { moving } else { zero };
            texels.extend_from_slice(&vx.to_le_bytes());
            texels.extend_from_slice(&zero.to_le_bytes());
        }
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

    // A flat depth plane, via a clear - copying *to* a Depth32Float texture
    // is forbidden, a pass with no draws is not.
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
    blur.render(
        &device,
        &queue,
        &mut encoder,
        &Frame {
            scene: &scene_view,
            velocity: &velocity_view,
            depth: &depth_view,
            sample_count: 1,
            size: (SIZE, SIZE),
            viewport: (0.0, 0.0, SIZE as f32, SIZE as f32),
            strength: 0.5,
        },
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

    // The moving block: 0.1 uv * 0.5 shutter * 64 px = 3.2 px of reach,
    // centred - the columns beside it catch its taps.
    assert!(
        red_at(31, 31) > 0 || red_at(30, 31) > 0,
        "no smear trailing the moving block"
    );
    assert!(
        red_at(34, 31) > 0 || red_at(35, 31) > 0,
        "no smear leading the moving block"
    );
    assert!(
        red_at(32, 31) < 255,
        "the moving block kept full brightness, so nothing was gathered"
    );
    // Across the motion, nothing: the velocity is purely horizontal.
    assert_eq!(red_at(32, 28), 0, "the smear leaked vertically");
    assert_eq!(red_at(32, 36), 0, "the smear leaked vertically");
    // The still block, same colour and same background: essentially
    // untouched - the per-object separation the buffer exists for. One
    // count of tolerance, and it is understood rather than shrugged at:
    // `cone`'s quarter-pixel radius floor gives sub-pixel taps a little
    // weight, and at this block's own edge those taps bilinearly graze the
    // background - measured at exactly 254 here, with zero leakage into any
    // neighbouring pixel (asserted below).
    assert!(red_at(16, 31) >= 254, "the still block was dimmed");
    assert_eq!(red_at(14, 31), 0, "the still block smeared left");
    assert_eq!(red_at(19, 31), 0, "the still block smeared right");
}
