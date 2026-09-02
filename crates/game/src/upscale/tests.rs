//! What the upscale pass in [`super`] is asserted to do: when it runs at all,
//! what the grade does to a colour, and the target size a scale asks for.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `upscale.rs`: the tests are 434 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

const LIMIT: u32 = 8192;

/// Blits a flat colour with and without FSR 1 and returns both results.
///
/// Takes `format` rather than fixing it, and both callers below run it at
/// **both** `Rgba8UnormSrgb` and `Rgba8Unorm` - the former is the only
/// configuration where the perceptual view differs from the ordinary one and
/// so exercises the whole colour-space arrangement rather than an accidental
/// identity, but the latter is what `Framebuffer::new` is actually built with
/// everywhere in the game since
/// [ADR-0020](../../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
/// forced the window surface and every capture target non-sRGB. A version of
/// this test that only ever built the sRGB case would stay green forever
/// against a `decode` flag that fires on "a pass ran" instead of "the source
/// is sRGB" - which is exactly the bug this pins down. The readback surface
/// reuses the same `format` as the offscreen target, exactly as the game's
/// own surface and `Framebuffer` do: whatever encode-on-write that costs at
/// the sRGB `format` is paid identically by both iterations of the loop
/// below, so it cancels out of the comparison rather than needing to be
/// avoided.
///
/// Returns `None` with no adapter, so the caller skips.
fn both_paths(format: wgpu::TextureFormat, input: [f64; 3]) -> Option<([u8; 4], [u8; 4])> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("upscale fsr test"),
        ..Default::default()
    }))
    .ok()?;

    let mut fsr = oag_render::post::fsr1::Fsr1::new(&device, format).expect("the fsr pipelines");
    let mut out = Vec::new();
    for upscaling in [false, true] {
        // Four texels in, sixteen out. Small, and still a real upscale: at
        // one texel EASU's twelve taps would all be the same clamped edge.
        let mut framebuffer = Framebuffer::new(&device, format, (2, 2)).expect("the pipeline");

        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fsr readback"),
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
        let surface_view = surface.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fsr readback"),
            size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fsr input"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: framebuffer.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: input[0],
                        g: input[1],
                        b: input[2],
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

        let source = upscaling.then(|| {
            fsr.render(
                &device,
                &queue,
                &mut encoder,
                oag_render::post::fsr1::Frame {
                    source: framebuffer.perceptual(),
                    input: framebuffer.size(),
                    output: (4, 4),
                    sharpness: oag_render::post::fsr1::Sharpness::DEFAULT,
                },
            );
            framebuffer.source(&device, fsr.output().expect("an fsr output"))
        });
        // Mirrors `resolve`'s own gate exactly, rather than `source.is_some()`
        // alone: at `Rgba8Unorm` this is always `false`, which is the
        // configuration the bug lived in - `source.is_some()` alone would
        // have decoded an already-linear-in-name value here too.
        framebuffer.set_grade(
            &queue,
            Brightness::NEUTRAL,
            Gamma::NEUTRAL,
            source.is_some() && format.is_srgb(),
        );
        framebuffer.present(
            &mut encoder,
            &surface_view,
            (0.0, 0.0, 4.0, 4.0),
            source.as_ref(),
        );

        encoder.copy_texture_to_buffer(
            surface.as_image_copy(),
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
        // The middle of the picture, away from the edge clamping.
        let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize + 4;
        let mapped = slice.get_mapped_range().expect("the readback");
        out.push([mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]);
        drop(mapped);
        readback.unmap();
    }
    Some((out[0], out[1]))
}

/// FSR 1 is a magnifier, and running it as a minifier undoes supersampling.
#[test]
fn an_upscaler_only_runs_when_it_is_actually_upscaling() {
    // Half scale: the whole point.
    assert!(magnifies((720, 408), (1440, 816)));
    // Supersampling, where EASU would undersample its own input.
    assert!(!magnifies((2880, 1632), (1440, 816)));
    // Exactly one to one, where there is nothing to reconstruct.
    assert!(!magnifies((1440, 816), (1440, 816)));
    // Anisotropic cases still count: a scale is applied to both axes, but a
    // clamped target and an odd rectangle can leave one axis short, and one
    // short axis is still something to reconstruct.
    assert!(magnifies((1440, 408), (1440, 816)));
    assert!(magnifies((720, 816), (1440, 816)));
}

