use super::*;

/// A PS2 front end is fitted as 480/272, not as its own 640x448.
///
/// The difference is 24%, it is a horizontal squeeze of the entire front
/// end, and it is invisible on the PSP because there the two numbers agree.
/// See `oag_display::space::Space` and `docs/ps2/aspect-ratio.md`.
#[test]
fn a_ps2_screen_is_letterboxed_by_what_it_is_shown_as() {
    // A window of the shape the artwork was stretched for: it fills.
    let window = (480 * 3, 272 * 3);
    let fitted = letterbox_in(window, Space::PS2.display_aspect);
    assert!((fitted[0] - 1.0).abs() < 1e-6, "{fitted:?}");
    assert!((fitted[1] - 1.0).abs() < 1e-6, "{fitted:?}");
    // Fitting it by the grid instead: 640/448 is *narrower* than that
    // window, so it gives the width away and leaves the front end in a
    // pillarboxed column a fifth short of the window it should have filled.
    let by_grid = letterbox_in(window, Space::PS2.size.0 / Space::PS2.size.1);
    assert!(
        by_grid[0] < 0.82,
        "the bug this exists to stop: {by_grid:?}"
    );
}

#[test]
fn a_matching_aspect_ratio_needs_no_letterboxing() {
    let scale = letterbox((960, 544));
    assert!((scale[0] - 1.0).abs() < 1e-6, "{scale:?}");
    assert!((scale[1] - 1.0).abs() < 1e-6, "{scale:?}");
}

#[test]
fn a_wide_window_shrinks_horizontally() {
    let scale = letterbox((1920, 544));
    assert!(scale[0] < 1.0);
    assert!((scale[1] - 1.0).abs() < 1e-6);
}

#[test]
fn a_tall_window_shrinks_vertically() {
    let scale = letterbox((480, 1000));
    assert!((scale[0] - 1.0).abs() < 1e-6);
    assert!(scale[1] < 1.0);
}

#[test]
fn a_zero_sized_window_does_not_divide_by_zero() {
    let scale = letterbox((0, 0));
    assert!(scale[0].is_finite() && scale[1].is_finite());
}

/// `render_with(LoadOp::Load, ...)` keeps what was already in `view` and
/// blends a translucent [`Draw::Fill`] over it, rather than clearing to
/// black the way [`Renderer::render`] does.
///
/// This is the mechanism [`crate::main::menu_stage`]'s pause overlay
/// stands on: `Session::frame` resolves a parked race's scene into the
/// presentation target and the menus then have to darken *that* rather
/// than replace it. Pinned here, at the one new primitive
/// (`render_with`'s visibility and the `LoadOp` choice), rather than
/// through a whole `Session` - building one needs a disc image this test
/// does not have.
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn a_translucent_fill_blends_over_whatever_load_kept() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    // A 1:1 grid over the 4x4 target below, so a full-screen `Draw::Fill`
    // covers it exactly with no letterbox bars to land the readback
    // texel in by accident.
    renderer.set_space(Space {
        size: (4.0, 4.0),
        display_aspect: 1.0,
    });

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pause overlay test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pause overlay readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    // Seeds `view` the way `Session::frame` leaves it after resolving a
    // parked race's scene: a real, non-black picture, so a bug that
    // cleared instead of loading would be visible rather than
    // coincidentally matching a black overlay.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("seed"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.8,
                    g: 0.4,
                    b: 0.2,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    renderer.render_with(
        wgpu::LoadOp::Load,
        &device,
        &queue,
        &mut encoder,
        &view,
        &[Draw::Fill {
            rect: [0.0, 0.0, 4.0, 4.0],
            // Half the alpha `menu_stage::PAUSE_OVERLAY` actually draws
            // with - a value of its own, not a re-import, so this test
            // still catches a blend-order mistake if that constant ever
            // changes.
            color: [0.0, 0.0, 0.0, 0.5],
        }],
        (0.0, 0.0, 4.0, 4.0),
        None,
    );

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(4),
            },
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    // The middle of the picture, away from any edge rounding.
    let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize + 4;
    let mapped = slice.get_mapped_range().expect("the readback");
    let pixel = [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]];
    drop(mapped);
    readback.unmap();

    // `src * a + dst * (1 - a)`, at `a = 0.5`: half the seeded colour.
    // `Rgba8Unorm` writes straight through with no sRGB encode, so this
    // is an ordinary linear blend rather than one that needs decoding
    // first - unlike `oag_present::upscale::tests::both_paths`, which is
    // exactly why that one exists.
    let expect = |channel: f64| ((channel * 0.5) * 255.0).round() as i32;
    for (index, channel) in [0.8, 0.4, 0.2].into_iter().enumerate() {
        let got = i32::from(pixel[index]);
        let want = expect(channel);
        assert!(
            (got - want).abs() <= 2,
            "channel {index}: got {got}, wanted {want} (blending {channel} at alpha 0.5), \
             full pixel {pixel:?} - `render_with(LoadOp::Load, ..)` did not blend over the \
             seeded picture the way the pause overlay depends on"
        );
    }
    // Alpha itself, which is not the same arithmetic as the colour
    // channels: `wgpu::BlendState::ALPHA_BLENDING`'s alpha component is
    // `BlendComponent::OVER` - source factor `One`, not `SrcAlpha` - so
    // it resolves to `0.5 * 1 + 1.0 * (1 - 0.5) = 1.0`. Compositing a
    // translucent layer over an opaque picture staying opaque is the
    // right answer (there is nothing behind either one for a resolve
    // downstream to show through), so this is what the blend mode
    // *should* do, not merely what it happens to do.
    assert!(
        i32::from(pixel[3]) >= 253,
        "alpha: got {}, wanted 255 (opaque stays opaque), full pixel {pixel:?}",
        pixel[3]
    );
}

