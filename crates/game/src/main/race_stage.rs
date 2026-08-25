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
    ) -> race::SceneStats {
        let stats = self.scene.render(
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
        );

        // Over the scene and inside the same target, so the HUD is drawn at the
        // render scale the game is and lands in a `--screenshot` too.
        //
        // **The HUD goes when the race does.** Once the flag is out the speed
        // bar, the lap counter and the clock are all reporting a simulation
        // nobody is stepping any more, and the original hides the HUD at its own
        // race end too - see the `_BLOWUP` note in `race::tick`.
        match self.race.results() {
            Some(board) => {
                self.scoreboard
                    .draw(&gpu.device, &gpu.queue, encoder, view, board, viewport)
            }
            None => {
                if let Some(hud) = &mut self.hud {
                    let readout = self.race.readout();
                    hud.draw(&gpu.device, &gpu.queue, encoder, view, &readout, viewport);
                }
            }
        }
        stats
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
        );
        gpu.queue.submit(Some(encoder.finish()));
        if let Err(e) = gpu.device.poll(wgpu::PollType::wait_indefinitely()) {
            // Not fatal: the worst outcome is the compile this call exists to
            // avoid happening on the first real frame instead, exactly as it
            // did before this existed.
            warn!("could not wait for the race scene's warmup submit: {e}");
        }
    }
}
