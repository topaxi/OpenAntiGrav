//! What the screen filter is asserted to do in [`super::composite`]: run
//! between the presentation target and the grade, see last frame's output,
//! honour the strength row, and fall back to the untouched frame when a
//! preset will not build.
//!
//! Its own file rather than more of `tests.rs`, which is already past the
//! point `just check-size` would let it grow to.

use super::*;

use oag_display::display::FilterStrength;
use oag_post::screen::Preset;

/// One 4x4 frame through `composite`, filtered by `preset` at `strength`,
/// with the presentation target cleared to `color` first. Returns the
/// surface's centre pixel, or `None` with no adapter.
///
/// `frames` is how many times the same colour is composited: a filter that
/// reads `previous` needs a history, and the second frame is where it has
/// one. The framebuffer is built per call, so every call starts with a black
/// history.
fn filtered(
    preset: Option<&Preset>,
    strength: FilterStrength,
    colours: &[wgpu::Color],
) -> Option<([u8; 4], bool)> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("screen filter test"),
        ..Default::default()
    }))
    .ok()?;

    let mut framebuffer = Framebuffer::new(&device, format, (4, 4)).expect("the pipeline");
    framebuffer.resize_output(&device, (4, 4));
    framebuffer.set_screen_filter(&device, preset);
    let active = framebuffer.screen_filter_active();

    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screen filter readback"),
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
    let surface_view = surface.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screen filter readback"),
        size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let frame = Composite {
        brightness: Brightness::NEUTRAL,
        gamma: Gamma::NEUTRAL,
        screen: ScreenFrame {
            native: (480.0, 272.0),
            strength,
        },
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    for colour in colours {
        // Straight into the presentation target, the way a menu frame lands
        // there: no scene, no resolve.
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("presentation"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: framebuffer.output(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(*colour),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        framebuffer.composite(&device, &queue, &mut encoder, &surface_view, frame);
    }
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
    let mapped = slice.get_mapped_range().expect("the readback");
    let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize * 2 + 2 * 4;
    let mut pixel = [0u8; 4];
    pixel.copy_from_slice(&mapped[at..at + 4]);
    drop(mapped);
    readback.unmap();
    Some((pixel, active))
}

fn preset(id: &str, body: &str) -> Preset {
    Preset::parse(id, body).expect("a preset")
}

const RED: wgpu::Color = wgpu::Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
const GREEN: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 1.0,
    b: 0.0,
    a: 1.0,
};

fn close(actual: [u8; 4], expected: [u8; 3]) {
    for (channel, (a, e)) in actual[..3].iter().zip(expected).enumerate() {
        assert!(
            a.abs_diff(e) <= 2,
            "channel {channel}: {actual:?} against {expected:?}"
        );
    }
}

#[test]
fn a_passthrough_preset_composites_the_frame_it_was_given() {
    let pass = preset(
        "pass",
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }",
    );
    let Some((through, active)) = filtered(Some(&pass), FilterStrength::FULL, &[RED]) else {
        return;
    };
    assert!(active);
    close(through, [255, 0, 0]);
    let Some((plain, active)) = filtered(None, FilterStrength::FULL, &[RED]) else {
        return;
    };
    assert!(!active);
    assert_eq!(through, plain);
}

#[test]
fn a_preset_sees_its_own_previous_output() {
    // Half this frame, half what it drew last time. The first frame's
    // history is black, so red comes out at half; the second frame is green
    // over that, so a quarter red and half green.
    let smear = preset(
        "smear",
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return mix(frame_at(uv), previous_at(uv), 0.5); }",
    );
    let Some((first, _)) = filtered(Some(&smear), FilterStrength::FULL, &[RED]) else {
        return;
    };
    close(first, [128, 0, 0]);
    let Some((second, _)) = filtered(Some(&smear), FilterStrength::FULL, &[RED, GREEN]) else {
        return;
    };
    close(second, [64, 128, 0]);
}

