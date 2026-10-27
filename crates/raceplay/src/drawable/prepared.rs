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
        Some(prepared)
    }
}
