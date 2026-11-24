//! An ordinary craft's debris drawn (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The
//! debris throw"): one drawable over every copy of every piece, re-posed each frame from the
//! flight in [`crate::ship_debris`] and drawn at the identity, as the Zone end's pieces are.

use super::per_slot::Build;
use super::*;
use crate::drawable::HullLayers;

/// The debris, `None` where the title loaded none.
#[derive(Debug)]
pub(super) struct ShipDebrisDraws {
    debris: Option<Drawable>,
}

impl ShipDebrisDraws {
    pub(super) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        for drawable in self.debris.iter_mut() {
            drawable.share_uniforms(device, blocks);
        }
    }

    pub(super) fn share_scene(&mut self, device: &wgpu::Device, blocks: &mut SceneBlocks) {
        for drawable in self.debris.iter_mut() {
            drawable.share_scene(device, blocks, SceneKind::Ship);
        }
    }

    pub(super) fn build(
        slots: &Build<'_>,
        depth: mesh_render::Depth,
        zone_art: &mesh_render::zone::StageArt,
    ) -> Result<Self> {
        let Some(models) = slots.liveries.first().and_then(|l| l.ship_debris.as_ref()) else {
            return Ok(Self { debris: None });
        };
        if models.debris.indices.is_empty() {
            return Ok(Self { debris: None });
        }
        let debris = Drawable::new(
            slots.device,
            slots.queue,
            models.debris.clone(),
            slots.format,
            slots.store,
            slots.anisotropy,
            slots.sample_count,
            depth,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
            zone_art,
            slots.shadow_maps,
            mesh_render::ShadowReceiver::Mapped,
            mesh_render::Velocity::Write,
        )?;
        Ok(Self {
            debris: Some(debris),
        })
    }
}

impl Scene {
    /// Writes the debris' posed vertices while any piece is in the air.
    pub(super) fn write_ship_debris(
        &self,
        queue: &Uploads<'_>,
        race: &Race,
        view_projection: Mat4,
        prev_vp: Mat4,
    ) {
        if let Some(debris) = &self.ship_debris.debris
            && race.view.ship_debris_thrown()
        {
            debris.write_hull(
                queue,
                view_projection,
                Mat4::IDENTITY,
                prev_vp,
                HullLayers::default(),
            );
            let mut posed = Vec::new();
            debris.recolour_with(queue, &mut posed, |_, out| {
                race.view.ship_debris_pose(out);
            });
        }
    }

    pub(super) fn draw_ship_debris<'a>(
        &'a self,
        race: &'a Race,
        list: &mut super::record::DrawList<'a>,
    ) {
        if let (Some(debris), true) = (&self.ship_debris.debris, race.view.ship_debris_thrown()) {
            list.mesh(debris);
        }
    }
}
