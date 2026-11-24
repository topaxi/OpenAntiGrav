//! Photo mode's picture controls over the finished frame: exposure, saturation and depth of
//! field ([`oag_post::photo_fx`]). Built at the first frame that asks for it, so a race that
//! never opens the mode builds and binds nothing.

use super::*;

/// The pass, in the caller's frame format.
#[derive(Debug)]
pub(super) struct PhotoFxPass {
    pass: std::cell::RefCell<Option<oag_post::photo_fx::PhotoFx>>,
    format: wgpu::TextureFormat,
}

impl PhotoFxPass {
    pub(super) fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            pass: std::cell::RefCell::new(None),
            format,
        }
    }
}

impl Scene {
    /// Whether the picture controls' pass is built, for the test that proves it is freed when
    /// the mode closes.
    #[cfg(test)]
    pub(crate) fn photo_fx_built(&self) -> bool {
        self.photo_fx.pass.borrow().is_some()
    }

    /// Applies the mode's controls to `view`, last: after the glow of HD, 2048 and Omega and the
    /// motion blur, before the HUD. Pulse's, the PS2's and Pure's glow is added over or under the
    /// HUD in the original's order, but while a control is on `finish_view` adds it just ahead of
    /// this pass, so the controls cover it too. Returns whether anything was drawn.
    pub(super) fn run_photo_fx(
        &self,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        race: &Race,
    ) -> bool {
        let mut slot = self.photo_fx.pass.borrow_mut();
        // The pass holds three pipelines and a target of the render size: it goes with the mode,
        // and the next visit builds it again.
        if race.photo().is_none() {
            *slot = None;
            return false;
        }
        let Some(params) = race.photo_fx(crate::camera::NEAR_PLANE, self.far) else {
            return false;
        };
        if slot.is_none() {
            match oag_post::photo_fx::PhotoFx::new(device, self.photo_fx.format) {
                Ok(pass) => *slot = Some(pass),
                Err(e) => {
                    warn!("photo mode's picture controls are unavailable ({e})");
                    return false;
                }
            }
        }
        let Some(pass) = slot.as_mut() else {
            return false;
        };
        let size = self.depth.size();
        pass.render(
            device,
            queue,
            encoder,
            &oag_post::photo_fx::Frame {
                scene: view,
                depth: &self.attachment_views.depth,
                sample_count: self.msaa.samples(),
                size: (size.width, size.height),
                viewport,
                params,
            },
        )
    }
}
