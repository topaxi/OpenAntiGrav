//! The absorb hull overlay's per-frame vertex write - see
//! `oag_render::hull_overlay` and `race::scene::absorb_overlay`.

use oag_render::mesh;

use super::Drawable;

impl Drawable {
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
