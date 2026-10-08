//! The HD-lineage magstrip arc wake's two draws: the arc bodies sampling the
//! 8-by-8 atlas, and the contact quads sampling the second texture. Both are
//! [`oag_fx::beam::Pipeline`]s in [`oag_fx::beam::pipeline::Style::magstrip`]'s
//! additive ONE/ONE state - `MagstripWake_Draw` pushes two draw items with the
//! one blend word. See `oag_fx::magstrip` for what is measured and what is not.

use oag_fx::beam::pipeline::Style;
use oag_fx::exhaust::FlareTexture;

use crate::Race;

/// The wake's pipelines.
#[derive(Debug)]
pub(super) struct Magstrip {
    atlas: std::cell::RefCell<oag_fx::beam::Pipeline>,
    contact: std::cell::RefCell<oag_fx::beam::Pipeline>,
}

/// `None` when the title builds no wake or a texture did not decode - the loader
/// report says which, and nothing is drawn in its place.
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    textures: Option<&[FlareTexture; 2]>,
    sample_count: u32,
) -> Option<Magstrip> {
    let [atlas, contact] = textures?;
    let make = |texture: &FlareTexture, capacity: usize| {
        std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
            device,
            queue,
            format,
            texture,
            sample_count,
            super::mesh_render::Velocity::Write,
            Style::magstrip(capacity),
        ))
    };
    let ships = oag_gameplay::MAX_SHIPS;
    Some(Magstrip {
        atlas: make(atlas, ships * oag_fx::magstrip::BODY_VERTICES),
        contact: make(contact, ships * oag_fx::magstrip::CONTACT_VERTICES),
    })
}

impl super::Scene {
    /// Uploads this frame's arcs; a title without the wake does nothing.
    pub(super) fn upload_magstrip_wake(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        vp: &[[f32; 4]; 4],
    ) {
        if let Some(magstrip) = &self.ribbons.magstrip {
            let (atlas, contact) = race.magstrip_wake_vertices();
            magstrip.atlas.borrow_mut().upload(queue, vp, &atlas);
            magstrip.contact.borrow_mut().upload(queue, vp, &contact);
        }
    }

    /// Draws the contact quads under the arc bodies, into the pass `draw_effects`
    /// owns: depth-tested, unwritten, after the solid scene.
    pub(super) fn draw_magstrip_wake(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(magstrip) = &self.ribbons.magstrip {
            magstrip.contact.borrow().draw(pass);
            magstrip.atlas.borrow().draw(pass);
        }
    }
}
