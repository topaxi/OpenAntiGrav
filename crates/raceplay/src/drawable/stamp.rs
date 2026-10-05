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
    /// the way [`Self::draw`] just did, through the same LOD, section and
    /// frustum tests, so a batch the colour pass dropped stamps nothing.
    ///
    /// Every draw is submitted, not only the glow ones: `Model` carries no
    /// per-draw flag for it, and a batch without the glow bits has vertex glow
    /// `0`, which `fs_main_stamp` discards whole. A model that does not stamp
    /// has no pipeline and returns before binding anything.
    pub(super) fn draw_stamps(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        transparent: &[u64],
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) {
        let Some(stamp) = &self.stamp_pipeline else {
            return;
        };
        let mut current: Option<&wgpu::RenderPipeline> = None;
        let mut last_bound: Option<usize> = None;
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !self.lod_shows(draw)
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
            if last_bound != Some(slot) {
                pass.set_bind_group(1, &self.textures[slot], &[]);
                last_bound = Some(slot);
            }
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
}