/// Turning the upscaler on must not move a flat colour, at either format
/// `Framebuffer` is actually built with.
///
/// This is the whole colour-space arrangement in one assertion, and it is
/// the cheapest way to catch the mistake most likely to be made here.
/// A flat picture is a fixed point of both EASU and RCAS, so the only thing
/// that can differ between the two paths is the transfer function.
///
/// At `Rgba8UnormSrgb`, the bilinear path lets the sampler decode an sRGB
/// view and the FSR path reads a non-sRGB view and decodes in the shader; if
/// those two disagree, whether the decode were `pow(c, 2.2)` standing in for
/// the real curve or the non-sRGB view were not actually reaching the shader,
/// a flat grey would come out at two different values and this fails.
///
/// At `Rgba8Unorm`, **neither** path holds sRGB-encoded values, so neither
/// should decode - and that is what `Framebuffer::new` is actually built
/// with everywhere in the game, per
/// [ADR-0020](../../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
/// This is the case that caught the real bug: `decode` used to fire on "a
/// post-process ran" rather than "the format is sRGB", so the FSR path
/// decoded a value that was never encoded and came out measurably darker than
/// the bilinear path even though nothing here is sRGB at all.
///
/// **Skips with no adapter**, so a green CI run is not evidence it ran.
#[test]
fn turning_the_upscaler_on_does_not_shift_a_flat_colour() {
    // Three levels, because the sRGB curve's two pieces meet in the darks
    // and an approximation goes wrong there first.
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        for level in [0.02, 0.25, 0.5] {
            let Some((bilinear, fsr)) = both_paths(format, [level, level, level]) else {
                eprintln!("no GPU adapter: skipping");
                return;
            };
            for channel in 0..3 {
                let drift = i32::from(bilinear[channel]).abs_diff(i32::from(fsr[channel]));
                assert!(
                    drift <= 2,
                    "at {format:?} level {level}, channel {channel}: bilinear gave \
                     {bilinear:?} and fsr gave {fsr:?}, a drift of {drift}/255 - the two \
                     paths disagree about the transfer function"
                );
            }
        }
    }
}