/// `ui.wesl`'s `to_clip`, reimplemented independently of `to_grid` so the
/// round trip below cannot pass by sharing a mistake with it - the same
/// arrangement `menu_stage.rs`'s own `overlay_rect` test uses.
fn to_clip(space: Space, viewport: (f32, f32, f32, f32), grid: (f32, f32)) -> (f32, f32) {
    let scale = letterbox_in((viewport.2 as u32, viewport.3 as u32), space.display_aspect);
    (
        (2.0 * (grid.0 / space.size.0) - 1.0) * scale[0],
        (1.0 - 2.0 * (grid.1 / space.size.1)) * scale[1],
    )
}

/// And `RenderPass::set_viewport`'s: clip `-1..1` onto the rectangle.
fn to_window(viewport: (f32, f32, f32, f32), clip: (f32, f32)) -> (f32, f32) {
    (
        viewport.0 + (clip.0 + 1.0) * 0.5 * viewport.2,
        viewport.1 + (1.0 - clip.1) * 0.5 * viewport.3,
    )
}

/// A grid point drawn through the real chain and read back through
/// `to_grid` lands where it started - on every space, at every viewport
/// shape, including the ones where the letterbox actually bars something.
#[test]
fn to_grid_inverts_the_draw_chain_on_every_aspect() {
    let viewports = [
        (0.0, 0.0, 960.0, 544.0),
        // Wider than any title: bars down the sides.
        (0.0, 0.0, 1920.0, 800.0),
        // Taller: bars top and bottom.
        (0.0, 0.0, 800.0, 1000.0),
        // Offset by an aspect setting that did not fill the window.
        (240.0, 60.0, 1440.0, 810.0),
    ];
    for space in [Space::PSP, Space::PS2] {
        for viewport in viewports {
            for grid in [
                (0.0, 0.0),
                (space.size.0, space.size.1),
                (100.0, 50.0),
                (space.size.0 * 0.5, space.size.1 * 0.5),
                (space.size.0 - 1.0, 17.0),
            ] {
                let window = to_window(viewport, to_clip(space, viewport, grid));
                let back = to_grid(space, viewport, window);
                assert!(
                    (back.0 - grid.0).abs() < 1e-3 && (back.1 - grid.1).abs() < 1e-3,
                    "{space:?} at {viewport:?}: {grid:?} -> {window:?} -> {back:?}"
                );
            }
        }
    }
}

/// A click in the letterbox bars is off the grid, not clamped onto its
/// edge: nothing is drawn there, so nothing should be hit there.
#[test]
fn to_grid_maps_a_bar_to_a_point_off_the_grid() {
    // A window twice as wide as the PSP's shape: the picture fills the
    // middle half, and the leftmost quarter is a bar.
    let viewport = (0.0, 0.0, 960.0 * 2.0, 544.0);
    let (x, _) = to_grid(Space::PSP, viewport, (10.0, 272.0));
    assert!(x < 0.0, "{x}");
    let (x, _) = to_grid(Space::PSP, viewport, (1920.0 - 10.0, 272.0));
    assert!(x > Space::PSP.size.0, "{x}");
}
