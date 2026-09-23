//! The absorb hull overlay's per-frame vertex write and its draw - see
//! `oag_render::hull_overlay` and `race::scene::absorb_overlay` - plus the
//! every-list draw loop it shares with [`Drawable::draw_additive`].

use oag_render::mesh;

use super::{Drawable, SceneStats};

impl Drawable {
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
}
