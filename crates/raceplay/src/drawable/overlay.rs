//! The absorb hull overlay's per-frame vertex write and its draw - see
//! `oag_fx::hull_overlay` and `race::scene::absorb_overlay` - plus the
//! every-list draw loop it shares with [`Drawable::draw_additive`].

use oag_mesh::mesh;

use super::{ChunkSet, DrawSections, Drawable, Frustum, SceneStats, ViewBlock, VisibleSet};

/// What one view culls the circuit's draws by: the authored sections, the two visibility
/// tiers and the frustum.
#[derive(Clone, Copy)]
pub(crate) struct TrackCull<'a> {
    pub(crate) sections: Option<&'a DrawSections>,
    pub(crate) set: Option<&'a VisibleSet>,
    pub(crate) chunks: Option<&'a ChunkSet>,
    pub(crate) frustum: Option<&'a Frustum>,
}

impl Drawable {
    /// Draws **every** list of this model through the additive pipeline,
    /// ignoring which list each batch's `pass_mask` put it in.
    ///
    /// **Only the PS2 boost plume uses this, and only because a reference
    /// frame settled it.** That model's four batches carry no `0x0700` class
    /// bit, so they land in [`Model::draws`] and [`Self::draw`] would submit
    /// them through the opaque pipeline - which draws each nozzle as a solid
    /// hexagon with hard edges, occluding the hull behind it. A PCSX2 capture
    /// of the original shows
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
    /// [`oag_fx::exhaust::BLEND`], and it is what the **PSP** plume
    /// already draws with - its batches carry `0x200` and
    /// `TransparentPipelines::select` routes them there. So this puts the two
    /// discs' plumes on one blend rather than giving them two.
    ///
    /// A PSP plume never reaches this method's opaque or cutout lists, both
    /// being empty there, so calling it for both titles changes nothing on
    /// PSP.
    pub(crate) fn draw_additive(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
    ) -> SceneStats {
        self.draw_every_list(pass, block, &self.additive_pipeline)
    }

    /// Draws the absorb overlay: every draw of every list through the
    /// pipelines built with this drawable's own blend, which for the overlay
    /// is [`oag_fx::hull_overlay::BLEND`] rather than the fixed additive
    /// equation, so the glow mask takes the stencil's full value.
    pub(crate) fn draw_overlay(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
    ) -> SceneStats {
        self.draw_every_list(pass, block, &self.blend_pipeline)
    }

