//! The invariant [`Countdown::draw`]'s own doc states: `target_size` has to
//! be `view`'s real pixel dimensions, or its own depth attachment disagrees
//! with the colour one it is paired with.
//!
//! **Why this lives here rather than exercising `RaceStage::warm_up`
//! directly.** The crash this guards
//! (`crates/game/src/main/race_stage.rs`'s `draw_hud` and `warm_up`, fixed
//! alongside this file) needs a loaded circuit to reach through `RaceStage` -
//! a real `race::Scene` and `race::Race` - which is what a ground-truth test
//! is for, not a unit test. The validation error itself has nothing to do
//! with a track, though: it is `Countdown::depth_view` building a depth
//! attachment from whatever `target_size` it is handed, independent of
//! `view`. Reproducing it at that level needs a device and one non-empty,
//! textureless triangle - nothing off a disc - and catches the same wgpu
//! validation the original crash did.
//!
//! Its own file rather than an inline `#[cfg(test)] mod`, so the fixture
//! helpers below have room without the 200-line ratchet
//! (`scripts/check-file-size.py`) forcing them out anyway.

use super::*;
use oag_mesh::mesh::{Bounds, DEFAULT_SPECULAR_EXPONENT, DrawCall, GpuVertex, slots};

/// A device, or `None` so the caller skips - the same shape
/// `upscale::extent_tests` uses for a machine with no adapter.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

/// One flat, untextured triangle - enough for [`Countdown::new`]'s pipeline
/// build and [`Countdown::draw`]'s depth pass to actually run a draw call,
/// which an all-empty `Model` (`Countdown::draw` returns early on one)
/// would not reach.
fn one_triangle_model(label: &str) -> Model {
    let mut model = Model::none(label);
    let vertex = |position: [f32; 3]| GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        lightmap_texcoord: [0.0, 0.0],
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    };
    model.vertices = vec![
        vertex([-1.0, -1.0, 0.0]),
        vertex([1.0, -1.0, 0.0]),
        vertex([0.0, 1.0, 0.0]),
    ];
    model.indices = vec![0, 1, 2];
    model.draws = vec![DrawCall {
        range: 0..3,
        // Untextured: `Built::texture_binds` always carries the white
        // placeholder at slot 0, which is what `None` binds to - see
        // `oag_mesh::mesh_render::build`.
        texture: None,
        bounds: Bounds {
            centre: [0.0, 0.0, 0.0],
            radius: 2.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        node: None,
        chunk: None,
        alpha_test_ref: None,
    }];
    model
}

/// A widget in the orthographic `<Mode3D>` dialect, at the origin - the one
/// case [`Countdown::screen_position`] resolves without a recovered
/// perspective camera. What the widget draws is irrelevant to this file; only
/// that [`Countdown::new`] accepts it.
fn origin_widget() -> oag_hud::Model {
    oag_hud::Model {
        name: "test".to_string(),
        src: "test.vex".to_string(),
        position: [0.0, 0.0, -70.0],
        colour: None,
        orthographic: true,
        origin: [0.0, 0.0],
    }
}

fn colour_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("countdown test target"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Draws once and reports whichever wgpu validation error it raised, if any -
/// via `push_error_scope`/`pop_error_scope` rather than the default
/// uncaptured-error handler, which panics the whole process (`wgpu_core.rs`'s
/// own "Handling wgpu errors as fatal by default" - the exact panic the
/// reported crash carried), and this file wants a `Result` it can assert on
/// instead.
fn draw_and_catch_validation(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    countdown: &mut Countdown,
    view: &wgpu::TextureView,
    viewport: (f32, f32, f32, f32),
    target_size: (u32, u32),
) -> Option<wgpu::Error> {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    countdown.draw(
        device,
        queue,
        &mut encoder,
        view,
        0.0,
        viewport,
        target_size,
    );
    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    pollster::block_on(scope.pop())
}

/// `RaceStage::warm_up` used to pass `(gpu.config.width, gpu.config.height)` -
/// the raw window size - as `target_size` even where `view` was really the
/// scene's own, letterboxed-smaller texture: the presentation target tracks
/// the window, but the scene target this pass drew into during warmup does
/// not. 1920x1200 (16:10) at the default PSP aspect (480:272, i.e. 30:17)
/// letterboxes to exactly 1920x1088 - `1920 * 17 / 30 = 1088` with no
/// rounding - which is the reported crash verbatim: "the depth attachment's
/// texture view has extent (1920, 1200, 1) but is followed by the color
/// attachment at index 0's texture view which has (1920, 1088, 1)".
///
/// Both halves are checked on the same (correctly small) `view`, so the only
/// variable is `target_size`: matching it is silent, and the window's raw
/// size in its place reproduces the validation error the crash report
/// carried, not merely *a* validation error.
#[test]
fn the_depth_attachment_matches_the_view_actually_drawn_into_not_the_window() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut countdown = Countdown::new(
        &device,
        &queue,
        format,
        one_triangle_model("countdown regression"),
        &origin_widget(),
    )
    .expect("a minimal countdown model builds");

    // The scene's own letterboxed target - what `Framebuffer::view` actually
    // is during warmup, per `RaceStage::warm_up`'s own doc.
    let view = colour_target(&device, format, (1920, 1088));
    let viewport = (0.0, 0.0, 1920.0, 1088.0);

    let matched = draw_and_catch_validation(
        &device,
        &queue,
        &mut countdown,
        &view,
        viewport,
        (1920, 1088),
    );
    assert!(
        matched.is_none(),
        "target_size matching view's real size must not validate: {matched:?}"
    );

    let mismatched = draw_and_catch_validation(
        &device,
        &queue,
        &mut countdown,
        &view,
        viewport,
        (1920, 1200),
    );
    assert!(
        mismatched.is_some(),
        "a depth built off the window's raw (1920, 1200) while `view` is the \
         scene's smaller (1920, 1088) should still validate - if this starts \
         failing, wgpu stopped catching the mismatch this test exists to \
         guard against, not that the underlying bug came back"
    );
}
