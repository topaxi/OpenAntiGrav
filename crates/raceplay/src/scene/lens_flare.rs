//! Pure's sun lens flare on the GPU side of [`super::Scene`]: the pipeline is built once
//! from the circuit's own atlas, each view places its rings in `prepare_view` and draws
//! them over its resolved scene in `finish_view`, before the post chain and the HUD. See
//! `oag_fx::lens_flare` for the law and what is chosen.

use super::*;

impl Scene {
    /// Builds the flare pipeline when the circuit has one (`setup`, from
    /// [`crate::scenery_fx::SceneryFx::lens_flare`]); a no-op on `None`.
    ///
    /// Not built on a linear scene target (HD's chain): the law is Pure's, and the pass
    /// writes a display target.
    pub fn attach_lens_flare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        setup: Option<&oag_fx::lens_flare::Setup>,
    ) {
        *self.lens_flare.borrow_mut() = setup.filter(|_| !self.draws_linear()).and_then(|setup| {
            oag_fx::lens_flare::pipeline::Pipeline::new(
                device,
                queue,
                format,
                self.store(),
                self.sample_count() > 1,
                &setup.atlas,
            )
        });
    }

    /// Places this view's rings from its camera, for [`Self::encode_lens_flare`].
    pub(super) fn place_lens_flare(
        &self,
        race: &Race,
        projection: Mat4,
        viewport: (f32, f32, f32, f32),
    ) {
        let placement = self
            .lens_flare
            .borrow()
            .as_ref()
            .and(race.scenery_fx().lens_flare())
            .and_then(|setup| {
                oag_fx::lens_flare::Placement::new(
                    setup.direction,
                    setup.colour,
                    race.view(),
                    projection,
                    [viewport.0, viewport.1, viewport.2, viewport.3],
                    oag_fx::lens_flare::Random::FLAGGED,
                )
            });
        self.history().lens_flare.set(placement);
    }

    /// Draws the flare [`Self::place_lens_flare`] left for this view over `view`, reading
    /// the scene's depth for the sun's occlusion.
    pub(super) fn encode_lens_flare(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let Some(placement) = self.history().lens_flare.take() else {
            return;
        };
        if let Some(pipeline) = self.lens_flare.borrow().as_ref() {
            pipeline.encode(
                device,
                encoder,
                view,
                &self.attachment_views.depth,
                &placement,
            );
        }
    }
}
