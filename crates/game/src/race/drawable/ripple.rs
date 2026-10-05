//! The Quake's road ripple on one drawable: which of its vertices move this
//! frame, uploaded - see [`oag_render::ripple`] for what moves and why.

use oag_mesh::mesh::GpuVertex;
use oag_render::ripple::{Ripple, Wave};

use super::Drawable;

impl Drawable {
    /// Gives this drawable the road spans a Quake ripples through it.
    pub(in crate::race) fn set_ripple(&mut self, ripple: Option<Ripple>) {
        *self.ripple.get_mut() = ripple;
    }

    /// Moves this drawable's rippling vertices to `wave` - `None` once no
    /// Quake is in flight, which settles any it moved last frame back to where
    /// the artists put them. A no-op on a drawable nothing ripples.
    ///
    /// `scratch` is the caller's, refilled per batch - see `Scene::scratch`.
    pub(in crate::race) fn write_ripple(
        &self,
        queue: &wgpu::Queue,
        wave: Option<Wave>,
        scratch: &mut Vec<GpuVertex>,
    ) {
        let mut slot = self.ripple.borrow_mut();
        let Some(ripple) = slot.as_mut() else {
            return;
        };
        let stride = std::mem::size_of::<GpuVertex>() as u64;
        ripple.update(wave, &self.model.vertices, scratch, |first, vertices| {
            queue.write_buffer(
                &self.vertices,
                u64::from(first) * stride,
                bytemuck::cast_slice(vertices),
            );
        });
    }
}
