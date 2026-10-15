//! Which drawables read which of the race's scene blocks ([`SceneBlocks`]), and the frame's
//! writes of them.
//!
//! A drawable is moved onto a kind once, at the end of [`Scene::new`], and the frame writes the
//! kind's block with the value it used to write into each drawable of the kind (see
//! `Scene::write_frame`): one `queue.write_buffer` instead of one per drawable. **A kind holds
//! only drawables the frame wrote on every view**: the shield shells, the weapon models and the
//! sky are never written, or written only while live, and keep a buffer of their own (holding
//! [`mesh_render::Scene::off`], or what the live weapon last had).

use super::*;

/// Moves each of `drawables` onto the shared uniform buffer.
fn share_all<'a>(
    device: &wgpu::Device,
    blocks: &mut UniformBlocks,
    drawables: impl IntoIterator<Item = &'a mut Drawable>,
) {
    for drawable in drawables {
        drawable.share_uniforms(device, blocks);
    }
}

impl Scene {
    /// Points every drawable the frame writes a scene block into at the shared buffer, by kind.
    pub(super) fn share_scene_blocks(&mut self, device: &wgpu::Device) {
        let blocks = &mut self.scene_blocks;
        for drawable in [
            Some(&mut self.track),
            self.collision.as_mut(),
            self.pads.as_mut(),
            self.weapon_pads.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.share_scene(device, blocks, SceneKind::Circuit);
        }
        if let Some(gantry) = &mut self.gantry {
            gantry.share_scene(device, blocks);
        }
        if let Some(target) = &mut self.behind_glass {
            target.share_scene(device, blocks);
        }
        for drawable in &mut self.ships {
            drawable.share_scene(device, blocks, SceneKind::Ship);
        }
        self.wrecks.share_scene(device, blocks);
        for drawable in self.shine.iter_mut().flatten() {
            drawable.share_scene(device, blocks, SceneKind::Shine);
        }
        if let Some(shine) = &mut self.track_shine {
            shine.share_scene(device, blocks);
        }
        for drawable in self.flares.iter_mut().chain(&mut self.boost).flatten() {
            drawable.share_scene(device, blocks, SceneKind::Flame);
        }
    }

    /// Moves every drawable the frame writes matrices for onto the shared uniform buffer
    /// ([`UniformBlocks`]), the ones written on every view first so a view's writes are runs of
    /// neighbouring slots. One a pool or a pass nobody walks here keeps its own buffer.
    pub(super) fn share_uniform_blocks(&mut self, device: &wgpu::Device) {
        let blocks = &mut self.uniform_blocks;
        share_all(device, blocks, &mut self.ships);
        share_all(device, blocks, self.shine.iter_mut().flatten());
        share_all(
            device,
            blocks,
            [
                Some(&mut self.track),
                self.sky.as_mut(),
                self.collision.as_mut(),
                self.pads.as_mut(),
                self.weapon_pads.as_mut(),
            ]
            .into_iter()
            .flatten(),
        );
        share_all(
            device,
            blocks,
            self.flares.iter_mut().chain(&mut self.boost).flatten(),
        );
        share_all(
            device,
            blocks,
            self.shield
                .iter_mut()
                .chain(&mut self.absorb_shell)
                .chain(self.absorb_overlay.iter_mut().flatten())
                .flatten()
                .chain(self.shield_cockpit.as_mut()),
        );
        self.wrecks.share_uniforms(device, blocks);
        if let Some(shine) = &mut self.track_shine {
            shine.share_uniforms(device, blocks);
        }
        if let Some(gantry) = &mut self.gantry {
            gantry.share_uniforms(device, blocks);
        }
        if let Some(target) = &mut self.behind_glass {
            target.share_uniforms(device, blocks);
        }
        // The weapon pools last: a slot is written only while its projectile is live, and a
        // pool past the slots keeps its own buffers.
        let [mag_floor_near, mag_floor_far] = &mut self.bomb_blast.mag_floor;
        let hd = &mut self.bomb_blast.hd;
        for pool in [
            &mut self.rockets,
            &mut self.mines,
            &mut self.bombs,
            &mut self.cannon_rounds,
            &mut self.plasma_blast.halo,
            &mut self.plasma_blast.hemisphere2,
            &mut self.plasma_blast.hemisphere1,
            &mut self.plasma_blast.ball,
            &mut self.plasma_blast.shuriken,
            &mut self.bomb_blast.hemisphere,
            &mut self.bomb_blast.shockwave,
            &mut self.bomb_blast.repulser_field,
            mag_floor_near,
            mag_floor_far,
            &mut hd.fireball,
            &mut hd.core,
            &mut hd.bloom,
            &mut hd.rings,
            &mut hd.halo,
            &mut hd.missile,
        ] {
            share_all(device, blocks, pool);
        }
        share_all(device, blocks, self.leach_ball.as_mut());
    }

    /// The blocks of the view following the craft `race` is drawing, one kind each.
    pub(super) fn write_scene_blocks(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        block: ViewBlock,
        circuit: &mesh_render::Scene,
        ship: &mesh_render::Scene,
        flame: &mesh_render::Scene,
    ) {
        let blocks = &self.scene_blocks;
        blocks.write(queue, block, SceneKind::Circuit, circuit);
        blocks.write(queue, block, SceneKind::Ship, ship);
        blocks.write(queue, block, SceneKind::Shine, &shine::shine_scene(ship));
        blocks.write(queue, block, SceneKind::Flame, flame);
    }
}

#[cfg(test)]
impl Scene {
    /// Craft `ship`'s matrices as the view following craft `view` wrote them, read back.
    pub(crate) fn ship_matrices(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: usize,
        ship: usize,
    ) -> Option<crate::drawable::Matrices> {
        let slot = self.ships[ship].uniform_slot()?;
        Some(self.uniform_blocks.read(device, queue, view, slot))
    }

    /// `kind`'s block of the view following craft `slot`, read back off the GPU.
    pub(crate) fn scene_block(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        kind: SceneKind,
    ) -> mesh_render::Scene {
        self.scene_blocks.read(device, queue, slot, kind)
    }
}
