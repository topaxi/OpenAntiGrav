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
    ///
    /// `prepared`, when given, is what [`Drawable::prepare_solid`] already gathered for this
    /// view: the draw uses its lists instead of culling again, and its depth half goes out
    /// as one indirect multi-draw where the drawable has [`Drawable::draws_indirect`].
    Solid {
        order: Option<&'a [(f32, u32)]>,
        prepared: Option<&'a Prepared>,
    },
    /// The blended list and the glow stamps over it. `depth_plane`, when given, is
    /// the fourth row of the view's view-projection matrix and puts list-B meshes
    /// in the original's back-to-front order ([`oag_mesh::mesh::Model::blended_order`]);
    /// without it the list keeps its build order.
    Blended { depth_plane: Option<[f32; 4]> },
}

/// [`Lists::Solid`]'s `order`.
pub(super) type SolidOrder<'a> = Option<&'a [(f32, u32)]>;

/// What [`Drawable::draw_lists_into`] culls with: the placement table, the two PVS tiers and the
/// frustum.
pub(super) type Culling<'a> = (
    Option<&'a DrawSections>,
    Option<&'a VisibleSet>,
    Option<&'a ChunkSet>,
    Option<&'a Frustum>,
);

/// The scratch [`Drawable::draw_lists`] gathers its opaque draws into, one per thread: the
/// views of a split record on threads of their own, and two of them can draw one drawable at
/// once.
#[derive(Debug, Default)]
pub(crate) struct Shown {
    /// What this frame shows, in the model's order.
    pub(super) natural: Vec<u32>,
    /// The same, in the prepass's order.
    pub(super) depth: Vec<u32>,
    /// Per draw, whether it is in `natural`.
    pub(super) flags: Vec<bool>,
}

thread_local! {
    static SHOWN: std::cell::RefCell<Shown> = std::cell::RefCell::default();
}

impl Drawable {
    /// Appends one line per submitted draw to the file `OAG_DRAW_DUMP` names:
    /// the list, the diffuse texture's label, the index count and the draw's
    /// bounds. A diagnostic for comparing a frame's draw list against a
    /// capture of the original (`docs/reverse-engineering/rpcs3-capture.md`);
    /// off, it is one environment read per draw.
    pub(super) fn trace_draw(&self, list: &str, draw: &DrawCall) {
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
        block: ViewBlock,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        self.draw_lists(pass, block, sections, set, chunks, frustum, Lists::All)
    }