    /// Every draw of the opaque, cutout and transparent lists, each through
    /// `pipelines`, picked by the draw's own two-sidedness.
    pub(super) fn draw_every_list(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
        pipelines: &[wgpu::RenderPipeline; 2],
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        let mut binds = oag_gpu::perfprobe::Binds::default();
        let mut last_bound: Option<usize> = None;
        if self.model.indices.is_empty() {
            return stats;
        }
        pass.set_bind_group(0, &self.uniform_bind, &self.uniform_offset(block));
        pass.set_bind_group(2, &self.fog_bind, &self.scene_offset(block));
        pass.set_bind_group(3, &self.anim_bind, &[]);
        self.bind_arrays(pass);
        let mut cursor = self.geometry.cursor(pass);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current: Option<&wgpu::RenderPipeline> = None;
        let shows = self.shows(block);
        for draw in self
            .model
            .draws
            .iter()
            .chain(&self.model.alpha_tested_draws)
            .chain(&self.model.transparent_draws)
            .filter(|draw| shows.draw(draw))
        {
            // Two-sidedness stays the batch's own: the plume's `0x20` is set
            // on both discs, so nothing here is culled, and a future model
            // that sets it differently should still be obeyed.
            let pipeline = &pipelines[usize::from(draw.culled)];
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                oag_gpu::perfprobe::pipeline_set();
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            // The probe counts group 1 switches, which the arrays do not make.
            if self.bindless.is_none() {
                binds.record(slot);
            }
            // Elided when unchanged, as in `draw`.
            self.bind_slot(pass, &mut last_bound, slot);
            cursor.draw(pass, &draw.range);
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
    pub(crate) fn write_overlay(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
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
        self.geometry.write(queue, 0, scratch);
    }

    /// Uploads this model's vertices with texture coordinates generated for a
    /// craft posed by `ship` - [`oag_render::shine::write`], the environment-mapped
    /// shine pass, whose coordinates follow the craft's rotation and so cannot be
    /// baked, with the airbrake flaps at `flaps` radians (the hull's own base draw
    /// swings the same two), every vertex colour multiplied by `colour`
    /// ([`crate::Race::ship_shine_colour`]). From the model's own vertices every
    /// time, like [`Self::write_overlay`].
    pub(crate) fn write_environment_map(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        ship: oag_core::math::Mat4,
        flaps: [f32; 2],
        colour: [f32; 3],
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        // The vertices are a function of the pose, the flaps and the colour alone, and nothing
        // else writes this buffer: a craft that has not moved (a paused race, the grid) has
        // them already.
        let mut pose = [0_u32; 21];
        for (bits, value) in pose
            .iter_mut()
            .zip(ship.to_cols_array().into_iter().chain(flaps).chain(colour))
        {
            *bits = value.to_bits();
        }
        if let Ok(mut last) = self.shine_pose.lock() {
            if *last == Some(pose) {
                return;
            }
            *last = Some(pose);
        }
        oag_render::shine::write(&self.model, scratch, ship, flaps, colour);
        self.geometry.write(queue, 0, scratch);
    }

    /// Uploads texture coordinates generated for the camera `view` -
    /// [`oag_render::shine::TrackNormals::write_view_spans`], a circuit's extra pass, whose
    /// coordinates follow the camera and so cannot be baked. Into the streamed
    /// coordinate buffer when this drawable has one, which is the eight bytes
    /// a vertex that change; the whole vertex is never rewritten. `normals` is the
    /// frame's turned normals, shared by every camera drawn at `seconds`.
    ///
    /// `spans` are the vertices this view draws, sorted. One write from the first to the last:
    /// a write is a staging buffer, and what lies between is a coordinate no draw of this view
    /// reads, so it may hold another view's.
    pub(crate) fn write_view_map(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        view: oag_core::math::Mat4,
        seconds: f32,
        normals: &mut oag_render::shine::TrackNormals,
        scratch: &mut Vec<[f32; 2]>,
        spans: &[std::ops::Range<usize>],
    ) {
        if self.geometry.texcoords().is_none() {
            return;
        }
        let (Some(first), Some(last)) = (spans.first(), spans.last()) else {
            return;
        };
        normals.write_view_spans(&self.model, scratch, view, seconds, spans);
        self.geometry
            .write_texcoords(queue, first.start as u32, &scratch[first.start..last.end]);
    }

    /// The indices into this pass's draws of those view `block` draws: each that redraws a
    /// circuit draw `sources[i]` of `track` the view shows - its level-of-detail child on, its
    /// section allowed, its bound in the frustum. [`Self::draw_track_shine`] draws exactly
    /// these, and the view's coordinates are written for exactly these.
    pub(crate) fn track_shine_shown<'a>(
        &'a self,
        block: ViewBlock,
        track: &'a Drawable,
        sources: &'a [usize],
        cull: TrackCull<'a>,
    ) -> impl Iterator<Item = usize> + 'a {
        let opaque: &[u64] = cull.sections.map_or(&[], |s| &s.opaque[..]);
        let shows = track.shows(block);
        sources
            .iter()
            .enumerate()
            .filter_map(move |(index, &source)| {
                let circuit = &track.model.draws[source];
                let shown = shows.draw(circuit)
                    && oag_render::pvs::visible(
                        circuit,
                        DrawSections::at(opaque, source),
                        cull.set,
                        cull.chunks,
                        cull.frustum,
                    );
                shown.then_some(index)
            })
    }

    /// Draws a circuit's extra pass: the draws [`Self::track_shine_shown`] answers, through the
    /// pipelines built with this drawable's own blend.
    pub(crate) fn draw_track_shine(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
        track: &Drawable,
        sources: &[usize],
        cull: TrackCull<'_>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        if self.model.indices.is_empty() {
            return stats;
        }
        let mut last_bound: Option<usize> = None;
        pass.set_bind_group(0, &self.uniform_bind, &self.uniform_offset(block));
        pass.set_bind_group(2, &self.fog_bind, &self.scene_offset(block));
        pass.set_bind_group(3, &self.anim_bind, &[]);
        self.bind_arrays(pass);
        let mut cursor = self.geometry.cursor(pass);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current: Option<&wgpu::RenderPipeline> = None;
        for index in self.track_shine_shown(block, track, sources, cull) {
            let draw = &self.model.draws[index];
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
            self.bind_slot(pass, &mut last_bound, slot);
            cursor.draw(pass, &draw.range);
        }
        stats
    }
}
