//! One [`Drawable`] standing in for many: the projectile pools.

use super::*;

/// Where a drawable's blocks start in its uniform and animation buffers - see
/// [`Drawable::instances`].
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Offsets {
    pub(super) uniforms: u64,
    pub(super) anims: u64,
    pub(super) node_anims: u64,
    pub(super) emissive: u64,
}

impl Drawable {
    /// `count` more drawables of the same model, each sharing every resource this one owns
    /// except its own uniform block - what a projectile pool's slots need.
    ///
    /// A pool is `MAX_PROJECTILES` slots of one weapon model, each drawn at its own matrix.
    /// Built with [`Self::new`] each slot would repeat the vertex and index buffers, the
    /// texture uploads, the fog and animation buffers and their bind groups, and the zone
    /// lookup texture, which are identical across slots: about 250 MiB for a Pulse race.
    /// `wgpu`'s handles are reference counted, so a clone is another name for the same GPU
    /// object.
    ///
    /// **A model that animates gets animation blocks of its own per slot.** Both are
    /// per-drawable state a caller may scrub per slot - a Plasma blast does, one clock per
    /// slot - and sharing them would show every slot the last clock written. They start as
    /// `mesh_render::build` makes them (all-identity, plus the model's own emissive table),
    /// so an unscrubbed slot draws as it always did. A model that does not animate shares
    /// this drawable's, which nobody can write.
    ///
    /// **One buffer per kind of block for the whole pool, each slot a range of it**
    /// ([`Offsets`]), not four buffers a slot. wgpu sizes every pass's usage scope to the
    /// device's buffer count: HD's pools (about 1,800 slots) held 6,235 buffers alive, so
    /// each of a frame's ~54 passes resized and cleared that many entries; the shared
    /// buffers leave about 1,000. The bytes a slot reads and writes are the same.
    ///
    /// Sharing the fog buffer and the vertex buffer is safe for a pool because nothing
    /// writes either of them for a pool slot: `Scene::write_frame` never lists one, so they
    /// stay at the `Fog::off` and the authored geometry [`Self::new`] gave them, in every
    /// slot alike. A caller that starts writing per-slot fog or vertices must stop sharing
    /// them first.
    pub(crate) fn instances(&self, device: &wgpu::Device, count: usize) -> Vec<Self> {
        let align = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = |size: u64| size.div_ceil(align) * align;
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let pool = |label, size: u64, initial: &[u8]| {
            let length = stride(size) * count.max(1) as u64;
            if initial.is_empty() {
                return device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: length,
                    usage,
                    mapped_at_creation: false,
                });
            }
            let mut contents = vec![0; length as usize];
            for from in (0..count).map(|at| at * stride(size) as usize) {
                contents[from..from + initial.len()].copy_from_slice(initial);
            }
            oag_gpu::init_buffer::init(device, label, usage, &contents)
        };
        let uniforms = pool("race uniforms", mesh_render::UNIFORMS_SIZE, &[]);
        let animates = !(self.model.anim_tracks.is_empty() && self.model.anim_nodes.is_empty());
        let animation = animates.then(|| {
            (
                pool(
                    "texture animation",
                    mesh_render::TEX_ANIMS_SIZE,
                    bytemuck::bytes_of(&mesh_render::TexAnims::default()),
                ),
                pool(
                    "node animation",
                    mesh_render::NODE_ANIMS_SIZE,
                    bytemuck::bytes_of(&mesh_render::NodeAnims::default()),
                ),
                pool(
                    "emissive glow",
                    mesh_render::EMISSIVES_SIZE,
                    bytemuck::bytes_of(&mesh_render::Emissives::of(&self.model)),
                ),
            )
        });
        (0..count as u64)
            .map(|at| {
                fn range(
                    buffer: &wgpu::Buffer,
                    offset: u64,
                    size: u64,
                ) -> wgpu::BindingResource<'_> {
                    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset,
                        size: std::num::NonZeroU64::new(size),
                    })
                }
                let range = |buffer, size| range(buffer, at * stride(size), size);
                let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("race uniforms"),
                    layout: &self.pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: range(&uniforms, mesh_render::UNIFORMS_SIZE),
                    }],
                });
                let mut offsets = Offsets {
                    uniforms: at * stride(mesh_render::UNIFORMS_SIZE),
                    ..self.offsets
                };
                let (anims, node_anims, emissive, anim_bind) = match &animation {
                    None => (
                        self.anims.clone(),
                        self.node_anims.clone(),
                        self.emissive.clone(),
                        self.anim_bind.clone(),
                    ),
                    Some((anims, node_anims, emissive)) => {
                        offsets.anims = at * stride(mesh_render::TEX_ANIMS_SIZE);
                        offsets.node_anims = at * stride(mesh_render::NODE_ANIMS_SIZE);
                        offsets.emissive = at * stride(mesh_render::EMISSIVES_SIZE);
                        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("animation"),
                            layout: &self.pipeline.get_bind_group_layout(3),
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: range(anims, mesh_render::TEX_ANIMS_SIZE),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: range(node_anims, mesh_render::NODE_ANIMS_SIZE),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: range(emissive, mesh_render::EMISSIVES_SIZE),
                                },
                            ],
                        });
                        (anims.clone(), node_anims.clone(), emissive.clone(), bind)
                    }
                };
                self.slot(
                    uniforms.clone(),
                    uniform_bind,
                    offsets,
                    (anims, node_anims, emissive, anim_bind),
                )
            })
            .collect()
    }

    /// A pool slot of this drawable: its resources, with these uniforms and animation
    /// blocks.
    fn slot(
        &self,
        uniforms: wgpu::Buffer,
        uniform_bind: wgpu::BindGroup,
        offsets: Offsets,
        (anims, node_anims, emissive, anim_bind): (
            wgpu::Buffer,
            wgpu::Buffer,
            wgpu::Buffer,
            wgpu::BindGroup,
        ),
    ) -> Self {
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
            hidden: std::sync::Mutex::default(),
            reversed_indices: self.reversed_indices.clone(),
            indirect: None,
            vertices: self.vertices.clone(),
            texcoords: self.texcoords.clone(),
            indices: self.indices.clone(),
            uniforms,
            uniform_bind,
            uniform_slot: None,
            offsets,
            textures: self.textures.clone(),
            bindless: self.bindless.clone(),
            card: self.card.clone(),
            fog_bind: self.fog_bind.clone(),
            fog: self.fog.clone(),
            scene_kind: self.scene_kind,
            anim_bind,
            anims,
            node_anims,
            anims_at: std::sync::atomic::AtomicU64::new(0),
            node_anims_at: std::sync::atomic::AtomicU64::new(0),
            shine_pose: std::sync::Mutex::default(),
            emissive,
            zone_vis: self.zone_vis.clone(),
            zone_rebind: self.zone_rebind.clone(),
            opaque_ranges: self.opaque_ranges.clone(),
            moving: self.moving.clone(),
            lod: super::lod_switches(&self.model.lod_groups),
            bound: self.bound.clone(),
            ripple: std::sync::Mutex::new(None),
            pad_glow: std::sync::Mutex::default(),
        }
    }

    /// Whether `other` draws from this drawable's own vertex and index buffers
    /// rather than copies of them.
    #[cfg(test)]
    pub(crate) fn shares_geometry_with(&self, other: &Self) -> bool {
        self.vertices == other.vertices && self.indices == other.indices
    }
}
