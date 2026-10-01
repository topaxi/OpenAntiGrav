//! The absorb hull overlay's per-frame vertex write and its draw - see
//! `oag_render::hull_overlay` and `race::scene::absorb_overlay` - plus the
//! every-list draw loop it shares with [`Drawable::draw_additive`].

use oag_render::mesh;

use super::{ChunkSet, DrawSections, Drawable, Frustum, SceneStats, VisibleSet};

impl Drawable {
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
    /// The PS2 shield shell reaches the same layer through the same
    /// constructor but does not come through here: the model it draws,
    /// `extrashield.vex`, names the additive class in its own batches, so its
    /// ordinary [`Self::draw`] routes them.
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
    pub(in crate::race) fn draw_additive(&self, pass: &mut wgpu::RenderPass<'_>) -> SceneStats {
        self.draw_every_list(pass, &self.additive_pipeline)
    }

    /// Draws the absorb overlay: every draw of every list through the
    /// pipelines built with this drawable's own blend, which for the overlay
    /// is [`oag_render::hull_overlay::BLEND`] rather than the fixed additive
    /// equation, so the glow mask takes the stencil's full value.
    pub(in crate::race) fn draw_overlay(&self, pass: &mut wgpu::RenderPass<'_>) -> SceneStats {
        self.draw_every_list(pass, &self.blend_pipeline)
    }

    /// Every draw of the opaque, cutout and transparent lists, each through
    /// `pipelines`, picked by the draw's own two-sidedness.
    pub(super) fn draw_every_list(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipelines: &[wgpu::RenderPipeline; 2],
    ) -> SceneStats {
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
            .filter(|draw| self.lod_shows(draw))
        {
            // Two-sidedness stays the batch's own: the plume's `0x20` is set
            // on both discs, so nothing here is culled, and a future model
            // that sets it differently should still be obeyed.
            let pipeline = &pipelines[usize::from(draw.culled)];
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
            // Elided when unchanged, as in `draw`.
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }

    /// [`Self::tint`] and a `v` offset in one upload: every vertex colour
    /// multiplied by `rgba` and every texture coordinate's `v` moved by
    /// `v_offset`, from the model's own vertices every time.
    ///
    /// One write rather than `tint` then `apply_uv_transform`, because each of
    /// those rewrites the whole buffer from the authored vertices and the
    /// second would undo the first. The overlay is the one model that needs
    /// both at once: `HullOverlay_Submit` drives its colour and its texture
    /// matrix's translation off the same pulse.
    pub(in crate::race) fn write_overlay(
        &self,
        queue: &wgpu::Queue,
        rgba: [f32; 4],
        v_offset: f32,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        scratch.clear();
        scratch.extend(self.model.vertices.iter().map(|v| {
            let mut out = *v;
            for (channel, scale) in rgba.iter().enumerate() {
                out.colour[channel] = v.colour[channel] * scale;
            }
            out.texcoord[1] = v.texcoord[1] + v_offset;
            out
        }));
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(scratch));
    }

    /// Uploads this model's vertices with texture coordinates generated for a
    /// craft posed by `ship` - [`oag_render::shine::write`], the environment-mapped
    /// shine pass, whose coordinates follow the craft's rotation and so cannot be
    /// baked, with the airbrake flaps at `flaps` radians (the hull's own base draw
    /// swings the same two). From the model's own vertices every time, like
    /// [`Self::write_overlay`].
    pub(in crate::race) fn write_environment_map(
        &self,
        queue: &wgpu::Queue,
        ship: oag_core::math::Mat4,
        flaps: [f32; 2],
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        oag_render::shine::write(&self.model, scratch, ship, flaps);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(scratch));
    }

    /// Uploads this model's vertices with texture coordinates generated for
    /// the camera `view` - [`oag_render::shine::write_view`], a circuit's extra
    /// pass, whose coordinates follow the camera and so cannot be baked.
    pub(in crate::race) fn write_view_map(
        &self,
        queue: &wgpu::Queue,
        view: oag_core::math::Mat4,
        seconds: f32,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        oag_render::shine::write_view(&self.model, scratch, view, seconds);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(scratch));
    }

    /// Draws a circuit's extra pass: each of this model's draws when the
    /// circuit draw `sources[i]` of `track` is drawn - its level-of-detail
    /// child on, its section allowed, its bound in the frustum - through the
    /// pipelines built with this drawable's own blend.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::race) fn draw_track_shine(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        track: &Drawable,
        sources: &[usize],
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        if self.model.indices.is_empty() {
            return stats;
        }
        let opaque: &[u64] = sections.map_or(&[], |s| &s.opaque[..]);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        pass.set_bind_group(2, &self.fog_bind, &[]);
        pass.set_bind_group(3, &self.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current: Option<&wgpu::RenderPipeline> = None;
        let mut last_bound: Option<usize> = None;
        for (draw, &source) in self.model.draws.iter().zip(sources) {
            let circuit = &track.model.draws[source];
            if !track.lod_shows(circuit)
                || !oag_render::pvs::visible(
                    circuit,
                    DrawSections::at(opaque, source),
                    set,
                    chunks,
                    frustum,
                )
            {
                continue;
            }
            let pipeline = &self.blend_pipeline[usize::from(draw.culled)];
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }
}
