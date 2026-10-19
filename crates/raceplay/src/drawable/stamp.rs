//! The blended batches' glow-mask stamp, drawn over the colour they just drew.

use super::{ChunkSet, DrawSections, Drawable, Frustum, VisibleSet};
use crate::visibility::visible;

impl Drawable {
    /// Stamps every visible blended batch's glow byte into the mask.
    ///
    /// **The original does this inside the blended draw**, with the stencil
    /// left on under the blend, so a blended batch with the glow bits writes
    /// its texture's byte wherever it passes the alpha and depth tests. This
    /// is the second submission that stands in for that - see
    /// `mesh_render::GlowMask::Stamped` - and it walks the transparent list
    /// in the order [`Self::draw`] just did (`order`), through the same LOD, section and
    /// frustum tests, so a batch the colour pass dropped stamps nothing.
    ///
    /// Every draw is submitted, not only the glow ones: `Model` carries no
    /// per-draw flag for it, and a batch without the glow bits has vertex glow
    /// `0`, which `fs_main_stamp` discards whole. A model that does not stamp
    /// has no pipeline and returns before binding anything.
    pub(super) fn draw_stamps(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        shows: &super::Shows<'_>,
        transparent: &[u64],
        order: Option<&[u32]>,
        (set, chunks, frustum): (Option<&VisibleSet>, Option<&ChunkSet>, Option<&Frustum>),
    ) {
        let Some(stamp) = &self.stamp_pipeline else {
            return;
        };
        let mut current: Option<&wgpu::RenderPipeline> = None;
        let mut last_bound: Option<usize> = None;
        for index in (0..self.model.transparent_draws.len())
            .map(|at| order.map_or(at, |order| order[at] as usize))
        {
            let draw = &self.model.transparent_draws[index];
            if !shows.draw(draw)
                || !visible(
                    draw,
                    DrawSections::at(transparent, index),
                    set,
                    chunks,
                    frustum,
                )
            {
                continue;
            }
            let pipeline = stamp.select(draw);
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                oag_gpu::perfprobe::pipeline_set();
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            let slot = draw
                .texture
                .map_or(0, |t| t + 1)
                .min(self.textures.len() - 1);
            self.bind_slot(pass, &mut last_bound, slot);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
}
