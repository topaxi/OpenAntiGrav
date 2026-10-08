//! One [`Drawable`] standing in for many: the projectile pools.

use super::*;

impl Drawable {
    /// Another drawable of the same model that shares every resource this one
    /// owns except its own uniform block - what a pool slot needs.
    ///
    /// A pool is `MAX_PROJECTILES` slots of one weapon model, each drawn at its
    /// own matrix. Built with [`Self::new`] each slot repeats the vertex and
    /// index buffers, the texture uploads, the fog and animation buffers and
    /// their bind groups, and the zone lookup texture, which are identical
    /// across slots: about 250 MiB for a Pulse race. `wgpu`'s handles are
    /// reference counted, so a clone is another name for the same GPU object.
    ///
    /// **A model that animates gets animation buffers of its own.** Both are
    /// per-drawable state a caller may scrub per slot - a Plasma blast does,
    /// one clock per slot - and sharing them would show every slot the last
    /// clock written. They are rebuilt exactly as `mesh_render::build` makes
    /// them (all-identity, plus the model's own emissive table), so an
    /// unscrubbed slot draws as it always did. A model that does not animate
    /// shares the first slot's, which nobody can write.
    ///
    /// Sharing the fog buffer and the vertex buffer is safe for a pool because
    /// nothing writes either of them for a pool slot: `Scene::write_frame` never
    /// lists one, so they stay at the `Fog::off` and the authored geometry
    /// [`Self::new`] gave them, in every slot alike. A caller that starts
    /// writing per-slot fog or vertices must stop sharing them first.
    pub(crate) fn instance(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let (anims, node_anims, emissive, anim_bind) =
            if self.model.anim_tracks.is_empty() && self.model.anim_nodes.is_empty() {
                (
                    self.anims.clone(),
                    self.node_anims.clone(),
                    self.emissive.clone(),
                    self.anim_bind.clone(),
                )
            } else {
                self.own_animation(device, queue)
            };
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("race uniforms"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        Self {
            model: self.model.clone(),
            pipeline: self.pipeline.clone(),
            alpha_test_pipeline: self.alpha_test_pipeline.clone(),
            cutout_pipelines: self.cutout_pipelines.clone(),
            blend_pipeline: self.blend_pipeline.clone(),
            additive_pipeline: self.additive_pipeline.clone(),
            unblended_pipeline: self.unblended_pipeline.clone(),
            authored_pipelines: self.authored_pipelines.clone(),
            stamp_pipeline: self.stamp_pipeline.clone(),
            prepass: self.prepass.clone(),
            shown: std::cell::RefCell::default(),
            hidden: std::cell::RefCell::default(),
            reversed_indices: self.reversed_indices.clone(),
            vertices: self.vertices.clone(),
            texcoords: self.texcoords.clone(),
            indices: self.indices.clone(),
            uniforms,
            uniform_bind,
            textures: self.textures.clone(),
            fog_bind: self.fog_bind.clone(),
            fog: self.fog.clone(),
            anim_bind,
            anims,
            node_anims,
            emissive,
            zone_vis: self.zone_vis.clone(),
            zone_rebind: self.zone_rebind.clone(),
            opaque_ranges: self.opaque_ranges.clone(),
            lod: oag_mesh::mesh::LodSwitch::new(&self.model.lod_groups),
            ripple: std::cell::RefCell::new(None),
            pad_glow: std::cell::RefCell::default(),
        }
    }

    /// Fresh texture-animation, node-animation and emissive buffers and the
    /// bind group over them, as `mesh_render::build` initialises a drawable's.
    fn own_animation(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::BindGroup) {
        let buffer = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let anims = buffer("texture animation", mesh_render::TEX_ANIMS_SIZE);
        queue.write_buffer(
            &anims,
            0,
            bytemuck::bytes_of(&mesh_render::TexAnims::default()),
        );
        let node_anims = buffer("node animation", mesh_render::NODE_ANIMS_SIZE);
        queue.write_buffer(
            &node_anims,
            0,
            bytemuck::bytes_of(&mesh_render::NodeAnims::default()),
        );
        let emissive = buffer("emissive glow", mesh_render::EMISSIVES_SIZE);
        queue.write_buffer(
            &emissive,
            0,
            bytemuck::bytes_of(&mesh_render::Emissives::of(&self.model)),
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("animation"),
            layout: &self.pipeline.get_bind_group_layout(3),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: anims.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: node_anims.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: emissive.as_entire_binding(),
                },
            ],
        });
        (anims, node_anims, emissive, bind)
    }

    /// Whether `other` draws from this drawable's own vertex and index buffers
    /// rather than copies of them.
    #[cfg(test)]
    pub(crate) fn shares_geometry_with(&self, other: &Self) -> bool {
        self.vertices == other.vertices && self.indices == other.indices
    }
}
