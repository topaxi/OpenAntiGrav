//! Which drawables read which of the race's scene blocks ([`SceneBlocks`]), and the frame's
//! writes of them.
//!
//! A drawable is moved onto a kind once, at the end of [`Scene::new`], and the frame writes the
//! kind's block with the value it used to write into each drawable of the kind (see
//! `Scene::write_frame`): one `queue.write_buffer` instead of one per drawable. **A kind holds
//! only drawables the frame wrote on every view**: the shield shells, the weapon models and the
//! sky are never written, or written only while live, and keep a buffer of their own (holding
//! [`mesh_render::Scene::off`], or what the live weapon last had). The one exception is HD's
//! absorb shell, which is written only while live but reads the craft's block, because its
//! program needs the output scale, the fog and the clock that block carries.

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
        self.zone_end.share_scene(device, blocks);
        self.ship_debris.share_scene(device, blocks);
        // HD's absorb shell is a craft's own scene model (`ShipAbsorbShell_Load` links it under the craft), drawn by
        // the programs that close on `MAD /2` with the fog lerp inside it, so it reads the craft's block: the
        // output scale, the fog and the clock its texture scrolls on.
        for drawable in self.absorb_shell.iter_mut().flatten() {
            drawable.share_scene(device, blocks, SceneKind::Ship);
        }
        for drawable in self.shine.iter_mut().flatten() {
            drawable.share_scene(device, blocks, SceneKind::Shine);
        }
        if let Some(shine) = &mut self.track_shine {
            shine.share_scene(device, blocks);
        }
        if let Some(caustic) = &mut self.track_caustic {
            caustic.share_scene(device, blocks);
        }
        for drawable in self.flares.iter_mut().chain(&mut self.boost).flatten() {
            drawable.share_scene(device, blocks, SceneKind::Flame);
        }
        // The shield shell and cockpit sphere scroll on the shader's clock (`PROGRAM_SHIELD_LIGHTNING`,
        // `own_shield.wesl`), and our own shield also reads the eye. Their drawables' own block stays
        // `Scene::off` with `time` at zero, so they read the flame's: `Scene::off` plus the clock and
        // the eye.
        for drawable in self
            .shield
            .iter_mut()
            .chain([&mut self.shield_cockpit])
            .flatten()
        {
            drawable.share_scene(device, blocks, SceneKind::Flame);
        }
        for spike in self.flare_spikes.iter_mut().flatten() {
            spike.drawable.share_scene(device, blocks, SceneKind::Flame);
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
            self.flare_spikes
                .iter_mut()
                .flatten()
                .map(|s| &mut s.drawable),
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
        self.zone_end.share_uniforms(device, blocks);
        self.ship_debris.share_uniforms(device, blocks);
        if let Some(shine) = &mut self.track_shine {
            shine.share_uniforms(device, blocks);
        }
        if let Some(caustic) = &mut self.track_caustic {
            caustic.share_uniforms(device, blocks);
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
            &mut self.plasma_blast.missile_head,
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
        for pool in self.bomb_blast.detonator.pools_mut() {
            share_all(device, blocks, pool.iter_mut());
        }
        share_all(device, blocks, self.leach_ball.iter_mut());
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
        self.scene_blocks.write_view(
            queue,
            block,
            [circuit, ship, &shine::shine_scene(ship), flame],
        );
    }
}

/// The flame surfaces' block (flares, boost plumes, flare spikes, and the shield shells and
/// cockpit sphere that share it): `Scene::off` plus the clock, the view's eye and the title's
/// output scale, taken from the circuit's `scene`.
///
/// HD's live flame program measures its rim from the camera's world position
/// (`docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, "The live vertex program") and closes
/// its colour on `MAD /2` ("The live fragment program is block #2"), which is `Light::output_scale` (0.5 on HD, 1.0 on every other title).
/// No other draw on this block reads the scale. The program also fogs the colour with the
/// circuit's fog, which at chase distance is a factor of 1 to four places; the block keeps fog
/// off, so the shields sharing it are not fogged on a guess.
/// The flame surfaces' clock: a capture's imposed flame clock ([`crate::Options::flame_seconds`],
/// the original's frame context clock at the grab) or else the world clock every other animation
/// rides (`docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, "The flame's clock at a pair's grab").
pub(super) fn flame_seconds(imposed: Option<f32>, world: f32) -> f32 {
    imposed.unwrap_or(world)
}

pub(super) fn flame_scene(
    scene: &mesh_render::Scene,
    eye: [f32; 3],
    seconds: f32,
) -> mesh_render::Scene {
    let off = mesh_render::Scene::off();
    mesh_render::Scene {
        fog: mesh_render::Fog {
            camera: eye,
            ..mesh_render::Fog::off()
        },
        time: [seconds; 4],
        light: off.light.with_output_scale(scene.light.output_scale),
        ..off
    }
}

#[cfg(test)]
impl Scene {
    /// Sets the circuit's light rig's output scale, as HD's loader does (0.5).
    pub(crate) fn set_output_scale_for_tests(&mut self, output_scale: f32) {
        self.light = self.light.with_output_scale(output_scale);
    }

    /// Gives slot 0 a shield shell and the race a cockpit sphere, each a copy of its hull on a
    /// block of its own.
    pub(crate) fn give_a_shield_for_tests(&mut self, device: &wgpu::Device) {
        let copy = || {
            let mut shell = self.ships[0].instances(device, 1).remove(0);
            shell.clear_scene_kind_for_tests();
            shell
        };
        let (shell, sphere) = (copy(), copy());
        self.shield = vec![Some(shell)];
        self.shield_cockpit = Some(sphere);
    }

    /// The kind of scene block slot 0's engine flare reads.
    pub(crate) fn flare_scene_kind_for_tests(&self) -> Option<SceneKind> {
        self.flares
            .first()
            .and_then(Option::as_ref)
            .and_then(Drawable::scene_kind_for_tests)
    }

    /// The kinds of scene block slot 0's shield shell and the cockpit sphere read.
    pub(crate) fn shield_scene_kinds_for_tests(&self) -> [Option<SceneKind>; 2] {
        [
            self.shield.first().and_then(Option::as_ref),
            self.shield_cockpit.as_ref(),
        ]
        .map(|drawable| drawable.and_then(Drawable::scene_kind_for_tests))
    }

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
