//! `--loading-screen`: one frame of the loading screen, and the wave pass the
//! window shares with it.
//!
//! Its own file rather than the middle of `capture.rs`, which is at the
//! 1,000-line ratchet (`just check-size`). Nothing here changed in the move;
//! `capture::draw_wave` and `capture::LoadingOptions` are re-exported so
//! `main::loading_stage` and `main::headless` name what they always did.

use anyhow::{Context, Result};

use super::{offscreen, read_back, write_png};
use crate::render::Renderer;

/// What `--loading-screen` draws.
#[derive(Debug, Clone)]
pub struct LoadingOptions {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Image size.
    pub size: (u32, u32),
    /// How many frames the screen has been up.
    ///
    /// Both the wave's phase and the tip on show are functions of this, so it
    /// is how a capture reaches a particular beat of the heartbeat: the
    /// envelope peaks at 5 and 11 of its 24 frames, and phase 0 idles at the
    /// amplitude floor.
    pub ticks: u32,
    /// The conversion state to draw.
    ///
    /// **Stated rather than observed, and that is the flag's whole reason for
    /// existing.** A warm cache reaches a real mid-conversion state for a
    /// fraction of a second and a cold one takes ten minutes to leave it, so
    /// neither is a way to look at the screen. What is drawn from here is the
    /// window's own [`crate::loading::Screen::draw_list`] with a known input,
    /// not a second layout.
    pub progress: crate::prefetch::Progress,
    /// Which of the screen's two waits to draw, and what the current load is
    /// doing - `--loading-step`. Stated for the same reason `progress` is.
    pub phase: crate::loading::Phase,
    /// A real race load to run on a worker while the ticks are paced at 60 Hz,
    /// `--loading-live`: the bar then shows the stage the load has reached by
    /// tick `ticks`, instead of a stated state. `None` draws a stated one.
    pub live: Option<oag_raceplay::Options>,
    /// Which adapter to draw with, from `[graphics] renderer`.
    pub renderer: oag_display::display::Renderer,
    /// The shape the game is drawn in, from `[display] aspect`.
    pub aspect: oag_display::display::Aspect,
}

