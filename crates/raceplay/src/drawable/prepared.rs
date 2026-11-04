//! The opaque list's culling, shared by [`Drawable::draw_lists`] and
//! [`Drawable::prepare_solid`], which runs it before the view's pass so the prepass's depth half
//! can go out as one indirect multi-draw (`oag_mesh::mesh_render::indirect`).

use super::draw::{Culling, Shown, SolidOrder};
use super::*;

/// One view's opaque list, gathered before its pass by [`Drawable::prepare_solid`].
#[derive(Debug, Default)]
pub(crate) struct Prepared {
    pub(super) shown: Shown,
    /// What the gathering counted: the opaque list's draws, culled draws and triangles.
    pub(super) stats: SceneStats,
    /// The view's indirect list, `None` when none was written.
    pub(super) indirect: Option<IndirectRuns>,
    /// The view's colour list for the shaded half, in draw order, `None` when none was
    /// written (`Drawable::prepare_colour`).
    pub(super) colour: Option<Vec<ColourRun>>,
}

/// How one view's indirect list splits: `plain` two-sided draws first, then `culled`
/// back-face-culled ones, which need the culling twin's depth pipeline.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct IndirectRuns {
    pub(super) plain: u32,
    pub(super) culled: u32,
}

thread_local! {
    static PACKED: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A drawable's per-view indirect lists: the prepass depth half's where the device takes the
/// path (`mesh_render::indirect::wanted`), and the shaded half's under `OAG_INDIRECT=colour`
/// where the per-draw-material pipelines were built (`mesh_render::indirect::colour`).
pub(super) fn indirect_lists(
    device: &wgpu::Device,
    model: &oag_mesh::mesh::Model,
    prepass: Option<&mesh_render::Prepass>,
    bindless: bool,
) -> (
    Option<mesh_render::indirect::Args>,
    Option<mesh_render::indirect::Args>,
) {
    let list = |label: &str, half: &str| {
        log::info!("{}: prepass {half} as indirect multi-draws", model.label);
        mesh_render::indirect::Args::new(device, label, SceneBlocks::VIEWS, model.draws.len())
    };
    let depth = prepass
        .filter(|_| mesh_render::indirect::wanted(device))
        .map(|_| list("prepass depth draws", "depth half"));
    let colour = prepass
        .filter(|p| depth.is_some() && bindless && p.shade_indirect.is_some())
        .map(|_| list("prepass shade draws", "shade half, material per draw,"));
    (depth, colour)
}

impl Drawable {
    /// Whether this drawable's prepass depth half draws indirect: it has a prepass and the
    /// device takes the path (`mesh_render::indirect::wanted`).
    pub(crate) fn draws_indirect(&self) -> bool {
        self.indirect.is_some()
    }

    /// Gathers the opaque list `view` shows into `shown`, in the model's order and in
    /// `order`'s (the prepass's), and counts it. `solid` false gathers nothing.
    pub(super) fn gather_opaque(
        &self,
        shows: &Shows<'_>,
        (sections, set, chunks, frustum): Culling<'_>,
        order: SolidOrder<'_>,
        solid: bool,
        shown: &mut Shown,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        let opaque: &[u64] = sections.map_or(&[], |s| &s.opaque[..]);
        let opaque_draws: &[DrawCall] = if solid { &self.model.draws[..] } else { &[] };
        let Shown {
            natural: shown,
            depth: depth_order,
            flags,
        } = shown;
        shown.clear();
        for (index, draw) in opaque_draws.iter().enumerate() {
            // A tier the switch has off is not culled, it is not there.
            if !shows.draw(draw) {
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
        if let Some(order) = order.filter(|_| self.solid(false).1.is_some()) {
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
        stats
    }

    /// Gathers the opaque list `view` shows before its pass and, where this drawable
    /// [`Self::draws_indirect`], writes the prepass's depth half as `view`'s indirect list
    /// through `writes`. `None` for a drawable that does not draw indirect: its draw gathers
    /// for itself, as it always has.
    ///
    /// **Call it after everything that changes what the view shows** (the LOD switch, the
    /// hidden keys): the draw trusts these lists for both halves.
    pub(crate) fn prepare_solid(
        &self,
        writes: &dyn oag_gpu::writes::Writes,
        view: ViewBlock,
        culling: Culling<'_>,
        order: SolidOrder<'_>,
    ) -> Option<Prepared> {
        let args = self.indirect.as_ref()?;
        let mut prepared = Prepared::default();
        if self.model.indices.is_empty() {
            return Some(prepared);
        }
        let shows = self.shows(view);
        prepared.stats = self.gather_opaque(&shows, culling, order, true, &mut prepared.shown);
        let draws = &self.model.draws;
        let depth = &prepared.shown.depth;
        // Two runs, because a multi-draw cannot switch the cull mode inside it: the two-sided
        // draws, then the back-face-culled ones, each in the depth order. Depth under `Less`
        // does not depend on the order it arrives in, so the split changes no pixel.
        let run = |culled: bool| {
            depth
                .iter()
                .map(|&index| &draws[index as usize])
                .filter(move |draw| self.wants_cull(draw) == culled)
                .map(|draw| draw.range.clone())
        };
        let runs = PACKED.with_borrow_mut(|bytes| {
            bytes.clear();
            let runs = IndirectRuns {
                plain: args.pack(run(false), bytes),
                culled: args.pack(run(true), bytes),
            };
            args.write(writes, view.index(), bytes);
            runs
        });
        // A list the buffer could not hold whole is drawn the per-draw way.
        prepared.indirect = ((runs.plain + runs.culled) as usize == depth.len()).then_some(runs);
        prepared.colour = self.prepare_colour(writes, view, &prepared.shown.natural);
        Some(prepared)
    }

    /// Writes the prepass's shaded half as `view`'s colour list, where this drawable draws it
    /// indirect (`mesh_render::indirect::colour`): `shown` backwards, by triangle as well
    /// (`reversed_indices`), each draw's material slot as its first instance. Split into
    /// runs only where the cull mode changes, so the order, which decides every coplanar
    /// tie under `LessEqual`, is the per-draw path's.
    fn prepare_colour(
        &self,
        writes: &dyn oag_gpu::writes::Writes,
        view: ViewBlock,
        shown: &[u32],
    ) -> Option<Vec<ColourRun>> {
        let args = self.indirect_colour.as_ref()?;
        let bindless = self.bindless.as_ref()?;
        let draws = &self.model.draws;
        let index_count = self.model.indices.len() as u32;
        let reversed = self.reversed_indices.is_some();
        let last_slot = self.textures.len().saturating_sub(1);
        let mut runs: Vec<ColourRun> = Vec::new();
        for &index in shown.iter().rev() {
            let culled = self.wants_cull(&draws[index as usize]);
            match runs.last_mut() {
                Some(run) if run.culled == culled => run.count += 1,
                _ => runs.push(ColourRun {
                    culled,
                    first: runs.last().map_or(0, |run| run.first + run.count),
                    count: 1,
                }),
            }
        }
        // Every cull mode a run needs must have its per-draw-material pipeline.
        if runs.iter().any(|run| {
            self.solid(run.culled)
                .1
                .is_none_or(|prepass| prepass.shade_indirect.is_none())
        }) {
            return None;
        }
        let count = PACKED.with_borrow_mut(|bytes| {
            bytes.clear();
            let count = args.pack_with(
                shown.iter().rev().map(|&index| {
                    let draw = &draws[index as usize];
                    let range = if reversed {
                        index_count - draw.range.end..index_count - draw.range.start
                    } else {
                        draw.range.clone()
                    };
                    let slot = draw.texture.map_or(0, |t| t + 1).min(last_slot);
                    (range, bindless.row_index(slot))
                }),
                bytes,
            );
            args.write(writes, view.index(), bytes);
            count
        });
        (count as usize == shown.len()).then_some(runs)
    }
}

/// One run of a view's colour list under one cull mode: `count` draws from entry `first`.
#[derive(Debug, Clone, Copy)]
pub(super) struct ColourRun {
    pub(super) culled: bool,
    pub(super) first: u32,
    pub(super) count: u32,
}
