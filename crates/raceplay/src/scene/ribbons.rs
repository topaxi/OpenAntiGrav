//! The scene's ribbon effects built from [`RibbonTextures`]: the HD-lineage
//! magstrip arc wake (`magstrip_wake`) and Wipeout HD's Rocket smoke, each
//! `None` where its textures did not load. Grouped so the scene builds,
//! uploads and draws them through one seat.

use oag_fx::beam::pipeline::Style;
use oag_fx::rocket_smoke::{CAPACITY, FINS, MAX_TRAILS, OpacityRamp};

use crate::Race;
use crate::rocket_smoke::RibbonTextures;

/// Vertices the smoke's buffer holds: every ribbon full, six per fin per
/// segment.
const SMOKE_VERTICES: usize = MAX_TRAILS * (CAPACITY - 1) * FINS * 6;

#[derive(Debug)]
pub(super) struct Ribbons {
    pub(super) magstrip: Option<super::magstrip_wake::Magstrip>,
    rocket_smoke: Option<RocketSmoke>,
}

#[derive(Debug)]
struct RocketSmoke {
    pipeline: std::cell::RefCell<oag_fx::beam::Pipeline>,
    ramp: OpacityRamp,
}

pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    textures: &RibbonTextures,
    sample_count: u32,
) -> Ribbons {
    let magstrip = super::magstrip_wake::build(
        device,
        queue,
        format,
        textures.magstrip.as_ref(),
        sample_count,
    );
    let rocket_smoke = textures.rocket_smoke.as_ref().map(|assets| RocketSmoke {
        pipeline: std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
            device,
            queue,
            format,
            &assets.texture,
            sample_count,
            super::mesh_render::Velocity::Write,
            Style::rocket_smoke(SMOKE_VERTICES),
        )),
        ramp: assets.ramp.clone(),
    });
    Ribbons {
        magstrip,
        rocket_smoke,
    }
}

impl super::Scene {
    /// Uploads this frame's ribbons. The smoke's facing fade needs the eye,
    /// read out of the view matrix's inverse.
    pub(super) fn upload_ribbons(&self, race: &Race, queue: &wgpu::Queue, vp: &[[f32; 4]; 4]) {
        self.upload_magstrip_wake(race, queue, vp);
        if let Some(smoke) = &self.ribbons.rocket_smoke {
            let eye = race.view().inverse().w_axis.truncate();
            let vertices = race.rocket_smoke_vertices(&smoke.ramp);
            smoke
                .pipeline
                .borrow_mut()
                .upload_with_eye(queue, vp, eye.to_array(), &vertices);
        }
    }

    /// Draws the ribbons into the pass `draw_effects` owns: the wake, then
    /// the smoke over it.
    pub(super) fn draw_ribbons(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.draw_magstrip_wake(pass);
        if let Some(smoke) = &self.ribbons.rocket_smoke {
            smoke.pipeline.borrow().draw(pass);
        }
    }
}