/// Runs one grade through the real pipeline and reads back the pixel.
///
/// `Rgba8Unorm` and not the surface's sRGB format, so the arithmetic is
/// exact rather than encoded on the way out: this is asserting what the
/// shader *did*, and a gamma encode on top of it would put every expected
/// value behind a second curve.
///
/// Returns `None` on a machine with no adapter, which is what CI's runners
/// are - so the caller skips rather than fails there.
fn graded(input: [f32; 4], brightness: Brightness, gamma: Gamma) -> Option<[u8; 4]> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("upscale grade test"),
        ..Default::default()
    }))
    .ok()?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut framebuffer = Framebuffer::new(&device, format, (1, 1)).expect("the pipeline");
    framebuffer.set_grade(&queue, brightness, gamma, false);

    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("grade readback"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
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
        label: Some("grade readback"),
        size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    // The offscreen target is filled by clearing it, which is the cheapest
    // way to put a known colour under the blit.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("grade input"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: framebuffer.view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(input[0]),
                    g: f64::from(input[1]),
                    b: f64::from(input[2]),
                    a: f64::from(input[3]),
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    framebuffer.present(&mut encoder, &surface_view, (0.0, 0.0, 1.0, 1.0), None);
    encoder.copy_texture_to_buffer(
        surface.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
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
    Some([mapped[0], mapped[1], mapped[2], mapped[3]])
}

/// The one thing no other check in this repository can see.
///
/// `--screenshot` deliberately bypasses this pass, so brightness and gamma
/// have no picture anywhere to be compared against, and at their defaults
/// both are 1.0 - a swapped pair or a misbound uniform is invisible in
/// every other test here and would ship looking fine. So this one renders
/// through the real pipeline and reads the pixel back.
#[test]
fn the_grade_moves_the_picture_in_the_direction_the_setting_names() {
    // A midtone: black and white are the two values gamma cannot move, so
    // either would pass a broken exponent.
    const INPUT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
    let neutral = Brightness::NEUTRAL;
    let plain = Gamma::NEUTRAL;

    let Some(untouched) = graded(INPUT, neutral, plain) else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    // 0.25 in, 0.25 out - the neutral grade is not a grade.
    assert!(
        untouched[0].abs_diff(64) <= 1,
        "neutral changed it: {untouched:?}"
    );
    assert_eq!(untouched[3], 255, "alpha must come through untouched");

    let up = graded(INPUT, neutral, "140".parse().expect("parse")).expect("an adapter");
    let down = graded(INPUT, neutral, "60".parse().expect("parse")).expect("an adapter");
    assert!(up[0] > untouched[0], "gamma 140 has to brighten: {up:?}");
    assert!(down[0] < untouched[0], "gamma 60 has to darken: {down:?}");

    // Brightness on its own, at the neutral gamma, is exactly the
    // multiply - which is also what proves the two are not swapped, since
    // a gamma of 1.5 applied to 0.25 is nothing like 1.5 times it.
    let brighter = graded(INPUT, "150".parse().expect("parse"), plain).expect("an adapter");
    assert!(
        brighter[0].abs_diff(96) <= 1,
        "150 % of 0.25 is 0.375: {brighter:?}"
    );
    let darker = graded(INPUT, "50".parse().expect("parse"), plain).expect("an adapter");
    assert!(darker[0].abs_diff(32) <= 1, "half of 0.25: {darker:?}");
}

/// The uniform has to be the size a uniform binding may be, and the
/// neutral grade has to be the one that changes nothing - `set_grade`
/// compares against it to decide whether to write at all, so a wrong
/// neutral would leave a fresh install grading its own picture.
#[test]
fn the_neutral_grade_changes_nothing_and_fills_a_uniform_binding() {
    assert_eq!(std::mem::size_of::<Grade>(), 16);
    let neutral = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
    assert_eq!(neutral.brightness, 1.0);
    assert_eq!(neutral.exponent, 1.0);
    assert_eq!(neutral.padding, 0.0);
    // And it is what `Default` gives, which is what an untouched settings
    // file loads as.
    assert_eq!(
        Grade::new(Brightness::default(), Gamma::default(), false),
        neutral
    );
}

/// The decode flag is what stops an upscaled frame being graded in the
/// wrong space, and it is a float in the uniform because WGSL has no
/// `bool` it can read from a buffer. Zero and one, not "anything truthy".
#[test]
fn the_decode_flag_is_zero_or_one_and_nothing_else() {
    let plain = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
    let decoded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, true);
    assert_eq!(plain.decode, 0.0);
    assert_eq!(decoded.decode, 1.0);
    // And it is part of what `set_grade` compares, so flipping the source
    // rewrites the uniform even when neither menu row moved.
    assert_ne!(plain, decoded);
}

#[test]
fn full_scale_is_the_rectangle_itself() {
    let size = target_size((0.0, 0.0, 1440.0, 816.0), Scale::FULL, LIMIT);
    assert_eq!(size, (1440, 816));
}

#[test]
fn a_half_scale_halves_both_axes() {
    let size = target_size(
        (0.0, 0.0, 1920.0, 1080.0),
        "50".parse().expect("parse"),
        LIMIT,
    );
    assert_eq!(size, (960, 540));
}

#[test]
fn a_scale_above_full_supersamples() {
    let size = target_size(
        (0.0, 0.0, 1920.0, 1080.0),
        "200".parse().expect("parse"),
        LIMIT,
    );
    assert_eq!(size, (3840, 2160));
}

/// Measured against the rectangle, so the pillarbox a 4:3 aspect leaves
/// costs no offscreen pixels: the same scale on the same window renders
/// fewer of them than at `free`, which is the point.
#[test]
fn the_scale_follows_the_rectangle_rather_than_the_window() {
    let free = target_size(
        crate::display::viewport((1920, 1080), crate::display::Aspect::Free),
        Scale::FULL,
        LIMIT,
    );
    let ps2 = target_size(
        crate::display::viewport((1920, 1080), crate::display::Aspect::Ps2),
        Scale::FULL,
        LIMIT,
    );
    assert_eq!(free.0, 1920);
    assert!(ps2.0 < free.0, "{ps2:?} against {free:?}");
    assert_eq!(ps2.1, free.1, "the pillarbox loses width, not height");
}