#[test]
fn the_strength_row_mixes_the_filter_with_the_untouched_frame() {
    let invert = preset(
        "invert",
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return vec3<f32>(1.0) - frame_at(uv); }",
    );
    let Some((full, _)) = filtered(Some(&invert), FilterStrength::FULL, &[RED]) else {
        return;
    };
    close(full, [0, 255, 255]);
    let Some((half, _)) = filtered(Some(&invert), "50".parse().expect("a strength"), &[RED]) else {
        return;
    };
    close(half, [128, 128, 128]);
}

#[test]
fn a_preset_that_declares_a_fixed_strength_is_never_mixed() {
    let blue = "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return vec3<f32>(0.0, 0.0, 1.0); }";
    let half: FilterStrength = "50".parse().expect("a strength");
    let fixed = preset("fixed", &format!("//! fixed_strength = true\n{blue}"));
    let Some((pixel, active)) = filtered(Some(&fixed), half, &[RED]) else {
        return;
    };
    assert!(active);
    close(pixel, [0, 0, 255]);
    let Some((pixel, _)) = filtered(Some(&preset("mixed", blue)), half, &[RED]) else {
        return;
    };
    close(pixel, [128, 0, 128]);
}

/// The two shipped barrel filters draw the same at a stored 50 as at 100;
/// `psp-lcd` still draws halfway between the untouched frame and itself.
#[test]
fn a_stored_fifty_mixes_psp_lcd_and_leaves_the_barrel_built_ins_alone() {
    let half: FilterStrength = "50".parse().expect("a strength");
    let white = wgpu::Color::WHITE;
    let shipped = |id: &str, source: &str, strength: FilterStrength| {
        filtered(Some(&preset(id, source)), strength, &[white]).map(|(pixel, _)| pixel)
    };
    for (id, source) in [
        (
            "crt-interlaced",
            include_str!("../../../../assets/shaders/screen/crt-interlaced.wgsl"),
        ),
        (
            "crt-shadow-mask",
            include_str!("../../../../assets/shaders/screen/crt-shadow-mask.wgsl"),
        ),
    ] {
        let Some(full) = shipped(id, source, FilterStrength::FULL) else {
            return;
        };
        assert!(
            full[..3].iter().any(|&c| c < 245),
            "{id} must change the picture for this to prove anything: {full:?}"
        );
        assert_eq!(shipped(id, source, half), Some(full), "{id}");
    }

    let lcd = include_str!("../../../../assets/shaders/screen/psp-lcd.wgsl");
    let Some(full) = shipped("psp-lcd", lcd, FilterStrength::FULL) else {
        return;
    };
    let mixed = full.map(|c| u8::try_from((u16::from(c) + 255).div_ceil(2)).expect("a channel"));
    assert!(
        full[..3].iter().any(|&c| c < 245),
        "psp-lcd must change the picture for this to prove anything: {full:?}"
    );
    close(
        shipped("psp-lcd", lcd, half).expect("the same adapter"),
        [mixed[0], mixed[1], mixed[2]],
    );
}

#[test]
fn a_preset_that_will_not_build_draws_the_frame_unfiltered() {
    let broken = preset(
        "broken",
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv) +; }",
    );
    let Some((pixel, active)) = filtered(Some(&broken), FilterStrength::FULL, &[RED]) else {
        return;
    };
    assert!(!active, "a broken preset must not count as filtering");
    close(pixel, [255, 0, 0]);
}

#[test]
fn a_preset_reads_its_tunables_through_the_generated_accessors() {
    let scaled = preset(
        "scaled",
        "//! [[param]]\n//! name = \"amount\"\n//! default = 0.5\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv) * param_amount(); }",
    );
    let Some((pixel, active)) = filtered(Some(&scaled), FilterStrength::FULL, &[RED]) else {
        return;
    };
    assert!(active);
    close(pixel, [128, 0, 0]);
}
