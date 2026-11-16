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
    /// Applies the mode's controls to `view`, last: after the glow of HD, 2048 and Omega and the
    /// motion blur, before the HUD. (Pulse's and Pure's glow is added over the HUD afterwards,
    /// as in the original, so it is not graded or blurred.) Returns whether anything was drawn.
    pub(super) fn run_photo_fx(
        &self,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        race: &Race,
    ) -> bool {
        let Some(params) = race.photo_fx(crate::camera::NEAR_PLANE, self.far) else {
            return false;
        };
        let mut slot = self.photo_fx.pass.borrow_mut();
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
