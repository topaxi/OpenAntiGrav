//! [`Drawable::draw`] and the list-restricted [`Drawable::draw_lists`] the
//! circuit draws through. Split out of `drawable.rs` under the 1,000-line rule
//! in `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Which of a drawable's lists one [`Drawable::draw_lists`] call draws.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Lists<'a> {
    /// Opaque, cutout, then blended - [`Drawable::draw`].
    All,
    /// Opaque and cutout: everything that writes depth. `order`, when given,
    /// is the order a drawable with a depth prepass lays its opaque depth
    /// down in, as `(key, index)` pairs into [`Drawable::opaque_draws`]. The
    /// shading always keeps the model's own order, which is what keeps the
    /// picture the original's; without a prepass `order` is ignored.
    Solid { order: Option<&'a [(f32, u32)]> },
    /// The blended list and the glow stamps over it.
    Blended,
}

/// The scratch [`Drawable::draw_lists`] gathers its opaque draws into.
#[derive(Debug, Default)]
pub(crate) struct Shown {
    /// What this frame shows, in the model's order.
    natural: Vec<u32>,
    /// The same, in the prepass's order.
    depth: Vec<u32>,
    /// Per draw, whether it is in `natural`.
    flags: Vec<bool>,
}

impl Drawable {
    /// Appends one line per submitted draw to the file `OAG_DRAW_DUMP` names:
    /// the list, the diffuse texture's label, the index count and the draw's
    /// bounds. A diagnostic for comparing a frame's draw list against a
    /// capture of the original (`docs/reverse-engineering/rpcs3-capture.md`);
    /// off, it is one environment read per draw.
    fn trace_draw(&self, list: &str, draw: &DrawCall) {
        use std::io::Write;
        static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
        let Some(path) = PATH.get_or_init(|| std::env::var_os("OAG_DRAW_DUMP").map(Into::into))
        else {
            return;
        };
        let label = draw
            .texture
            .and_then(|t| self.model.textures.get(t))
            .and_then(Option::as_ref)
            .map_or("-", |t| t.label.as_str());
        let c = draw.bounds.centre;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(
                file,
                "{list}\t{label}\t{}\t{:.1}\t{:.1}\t{:.1}\t{:.0}\t{}",
                draw.range.end - draw.range.start,
                c[0],
                c[1],
                c[2],
                draw.bounds.radius,
                draw.chunk.map_or(-1, i64::from)
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
    pub(crate) fn draw(
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
    pub(crate) fn draw_lists(
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
        let mut binds = oag_gpu::perfprobe::Binds::default();
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
        oag_gpu::perfprobe::pipeline_set();
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        // Bound once for the whole drawable: fog is per-frame, not per draw call,
        // and group 2 survives the `set_pipeline` calls below.
        pass.set_bind_group(2, &self.fog_bind, &[]);
        // Same reasoning as fog: the table is per frame, not per draw call.
        pass.set_bind_group(3, &self.anim_bind, &[]);
        self.bind_vertices(pass);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        let opaque_draws = if solid { &self.model.draws[..] } else { none };
        // The draws this frame shows, in the model's order, gathered first
        // because a prepass walks them twice - the second time backwards.
        let mut shown = self.shown.borrow_mut();
        let Shown {
            natural: shown,
            depth: depth_order,
            flags,
        } = &mut *shown;
        shown.clear();
        for (index, draw) in opaque_draws.iter().enumerate() {
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
            self.trace_draw("opaque", draw);
            shown.push(index as u32);
        }
        // The prepass's own order: `order` where given, filtered to what is
        // shown. Depth under `Less` is the nearest surface whatever the order
        // it arrives in, so nearest first only lets the depth test reject
        // sooner - it cannot change a pixel.
        depth_order.clear();
        if let Some(order) = order.filter(|_| self.prepass.is_some()) {
            flags.clear();
            flags.resize(opaque_draws.len(), false);
            for &index in shown.iter() {
                flags[index as usize] = true;
            }
            depth_order.extend(
                order
                    .iter()
                    .map(|&(_, index)| index)
                    .filter(|&index| flags.get(index as usize).copied().unwrap_or(false)),
            );
        } else {
            depth_order.extend_from_slice(shown);
        }
        // `reversed`: draw from `reversed_indices`, where a range `s..e` of
        // the model's own sits at `n - e..n - s`.
        let index_count = self.model.indices.len() as u32;
        let mut draw_one = |pass: &mut wgpu::RenderPass<'_>, index: u32, reversed: bool| {
            let draw = &opaque_draws[index as usize];
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
            let range = if reversed {
                index_count - draw.range.end..index_count - draw.range.start
            } else {
                draw.range.clone()
            };
            pass.draw_indexed(range, 0, 0..1);
        };
        match &self.prepass {
            // Depth first, then the shading against it, **backwards** - see
            // `mesh_render::Prepass` for why the reversal keeps every
            // coplanar tie where the plain pipeline puts it.
            //
            // Backwards by triangle as well as by draw: two coplanar
            // triangles in one draw tie the same way two draws do, so the
            // shaded half reads `reversed_indices`, every triangle in reverse
            // order with its own vertices - and so its provoking vertex -
            // untouched.
            Some(prepass) if !shown.is_empty() => {
                pass.set_pipeline(&prepass.depth);
                for &index in depth_order.iter() {
                    draw_one(pass, index, false);
                }
                pass.set_pipeline(&prepass.shade);
                let reversed = self.reversed_indices.as_ref();
                if let Some(reversed) = reversed {
                    pass.set_index_buffer(reversed.slice(..), wgpu::IndexFormat::Uint32);
                }
                for &index in shown.iter().rev() {
                    draw_one(pass, index, reversed.is_some());
                }
                if reversed.is_some() {
                    pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                }
            }
            _ => {
                for &index in shown.iter() {
                    draw_one(pass, index, false);
                }
            }
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
            self.trace_draw("cutout", draw);
            // Switched only when the reference changes, so a model naming one
            // (or none) still pays for a single `set_pipeline`.
            let cutout = cutouts.select(draw);
            if !cutout_set.is_some_and(|set| std::ptr::eq(set, cutout)) {
                oag_gpu::perfprobe::pipeline_set();
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
                oag_gpu::perfprobe::pipeline_set();
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            self.trace_draw("blended", draw);
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