/// Draws the loading screen once and writes it, without a window or a worker.
///
/// The same two passes the window makes, in the same order: the UI list clears
/// the frame and draws the text, and the wave goes over it additively with no
/// depth attachment. A capture that composited them differently would prove
/// nothing about what a player sees, which is the rule the rest of this module
/// follows.
///
/// # Errors
///
/// Propagates the adapter, the device and the file. Also fails when a screen
/// that **has** a wave produced no geometry for it:
/// [`oag_render::loading::Pipeline::draw`] draws nothing at all when nothing
/// was uploaded, which would otherwise write a perfectly plausible
/// text-on-black PNG with no error anywhere.
///
/// **A title that authors no wave is not that failure**, and telling the two
/// apart is why the check reads [`crate::loading::Screen::has_wave`] rather
/// than the vertex count alone. Wipeout HD's screen is a full-screen still and
/// a caption with no wave at all - see `docs/formats/hd-loading.md` - so an
/// empty upload there is the correct picture rather than a missing one, and
/// refusing it refused the very screen this capture exists to look at.
pub fn loading(
    assets: &crate::loading::Assets,
    font: oag_ui::font::Atlas,
    sprites: &oag_hud::sprite::Sheet,
    options: &LoadingOptions,
) -> Result<()> {
    // Seed 0: a capture has to be reproducible, and which feature it draws is
    // part of the picture. See `loading::Screen::new`. Language `None`: this
    // CLI capture has no `Settings` to read one from, so it takes the same
    // English fallback `oag_ui::strings::project_table` gives any other
    // caller with nothing to name.
    let mut screen = crate::loading::Screen::new(assets, font.line_height, 0, None);
    // Stepped rather than jumped to: the tip rotation counts frames, and the
    // wave draws from its own `Rng` on every one of them, so frame `n` is only
    // reachable by having drawn the `n - 1` before it.
    let mut quads = screen.quads();
    // A live capture is one real load, paced at the window's own 60 Hz so the
    // frame the capture stops on is a moment of that load. Detached when the
    // capture returns, as a window's own worker is.
    let mut worker = options
        .live
        .clone()
        .map(|race| oag_raceplay::LoadWorker::spawn(race, options.progress.current.clone(), None));
    for _ in 0..options.ticks {
        let finished = match &worker {
            Some(worker) => {
                std::thread::sleep(std::time::Duration::from_nanos(16_666_667));
                let progress: crate::prefetch::Progress = worker.progress().into();
                screen.set_load_stage(progress.load_stage);
                progress.finished
            }
            None => options.progress.finished,
        };
        screen.advance(finished);
        quads = screen.quads();
    }
    // Said aloud rather than dropped: a load that failed at once would
    // otherwise be a bar parked at its first target with nothing to explain it.
    if let Some(worker) = worker.as_mut().filter(|worker| worker.is_finished()) {
        match worker.join() {
            Some(Ok(_)) => log::info!("the live race load finished"),
            Some(Err(why)) => log::warn!("the live race load failed: {why:#}"),
            None => {}
        }
    }
    let vertices = oag_render::loading::vertices(&quads);
    anyhow::ensure!(
        !vertices.is_empty() || !screen.has_wave(),
        "the wave produced no geometry, so there would be nothing to draw"
    );

    let (width, height) = options.size;
    let instance = crate::adapter::instance();
    let adapter = crate::adapter::choose(&instance, None, &options.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_mesh::mesh_render::device_descriptor("oag-game loading screen", &adapter),
    ))
    .context("requesting the device")?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Kept before the renderer takes it: the layout measures its own wrapping
    // and eliding through the atlas that will draw it, and `Renderer` owns
    // rather than borrows one.
    let atlas = font.clone();
    // The feature illustration's own sheet where the title ships one, exactly
    // as `Stage::loading` and `Stage::race_loading` choose - `Draw::Sprite`
    // addresses whichever sheet the renderer was built with, so a capture given
    // the front end's would draw the loading screen's picture from the wrong
    // atlas and silently miss it.
    let sprites = assets.art.as_ref().map_or(sprites, |art| &art.sheet);
    let mut renderer = Renderer::new(&device, &queue, format, None, font, sprites)?;
    // Sample count 1, matching `upscale::Framebuffer`'s own target and this
    // capture's texture. The wave's pipeline bakes it in, so a mismatch here is
    // a validation error rather than a soft failure.
    let mut wave = oag_render::loading::Pipeline::new(&device, &queue, format, &assets.strip, 1);
    wave.upload(&queue, [1.0, 1.0, 1.0, screen.opacity()], &vertices);

    let target = offscreen(&device, format, width, height);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let viewport = oag_display::display::viewport((width, height), options.aspect);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("loading screen"),
    });
    renderer.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &screen.draw_list(options.phase, &options.progress, &atlas),
        viewport,
        None,
    );
    draw_wave(&mut encoder, &view, &wave, viewport);

    let pixels = read_back(&device, &queue, encoder, &target, width, height)?;
    write_png(&options.path, width, height, &pixels)
}

/// Adds the wave's pass to `encoder`, over whatever is already in `view`.
///
/// One place rather than two so the window and the capture cannot drift apart,
/// and three things it has to get right:
///
/// - **Load, not clear**: the wave sits over the text the UI pass drew.
/// - **No depth attachment.** `oag_render::loading::Pipeline` is built with
///   `depth_stencil: None` and a pass that attached one would not match it.
/// - **The same viewport the UI pass used.** The wave's vertices are normalised
///   `0..1` with no idea where the game's rectangle is, so without this it
///   spans the whole surface while the text sits letterboxed inside it. Invisible
///   at the PSP's own aspect, where the two rectangles are the same.
pub fn draw_wave(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    wave: &oag_render::loading::Pipeline,
    viewport: (f32, f32, f32, f32),
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("loading wave"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
    wave.draw(&mut pass);
}
