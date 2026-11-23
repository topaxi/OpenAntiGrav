//! A Zone craft's end drawn: the death shell over the hull while it plays, then the debris
//! (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`, rows Z7 to Z11).
//!
//! The shell is one drawable at the craft's own matrix whose `Anim Transform` is scrubbed to the
//! shell's time. The debris is one drawable whose vertices are re-posed every frame from the
//! pieces' flight ([`crate::zone_end`]), drawn at the identity: 112 drawables would repeat the
//! pipelines 112 times for one model's worth of geometry.

use super::per_slot::Build;
use super::*;
use crate::drawable::HullLayers;

/// The shell and the debris, `None` where the title loaded none.
#[derive(Debug)]
pub(super) struct ZoneEndDraws {
    shell: Option<Drawable>,
    debris: Option<Drawable>,
}

impl ZoneEndDraws {
    pub(super) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        for drawable in self.shell.iter_mut().chain(&mut self.debris) {
            drawable.share_uniforms(device, blocks);
        }
    }

    pub(super) fn share_scene(&mut self, device: &wgpu::Device, blocks: &mut SceneBlocks) {
        for drawable in self.shell.iter_mut().chain(&mut self.debris) {
            drawable.share_scene(device, blocks, SceneKind::Ship);
        }
    }

    /// Builds the two drawables from the player's livery, the hull's own pipeline and depth.
    pub(super) fn build(
        slots: &Build<'_>,
        depth: mesh_render::Depth,
        zone_art: &mesh_render::zone::StageArt,
    ) -> Result<Self> {
        let Some(models) = slots.liveries.first().and_then(|l| l.zone_end.as_ref()) else {
            return Ok(Self {
                shell: None,
                debris: None,
            });
        };
        let make = |model: &oag_mesh::mesh::Model, blend| {
            if model.indices.is_empty() {
                return Ok(None);
            }
            Drawable::new(
                slots.device,
                slots.queue,
                model.clone(),
                slots.format,
                slots.store,
                slots.anisotropy,
                slots.sample_count,
                depth,
                blend,
                mesh_render::GlowMask::Protected,
                zone_art,
                slots.shadow_maps,
                mesh_render::ShadowReceiver::Mapped,
                mesh_render::Velocity::Write,
            )
            .map(Some)
        };
        Ok(Self {
            shell: make(&models.shell, mesh_render::TRANSPARENT_BLEND)?,
            debris: make(&models.debris, mesh_render::TRANSPARENT_BLEND)?,
        })
    }
}

impl Scene {
    /// Writes the shell's pose and animation time, and the debris' posed vertices.
    pub(super) fn write_zone_end(
        &self,
        queue: &Uploads<'_>,
        race: &Race,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
    ) {
        let slot = race.core.player_slot();
        if let (Some(shell), Some(time)) = (
            &self.zone_end.shell,
            race.view.zone_end_shell_time(&race.core),
        ) {
            shell.write_hull(
                queue,
                view_projection,
                race.view.ship_model_matrix_of(&race.core, slot),
                prev_vp * prev.ship(slot, race),
                HullLayers::default(),
            );
            // The shell's own clock: what its `Anim Transform` and any texture track author
            // (HD's is one static key, so there it moves nothing; 2048's carries a track).
            shell.write_node_anims(queue, time);
            shell.write_anims(queue, time);
        }
        if let Some(debris) = &self.zone_end.debris
            && race.view.zone_end_thrown()
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
                race.view.zone_end_debris(out);
            });
        }
    }

    /// Draws the shell while it plays and the debris once it is thrown, where the wrecks are.
    pub(super) fn draw_zone_end<'a>(
        &'a self,
        race: &'a Race,
        list: &mut super::record::DrawList<'a>,
    ) {
        if let (Some(shell), true) = (
            &self.zone_end.shell,
            race.view.zone_end_draws_shell(&race.core),
        ) {
            list.mesh(shell);
        }
        if let (Some(debris), true) = (&self.zone_end.debris, race.view.zone_end_thrown()) {
            list.mesh(debris);
        }
    }
}
