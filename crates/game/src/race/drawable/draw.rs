//! [`Drawable::draw`] and the list-restricted [`Drawable::draw_lists`] the
//! circuit draws through. Split out of `drawable.rs` under the 1,000-line rule
//! in `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Which of a drawable's lists one [`Drawable::draw_lists`] call draws.
#[derive(Debug, Clone, Copy)]
pub(in crate::race) enum Lists<'a> {
    /// Opaque, cutout, then blended - [`Drawable::draw`].
    All,
    /// Opaque and cutout: everything that writes depth. `order`, when given,
    /// is the order the opaque list draws in, as `(key, index)` pairs into
    /// [`Drawable::opaque_draws`] - otherwise the model's own. Only the
    /// opaque list: the cutout list's pipeline switches and the blended
    /// list's order both matter.
    Solid { order: Option<&'a [(f32, u32)]> },
    /// The blended list and the glow stamps over it.
    Blended,
}

impl Drawable {
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
    pub(in crate::race) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        self.draw_lists(pass, sections, set, chunks, frustum, Lists::All)
    }

    /// [`Self::draw`], restricted to `lists`. The circuit draws its solid
    /// lists and its blended one in two calls so the sky can go between them -
    /// see `Scene::draw_track`.
    pub(in crate::race) fn draw_lists(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
        lists: Lists<'_>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        let none: &[DrawCall] = &[];
        let solid = !matches!(lists, Lists::Blended);
        let blended = !matches!(lists, Lists::Solid { .. });
        let order = match lists {
            Lists::Solid { order } => order,
            Lists::All | Lists::Blended => None,
        };
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
        let opaque_draws = if solid { &self.model.draws[..] } else { none };
        let ordered = order.map(|order| {
            order
                .iter()
                .filter_map(|&(_, i)| Some((i as usize, opaque_draws.get(i as usize)?)))
        });
        let natural = order.is_none().then(|| opaque_draws.iter().enumerate());
        for (index, draw) in ordered
            .into_iter()
            .flatten()
            .chain(natural.into_iter().flatten())
        {
            // A tier the switch has off is not culled, it is not there.
            if !self.lod_shows(draw) {
                continue;
            }
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

        // Second pipeline group, same pass: alpha-tested batches, cutout.
        //
        // **The alpha-test reference is the batch's own, not the model's.**
        // `Gfx_BuildBatchStateList` picks between `0x7f`, `0x10` and `0` from
        // two of the batch's flag bits, and a circuit mixes them, so the
        // pipeline is chosen per draw call the same way the blend class is
        // below. This crate drew every cutout at `1/255` until that was
        // recovered, which painted a 3/255 texel solid rather than discarding
        // it. See `oag_vex::vex::Batch::alpha_test_reference` and
        // `mesh_render::cutout`.
        let cutouts = mesh_render::CutoutPipelines {
            default: &self.alpha_test_pipeline,
            by_reference: &self.cutout_pipelines,
        };
        let mut cutout_set: Option<&wgpu::RenderPipeline> = None;
        for (index, draw) in (if solid {
            &self.model.alpha_tested_draws[..]
        } else {
            none
        })
        .iter()
        .enumerate()
        {
            if !self.lod_shows(draw) {
                continue;
            }
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
            // Switched only when the reference changes, so a model naming one
            // (or none) still pays for a single `set_pipeline`.
            let cutout = cutouts.select(draw);
            if !cutout_set.is_some_and(|set| std::ptr::eq(set, cutout)) {
                oag_render::perfprobe::pipeline_set();
                pass.set_pipeline(cutout);
                cutout_set = Some(cutout);
            }
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
        // recovered. See `oag_vex::vex::Batch::blend_class` and
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
        if !blended {
            return stats;
        }
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !self.lod_shows(draw) {
                continue;
            }
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
        self.draw_stamps(pass, transparent, set, chunks, frustum);
        stats
    }
}
