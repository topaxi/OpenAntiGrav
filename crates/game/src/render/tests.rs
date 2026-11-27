use super::*;

/// A PS2 front end is fitted as 16:9, not as its own 640x448.
///
/// The difference is 24%, it is a horizontal squeeze of the entire front
/// end, and it is invisible on the PSP because there the two numbers agree.
/// See `oag_display::space::Space` and `docs/ps2/aspect-ratio.md`.
#[test]
fn a_ps2_screen_is_letterboxed_by_what_it_is_shown_as() {
    // A window of the shape the console outputs: it fills.
    let window = (16 * 80, 9 * 80);
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
/// This is the mechanism [`crate::app::menu_stage`]'s pause overlay
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
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
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
        texture_binding_view_dimension: None,
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

/// `from_grid` is `to_grid` run backwards, bars included: the pad's cursor is
/// drawn where a mouse at that grid point would sit.
#[test]
fn from_grid_is_the_inverse_of_to_grid() {
    for space in [Space::PSP, Space::PS2] {
        for viewport in [(0.0, 0.0, 1920.0, 800.0), (240.0, 60.0, 1440.0, 810.0)] {
            for window in [(300.0, 200.0), (1000.0, 500.0), (5.0, 790.0)] {
                let grid = to_grid(space, viewport, window);
                let back = from_grid(space, viewport, grid);
                assert!(
                    (back.0 - window.0).abs() < 1e-2 && (back.1 - window.1).abs() < 1e-2,
                    "{space:?} {viewport:?}: {window:?} -> {grid:?} -> {back:?}"
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

/// An outlined font is drawn as the PSP draws it, in two passes: the silhouette's alpha in the
/// border colour, then the body's grey level as alpha in the text colour.
///
/// The texel here is an antialiased edge: palette alpha 200 and grey 100. One pass that mixes the
/// two colours by the grey first (what the menu fonts need) gives it `100/255` white at about
/// `0.54` alpha, a pixel of about 42 over black; the two passes give `100/255` white over a
/// quarter-alpha black, a pixel of about 100. The original's HUD strokes read the second, which
/// is the stroke brightness this pins (`PulseHud.fnt`'s two CLUTs in a GE list of a Pulse grid).
///
/// **Skips with no adapter.**
#[test]
fn an_outlined_font_draws_its_body_alpha_over_its_outline_alpha() {
    use oag_texture::fnt::{Font, Glyph};
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
    let mut palette = vec![[0, 0, 0, 0]; 16];
    palette[1] = [100, 100, 100, 200];
    palette[2] = [255, 255, 255, 255];
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette,
        indices: (0..64)
            .map(|at| u8::from(at % 8 < 4 && at / 8 < 4))
            .collect(),
        glyphs: vec![Glyph {
            codepoint: u16::from(b'8'),
            width: 4,
            height: 4,
            u0: 0,
            u1: 4,
            v0: 0,
            v1: 4,
            advance: 4,
            bearing: 0,
        }],
    };
    assert!(font.is_outlined(), "the fixture must be an outlined font");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_space(Space {
        size: (4.0, 4.0),
        display_aspect: 1.0,
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("outlined font test"),
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
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("outlined font readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render_with(
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        &device,
        &queue,
        &mut encoder,
        &view,
        &[Draw::Text {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            border: Some([0.0, 0.0, 0.0, 0.25]),
            align: oag_ui::frontend::Align::Left,
            text: "8".to_owned(),
            wrap_width: None,
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
    let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize + 4;
    let red = i32::from(slice.get_mapped_range().expect("the readback")[at]);
    assert!(
        (red - 100).abs() <= 6,
        "an edge texel of grey 100 should read about 100 over black, got {red}: the two passes are not live"
    );
}

/// A translucent fill under an outlined font's atlas is drawn once, not twice.
///
/// An outlined atlas draws its text in two passes (the border's alpha, then the body's), and a fill
/// samples the same atlas's solid patch: with its border carrying the fill's own alpha the second
/// pass stacked the fill on itself, `2a - a^2` for an `a` of a quarter, 112 of 255 where 64 is one
/// draw. Pulse PS2's stat-row bars and panels read that bright against the original's.
///
/// **Skips with no adapter.**
#[test]
fn a_translucent_fill_under_an_outlined_font_is_drawn_once() {
    use oag_texture::fnt::{Font, Glyph};
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
    let mut palette = vec![[0, 0, 0, 0]; 16];
    palette[1] = [100, 100, 100, 200];
    palette[2] = [255, 255, 255, 255];
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette,
        indices: (0..64)
            .map(|at| u8::from(at % 8 < 4 && at / 8 < 4))
            .collect(),
        glyphs: vec![Glyph {
            codepoint: u16::from(b'8'),
            width: 4,
            height: 4,
            u0: 0,
            u1: 4,
            v0: 0,
            v1: 4,
            advance: 4,
            bearing: 0,
        }],
    };
    assert!(font.is_outlined(), "the fixture must be an outlined font");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_space(Space {
        size: (4.0, 4.0),
        display_aspect: 1.0,
    });
    let quarter = [1.0, 1.0, 1.0, 0.25];
    // Left half a flat fill, right half a gradient that is flat at the same alpha, in the top two rows, and a
    // solid triangle over the bottom two.
    let list = [
        Draw::Fill {
            rect: [0.0, 0.0, 2.0, 2.0],
            color: quarter,
        },
        Draw::GradientFill {
            rect: [2.0, 0.0, 2.0, 2.0],
            left: quarter,
            right: quarter,
        },
        Draw::SolidTriangles {
            triangles: vec![[[0.0, 2.0], [0.0, 10.0], [8.0, 2.0]]],
            color: quarter,
        },
    ];
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fill under an outlined font"),
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
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fill under an outlined font readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render_with(
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        &device,
        &queue,
        &mut encoder,
        &view,
        &list,
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
    let mapped = slice.get_mapped_range().expect("the readback");
    for (name, x, row) in [("flat", 0, 0), ("gradient", 3, 0), ("solid triangle", 1, 3)] {
        let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize * row + x * 4;
        let red = i32::from(mapped[at]);
        assert!(
            (red - 64).abs() <= 3,
            "a quarter-alpha white {name} fill over black should read about 64, got {red}: it was drawn twice"
        );
    }
}

/// A right-aligned HUD string ends on its last glyph's ink, not on its advance.
///
/// The Pulse grid's GE lists put `0 kmh`, `100%` and `Record` 2, 1 and 1 px right of the
/// advance-aligned pens: each last glyph's trailing bearing (advance less cell width), where the
/// record time's `8` has none. The fixture's last glyph is 4 wide with an advance of 7, so a
/// string right-aligned at 100 starts its last glyph 3 px further right than the advance would.
///
/// **Skips with no adapter.**
#[test]
fn a_right_aligned_hud_string_ends_on_its_last_glyphs_ink() {
    use oag_texture::fnt::{Font, Glyph};
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
    let glyph = |ch: u8, advance: u8| Glyph {
        codepoint: u16::from(ch),
        width: 4,
        height: 4,
        u0: 0,
        u1: 4,
        v0: 0,
        v1: 4,
        advance,
        bearing: 0,
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![glyph(b'a', 4), glyph(b'b', 7)],
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let last_left_at = |renderer: &mut Renderer, align: Align| {
        renderer.quads.clear();
        renderer.push_text(
            GlyphSlot::Primary,
            100.0,
            0.0,
            1.0,
            [1.0; 4],
            [0.0; 4],
            align,
            "ab",
            None,
        );
        *renderer.quads.last().expect("a glyph")
    };
    let last_left = |renderer: &mut Renderer| last_left_at(renderer, Align::Right);
    let by_advance = last_left(&mut renderer);
    renderer.set_whole_pixel_advances(true);
    let by_ink = last_left(&mut renderer);
    // The glyph rect is also the box grown half a pixel each side with the uv grown half a texel
    // (`225.5..249.5` for the box `226..249` in the GE list), so it starts 0.5 left of the ink.
    assert_eq!(by_ink.rect[0] - by_advance.rect[0], 3.0 - 0.5);
    assert_eq!((by_advance.rect[2], by_ink.rect[2]), (4.0, 5.0));
    assert_eq!((by_advance.uv[2], by_ink.uv[2]), (4.0, 5.0));
    // HD's rule (`HudArt::ink_aligned_text`) is the same on whole-pixel-free advances, and a centred
    // string loses half the bearing: 3 px right for the right-aligned one, 1.5 for the centred.
    renderer.set_whole_pixel_advances(false);
    let centred_by_advance = last_left_at(&mut renderer, Align::Centre);
    renderer.set_ink_aligned_text(true);
    assert_eq!(last_left(&mut renderer).rect[0] - by_advance.rect[0], 3.0);
    let centred_by_ink = last_left_at(&mut renderer, Align::Centre);
    assert_eq!(centred_by_ink.rect[0] - centred_by_advance.rect[0], 1.5);
}

/// `MenuSkin::snapped_pens`: each glyph is a whole-pixel sprite (`oag_ui::font::snapped_quad`).
///
/// A glyph of advance 9 and left bearing 1 at scale 0.8 (Pure's `Stats` face, `RACE RECORD`) drawn
/// from x 249 has its quads at 249, 257, 265 (ink edges 250, 258, 266), 3 px wide by 3 high, the
/// texture started half a texel early: the unsnapped pen lands at 249, 256.2, 263.4.
///
/// **Skips with no adapter.**
#[test]
fn a_snapped_pen_draws_each_glyph_at_a_whole_unit() {
    use oag_texture::fnt::{Font, Glyph};
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
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![
            Glyph {
                codepoint: u16::from(b'a'),
                width: 4,
                height: 4,
                u0: 2,
                u1: 6,
                v0: 0,
                v1: 4,
                advance: 9,
                bearing: 1,
            },
            Glyph {
                codepoint: u16::from(b'b'),
                width: 4,
                height: 4,
                u0: 0,
                u1: 4,
                v0: 0,
                v1: 4,
                advance: 9,
                bearing: 1,
            },
            Glyph {
                codepoint: u16::from(b'c'),
                width: 4,
                height: 4,
                u0: 2,
                u1: 6,
                v0: 2,
                v1: 6,
                advance: 9,
                bearing: 1,
            },
        ],
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let lefts = |renderer: &mut Renderer| -> Vec<f32> {
        renderer.quads.clear();
        renderer.push_text(
            GlyphSlot::Primary,
            249.0,
            0.0,
            0.8,
            [1.0; 4],
            [0.0; 4],
            Align::Left,
            "aaa",
            None,
        );
        renderer.quads.iter().map(|quad| quad.rect[0]).collect()
    };
    let loose = lefts(&mut renderer);
    let loose_uv = renderer.quads[0].uv[0];
    renderer.set_snapped_pens(true);
    assert_eq!(lefts(&mut renderer), [249.0, 257.0, 265.0]);
    assert!((loose[1] - 256.2).abs() < 1e-3, "{loose:?}");
    for quad in &renderer.quads {
        assert_eq!(
            quad.rect[2..],
            [3.0, 3.0],
            "whole pixels, floor of 4 texels at 0.8"
        );
    }
    assert_eq!(
        renderer.quads[0].uv[0],
        loose_uv - oag_ui::font::SNAPPED_UV_SHIFT,
        "the texture starts half a texel early"
    );
    // The box's `4 + 1` texels are spread over the sprite's fractional 3.2 pixels and cut to 3:
    // the original's `[u0 - 0.5, u1 + 0.5]` by `[v0 - 0.5, v1 + 0.5]`, on both axes.
    renderer.quads.clear();
    renderer.push_text(
        GlyphSlot::Primary,
        249.0,
        0.0,
        0.8,
        [1.0; 4],
        [0.0; 4],
        Align::Left,
        "c",
        None,
    );
    let span = 5.0 * 3.0 / (4.0 * 0.8);
    let uv = renderer.quads[0].uv;
    assert_eq!((uv[0], uv[1]), (1.5, 1.5), "half a texel early in x and y");
    assert!((uv[2] - span).abs() < 1e-5, "u span {}", uv[2]);
    assert!((uv[3] - span).abs() < 1e-5, "v span {}", uv[3]);
    // A glyph at the atlas's left edge has no texel before it: the shift stops there.
    renderer.quads.clear();
    renderer.push_text(
        GlyphSlot::Primary,
        249.0,
        0.0,
        0.8,
        [1.0; 4],
        [0.0; 4],
        Align::Left,
        "b",
        None,
    );
    assert_eq!(renderer.quads[0].uv[0], 0.0);

    // `font::snapped_run`, which every layout of a snapped face reads, is the extent the draw
    // makes: from the left edge, the last glyph's ink ends where `ink_right` says; a
    // right-aligned string puts that ink edge on the anchor.
    let atlas = Atlas::from_font(&font);
    let run = oag_ui::font::snapped_run(&atlas, "aaa", 0.8);
    let drawn = |renderer: &mut Renderer, align: Align| -> (f32, f32) {
        renderer.quads.clear();
        renderer.push_text(
            GlyphSlot::Primary,
            249.0,
            0.0,
            0.8,
            [1.0; 4],
            [0.0; 4],
            align,
            "aaa",
            None,
        );
        let last = renderer.quads.last().expect("a glyph");
        (renderer.quads[0].rect[0], last.rect[0] + last.rect[2])
    };
    let (first, ink_end) = drawn(&mut renderer, Align::Left);
    assert!((ink_end - 249.0 - run.ink_right).abs() < 1e-3);
    assert_eq!(first, 249.0);
    // Right-aligned at 249: the last glyph's ink ends on 249, the pen starting `ink_right` to
    // the left (`docs/ui/menus-original.md`, "Pure's right-aligned text ends on its ink").
    let (right_first, right_end) = drawn(&mut renderer, Align::Right);
    assert_eq!(right_first, (249.0 - run.ink_right).ceil());
    assert!((right_end - 249.0).abs() < 1.0, "ink ends at {right_end}");
    // Centred, the pen starts where the original's measure puts it (`snapped_centre_pen`), and
    // not at half the pen end the way a right-aligned or plain string is measured.
    let (centre_first, _) = drawn(&mut renderer, Align::Centre);
    let centre_pen = oag_ui::font::snapped_centre_pen(&atlas, "aaa", 0.8, 249.0);
    assert_eq!(centre_first, centre_pen.ceil());
    // `aaa` measures 18.4 (the pen end, 23.4, is not it): 249 less 9.2 less the bearing is 239.
    assert_eq!(centre_first, 239.0);
}

/// `MenuSkin::bearing_pens`: the first glyph's ink starts at x and each later glyph's ink sits its
/// bearing right of the pen, the next pen one advance on from the pen before. Track Select's heading
/// in `Pulse_14.fnt` (`T` bearing 0, every other letter -1) puts `r` at 59 and not 60.
///
/// **Skips with no adapter.**
#[test]
fn a_bearing_pen_starts_the_first_ink_at_x_and_steps_by_bearing() {
    use oag_texture::fnt::{Font, Glyph};
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
    let glyph = |ch: u8, advance: u8, bearing: i8| Glyph {
        codepoint: u16::from(ch),
        width: 4,
        height: 4,
        u0: 0,
        u1: 4,
        v0: 0,
        v1: 4,
        advance,
        bearing,
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![glyph(b'a', 9, 0), glyph(b'b', 8, -1)],
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let lefts = |renderer: &mut Renderer, text: &str| -> Vec<f32> {
        renderer.quads.clear();
        renderer.push_text(
            GlyphSlot::Primary,
            50.0,
            0.0,
            1.0,
            [1.0; 4],
            [0.0; 4],
            Align::Left,
            text,
            None,
        );
        renderer.quads.iter().map(|quad| quad.rect[0]).collect()
    };
    assert_eq!(lefts(&mut renderer, "ab"), [50.0, 59.0]);
    assert_eq!(lefts(&mut renderer, "ba"), [50.0, 58.0]);
    renderer.set_bearing_pens(true);
    assert_eq!(lefts(&mut renderer, "ab"), [50.0, 58.0]);
    assert_eq!(lefts(&mut renderer, "ba"), [50.0, 59.0]);
}

/// `HudArt::half_unit_glyph_quads`: a glyph is its cell grown half a grid unit on every side with the
/// uv grown half a texel, whatever the text scale - the lap count `3` of the RPCS3 draw capture is
/// 43 texels at 0.6 drawn 26.8 wide over a uv 44 wide (`43 * 0.6 + 1`). The authored
/// `borderExtendPixels` it replaces grows both by the same texels at the text's scale instead.
///
/// **Skips with no adapter.**
#[test]
fn a_half_unit_glyph_quad_grows_by_half_a_unit_at_any_scale() {
    use oag_texture::fnt::{Font, Glyph};
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
    let font = Font {
        line_height: 40,
        unknown: 0,
        width: 80,
        height: 80,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 6400],
        glyphs: vec![Glyph {
            codepoint: u16::from(b'3'),
            width: 20,
            height: 20,
            u0: 30,
            u1: 50,
            v0: 30,
            v1: 50,
            advance: 24,
            bearing: 0,
        }],
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::from_font(&font).with_border_extend(5),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let glyph = |renderer: &mut Renderer| {
        renderer.quads.clear();
        renderer.push_text(
            GlyphSlot::Primary,
            100.0,
            100.0,
            0.6,
            [1.0; 4],
            [0.0; 4],
            Align::Left,
            "3",
            None,
        );
        *renderer.quads.last().expect("a glyph")
    };
    let extended = glyph(&mut renderer);
    assert_eq!((extended.rect[2], extended.uv[2]), (30.0 * 0.6, 30.0));
    renderer.set_half_unit_glyph_quads(true);
    let half_unit = glyph(&mut renderer);
    assert_eq!(
        (half_unit.rect[2], half_unit.rect[3]),
        (20.0 * 0.6 + 1.0, 20.0 * 0.6 + 1.0)
    );
    assert_eq!((half_unit.uv[2], half_unit.uv[3]), (21.0, 21.0));
    assert_eq!(half_unit.rect[0], 100.0 - 0.5);
    assert_eq!(half_unit.uv[0], 29.5);
}

/// A renderer keeps its quad list between frames: the second frame of the same list lays out
/// into the buffer the first one grew, so a text-heavy HUD allocates nothing for its quads after
/// the first frame.
///
/// **Skips with no adapter.**
#[test]
fn the_quad_list_survives_into_the_next_frame() {
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
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("quad reuse test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let list: Vec<Draw> = (0..40)
        .map(|row| Draw::Text {
            x: 10.0,
            y: 10.0 + row as f32 * 8.0,
            scale: 1.0,
            color: [1.0; 4],
            border: None,
            align: Align::Left,
            text: "SPEED 0123456789".to_string(),
            wrap_width: None,
        })
        .collect();
    let frame = |renderer: &mut Renderer| {
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render_with(
            wgpu::LoadOp::Load,
            &device,
            &queue,
            &mut encoder,
            &view,
            &list,
            (0.0, 0.0, 4.0, 4.0),
            None,
        );
        queue.submit(Some(encoder.finish()));
    };
    frame(&mut renderer);
    assert!(
        renderer.quads.len() >= 40 * 16,
        "the frame lays out its text: {}",
        renderer.quads.len()
    );
    // A capacity no 640-quad frame would reach by itself: only a list kept between frames still
    // has it afterwards.
    renderer.quads.reserve(1 << 14);
    let kept = renderer.quads.capacity();
    frame(&mut renderer);
    assert_eq!(renderer.quads.capacity(), kept, "the list was rebuilt");
}

/// A bent draw list carries the `Dynamic HUD` warp on every quad up to [`Warp::until`], cuts a
/// columned bar into four, leaves the draws after `until` flat, and is spent by that draw: the
/// next one, with no warp set, is flat again.
///
/// **Skips with no adapter.**
#[test]
fn a_warp_bends_its_draws_cuts_its_bars_and_is_spent() {
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
    renderer.set_space(Space::PSP);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("warp test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let bar = [304.0, 240.0, 140.0, 8.0];
    let icon = [20.0, 10.0, 16.0, 16.0];
    let reticle = [200.0, 100.0, 16.0, 16.0];
    let list: Vec<Draw> = [bar, icon, reticle]
        .into_iter()
        .map(|rect| Draw::Sprite {
            rect,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [1.0; 4],
        })
        .collect();
    let frame = |renderer: &mut Renderer| {
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.overlay(
            &device,
            &queue,
            &mut encoder,
            &view,
            &list,
            (0.0, 0.0, 4.0, 4.0),
        );
        queue.submit(Some(encoder.finish()));
        renderer.quads.clone()
    };
    renderer.set_warp(Some(Warp {
        columns: vec![bar],
        until: 2,
        amplitude: oag_hud::concave::AMPLITUDE,
    }));
    let bent = frame(&mut renderer);
    assert_eq!(
        bent.len(),
        6,
        "the bar as four columns, the icon, the reticle"
    );
    let size = Space::PSP.size;
    for column in &bent[..4] {
        assert_eq!(
            column.warp,
            oag_hud::concave::corners(column.rect, size, 35.0)
        );
    }
    assert!(
        bent[0].warp[0] < -20.0,
        "the bar's left end rises: {:?}",
        bent[0].warp
    );
    assert_eq!(bent[4].warp, oag_hud::concave::corners(icon, size, 35.0));
    assert!(bent[4].warp[0] > 1.0, "the top-left icon comes down");
    assert_eq!(bent[5].warp, [0.0; 4], "the reticle stays flat");
    let flat = frame(&mut renderer);
    assert_eq!(flat.len(), 3);
    assert!(flat.iter().all(|quad| quad.warp == [0.0; 4]), "spent");
}

/// A [`Draw::Stretched`] text keeps its height and its anchor and has every glyph quad scaled by
/// `x_scale` about it: HD's `ScaleX`/`ScaleY` on a `Text`.
///
/// **Skips with no adapter.**
#[test]
fn a_stretched_text_is_scaled_across_about_its_anchor() {
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
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stretched text test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());
    let text = Draw::Text {
        x: 100.0,
        y: 10.0,
        scale: 1.0,
        color: [1.0; 4],
        border: None,
        align: Align::Centre,
        text: "0000000".to_string(),
        wrap_width: None,
    };
    let mut quads_of = |draw: Draw| {
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render_with(
            wgpu::LoadOp::Load,
            &device,
            &queue,
            &mut encoder,
            &view,
            &[draw],
            (0.0, 0.0, 4.0, 4.0),
            None,
        );
        queue.submit(Some(encoder.finish()));
        renderer.quads.iter().map(|q| q.rect).collect::<Vec<_>>()
    };
    let plain = quads_of(text.clone());
    let stretched = quads_of(text.stretched(0.9));
    assert!(!plain.is_empty());
    assert_eq!(plain.len(), stretched.len(), "no glyph lost");
    for (a, b) in plain.iter().zip(&stretched) {
        assert!(
            (b[0] - (100.0 + (a[0] - 100.0) * 0.9)).abs() < 1e-3,
            "{a:?} {b:?}"
        );
        assert!((b[2] - a[2] * 0.9).abs() < 1e-3, "{a:?} {b:?}");
        assert_eq!(
            (a[1], a[3]),
            (b[1], b[3]),
            "the vertical axis is the text's own"
        );
    }
}
