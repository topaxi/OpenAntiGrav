//! The race stage: a loaded track, and the scene that draws it.

use log::warn;
use oag_game::{display, race};

use crate::gpu::Gpu;

/// A race, and everything only it needs.
pub(crate) struct RaceStage {
    pub(crate) scene: race::Scene,
    pub(crate) race: race::Race,
    /// The HUD, or `None` when the disc's layout could not be read.
    pub(crate) hud: Option<oag_game::hud::Overlay>,
    /// The results table, drawn once the race has one.
    ///
    /// Not an `Option`, unlike the HUD: that one needs a layout off the disc and
    /// there may be none, where this needs only a font and
    /// `oag_game::font::Atlas::build` always answers with the built-in 5x7 set.
    pub(crate) scoreboard: oag_game::scoreboard::Overlay,
}

impl RaceStage {
    /// `&mut self` rather than `&self`, unlike the other stages: the HUD's
    /// renderers own a growable instance buffer, the same reason
    /// [`Renderer::overlay`] takes `&mut self`. The scene stays immutable behind
    /// its own `RefCell`.
    // The two culling flags are independent settings read from the same place,
    // and a struct to carry them past one call site would name the grouping
    // without clarifying it - the same call this file already makes for
    // `Scene::new`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        motion_blur: display::MotionBlur,
        zone_spectrum: &[f32],
    ) -> race::SceneStats {
        // The Zone stage grade, pointed at the zone the race has reached before
        // the frame is built - the same per-frame order `Zone_UpdateStage` runs
        // in on 2048, where it is called from the render update rather than the
        // simulation. See `race::Scene::sync_zone_grade`.
        self.scene.sync_zone_grade(&self.race);
        // The scene and nothing else. The HUD used to follow it here, into the
        // same target; it composites at presentation resolution now - see
        // [`RaceStage::draw_hud`].
        self.scene.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &self.race,
            viewport,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            motion_blur,
            zone_spectrum,
        )
    }

    /// Draws the HUD, or the results table once the race has one.
    ///
    /// **Separate from [`RaceStage::render`], and drawn into a different target
    /// than the scene.** It used to go over the scene inside the offscreen
    /// target, which meant it was rasterised at the render scale and then
    /// resampled - and, with `[graphics] upscaler` set, sharpened by FSR 1 as
    /// though a coverage-atlas glyph were scene content. It now composites into
    /// the presentation target after the resolve, so it is drawn in the pixels
    /// the player has whatever the render scale is, and no upscaler ever sees
    /// it. See
    /// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md);
    /// `viewport` is the aspect rectangle on the surface, not the offscreen
    /// extent.
    ///
    /// **The HUD goes when the race does.** Once the flag is out the speed bar,
    /// the lap counter and the clock are all reporting a simulation nobody is
    /// stepping any more, and the original hides the HUD at its own race end
    /// too - see the `_BLOWUP` note in `race::tick`.
    pub(crate) fn draw_hud(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        match self.race.results() {
            Some(board) => {
                self.scoreboard
                    .draw(&gpu.device, &gpu.queue, encoder, view, board, viewport)
            }
            None => {
                if let Some(hud) = &mut self.hud {
                    let mut readout = self.race.readout();
                    // The rung the grade is showing, which is what names the
                    // speed class - see `Scene::zone_stage`.
                    readout.zone_stage = self.scene.zone_stage().unwrap_or(0);
                    readout.zone_next_in = self
                        .scene
                        .zones_to_next_stage(u16::try_from(readout.zone).unwrap_or(u16::MAX));
                    hud.draw(&gpu.device, &gpu.queue, encoder, view, &readout, viewport);
                }
            }
        }
    }

    /// Forces the driver to actually compile every pipeline this scene
    /// reaches, before anyone is shown a frame that needs one.
    ///
    /// **Building a `wgpu::RenderPipeline` object is not the same as the
    /// backend having compiled it.** Several drivers defer that to the first
    /// real draw call that uses it, and building the scene eagerly - see
    /// `Session::advance_race_build` - does not touch that: it moves the CPU
    /// side of the build off the loading screen's fade, but the compile still
    /// waits for the first frame `Stage::Race` actually draws, wherever that
    /// lands. Measured at up to three seconds of a held, black frame - the
    /// loading screen's own last one, at zero opacity - between the hand-off
    /// and the first thing presented.
    ///
    /// So this draws the grid pose once, into `target`, and blocks until the
    /// GPU has actually finished it - `queue.submit` alone only queues the
    /// work, and returning before it lands would just move the stall back to
    /// whichever frame the driver gets around to it on. `target` is
    /// deliberately the caller's own framebuffer rather than a fresh scratch
    /// texture: it is the exact size and format the real first frame draws
    /// into, so the exact pipeline variants get warmed, and it is safe to
    /// overwrite because the loading screen clears the whole frame before it
    /// draws anything - see `loading::Screen::draw_list`'s own full-screen
    /// `Draw::Fill`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn warm_up(
        &mut self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
    ) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race warmup"),
            });
        self.render(
            gpu,
            &mut encoder,
            target,
            viewport,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            // Off: a warmup draws one frame of the grid pose, so there is no
            // previous camera and nothing the blur pipelines add to warm -
            // they are fullscreen passes with no per-scene variants.
            display::MotionBlur::Off,
            // Empty: this frame is discarded, and the Zone visualiser
            // pipeline variant it would warm is the same one every other
            // Zone draw uses regardless of what the lookup holds.
            &[],
        );
        // The HUD's own pipelines still need warming, and still into `target`
        // even though the real frame now draws them into the presentation
        // target instead. What a driver defers is a compile, and a pipeline is
        // keyed on its format - which is the same for both targets - not on
        // the size of the attachment it is first used against.
        self.draw_hud(gpu, &mut encoder, target, viewport);
        gpu.queue.submit(Some(encoder.finish()));
        if let Err(e) = gpu.device.poll(wgpu::PollType::wait_indefinitely()) {
            // Not fatal: the worst outcome is the compile this call exists to
            // avoid happening on the first real frame instead, exactly as it
            // did before this existed.
            warn!("could not wait for the race scene's warmup submit: {e}");
        }
    }
}
