//! The race stage: a loaded track, and the scene that draws it.

use oag_game::{display, race};

use crate::gpu::Gpu;

/// A race, and everything only it needs.
pub(crate) struct RaceStage {
    pub(crate) scene: race::Scene,
    pub(crate) race: race::Race,
    /// The HUD, or `None` when the disc's layout could not be read.
    pub(crate) hud: Option<oag_game::hud::Overlay>,
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
        animated_textures: bool,
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
            animated_textures,
        );

        // Over the scene and inside the same target, so the HUD is drawn at the
        // render scale the game is and lands in a `--screenshot` too.
        if let Some(hud) = &mut self.hud {
            let readout = self.race.readout();
            hud.draw(&gpu.device, &gpu.queue, encoder, view, &readout, viewport);
        }
        stats
    }
}
