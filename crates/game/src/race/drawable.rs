//! One model as the renderer sees it - [`Drawable`] - and the uniform block the
//! mesh shader reads.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// One model on the GPU: its pipeline, its geometry and its own uniform buffer.
///
/// Two of these are drawn into one render pass. Separate pipelines rather than one
/// shared between the models, because each `mesh_render::build` creates its own bind
/// group layouts and pairing a bind group with another pipeline's layout is a
/// validation error waiting to happen.
pub(super) struct Drawable {
    model: Model,
    pipeline: wgpu::RenderPipeline,
    alpha_test_pipeline: wgpu::RenderPipeline,
    blend_pipeline: [wgpu::RenderPipeline; 2],
    /// The other two recovered transparent blend classes. Selected per draw
    /// call, because a single model mixes them - see
    /// `oag_formats::vex::Batch::blend_class`.
    additive_pipeline: [wgpu::RenderPipeline; 2],
    unblended_pipeline: [wgpu::RenderPipeline; 2],
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    textures: Vec<wgpu::BindGroup>,
    /// Bind group 2: the fog this drawable is rendered with.
    fog_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame from the track's `fogCube`,
    /// or left at [`mesh_render::Fog::off`] for the sky and for a track that
    /// authors no fog.
    pub(super) fog: wgpu::Buffer,
    /// Bind group 3: this drawable's authored texture transforms, sampled for
    /// the current tick.
    anim_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame by [`Self::write_anims`], or
    /// left all-identity for a model that authors no track.
    anims: wgpu::Buffer,
}

impl std::fmt::Debug for Drawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawable")
            .field("model", &self.model.label)
            .field("triangles", &(self.model.indices.len() / 3))
            .finish_non_exhaustive()
    }
}

