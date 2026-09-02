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
    /// One pair per equation this model's own file authors - see
    /// `mesh_render::Built::authored_pipelines`. Empty on a Pulse model.
    authored_pipelines: Vec<(wgpu::BlendState, [wgpu::RenderPipeline; 2])>,
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
    /// This drawable's `Anim Transform` node matrices, bound as binding 1 of
    /// the same group 3 as [`Self::anims`] - see
    /// `mesh_render::Built::node_anim_buffer` for why they share a group.
    ///
    /// The buffer behind it. Rewritten each frame by
    /// [`Self::write_node_anims`], or left all-identity for a model with no
    /// `Anim Transform` at all - which is every ship, every sky and every
    /// synthetic overlay.
    node_anims: wgpu::Buffer,
    /// The Zone visualiser's own lookup texture - bind group 2's binding 4.
    /// All-black until [`Self::write_zone_vis`] is called, which
    /// `race::Scene::render` does once a frame alongside [`Self::fog`].
    zone_vis: wgpu::Texture,
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
        zone: Option<&std::sync::Arc<oag_render::mesh::ModelTexture>>,
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            authored_pipelines,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
            fog_bind,
            fog_buffer,
            zone_vis_texture,
            anim_bind,
            anim_buffer,
            node_anim_buffer,
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
            // Everything a race draws writes the velocity buffer (or, for
            // its blended pipelines, carries the masked second target) -
            // the buffer is always on in the game path, whatever the
            // motion blur setting says. See `mesh_render::Velocity` and
            // `race::Scene::velocity`.
            mesh_render::Velocity::Write,
            zone,
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
            authored_pipelines,
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
            fog_bind,
            fog: fog_buffer,
            anim_bind,
            anims: anim_buffer,
            node_anims: node_anim_buffer,
            zone_vis: zone_vis_texture,
        })
    }

    /// Rewrites the Zone visualiser's lookup from `bands` levels, tinted by
    /// `tint` - or blanks it when `tint` is `None`, which is what a stage
    /// with no authored `EQ colour tint` gets. See
    /// `oag_render::mesh_render::zone::write_vis`, which this forwards to.
    pub(super) fn write_zone_vis(&self, queue: &wgpu::Queue, bands: &[f32], tint: Option<[u8; 3]>) {
        mesh_render::zone::write_vis(queue, &self.zone_vis, bands, tint);
    }

    /// `prev_mvp` is the previous simulation tick's `view_projection * model`,
    /// premultiplied - what the velocity target measures this drawable's
    /// screen motion against. A drawable with no previous pose (a scene's
    /// first frame, a rocket that just spawned) passes this frame's product,
    /// which is zero object velocity rather than a smear from stale state.
    /// See `race::scene::frame::MotionState`.
    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
    ) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            _unused: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            prev_mvp: prev_mvp.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Samples every authored texture-transform track this model carries at
    /// `seconds` and uploads the table the vertex shader indexes.
    ///
    /// One buffer write per drawable per frame, whatever the geometry: the
    /// curve is evaluated once per *track*, not per vertex or per draw call.
    ///
    /// **Not called for the boost plume**, whose *texture-transform* table
    /// stays all-identity. The plume needs its own clock - its flare's life
    /// timer, reset at every reveal, rather than the race's - and already
    /// gets it through [`Self::apply_uv_transform`], which rewrites its
    /// vertices directly. Driving it from here as well would apply the
    /// transform twice.
    ///
    /// That says nothing about [`Self::write_node_anims`], which is a
    /// different buffer: the plume's *node* table is written, off the same
    /// reveal timer, because a PS2 plume's two meshes hang off `Anim
    /// Transform` anchors and draw at the craft's origin without it.
    pub(super) fn write_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_tracks.is_empty() {
            return;
        }
        let anims = mesh_render::TexAnims::sample(&self.model, seconds);
        queue.write_buffer(&self.anims, 0, bytemuck::bytes_of(&anims));
    }

    /// Samples every `Anim Transform` this model carries at `seconds` and
    /// uploads the matrix table the vertex shader indexes.
    ///
    /// The sibling of [`Self::write_anims`], and the same trade: one buffer
    /// write per drawable per frame, with each node's chain resolved once
    /// rather than per vertex. A model with no `Anim Transform` - every ship,
    /// the sky, both pad models, the collision overlay, and every **PSP**
    /// boost plume - writes nothing and keeps the identity table
    /// `mesh_render::build` initialised it with. A **PS2** plume carries two
    /// and is placed entirely by them.
    pub(super) fn write_node_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_nodes.is_empty() {
            return;
        }
        let anims = mesh_render::NodeAnims::sample(&self.model, seconds);
        queue.write_buffer(&self.node_anims, 0, bytemuck::bytes_of(&anims));
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
    ///
    /// `moved` is scratch the caller owns and this refills - see
    /// `Scene::scratch`. It runs on every frame of every race, so building the
    /// span into a fresh `Vec` was two allocations a frame that never survived
    /// the upload.
    pub(super) fn deflect_airbrakes(
        &self,
        queue: &wgpu::Queue,
        left: f32,
        right: f32,
        moved: &mut Vec<mesh::GpuVertex>,
    ) {
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
            moved.clear();
            moved.extend(base.iter().map(|v| {
                let mut out = *v;
                out.position = swing
                    .transform_point3(Vec3::from_array(v.position))
                    .to_array();
                out.normal = swing
                    .transform_vector3(Vec3::from_array(v.normal))
                    .to_array();
                out
            }));
            let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
            queue.write_buffer(
                &self.vertices,
                span.start as u64 * stride,
                bytemuck::cast_slice(moved),
            );
        }
    }

    /// Multiplies every vertex colour by `rgba`, from the model's own authored
    /// values.
    ///
    /// What the shield shell needs and the only thing it needs: the original
    /// hands its model a packed colour once a frame (`set_model_colour` in
    /// `ShipShield_Update`) and this engine's mesh pipeline has no per-draw
    /// colour uniform to put one in. Rewriting the vertex colours is the same
    /// trade [`Self::deflect_airbrakes`] makes for the flaps - one
    /// `write_buffer` of a small mesh against a shader change and a fourth bind
    /// group - and it is exactly a multiply, so the authored colour still
    /// decides the look and this only scales it.
    ///
    /// **From `self.model.vertices` every time, never from the last frame's**,
    /// for the reason the flaps give: compounding a per-frame multiply would
    /// darken the shell toward black over a few seconds and make the picture
    /// depend on how long the shield had been up.
    ///
    /// One write for the whole buffer rather than one per node: the shell is a
    /// single mesh, and a per-range loop would be machinery for a case that does
    /// not exist.
    /// `tinted` is scratch the caller owns and this refills - see
    /// `Scene::scratch`. A whole shell's vertices, once per craft with its
    /// shield up and once more for the cockpit sphere: the allocation this
    /// avoids is larger than the pads' and rarer.
    pub(super) fn tint(
        &self,
        queue: &wgpu::Queue,
        rgba: [f32; 4],
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        tinted.clear();
        tinted.extend(self.model.vertices.iter().map(|v| {
            let mut out = *v;
            for (channel, scale) in rgba.iter().enumerate() {
                out.colour[channel] = v.colour[channel] * scale;
            }
            out
        }));
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(tinted));
    }

    /// Recolours each weapon pad's own geometry by whether it currently hands
    /// out a pickup - see [`oag_render::weapon_pad`] for the recovered
    /// mechanism this reproduces. `ready[i]` pairs with this drawable's
    /// `i`-th [`mesh::Model::node_vertex_ranges`] entry - positional, not by
    /// name, for the reason that field's own doc comment gives.
    ///
    /// Rewrites the *replacement* colour, not a multiply against the pad's
    /// authored one - `pad+0x6c` overwrites the mesh's own GE colour command
    /// every tick in the original, it does not modulate it. Always from
    /// `self.model.vertices`, never from the last frame's buffer, for the
    /// same accumulation reason [`Self::deflect_airbrakes`] gives.
    ///
    /// A no-op on a model with no weapon pads - every track and the sky -
    /// since `ready` is empty there.
    pub(super) fn tint_weapon_pads(
        &self,
        queue: &wgpu::Queue,
        seconds: f32,
        ready: &[bool],
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
        for (range, &is_ready) in self.model.node_vertex_ranges.iter().zip(ready) {
            let span = range.start as usize..range.end as usize;
            let Some(base) = self.model.vertices.get(span) else {
                continue;
            };
            let colour = if is_ready {
                oag_render::weapon_pad::ready_colour(seconds)
            } else {
                oag_render::weapon_pad::COOLDOWN_COLOUR
            };
            // Refilled rather than rebuilt - one pad's worth of vertices, and
            // there is a pad's worth of them on every lap of every circuit
            // every frame. See `Scene::scratch`.
            tinted.clear();
            tinted.extend(base.iter().map(|v| {
                let mut out = *v;
                out.colour = [colour[0], colour[1], colour[2], out.colour[3]];
                out
            }));
            queue.write_buffer(
                &self.vertices,
                u64::from(range.start) * stride,
                bytemuck::cast_slice(tinted),
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
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        let mut binds = oag_render::perfprobe::Binds::default();
        let mut last_bound: Option<usize> = None;
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
        oag_render::perfprobe::pipeline_set();
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
            if !visible(draw, DrawSections::at(opaque, index), set, chunks, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            binds.record(slot);
            // Switched only when the slot changes, the same elision the
            // transparent list's `set_pipeline` already runs. Batches
            // sharing a texture arrive in runs - a model's own batch order
            // groups them - so this is not a bet: measured on Talon's
            // Junction, 87 of 580 binds a frame in a time trial and 101 of
            // 579 in a full grid re-bound the group already bound.
            //
            // Carried across all three lists rather than reset per list,
            // because a `set_pipeline` between them does not unbind
            // anything: every pipeline this drawable owns is built from one
            // layout, which is the same fact that already lets groups 0, 2
            // and 3 be bound once at the top and left alone through both
            // pipeline switches below. It restarts at `None` per
            // [`Self::draw`] call, where the texture array itself changes.
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Second pipeline, same pass: alpha-tested batches, cutout. See
        // `mesh_render::Built::alpha_test_pipeline`.
        oag_render::perfprobe::pipeline_set();
        pass.set_pipeline(&self.alpha_test_pipeline);
        for (index, draw) in self.model.alpha_tested_draws.iter().enumerate() {
            if !visible(
                draw,
                DrawSections::at(alpha_tested, index),
                set,
                chunks,
                frustum,
            ) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            binds.record(slot);
            // Elided when unchanged, as above.
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
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
        let pipelines = mesh_render::TransparentPipelines {
            alpha_over: &self.blend_pipeline,
            additive: &self.additive_pipeline,
            unblended: &self.unblended_pipeline,
            authored: &self.authored_pipelines,
        };
        let mut current: Option<&wgpu::RenderPipeline> = None;
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !visible(
                draw,
                DrawSections::at(transparent, index),
                set,
                chunks,
                frustum,
            ) {
                stats.draws_culled += 1;
                continue;
            }
            let pipeline = pipelines.select(draw);
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                oag_render::perfprobe::pipeline_set();
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            binds.record(slot);
            // Elided when unchanged, as above.
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }

    /// Draws **every** list of this model through the additive pipeline,
    /// ignoring which list each batch's `pass_mask` put it in.
    ///
    /// **Only the PS2 boost plume uses this, and only because a reference
    /// frame settled it.** That model's four batches carry no `0x0700` class
    /// bit, so they land in [`Model::draws`] and [`Self::draw`] would submit
    /// them through the opaque pipeline - which draws each nozzle as a solid
    /// hexagon with hard edges, occluding the hull behind it. A PCSX2 capture
    /// of the original (2026-08-23, the first this project has taken) shows
    /// the opposite: soft violet plumes with no geometry edge anywhere and the
    /// hull visible through them. So the original blends this model, and the
    /// question is only where it says so.
    ///
    /// **Where it says so is unrecovered, and that is why this is a
    /// model-scoped override rather than a decode.** `Gfx_BuildBatchStateList`
    /// (`0x001e9088`) does disable blending for a `0x0700`-clear batch - read
    /// on the PS2 executable, and every other `pass_mask` bit it tests matches
    /// the PSP's - but it is reached through `Mesh_DrawBatches` for sort keys
    /// of layer `0x750`, and the plume's own object queues at `0x7d0`. The
    /// draw its vtable (`0x0029a3a0`) reaches for that layer has not been
    /// followed yet. See
    /// `docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md`.
    ///
    /// The equation is not invented either: `mesh_render::ADDITIVE_BLEND` is
    /// the `0x200` class's own recovered equation, byte-identical to
    /// [`oag_render::exhaust::BLEND`], and it is what the **PSP** plume
    /// already draws with - its batches carry `0x200` and
    /// `TransparentPipelines::select` routes them there. So this puts the two
    /// discs' plumes on one blend rather than giving them two.
    ///
    /// A PSP plume never reaches this method's opaque or cutout lists, both
    /// being empty there, so calling it for both titles changes nothing on
    /// PSP.
    pub(super) fn draw_additive(&self, pass: &mut wgpu::RenderPass<'_>) -> SceneStats {
        let mut stats = SceneStats::default();
        let mut binds = oag_render::perfprobe::Binds::default();
        let mut last_bound: Option<usize> = None;
        if self.model.indices.is_empty() {
            return stats;
        }
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        pass.set_bind_group(2, &self.fog_bind, &[]);
        pass.set_bind_group(3, &self.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current: Option<&wgpu::RenderPipeline> = None;
        for draw in self
            .model
            .draws
            .iter()
            .chain(&self.model.alpha_tested_draws)
            .chain(&self.model.transparent_draws)
        {
            // Two-sidedness stays the batch's own: the plume's `0x20` is set
            // on both discs, so nothing here is culled, and a future model
            // that sets it differently should still be obeyed.
            let pipeline = &self.additive_pipeline[usize::from(draw.culled)];
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                oag_render::perfprobe::pipeline_set();
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            binds.record(slot);
            // Elided when unchanged, as above.
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
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
    /// The previous tick's `view_projection * model`, premultiplied - one
    /// matrix rather than a second pair, per `mesh.wgsl`'s own mirror: fog
    /// and lighting need world position, velocity needs only clip position.
    prev_mvp: [[f32; 4]; 4],
}

/// A craft's model matrix, which is the only correct way to place its mesh.
pub(super) fn model_matrix_of(ship: &Ship) -> Mat4 {
    let body = &ship.physics.body;
    Mat4::from_rotation_translation(body.orientation, body.position)
        * Mat4::from_rotation_y(MODEL_YAW)
        * Mat4::from_scale(Vec3::splat(oag_render::exhaust::CRAFT_ROW_SCALE))
}
