//! Detonator's medal models: `DetonatorGoldMedal`, `DetonatorSilverMedal` and
//! `DetonatorBronzeMedal`, `<ImageModel>`s of `Data\FE\Trophies\hd_*.vex`.
//!
//! Each is a model drawn in a rectangle of the HUD (`ImageX/Y/Width/Height`), switched on by
//! the Detonator HUD controller as the score passes the campaign cell's targets
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`). They are drawn through the
//! front-end's model preview ([`crate::preview::Preview`]) rather than flattened into the HUD's
//! sprite sheet: a trophy is a lit 3-D model that turns on its own `Anim Transform` (a spin
//! about Y), which a baked quad cannot show.
//!
//! **Chosen, not measured:** the camera. The widget authors `ModelZ -87`, `RotX 0` and
//! `RotY -0.5`, but the projection they sit in is unread, so the model is framed by the
//! preview's orbit camera in the widget's rectangle, with `RotX` and `RotY` as its pitch and
//! yaw. The spin is the file's own.

use oag_hud::{DETONATOR_MEDAL_WIDGETS, ImageModel};
use oag_mesh::mesh::Model;
use oag_mesh::mesh_render::Anisotropy;
use oag_mesh::orbit::Orbit;
use oag_tables::race_campaign::Medal;

use crate::preview::Preview;

/// The three medal models of a Detonator race, built for the GPU.
pub struct Medals {
    shown: Vec<(Medal, ImageModel, Preview)>,
}

impl std::fmt::Debug for Medals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Medals")
            .field("models", &self.shown.len())
            .finish()
    }
}

impl Medals {
    /// The previews for `models`, each with the widget that places it. A model that will not
    /// build is left out: that medal then draws nothing.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        models: Vec<(ImageModel, Model)>,
    ) -> Self {
        let shown = models
            .into_iter()
            .filter_map(|(widget, model)| {
                let medal = DETONATOR_MEDAL_WIDGETS
                    .iter()
                    .find(|(_, name)| *name == widget.name)?
                    .0;
                match Preview::new(device, queue, format, Anisotropy::default(), model) {
                    Ok(preview) => Some((medal, widget, preview)),
                    Err(error) => {
                        log::warn!("{}: {error:#} - {} draws nothing", widget.src, widget.name);
                        None
                    }
                }
            })
            .collect();
        Self { shown }
    }

    /// Whether any medal model is built.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// The model of `medal`, in its widget's rectangle, over what `view` holds. `place` maps a
    /// rectangle of the HUD's grid to where the HUD itself puts it (the safe area). Nothing for
    /// `None`, and nothing for a medal whose model was not built.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        medal: Option<Medal>,
        seconds: f32,
        space: oag_display::space::Space,
        place: &dyn Fn([f32; 4]) -> [f32; 4],
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        let Some(medal) = medal else {
            return;
        };
        let Some((_, widget, preview)) = self.shown.iter_mut().find(|(m, _, _)| *m == medal) else {
            return;
        };
        let orbit = Orbit {
            yaw: widget.rotation[1],
            pitch: widget.rotation[0],
            ..Orbit::default()
        };
        preview.draw(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            place(widget.rect),
            orbit,
            seconds,
        );
    }
}