    /// [`Self::draw`], restricted to `lists`. The circuit draws its solid
    /// lists and its blended one in two calls so the sky can go between them -
    /// see `Scene::draw_track`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_lists(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
        lists: Lists<'_>,
    ) -> SceneStats {
        let solid = !matches!(lists, Lists::Blended { .. });
        let blended = !matches!(lists, Lists::Solid { .. });
        let depth_plane = match lists {
            Lists::Blended { depth_plane } => depth_plane,
            Lists::All | Lists::Solid { .. } => None,
        };
        let (order, prepared) = match lists {
            Lists::Solid { order, prepared } => (order, prepared),
            Lists::All | Lists::Blended { .. } => (None, None),
        };
        if self.model.indices.is_empty() {
            return SceneStats::default();
        }
        SHOWN.with_borrow_mut(|shown| {
            self.draw_lists_into(
                pass,
                block,
                sections,
                set,
                chunks,
                frustum,
                (order, depth_plane),
                (solid, blended),
                prepared,
                shown,
            )
        })
    }

    /// [`Self::draw_lists`], with this thread's scratch.
    #[allow(clippy::too_many_arguments)]
    fn draw_lists_into(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        block: ViewBlock,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
        (order, depth_plane): (SolidOrder<'_>, Option<[f32; 4]>),
        (solid, blended): (bool, bool),
        prepared: Option<&Prepared>,
        shown: &mut Shown,
    ) -> SceneStats {
        let none: &[DrawCall] = &[];
        let mut binds = oag_gpu::perfprobe::Binds::default();
        let mut last_bound: Option<usize> = None;
        // A model with no placement table has every draw call unplaced, which
        // the first tier always allows. That is the ship and the collision
        // overlay, and any track whose sections did not decode.
        let empty: &[u64] = &[];
        let (alpha_tested, transparent) = match sections {
            Some(s) => (&s.alpha_tested[..], &s.transparent[..]),
            None => (empty, empty),
        };
        let shows = self.shows(block);
        // The draws this frame shows, gathered first because a prepass walks them twice -
        // the second time backwards: what `prepared` already holds, or gathered here.
        let (lists, mut stats, indirect) = match prepared.filter(|_| solid) {
            Some(prepared) => (&prepared.shown, prepared.stats, prepared.indirect),
            None => {
                let culling = (sections, set, chunks, frustum);
                let stats = self.gather_opaque(&shows, culling, order, solid, shown);
                (&*shown, stats, None)
            }
        };
        oag_gpu::perfprobe::pipeline_set();
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &self.uniform_offset(block));
        // Bound once for the whole drawable: fog is per-frame, not per draw call,
        // and group 2 survives the `set_pipeline` calls below.
        pass.set_bind_group(2, &self.fog_bind, &self.scene_offset(block));
        // Same reasoning as fog: the table is per frame, not per draw call.
        pass.set_bind_group(3, &self.anim_bind, &[]);
        self.bind_materials(pass, &mut last_bound);
        self.bind_vertices(pass);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        let opaque_draws = if solid { &self.model.draws[..] } else { none };
        let Shown {
            natural: shown,
            depth: depth_order,
            ..
        } = lists;
        // `reversed`: draw from `reversed_indices`, where a range `s..e` of
        // the model's own sits at `n - e..n - s`.
        let index_count = self.model.indices.len() as u32;
        // The depth half of a prepass reads no texture (`fs_depth_only` returns a constant
        // under an empty write mask), so group 1 is bound once, to the white fallback, and its
        // draws go out without switching it: half of HD's circuit binds, and what the wgpu
        // encoder validates at `finish`. The shade half binds as usual from this state.
        // With the arrays bound ([`Self::bind_materials`]) there is nothing to bind.
        if self.bindless.is_none() && self.prepass.is_some() && !shown.is_empty() {
            pass.set_bind_group(1, &self.textures[0], &[]);
            last_bound = Some(0);
        }
        let mut draw_one = |pass: &mut wgpu::RenderPass<'_>, index: u32, reversed: bool| {
            let draw = &opaque_draws[index as usize];
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            // The probe counts group 1 switches, which the arrays do not make.
            if self.bindless.is_none() {
                binds.record(slot);
            }
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
            self.bind_slot(pass, &mut last_bound, slot);
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
                match (&self.indirect, indirect) {
                    // The same draws in the same order, from the list `prepare_solid` wrote.
                    (Some(args), Some(count)) => {
                        oag_gpu::perfprobe::indirect_draws(count);
                        pass.multi_draw_indexed_indirect(
                            args.buffer(),
                            args.offset(block.index()),
                            count,
                        );
                    }
                    _ => {
                        for &index in depth_order.iter() {
                            let draw = &opaque_draws[index as usize];
                            pass.draw_indexed(draw.range.clone(), 0, 0..1);
                        }
                    }
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
            if !shows.draw(draw) {
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
            // The probe counts group 1 switches, which the arrays do not make.
            if self.bindless.is_none() {
                binds.record(slot);
            }
            // Elided when unchanged, as above.
            self.bind_slot(pass, &mut last_bound, slot);
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
        // The original's order for list B: far to near by the mesh's view depth,
        // which depends on this view - `Model::blended_order`.
        let blend_order = depth_plane.and_then(|plane| self.model.blended_order(plane));
        let blend_order = blend_order.as_deref();
        for index in (0..self.model.transparent_draws.len())
            .map(|at| blend_order.map_or(at, |order| order[at] as usize))
        {
            let draw = &self.model.transparent_draws[index];
            if !shows.draw(draw) {
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
            // The probe counts group 1 switches, which the arrays do not make.
            if self.bindless.is_none() {
                binds.record(slot);
            }
            // Elided when unchanged, as above.
            self.bind_slot(pass, &mut last_bound, slot);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        self.draw_stamps(
            pass,
            &shows,
            transparent,
            blend_order,
            (set, chunks, frustum),
        );
        stats
    }
}
