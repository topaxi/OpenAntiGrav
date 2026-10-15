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

    /// The blocks of the view following the craft `race` is drawing, one kind each.
    pub(super) fn write_scene_blocks(
        &self,
        queue: &wgpu::Queue,
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