impl Drawable {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        depth: mesh_render::Depth,
        blend: wgpu::BlendState,
        glow: mesh_render::GlowMask,
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
            fog_bind,
            fog_buffer,
            anim_bind,
            anim_buffer,
        } = mesh_render::build(
            device,
            queue,
            &model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
            glow,
        )?;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("race uniforms"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        Ok(Self {
            model,
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
            fog_bind,
            fog: fog_buffer,
            anim_bind,
            anims: anim_buffer,
        })
    }

    pub(super) fn write(&self, queue: &wgpu::Queue, view_projection: Mat4, model: Mat4) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            _unused: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Samples every authored texture-transform track this model carries at
    /// `seconds` and uploads the table the vertex shader indexes.
    ///
    /// One buffer write per drawable per frame, whatever the geometry: the
    /// curve is evaluated once per *track*, not per vertex or per draw call.
    ///
    /// **Not called for the boost plume**, whose table stays all-identity.
    /// The plume needs its own clock - its flare's life timer, reset at every
    /// reveal, rather than the race's - and already gets it through
    /// [`Self::apply_uv_transform`], which rewrites its vertices directly.
    /// Driving it from here as well would apply the transform twice.
    pub(super) fn write_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_tracks.is_empty() {
            return;
        }
        let anims = mesh_render::TexAnims::sample(&self.model, seconds);
        queue.write_buffer(&self.anims, 0, bytemuck::bytes_of(&anims));
    }

    /// Applies a texture transform to this model's **authored** UVs and
    /// uploads them: `uv' = uv * scale + offset`, the GE's own
    /// `TEXSCALE`/`TEXOFFSET` arithmetic.
    ///
    /// Called only for the plume, and only on the frames it is visible. This
    /// replaced environment-mapped generation (`oag_render::texgen`) on
    /// 2026-08-10: the plume's compiled list is replayed under `TEXMAPMODE` 0
    /// (settled live - mesh-draw.md, "The plume is replayed under
    /// `TEXMAPMODE` 0"), so the original reads the authored coordinates,
    /// through the keyframed transform `Loaded::boost_uv_transform` carries.
    ///
    /// From `self.model.vertices` every time rather than from the last
    /// frame's buffer, for the same reason [`Self::deflect_airbrakes`] does:
    /// accumulating offsets would drift.
    pub(super) fn apply_uv_transform(
        &self,
        queue: &wgpu::Queue,
        scale: (f32, f32),
        offset: (f32, f32),
    ) {
        let transformed: Vec<_> = self
            .model
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                v.texcoord = [
                    v.texcoord[0] * scale.0 + offset.0,
                    v.texcoord[1] * scale.1 + offset.1,
                ];
                v
            })
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&transformed));
    }

    /// Swings this model's airbrake flaps to `left` and `right` radians.
    ///
    /// Rewrites the two flaps' own vertices in place rather than giving them a
    /// transform of their own. The alternative - pulling each flap into its own
    /// `Drawable`, the way the boost plume is one - would duplicate the ship's
    /// whole eight-texture set for two meshes of forty vertices, and a
    /// per-draw-call matrix would need a fourth bind group set on every draw
    /// call of every track. This costs two `write_buffer`s of about a kilobyte
    /// and no GPU state at all, and `exhaust.rs` already rewrites a vertex
    /// buffer every frame, so a mutable one is not a new idea here.
    ///
    /// A no-op on a model with no flaps, which is every track, the sky, the
    /// collision overlay and any ship whose file does not carry them.
    pub(super) fn deflect_airbrakes(&self, queue: &wgpu::Queue, left: f32, right: f32) {
        for (flap, angle) in self.model.airbrakes.iter().zip([left, right]) {
            let Some(flap) = flap else { continue };
            let swing = flap.deflect(angle);
            let span = flap.vertices.start as usize..flap.vertices.end as usize;
            let Some(base) = self.model.vertices.get(span.clone()) else {
                continue;
            };
            // From the model's own vertices every time, never from the last
            // frame's: accumulating rotations would drift, and worse, would
            // make the rest position depend on how the ship got there.
            let moved: Vec<mesh::GpuVertex> = base
                .iter()
                .map(|v| {
                    let mut out = *v;
                    out.position = swing
                        .transform_point3(Vec3::from_array(v.position))
                        .to_array();
                    out.normal = swing
                        .transform_vector3(Vec3::from_array(v.normal))
                        .to_array();
                    out
                })
                .collect();
            let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
            queue.write_buffer(
                &self.vertices,
                span.start as u64 * stride,
                bytemuck::cast_slice(&moved),
            );
        }
    }

    /// Draws every list, in pipeline order, and reports what it submitted.
    ///
    /// `frustum` is `None` for the ship and the collision overlay: both draw a
    /// handful of batches next to the camera regardless, where the bookkeeping
    /// would cost more than the culling could ever save, and both carry a
    /// non-identity model matrix the track does not - the [`Bounds`] on a
    /// `DrawCall` are in the *model's own* space, valid to test directly
    /// against a world-space frustum only while that space and world space
    /// are the same transform, which is only true here for the track (see the
    /// `Mat4::IDENTITY` passed to [`Self::write`] at each call site).
    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        // A model with no placement table has every draw call unplaced, which
        // the first tier always allows. That is the ship and the collision
        // overlay, and any track whose sections did not decode.
        let empty: &[u64] = &[];
        let (opaque, alpha_tested, transparent) = match sections {
            Some(s) => (&s.opaque[..], &s.alpha_tested[..], &s.transparent[..]),
            None => (empty, empty, empty),
        };
        if self.model.indices.is_empty() {
            return stats;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        // Bound once for the whole drawable: fog is per-frame, not per draw call,
        // and group 2 survives the `set_pipeline` calls below.
        pass.set_bind_group(2, &self.fog_bind, &[]);
        // Same reasoning as fog: the table is per frame, not per draw call.
        pass.set_bind_group(3, &self.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        for (index, draw) in self.model.draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(opaque, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Second pipeline, same pass: alpha-tested batches, cutout. See
        // `mesh_render::Built::alpha_test_pipeline`.
        pass.set_pipeline(&self.alpha_test_pipeline);
        for (index, draw) in self.model.alpha_tested_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(alpha_tested, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Third pipeline group, same pass: transparent batches, blended.
        //
        // **The blend equation is the batch's own, not the model's.**
        // `Gfx_BuildBatchStateList` splits `is_transparent()`'s `0x0700` three
        // ways - `0x100` alpha-over, `0x200` additive, `0x400` unblended - and
        // a single model mixes them, so the pipeline is chosen per draw call.
        // This crate drew every one of them alpha-over until that was
        // recovered. See `oag_formats::vex::Batch::blend_class` and
        // `mesh_render::ADDITIVE_BLEND`.
        //
        // Switched only when the class changes rather than grouped by class
        // first: transparent draws are order-dependent by definition, and
        // sorting them to save `set_pipeline` calls would reorder the picture.
        let mut current: Option<(Option<oag_formats::vex::BlendClass>, bool)> = None;
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(transparent, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            let key = (draw.blend, draw.culled);
            if current != Some(key) {
                let set = match draw.blend {
                    Some(oag_formats::vex::BlendClass::Additive) => &self.additive_pipeline,
                    Some(oag_formats::vex::BlendClass::None) => &self.unblended_pipeline,
                    Some(oag_formats::vex::BlendClass::AlphaOver) | None => &self.blend_pipeline,
                };
                pass.set_pipeline(&set[usize::from(draw.culled)]);
                current = Some(key);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }
}

/// Uniforms shared with `oag-render`'s `mesh.wgsl`.
///
/// Declared here rather than reused because `oag_render::mesh_render` only exposes
/// a writer that computes an *orbit* camera from the model's bounding sphere, which
/// is what a viewer wants and is not something a chase camera can use. The layout is
/// the shader's own, and a test below asserts it against
/// `mesh_render::UNIFORMS_SIZE`, so a change on that side is a failing test rather
/// than a silently wrong picture.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    /// Unused, and padding rather than removed - see
    /// `mesh_render`'s own mirror of this layout. Was a global
    /// texture-animation phase, replaced by the authored per-material
    /// keyframe tracks in `mesh_render::TexAnims`.
    _unused: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

/// A craft's model matrix, which is the only correct way to place its mesh.
pub(super) fn model_matrix_of(ship: &Ship) -> Mat4 {
    let body = &ship.physics.body;
    Mat4::from_rotation_translation(body.orientation, body.position)
        * Mat4::from_rotation_y(MODEL_YAW)
        * Mat4::from_scale(Vec3::splat(oag_render::exhaust::CRAFT_ROW_SCALE))
}
