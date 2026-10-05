//! The window's own composite order, asserted against the real `Renderer`.
//!
//! These were in `oag-present`'s upscale tests until it was split out: they
//! draw UI through [`crate::render::Renderer`] and [`oag_hud::sprite::Sheet`],
//! which live here, so they stay with the host and reach the upscaler through
//! its public API alone.

use oag_display::display::{Brightness, Gamma, Reconstruction};
use oag_present::upscale::{Composite, Framebuffer, Presentation, ScreenFrame};

/// No filter: what every composite here is made with.
const UNFILTERED: ScreenFrame = ScreenFrame {
    native: (480.0, 272.0),
    strength: oag_display::display::FilterStrength::FULL,
};

/// Runs the window's own composite order and returns the middle row.
///
/// A 2x2 scene against an 8x8 surface, so one scene texel is a four-pixel
/// block and anything drawn at the scene's size is unable to produce a
/// single-pixel feature at all. Onto that go two one-pixel fills, one in each
/// of the places a piece of UI can now be drawn:
///
/// - **column 0, where the HUD goes** - into the presentation target, between
///   [`Framebuffer::resolve_scene`] and [`Framebuffer::composite`], which is
///   exactly where `Session::frame` puts `RaceStage::draw_hud`.
/// - **column 7, where the performance overlay goes** - onto the surface,
///   *after* `composite`, which is where `Session::frame` puts it.
///
/// Everything else in the row is the scene's own red. Returns `None` on a
/// machine with no adapter, which is what CI's runners are.
fn composited(brightness: Brightness, gamma: Gamma) -> Option<[[u8; 4]; 8]> {
    use crate::render::Renderer;
    use oag_display::space::SCREEN;
    use oag_ui::frontend::Draw;

    // What the window surface and every capture target are since ADR-0020.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ui composite test"),
        ..Default::default()
    }))
    .ok()?;

    let mut framebuffer = Framebuffer::new(&device, format, (2, 2)).expect("the pipeline");
    framebuffer.resize_output(&device, (8, 8));
    // **One renderer per pass, not one shared between them.** A `Renderer`
    // owns a quad buffer and a uniform buffer that it fills through the queue,
    // and a queue write lands before *any* of the submission's commands - so
    // two passes off one renderer both draw whatever the second one uploaded.
    // Written shared first and caught by this test's own assertion. The game
    // has two here for its own reasons - the HUD draws through
    // `crate::hud_overlay::Overlay` and the performance overlay through
    // `Session::overlay` - so this matches it rather than working around it.
    // `race/capture.rs` records the same trap for its primer frame.
    let mut ui = Renderer::new(
        &device,
        &queue,
        format,
        None,
        oag_ui::font::Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let mut instrument = Renderer::new(
        &device,
        &queue,
        format,
        None,
        oag_ui::font::Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the overlay pipeline");

    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("composite readback"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
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
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("composite readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 8) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // 480 authored units across 8 surface pixels is 60 units to the pixel, so
    // each of these fills is one column and nothing else.
    let column = SCREEN.0 / 8.0;
    let fill = |at: f32, color: [f32; 4]| Draw::Fill {
        rect: [at * column, 0.0, column, SCREEN.1],
        color,
    };
    let whole = (0.0, 0.0, 8.0, 8.0);

    let mut encoder = device.create_command_encoder(&Default::default());
    // The scene: a flat red over the whole offscreen target.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: framebuffer.view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let _ = framebuffer.resolve_scene(
        &device,
        &queue,
        &mut encoder,
        whole,
        &Presentation {
            reconstruction: Reconstruction::Off,
            sharpness: 0.0,
            // Carried but deliberately unread by this half - `resolve_scene`
            // grades neutrally and `composite` is handed the real values
            // below. Passing the real ones here is what a caller does, so the
            // test does too: if the pass ever started reading them again, the
            // scene would come out graded twice and the assertions would say so.
            brightness,
            gamma,
        },
        // No scene inputs: this test drives the bilinear path, which is the
        // rung `None` falls to.
        None,
        None,
    );
    ui.overlay(
        &device,
        &queue,
        &mut encoder,
        framebuffer.output(),
        &[fill(0.0, [0.0, 0.0, 1.0, 1.0])],
        whole,
    );
    framebuffer.composite(
        &device,
        &queue,
        &mut encoder,
        &surface_view,
        Composite {
            brightness,
            gamma,
            screen: UNFILTERED,
        },
    );
    instrument.overlay(
        &device,
        &queue,
        &mut encoder,
        &surface_view,
        &[fill(7.0, [0.0, 1.0, 0.0, 1.0])],
        whole,
    );

    encoder.copy_texture_to_buffer(
        surface.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(8),
            },
        },
        wgpu::Extent3d {
            width: 8,
            height: 8,
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
    // A middle row, away from any edge the blit's own sampler clamps at.
    let row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize * 4;
    let mut out = [[0u8; 4]; 8];
    for (x, pixel) in out.iter_mut().enumerate() {
        let at = row + x * 4;
        pixel.copy_from_slice(&mapped[at..at + 4]);
    }
    drop(mapped);
    readback.unmap();
    Some(out)
}

/// The UI composites **after** the resolve, at presentation resolution -
/// [ADR-0036](../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
///
/// Two properties in one picture, and both are what the arrangement is for:
///
/// 1. **The resolved scene survives.** `Renderer::overlay` loads rather than
///    clears, so what `resolve_scene` put in the presentation target - the
///    aspect bars its clear draws included - is still there under a UI pass
///    covering only part of it. Get this wrong and the frame is a UI on black.
/// 2. **A feature one presentation pixel wide comes out one pixel wide.** The
///    scene target here is 2x2 against an 8x8 surface, so one of its texels is
///    a four-pixel block. Drawing the same fill into the *scene* target was
///    tried while this test was written, and it does not merely widen: a
///    quarter-pixel quad in a two-pixel viewport covers no sample centre at
///    all, so the column came back the scene's own red and the UI was gone
///    entirely. Landing on exactly its own column is the assertion that the UI
///    was rasterised against the surface and not against the render scale.
///
/// `Draw::Fill`s rather than a real HUD layout or the overlay's own list,
/// deliberately: what is under test is where each pass lands, and pinning it to
/// glyph layout would make it fail for reasons that are not this.
#[test]
fn the_ui_composites_over_the_blit_at_presentation_resolution() {
    let Some(row) = composited(Brightness::NEUTRAL, Gamma::NEUTRAL) else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    assert_eq!(row[0], [0, 0, 255, 255], "the HUD's pass did not land");
    assert_eq!(row[7], [0, 255, 0, 255], "the overlay's pass did not land");
    for (x, pixel) in row.iter().enumerate().take(7).skip(1) {
        assert_eq!(
            *pixel,
            [255, 0, 0, 255],
            "column {x} lost the scene under the UI, or a fill was widened by \
             being rasterised at the render scale rather than at the surface"
        );
    }
}

/// The HUD is inside the grade and the performance overlay is outside it.
///
/// This is the assertion that pays for the presentation target. Drawing the UI
/// straight onto the surface after the blit - which is what the overlay does,
/// and what the HUD could have done for free - leaves it outside brightness and
/// gamma, and a menu or a HUD that does not respond to a calibration row a
/// player is standing on is the behaviour `crate::upscale`'s module doc
/// defends. So the scene resolves into a target *ungraded*, the HUD goes on
/// there, and the grade is applied on the way out over both.
///
/// The overlay staying outside it is not an oversight but
/// [ADR-0036](../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)'s
/// one standing exception: it is an instrument, and a frame-time graph a
/// brightness of 50 % makes hard to read is a worse instrument.
#[test]
fn the_hud_is_graded_with_the_scene_and_the_performance_overlay_is_not() {
    let half: Brightness = "50".parse().expect("parse");
    let Some(row) = composited(half, Gamma::NEUTRAL) else {
        eprintln!("no GPU adapter; skipping");
        return;
    };

    // Half of 255 is 127.5, and which side of it the hardware lands on is not
    // this test's business - only that the grade reached the pixel at all.
    assert!(
        row[0][2].abs_diff(128) <= 1,
        "the HUD's fill was not graded: {:?}",
        row[0]
    );
    assert!(
        row[3][0].abs_diff(128) <= 1,
        "the scene was not graded: {:?}",
        row[3]
    );
    // Graded once, not twice: 25 % would be `resolve_scene` grading as well.
    assert!(row[3][0] > 64, "the scene looks graded twice: {:?}", row[3]);
    assert_eq!(
        row[7],
        [0, 255, 0, 255],
        "the performance overlay must stay outside the grade"
    );
}

/// A stage with no scene draws straight into the presentation target, and its
/// own clear is what draws the aspect bars -
/// [ADR-0038](../../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md).
///
/// Three things at once, and the third is the one that would be missed:
///
/// 1. **`composite` does not need `resolve_scene` to have run.** The launcher,
///    the loading screen, the front end and the menus skip it entirely, so
///    whatever they drew has to reach the surface on its own.
/// 2. **`Renderer::render` clears, and that clear draws the bars.** `present`
///    used to do it, clearing the surface around the rectangle it blitted
///    into. A UI-only frame never reaches `present`, so the bars have to come
///    from the stage's own pass - which is only true because it clears the
///    whole attachment and then restricts itself to the viewport.
/// 3. **The bars land in the right pixels at a pillarboxed aspect.** A viewport
///    narrower than the target is the case the default `psp` aspect never
///    exercises, and it is where an off-by-a-rectangle would hide.
#[test]
fn a_ui_only_stage_reaches_the_surface_and_its_own_clear_draws_the_bars() {
    use crate::render::Renderer;
    use oag_display::space::SCREEN;
    use oag_ui::frontend::Draw;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ui only stage test"),
        ..Default::default()
    })) else {
        return;
    };

    // The scene target is built and never drawn into, exactly as it sits idle
    // while the front end is up.
    let mut framebuffer = Framebuffer::new(&device, format, (2, 2)).expect("the pipeline");
    framebuffer.resize_output(&device, (8, 8));
    let mut ui = Renderer::new(
        &device,
        &queue,
        format,
        None,
        oag_ui::font::Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");

    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ui only readback"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
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
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ui only readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 8) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    // Pillarboxed: four pixels of game with two of bar either side, which is
    // the shape `display::viewport` produces on a window wider than the aspect.
    // `render` rather than `overlay`, because a stage owns its frame and
    // clears - that clear is what is under test.
    ui.render(
        &device,
        &queue,
        &mut encoder,
        framebuffer.output(),
        &[Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: [0.0, 0.0, 1.0, 1.0],
        }],
        (2.0, 0.0, 4.0, 8.0),
        None,
    );
    // No `resolve_scene`. Straight to the surface.
    framebuffer.composite(
        &device,
        &queue,
        &mut encoder,
        &surface_view,
        Composite {
            brightness: Brightness::NEUTRAL,
            gamma: Gamma::NEUTRAL,
            screen: UNFILTERED,
        },
    );

    encoder.copy_texture_to_buffer(
        surface.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(8),
            },
        },
        wgpu::Extent3d {
            width: 8,
            height: 8,
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
    let row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize * 4;
    let pixel = |x: usize| {
        let at = row + x * 4;
        [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
    };

    for x in [0, 1, 6, 7] {
        assert_eq!(
            pixel(x),
            [0, 0, 0, 255],
            "column {x} is a bar and the stage's clear has to have drawn it"
        );
    }
    for x in [2, 3, 4, 5] {
        assert_eq!(
            pixel(x),
            [0, 0, 255, 255],
            "column {x} is inside the rectangle and never reached the surface"
        );
    }
    drop(mapped);
    readback.unmap();
}

/// Every preset this crate ships builds on the device: a built-in that fails
/// to compile would be a silently absent filter in the menu's own list.
#[test]
fn every_built_in_preset_builds_on_this_device() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let Ok((device, _queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("screen filter test"),
            ..Default::default()
        }))
    else {
        eprintln!("no GPU device; skipping");
        return;
    };
    for (id, source) in crate::screen::BUILT_IN {
        let preset = oag_post::screen::Preset::parse(id, source).expect("a preset");
        let mut framebuffer = Framebuffer::new(&device, wgpu::TextureFormat::Rgba8Unorm, (4, 4))
            .expect("the pipeline");
        framebuffer.resize_output(&device, (4, 4));
        framebuffer.set_screen_filter(&device, Some(&preset));
        assert!(framebuffer.screen_filter_active(), "{id} did not build");
    }
}