/// 200 % of a 4K window is past what some adapters allow, and drawing
/// slightly smaller beats refusing to draw.
#[test]
fn an_oversized_target_is_clamped_to_what_the_device_allows() {
    let size = target_size(
        (0.0, 0.0, 3840.0, 2160.0),
        "200".parse().expect("parse"),
        4096,
    );
    // 7680x4320 wanted, 4096 allowed on both axes.
    assert_eq!(size, (4096, 4096));
}

/// A minimised window times a small scale rounds to zero, and a zero-sized
/// texture is a validation error rather than a blank frame.
#[test]
fn a_degenerate_rectangle_still_gives_a_creatable_texture() {
    for rect in [(0.0, 0.0, 0.0, 0.0), (0.0, 0.0, 1.0, 1.0)] {
        for percent in ["25", "50", "100", "200"] {
            let size = target_size(rect, percent.parse().expect("parse"), LIMIT);
            assert!(
                size.0 >= 1 && size.1 >= 1,
                "{rect:?} at {percent}: {size:?}"
            );
        }
    }
}

/// The UI composites **after** the blit, onto the surface, at presentation
/// resolution - [ADR-0036](../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
///
/// Two properties in one picture, and both are what the arrangement is for:
///
/// 1. **The blit survives.** `Renderer::overlay` loads rather than clears, so
///    what the resolve put on the surface - including the aspect bars it
///    cleared around the rectangle - is still there under a UI pass that covers
///    only part of it. Get this wrong and the frame is a UI on black.
/// 2. **A feature one presentation pixel wide comes out one pixel wide.** The
///    offscreen target here is 2x2 against an 8x8 surface, so one of its texels
///    is a four-pixel block. Drawing the same fill *before* the resolve was
///    tried while this test was written, and it does not merely widen: a
///    quarter-pixel quad in a two-pixel viewport covers no sample centre at
///    all, so column zero came back the scene's own red and the UI was gone
///    entirely. Landing on exactly column zero is the assertion that the UI was
///    rasterised against the surface and not against the render scale.
///
/// A `Draw::Fill` rather than the performance overlay's own list, deliberately:
/// what is under test is where the pass lands, and pinning it to glyph layout
/// would make it fail for reasons that are not this.
///
/// Returns early on a machine with no adapter, which is what CI's runners are.
#[test]
fn the_ui_composites_over_the_blit_at_presentation_resolution() {
    use crate::frontend::{Draw, SCREEN};
    use crate::render::Renderer;

    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ui composite test"),
        ..Default::default()
    })) else {
        return;
    };

    // What the window surface and every capture target are since ADR-0020.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut framebuffer = Framebuffer::new(&device, format, (2, 2)).expect("the pipeline");
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        crate::font::Atlas::build(),
        &crate::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");

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

    let mut encoder = device.create_command_encoder(&Default::default());
    // The scene: a flat red over the whole offscreen target, which the blit
    // then stretches over the whole surface.
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
    framebuffer.set_grade(&queue, Brightness::NEUTRAL, Gamma::NEUTRAL, false);
    framebuffer.present(&mut encoder, &surface_view, (0.0, 0.0, 8.0, 8.0), None);

    // 480 authored units across 8 surface pixels is 60 units to the pixel, so
    // this fill is the leftmost column and nothing else.
    renderer.overlay(
        &device,
        &queue,
        &mut encoder,
        &surface_view,
        &[Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0 / 8.0, SCREEN.1],
            color: [0.0, 0.0, 1.0, 1.0],
        }],
        (0.0, 0.0, 8.0, 8.0),
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
    let pixel = |x: usize| {
        let at = row + x * 4;
        [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
    };

    assert_eq!(
        pixel(0),
        [0, 0, 255, 255],
        "the UI pass did not reach the surface"
    );
    for x in 1..8 {
        assert_eq!(
            pixel(x),
            [255, 0, 0, 255],
            "column {x} lost the blit under the UI, or the fill was widened by \
             being rasterised at the render scale rather than at the surface"
        );
    }
    drop(mapped);
    readback.unmap();
}
